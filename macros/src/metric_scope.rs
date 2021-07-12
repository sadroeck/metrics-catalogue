use crate::DEFAULT_SEPARATOR;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use syn::{Error, Result};

#[derive(Debug)]
pub struct MetricScope {
    pub struct_name: String,
    pub metrics: Vec<MetricInstance>,
    pub sub_metrics: HashMap<String, SubMetric>,
    pub other_fields: HashMap<String, String>,
}

impl MetricScope {
    pub fn generate(&self) -> proc_macro2::TokenStream {
        let initialize = self.generate_init();
        let registry_trait = self.generate_registry_trait();
        quote! {
            #initialize

            #registry_trait
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
                let prefix = format!("{}{}", k, DEFAULT_SEPARATOR);
                quote! { .or_else(|| name.strip_prefix(#prefix).and_then(|n| self.#sub.find_counter(n))) }
            });
        let gauges = match_metric_names(&self.metrics, MetricType::Gauge);
        let sub_gauges = self
            .sub_metrics
            .iter()
            .filter(|(_, m)| !m.hidden)
            .map(|(k, _v)| {
                let sub = format_ident!("{}", k);
                let prefix = format!("{}{}", k, DEFAULT_SEPARATOR);
                quote! { .or_else(|| name.strip_prefix(#prefix).and_then(|n| self.#sub.find_gauge(n))) }
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
}

#[derive(Debug)]
pub struct MetricInstance {
    pub key: String,
    pub instance: String,
    pub name: String,
    pub metric_type: MetricType,
    pub hidden: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    DiscreteGauge,
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    let v = format_ident!("{}", v.as_ref());
    quote! { #k: #v::new() }
}

fn match_instance(metric: &MetricInstance) -> proc_macro2::TokenStream {
    let name = format_ident!("{}", metric.name);
    let instance = format_ident!("{}", metric.instance);
    let quoted_name = name.to_string();
    quote! { #quoted_name => Some(&self.#instance) }
}

fn match_metric_names(
    instances: &[MetricInstance],
    metric_type: MetricType,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    let f_ident = format_ident!("{}", "_");
    let fallthrough = quote! { #f_ident => None };
    instances
        .iter()
        .filter(|m| !m.hidden)
        .filter(move |m| m.metric_type == metric_type)
        .map(match_instance)
        .chain(std::iter::once(fallthrough))
}
