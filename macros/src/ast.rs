use crate::{ROOT_MARKER, SKIP_MARKER};
use proc_macro2::Ident;
use syn::{Attribute, DataStruct, DeriveInput, Fields, Lit, Meta, NestedMeta, Result, Type};

pub struct Field<'a> {
    pub original: &'a syn::Field,
    pub ty: &'a Type,
    pub attributes: Attributes,
}

impl<'a> Field<'a> {
    pub fn multiple_from_syn(fields: &'a Fields) -> Result<Vec<Self>> {
        fields.iter().map(Field::from_syn).collect()
    }

    pub fn from_syn(node: &'a syn::Field) -> Result<Self> {
        Ok(Field {
            original: node,
            ty: &node.ty,
            attributes: Attributes::from_node(&node.attrs),
        })
    }

    pub fn get_metric(&self) -> Option<String> {
        if self.attributes.hidden {
            return None;
        }

        if let Some(name) = &self.attributes.name_override {
            Some(name.clone())
        } else {
            self.original.ident.as_ref().map(|x| x.to_string())
        }
    }
}

pub struct Struct<'a> {
    _original: &'a DeriveInput,
    pub ident: Ident,
    pub fields: Vec<Field<'a>>,
    pub attributes: Attributes,
}

impl<'a> Struct<'a> {
    pub fn from_syn(node: &'a DeriveInput, data: &'a DataStruct) -> Result<Self> {
        Ok(Struct {
            _original: node,
            ident: node.ident.clone(),
            fields: Field::multiple_from_syn(&data.fields)?,
            attributes: Attributes::from_node(&node.attrs),
        })
    }
}

#[derive(Default, Debug)]
pub struct Attributes {
    pub hidden: bool,
    pub name_override: Option<String>,
    pub is_root: bool,
}

impl Attributes {
    pub fn from_node(attrs: &[Attribute]) -> Self {
        let mut attributes = Attributes::default();
        for attr in attrs {
            if let Ok(meta) = attr.parse_meta() {
                if let Some(ident) = meta.path().get_ident() {
                    if ident == "metric" {
                        match &meta {
                            Meta::List(list) => {
                                for nested in &list.nested {
                                    match nested {
                                        NestedMeta::Meta(m) => {
                                            if m.path().is_ident(SKIP_MARKER) {
                                                attributes.hidden = true;
                                            }
                                            if m.path().is_ident(ROOT_MARKER) {
                                                attributes.is_root = true;
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
