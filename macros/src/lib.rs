extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use std::sync::Mutex;
use std::{collections::HashMap, convert::TryFrom, fmt, fmt::Formatter};
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Error, Fields, Ident, Lit, Meta,
    NestedMeta, Result, Type,
};

const HIDDEN_MARKER: &str = "hidden";
const SKIP_MARKER: &str = "ignore";

lazy_static::lazy_static! {
    /// Hierarchical mapping of metric scopes
    static ref METRIC_TREE: Mutex<MetricTree> = {
        Mutex::new(MetricTree::default())
    };
}

#[proc_macro_derive(Metrics, attributes(metric))]
pub fn derive_metrics(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate(input).unwrap_or_else(|err| err.to_compile_error().into())
}

fn generate(input: DeriveInput) -> Result<TokenStream> {
    let mut tree = METRIC_TREE.lock().unwrap();
    tree.parse_struct(input)?;
    // eprintln!("{:#?}", tree);
    Ok(if tree.is_complete() {
        // eprintln!("### COMPLETE - Generating..");
        tree.generate()
    } else {
        quote! {}
    }
    .into())
}

#[derive(Default, Debug)]
struct MetricTree {
    scopes: HashMap<String, MetricScope>,
    required_scopes: HashSet<String>,
    root_scope: Option<String>,
}

impl MetricTree {
    fn is_complete(&self) -> bool {
        self.root_scope.is_some()
            && self
                .required_scopes
                .iter()
                .all(|name| self.scopes.contains_key(name))
    }

    fn generate_root(&self) -> proc_macro2::TokenStream {
        let root_name = self.root_scope.clone().unwrap();
        let root_catalogue = format_ident!("catalogue_{}", root_name);
        let public_catalogue = quote! {
            pub mod catalogue {
                pub use super::#root_catalogue::*;
            }
        };

        let root_struct = format_ident!("{}", root_name);
        let recorder = quote! {
            impl Recorder for #root_struct {
                // The following are unused in Stats
                fn register_counter(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

                fn register_gauge(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

                fn register_histogram(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

                fn record_histogram(&self, _key: &Key, _value: f64) {}

                fn increment_counter(&self, key: &Key, value: u64) {
                    if let Some(metric) = self.find_counter(key.name()) {
                        metric.increment(value);
                    }
                }

                fn update_gauge(&self, key: &Key, value: GaugeValue) {
                    if let Some(metric) = self.find_gauge(key.name()) {
                        match value {
                            GaugeValue::Increment(val) => metric.increase(val),
                            GaugeValue::Decrement(val) => metric.decrease(val),
                            GaugeValue::Absolute(val) => metric.set(val),
                        }
                    }
                }
            }
        };

        quote! {
            #public_catalogue

            #recorder
        }
    }

    fn generate(&self) -> proc_macro2::TokenStream {
        let root = self.generate_root();
        let scopes = self.scopes.values().map(MetricScope::generate);
        let combined = std::iter::once(root).chain(scopes);

        quote! {
            #(#combined)*
        }
    }

    fn parse_struct(&mut self, input: DeriveInput) -> Result<()> {
        let struct_data = match &input.data {
            Data::Struct(data) => Struct::from_syn(&input, data),
            Data::Enum(_) | Data::Union(_) => Err(Error::new_spanned(
                &input,
                "Metrics are only supported as structs",
            )),
        }?;

        // TODO: Detection via 'root' attribute
        let is_root_scope = struct_data.ident == "Test";
        if is_root_scope && self.root_scope.is_some() {
            return Err(Error::new_spanned(
                &input,
                format!(
                    "Duplicate root attribute previously detected on {}",
                    self.root_scope.as_ref().unwrap()
                ),
            ));
        }
        self.root_scope.get_or_insert(struct_data.ident.to_string());

        let mut metrics = vec![];
        let mut other_fields = HashMap::new();
        let mut sub_metrics = HashMap::new();
        for field in &struct_data.fields {
            if field.attributes.is_metric {
                let name = field
                    .get_metric()
                    .ok_or_else(|| Error::new_spanned(&input, "No metric name"))?;
                let path = if let Type::Path(path) = field.ty {
                    path
                } else {
                    return Err(Error::new_spanned(&input, "Invalid type for metrics"));
                };

                let ident = path
                    .path
                    .get_ident()
                    .ok_or_else(|| Error::new_spanned(&input, "Field needs to be a named type"))?;

                match MetricType::try_from(ident) {
                    Ok(metric_type) => metrics.push(MetricInstance {
                        key: name.to_ascii_uppercase(),
                        name: name.clone(),
                        instance: field
                            .original
                            .ident
                            .as_ref()
                            .ok_or_else(|| Error::new_spanned(field.original, "No field identity"))?
                            .to_string(),
                        metric_type,
                        hidden: field.attributes.hidden,
                    }),
                    Err(_err) => {
                        // Should be a subtype
                        let orig = field.original;
                        let field_type = if let Type::Path(path) = &orig.ty {
                            path.path
                                .get_ident()
                                .ok_or_else(|| Error::new_spanned(&input, "Not valid field type"))?
                                .clone()
                        } else {
                            return Err(Error::new_spanned(&input, "Only structs are supported"));
                        };
                        sub_metrics.insert(
                            orig.ident.as_ref().expect("No identity").to_string(),
                            SubMetric {
                                ident: field_type.to_string(),
                                hidden: field.attributes.hidden,
                            },
                        );
                    }
                }
            } else {
                let orig = field.original;
                let field_type = if let Type::Path(path) = &orig.ty {
                    path.path
                        .get_ident()
                        .ok_or_else(|| Error::new_spanned(&input, "Not valid field type"))?
                        .clone()
                } else {
                    return Err(Error::new_spanned(&input, "Only structs are supported"));
                };
                other_fields.insert(
                    orig.ident.as_ref().expect("No identity").to_string(),
                    field_type.to_string(),
                );
            }
        }
        let scope = MetricScope {
            struct_name: struct_data.ident.to_string(),
            metrics,
            sub_metrics,
            other_fields,
        };

        self.required_scopes.insert(scope.struct_name.clone());
        self.required_scopes
            .extend(scope.sub_metrics.iter().map(|(_, m)| &m.ident).cloned());

        self.scopes.insert(struct_data.ident.to_string(), scope);

        Ok(())
    }
}

fn match_instance(metric: &MetricInstance) -> proc_macro2::TokenStream {
    let name = format_ident!("{}", metric.name);
    let instance = format_ident!("{}", metric.instance);
    let quoted_name = name.to_string();
    quote! { #quoted_name => Some(&self.#instance) }
}

fn match_metric_names<'a>(
    instances: &'a [MetricInstance],
    metric_type: MetricType,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    let f_ident = format_ident!("{}", "_");
    let fallthrough = quote! { #f_ident => None };
    instances
        .iter()
        .filter(|m| !m.hidden)
        .filter(move |m| m.metric_type == metric_type)
        .map(match_instance)
        .chain(std::iter::once(fallthrough))
}

fn default_init((k, v): (impl AsRef<str>, impl AsRef<str>)) -> proc_macro2::TokenStream {
    let k = format_ident!("{}", k.as_ref());
    let v = format_ident!("{}", v.as_ref());
    quote! { #k: #v::new() }
}

#[derive(Debug)]
struct SubMetric {
    ident: String,
    hidden: bool,
}

impl AsRef<str> for SubMetric {
    fn as_ref(&self) -> &str {
        &self.ident
    }
}

#[derive(Debug)]
struct MetricScope {
    struct_name: String,
    metrics: Vec<MetricInstance>,
    sub_metrics: HashMap<String, SubMetric>,
    other_fields: HashMap<String, String>,
}

impl MetricScope {
    fn generate(&self) -> proc_macro2::TokenStream {
        let initialize = self.generate_init();
        let registry_trait = self.generate_registry_trait();
        let internal_catalogue = self.generate_internal_catalogue();

        quote! {
            #initialize

            #registry_trait

            #internal_catalogue
        }
    }

    fn generate_init(&self) -> proc_macro2::TokenStream {
        let struct_name = format_ident!("{}", &self.struct_name);
        let metric_inits = self
            .metrics
            .iter()
            .map(|f| (f.instance.clone(), f.metric_type.to_string()))
            .map(default_init);
        let other_inits = self.other_fields.iter().map(default_init);
        let sub_metrics = self.sub_metrics.iter().map(default_init);
        let inits = metric_inits.chain(other_inits).chain(sub_metrics);

        quote! {
            impl #struct_name {
                const fn new() -> Self {
                    Self {
                        #(#inits),*
                    }
                }
            }
        }
    }

    fn generate_registry_trait(&self) -> proc_macro2::TokenStream {
        let struct_name = format_ident!("{}", &self.struct_name);
        let counters = match_metric_names(&self.metrics, MetricType::Counter);
        let sub_counters = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, _v)| {
                let sub = format_ident!("{}", k);
                quote! { .or_else(|| self.#sub.find_counter(name))}
            });
        let gauges = match_metric_names(&self.metrics, MetricType::Gauge);
        let sub_gauges = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, _v)| {
                let sub = format_ident!("{}", k);
                quote! { .or_else(|| self.#sub.find_gauge(name))}
            });

        quote! {
            impl Registry for #struct_name {
                fn find_counter(&self, name: &str) -> Option<&Counter> {
                    match name {
                        #(#counters),*
                    }
                    #(#sub_counters)*
                }

                fn find_gauge(&self, name: &str) -> Option<&Gauge> {
                    match name {
                        #(#gauges),*
                    }
                    #(#sub_gauges)*
                }
            }
        }
    }

    fn generate_internal_catalogue(&self) -> proc_macro2::TokenStream {
        let metric_keys = self.metrics.iter().filter(|m| !m.hidden).map(|metric| {
            let key = format_ident!("{}", metric.key);
            let name = format_ident!("{}", metric.name).to_string();
            let kv = quote! { #key: &str = #name };
            quote! { pub const #kv; }
        });
        let sub_metric_spaces = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, v)| {
                let internal_mod = format_ident!("catalogue_{}", v.ident);
                let public_mod = format_ident!("{}", k);
                quote! {
                    pub mod #public_mod {
                        #[allow(non_camel_case_types)]
                        pub use super::super::#internal_mod::*;
                    }
                }
            });
        let keys = metric_keys.chain(sub_metric_spaces);
        let name_mod = format_ident!("catalogue_{}", self.struct_name);
        quote! {
            #[allow(non_camel_case_types)]
            pub mod #name_mod {
                #(#keys)*
            }
        }
    }
}

#[derive(Debug)]
struct MetricInstance {
    key: String,
    instance: String,
    name: String,
    metric_type: MetricType,
    hidden: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum MetricType {
    Counter,
    Gauge,
    DiscreteGauge,
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            MetricType::Counter => "Counter",
            MetricType::Gauge => "Gauge",
            MetricType::DiscreteGauge => "DiscreteGauge",
        };
        write!(f, "{}", name)
    }
}

impl TryFrom<&Ident> for MetricType {
    type Error = Error;

    fn try_from(ident: &Ident) -> Result<Self> {
        // TODO: improve me
        match ident.to_string().as_str() {
            "Counter" => Ok(MetricType::Counter),
            "Gauge" => Ok(MetricType::Gauge),
            "DiscreteGauge" => Ok(MetricType::DiscreteGauge),
            unknown => Err(Error::new_spanned(
                ident,
                format!("Unknown metric type: {}", unknown),
            )),
        }
    }
}

struct Field<'a> {
    pub original: &'a syn::Field,
    pub ty: &'a Type,
    pub attributes: Attributes,
}

impl<'a> Field<'a> {
    fn multiple_from_syn(fields: &'a Fields) -> Result<Vec<Self>> {
        fields.iter().map(Field::from_syn).collect()
    }

    fn from_syn(node: &'a syn::Field) -> Result<Self> {
        Ok(Field {
            original: node,
            ty: &node.ty,
            attributes: Attributes::from_node(&node.attrs),
        })
    }

    fn get_metric(&self) -> Option<String> {
        if !self.attributes.is_metric {
            return None;
        }

        if let Some(name) = &self.attributes.name_override {
            Some(name.clone())
        } else {
            self.original.ident.as_ref().map(|x| x.to_string())
        }
    }
}

struct Struct<'a> {
    _original: &'a DeriveInput,
    pub ident: Ident,
    pub fields: Vec<Field<'a>>,
}

impl<'a> Struct<'a> {
    fn from_syn(node: &'a DeriveInput, data: &'a DataStruct) -> Result<Self> {
        Ok(Struct {
            _original: node,
            ident: node.ident.clone(),
            fields: Field::multiple_from_syn(&data.fields)?,
        })
    }
}

#[derive(Default, Debug)]
struct Attributes {
    pub hidden: bool,
    pub is_metric: bool,
    pub name_override: Option<String>,
}

impl Attributes {
    fn from_node(attrs: &[Attribute]) -> Self {
        let mut attributes = Attributes {
            is_metric: true,
            ..Default::default()
        };
        for attr in attrs {
            if let Ok(meta) = attr.parse_meta() {
                if let Some(ident) = meta.path().get_ident() {
                    if ident == "metric" {
                        match &meta {
                            Meta::List(list) => {
                                for nested in &list.nested {
                                    match nested {
                                        NestedMeta::Meta(m) => {
                                            if m.path().is_ident(HIDDEN_MARKER) {
                                                attributes.hidden = true;
                                            }
                                            if m.path().is_ident(SKIP_MARKER) {
                                                attributes.is_metric = false;
                                            }
                                        }
                                        NestedMeta::Lit(lit) => {
                                            if let Lit::Str(name) = lit {
                                                attributes.name_override = Some(name.value());
                                            }
                                        }
                                    }
                                }
                            }
                            Meta::Path(_) => {}
                            Meta::NameValue(_) => {}
                        }
                    }
                }
            }
        }

        attributes
    }
}
