extern crate proc_macro;

use crate::metric_tree::MetricTree;
use proc_macro::TokenStream;
use quote::quote;
use std::sync::Mutex;
use syn::{parse_macro_input, DeriveInput, Result};

mod ast;
mod metric_scope;
mod metric_tree;
mod scoped_catalogue;

const SKIP_MARKER: &str = "skip";
const ROOT_MARKER: &str = "root";
const DEFAULT_SEPARATOR: char = '.';

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
    Ok(if tree.is_complete() {
        tree.generate()
    } else {
        quote! {}
    }
    .into())
}
