use crate::ast::TypePath;
use inflector::Inflector;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use syn::{Error, Path, Result};

#[derive(Debug)]
pub struct MetricScope {
    pub struct_name: String,
    pub name_override: Option<String>,
    pub metrics: Vec<MetricInstance>,
    pub sub_metrics: HashMap<String, SubMetric>,
    pub other_fields: HashMap<String, String>,
}

impl MetricScope {
    pub fn generate(&self, key_separator: &str, is_root: bool) -> proc_macro2::TokenStream {
        let initialize = self.generate_init();
        let registry_trait = self.generate_registry_trait(key_separator, is_root);
        #[cfg(feature = "prometheus")]
        let prometheus = self.generate_prometheus(key_separator, is_root);
        #[cfg(not(feature = "prometheus"))]
        let prometheus = quote! {};

        quote! {
            #initialize

            #registry_trait

            #prometheus
        }
    }

    fn generate_init(&self) -> proc_macro2::TokenStream {
        let struct_name = format_ident!("{}", &self.struct_name);
        let metric_inits = self
            .metrics
            .iter()
            .map(|f| (f.instance.clone(), f.type_path.full_path()))
            .map(default_init);
        let other_inits = self.other_fields.iter().map(default_init);
        let sub_metrics = self.sub_metrics.iter().map(default_init);
        let inits = metric_inits.chain(other_inits).chain(sub_metrics);

        quote! {
            impl #struct_name {
                pub const fn new() -> Self {
                    Self {
                        #(#inits),*
                    }
                }
            }
        }
    }

    fn generate_registry_trait(
        &self,
        key_separator: &str,
        is_root: bool,
    ) -> proc_macro2::TokenStream {
        let struct_name = format_ident!("{}", &self.struct_name);
        let counters = match_metric_names(&self.metrics, &[MetricType::Counter], None);
        let sub_counters = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, _v)| {
                let sub = format_ident!("{}", k);
                let prefix = format!("{}{}", k, key_separator);
                quote! { .or_else(|| name.strip_prefix(#prefix).and_then(|n| ::metrics_catalogue::Registry::find_counter(&self.#sub, n))) }
            });
        let gauges = match_metric_names(
            &self.metrics,
            &[MetricType::Gauge, MetricType::DiscreteGauge],
            Some("GaugeMetric"),
        );
        let sub_gauges = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, _v)| {
                let sub = format_ident!("{}", k);
                let prefix = format!("{}{}", k, key_separator);
                quote! { .or_else(|| name.strip_prefix(#prefix).and_then(|n| ::metrics_catalogue::Registry::find_gauge(&self.#sub, n))) }
            });

        let histograms = match_metric_names(
            &self.metrics,
            &[MetricType::Histogram],
            Some("HistogramMetric"),
        );
        let sub_histograms = self.sub_metrics.iter().filter(|(_, m)| !m.hidden).map(|(k, _v)| {
            let sub = format_ident!("{}", k);
            let prefix = format!("{}{}", k, key_separator);
            quote! { .or_else(|| name.strip_prefix(#prefix).and_then(|n| ::metrics_catalogue::Registry::find_histogram(&self.#sub, n))) }
        });

        let with_strip_prefix = |segment| {
            if is_root
                && self
                    .name_override
                    .as_ref()
                    .map(|x| !x.is_empty())
                    .unwrap_or(true)
            {
                let prefix = format!("{}{}", self.struct_name.to_snake_case(), key_separator);
                quote! {
                    name.strip_prefix(#prefix).and_then(|name| {
                        #segment
                    })
                }
            } else {
                segment
            }
        };

        let find_counter = with_strip_prefix(quote! {
            match name {
                #(#counters),*
            }
            #(#sub_counters)*
        });
        let find_gauge = with_strip_prefix(quote! {
            match name {
                #(#gauges),*
            }
            #(#sub_gauges)*
        });
        let find_histogram = with_strip_prefix(quote! {
            match name {
                #(#histograms),*
            }
            #(#sub_histograms)*
        });

        quote! {
            impl ::metrics_catalogue::Registry for #struct_name {
                fn find_counter(&self, name: &str) -> Option<&::metrics_catalogue::Counter> {
                    #find_counter
                }

                fn find_gauge(&self, name: &str) -> Option<&dyn ::metrics_catalogue::GaugeMetric> {
                    #find_gauge
                }

                fn find_histogram(&self, name: &str) -> Option<&dyn ::metrics_catalogue::HistogramMetric> {
                    #find_histogram
                }
            }
        }
    }

    #[cfg(feature = "prometheus")]
    fn generate_prometheus(&self, key_separator: &str, is_root: bool) -> proc_macro2::TokenStream {
        let struct_name = format_ident!("{}", &self.struct_name);
        let needs_new_prefix = self.metrics.iter().filter(|m| !m.hidden).count()
            + self.sub_metrics.iter().filter(|(_, m)| !m.hidden).count()
            > 0;
        let new_prefix = if is_root {
            let s = self.name_override.as_ref().unwrap_or(&self.struct_name);
            let root_prefix = format!(
                "{}{}",
                if s.is_empty() { &self.struct_name } else { s }.to_snake_case(),
                key_separator
            );
            quote! {
                let prefix = #root_prefix;
            }
        } else if needs_new_prefix {
            let formatter = format!("{{}}{{}}{}", key_separator);
            let name_formatter = format!("{{}}{}", key_separator);

            quote! {
                let prefix = if !prefix.is_empty() {
                    std::borrow::Cow::Owned(format!(#formatter, prefix, name))
                } else if !name.is_empty() {
                    std::borrow::Cow::Owned(format!(#name_formatter, name))
                } else {
                    std::borrow::Cow::Borrowed("")
                };
            }
        } else {
            quote! {}
        };
        let fields = self.metrics.iter().filter(|m| !m.hidden).map(|metric| {
            let instance = format_ident!("{}", metric.instance);
            let name = metric.name.clone();
            quote! { ::metrics_catalogue::prometheus::StringRender::render(&self.#instance, &prefix, #name, s); }
        });
        let sub_metrics = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, _v)| {
                let sub = format_ident!("{}", k);
                let name = k.to_string();
                quote! { ::metrics_catalogue::prometheus::StringRender::render(&self.#sub, &prefix, #name, s); }
            });

        quote! {
            impl ::metrics_catalogue::prometheus::StringRender for #struct_name {
                fn render(&self, prefix: &str, name: &str, s: &mut String) {
                    #new_prefix
                    #(#fields)*

                    #(#sub_metrics)*
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct MetricInstance {
    pub key: String,
    pub instance: String,
    pub type_path: TypePath,
    pub name: String,
    pub metric_type: MetricType,
    pub hidden: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    DiscreteGauge,
    Histogram,
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            MetricType::Counter => "Counter",
            MetricType::Gauge => "Gauge",
            MetricType::DiscreteGauge => "DiscreteGauge",
            MetricType::Histogram => "Histogram",
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
            "Histogram" => Ok(MetricType::Histogram),
            unknown => Err(Error::new_spanned(
                ident,
                format!("Unknown metric type: {}", unknown),
            )),
        }
    }
}

#[derive(Debug)]
pub struct SubMetric {
    pub ident: String,
    pub hidden: bool,
}

impl AsRef<str> for SubMetric {
    fn as_ref(&self) -> &str {
        &self.ident
    }
}

fn default_init((k, v): (impl AsRef<str>, impl AsRef<str>)) -> proc_macro2::TokenStream {
    let k = format_ident!("{}", k.as_ref());
    let v = syn::parse_str::<Path>(v.as_ref())
        .unwrap_or_else(|_| panic!("invalid path: {}", v.as_ref()));
    quote! { #k: #v::new() }
}

fn match_instance(metric: &MetricInstance, as_trait: Option<&str>) -> proc_macro2::TokenStream {
    let name = format_ident!("{}", metric.name);
    let instance = format_ident!("{}", metric.instance);
    let quoted_name = name.to_string();
    if let Some(as_trait) = as_trait {
        let as_trait = format_ident!("{}", as_trait);
        quote! { #quoted_name => Some(&self.#instance as &dyn ::metrics_catalogue::#as_trait) }
    } else {
        quote! { #quoted_name => Some(&self.#instance) }
    }
}

fn match_metric_names<'a>(
    instances: &'a [MetricInstance],
    metric_types: &'a [MetricType],
    as_trait: Option<&'a str>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    let f_ident = format_ident!("{}", "_");
    let fallthrough = quote! { #f_ident => None };
    instances
        .iter()
        .filter(|m| !m.hidden)
        .filter(move |m| metric_types.iter().any(|t| m.metric_type == *t))
        .map(move |m| match_instance(m, as_trait))
        .chain(std::iter::once(fallthrough))
}
