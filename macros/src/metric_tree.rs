use crate::ast::{Attributes, Struct, TypePath};
use crate::metric_scope::{MetricInstance, MetricScope, MetricType, SubMetric};
use crate::scoped_catalogue::ScopedCatalogue;
use crate::DEFAULT_SEPARATOR;
use inflector::Inflector;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::iter::once;
use syn::{Data, DeriveInput, Error, Result, Type};

#[derive(Default, Debug)]
pub struct MetricTree {
    scopes: HashMap<String, MetricScope>,
    required_scopes: HashSet<String>,
    root_scope: Option<String>,
    key_separator: String,
}

impl MetricTree {
    pub fn is_complete(&self) -> bool {
        self.root_scope.is_some()
            && self
                .required_scopes
                .iter()
                .all(|name| self.scopes.contains_key(name))
    }

    pub fn generate(&self) -> proc_macro2::TokenStream {
        let root = self.generate_root();
        let catalogue = self.generate_catalogue();
        let root_name = self.root_scope.as_ref().expect("No root scope");
        let scopes = self
            .scopes
            .values()
            .map(|scope| scope.generate(&self.key_separator, scope.struct_name == *root_name));
        let combined = once(root).chain(once(catalogue)).chain(scopes);

        quote! {
            #(#combined)*
        }
    }

    fn generate_root(&self) -> proc_macro2::TokenStream {
        let root_name = self.root_scope.clone().expect("No root scope");
        let root_struct = format_ident!("{}", root_name);
        let recorder = quote! {
            impl ::metrics_catalogue::Recorder for #root_struct {
                // The following are unused in Stats
                fn register_counter(&self, _key: &::metrics_catalogue::Key, _unit: Option<::metrics_catalogue::Unit>, _desc: Option<&'static str>) {}

                fn register_gauge(&self, _key: &::metrics_catalogue::Key, _unit: Option<::metrics_catalogue::Unit>, _desc: Option<&'static str>) {}

                fn register_histogram(&self, _key: &::metrics_catalogue::Key, _unit: Option<::metrics_catalogue::Unit>, _desc: Option<&'static str>) {}

                fn record_histogram(&self, key: &::metrics_catalogue::Key, value: f64) {
                    if let Some(metric) = ::metrics_catalogue::Registry::find_histogram(self, key.name()) {
                        metric.insert(value);
                    }
                }

                fn increment_counter(&self, key: &::metrics_catalogue::Key, value: u64) {
                    if let Some(metric) = ::metrics_catalogue::Registry::find_counter(self, key.name()) {
                        metric.increment(value);
                    }
                }

                fn update_gauge(&self, key: &::metrics_catalogue::Key, value: ::metrics_catalogue::GaugeValue) {
                    use ::metrics_catalogue::GaugeValue;
                    if let Some(metric) = ::metrics_catalogue::Registry::find_gauge(self, key.name()) {
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
            #recorder
        }
    }

    fn generate_scoped_catalogue(&self, mod_name: &str, scope_name: &str) -> ScopedCatalogue {
        let scope = self.scopes.get(scope_name).expect("Invalid scope");
        ScopedCatalogue {
            mod_name: mod_name.to_string(),
            metrics: scope
                .metrics
                .iter()
                .filter(|m| !m.hidden)
                .map(|m| (m.key.clone(), m.name.clone()))
                .collect(),
            sub_scopes: scope
                .sub_metrics
                .iter()
                .filter(|(_, m)| !m.hidden)
                .map(|(k, v)| (k.clone(), self.generate_scoped_catalogue(k, &v.ident)))
                .collect(),
        }
    }

    fn generate_catalogue(&self) -> proc_macro2::TokenStream {
        let root_struct = self.root_scope.clone().expect("No root struct");
        let root_mod = root_struct.to_snake_case();
        let prefix = format!("{}{}", root_mod, self.key_separator);

        self.generate_scoped_catalogue("catalogue", &root_struct)
            .generate_prefix_keys(&prefix, &self.key_separator)
    }

    pub fn parse_struct(&mut self, input: DeriveInput) -> Result<()> {
        let struct_data = match &input.data {
            Data::Struct(data) => Struct::from_syn(&input, data),
            Data::Enum(_) | Data::Union(_) => Err(Error::new_spanned(
                &input,
                "Metrics are only supported as structs",
            )),
        }?;

        if matches!(struct_data.attributes, Attributes::Root(_)) && self.root_scope.is_some() {
            return Err(Error::new_spanned(
                &input,
                format!(
                    "Duplicate root attribute previously detected on {}",
                    self.root_scope.as_ref().expect("No root scope")
                ),
            ));
        }

        if let Attributes::Root(root) = &struct_data.attributes {
            self.root_scope.get_or_insert(struct_data.ident.to_string());
            self.key_separator = root
                .separator
                .as_deref()
                .unwrap_or(DEFAULT_SEPARATOR)
                .to_string();
        }

        let mut metrics = vec![];
        let mut other_fields = HashMap::new();
        let mut sub_metrics = HashMap::new();
        for field in &struct_data.fields {
            if !field.attributes.is_hidden() {
                let name = field.get_metric().ok_or_else(|| {
                    Error::new_spanned(
                        &input,
                        format!(
                            "No metric name for {}",
                            field.original.ident.as_ref().expect("No field name")
                        ),
                    )
                })?;
                let path = if let Type::Path(path) = field.ty {
                    path
                } else {
                    return Err(Error::new_spanned(&input, "Invalid type for metrics"));
                };

                let type_path = TypePath::from(&path.path);
                let ident = &path
                    .path
                    .segments
                    .iter()
                    .last()
                    .ok_or_else(|| Error::new_spanned(&input, "Field needs to be a named type"))?
                    .ident;

                match MetricType::try_from(ident) {
                    Ok(metric_type) => metrics.push(MetricInstance {
                        key: name.to_ascii_uppercase(),
                        name: name.clone(),
                        type_path,
                        instance: field
                            .original
                            .ident
                            .as_ref()
                            .ok_or_else(|| Error::new_spanned(field.original, "No field identity"))?
                            .to_string(),
                        metric_type,
                        hidden: field.attributes.is_hidden(),
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
                                hidden: field.attributes.is_hidden(),
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
