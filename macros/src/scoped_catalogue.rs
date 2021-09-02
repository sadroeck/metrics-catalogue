use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;

pub struct ScopedCatalogue {
    pub mod_name: String,
    pub metrics: Vec<(String, String)>,
    pub sub_scopes: HashMap<String, ScopedCatalogue>,
}

impl ScopedCatalogue {
    pub fn generate_prefix_keys(&self, prefix: &str, separator: &str) -> TokenStream {
        let metric_keys = self.metrics.iter().map(|(k, v)| {
            let key = format_ident!("{}", k);
            let name = format!("{}{}", prefix, v);
            let kv = quote! { #key: &str = #name };
            quote! { pub const #kv; }
        });
        let sub_metric_spaces = self.sub_scopes.iter().map(|(name, scope)| {
            let prefix = format!("{}{}{}", prefix, name, separator);
            scope.generate_prefix_keys(&prefix, separator)
        });
        let keys = metric_keys.chain(sub_metric_spaces);
        let name_mod = format_ident!("{}", self.mod_name);
        quote! {
            #[allow(non_camel_case_types)]
            pub mod #name_mod {
                #(#keys)*
            }
        }
    }
}
