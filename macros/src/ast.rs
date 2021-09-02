use crate::{ROOT_MARKER, SEPARATOR_MARKER, SKIP_MARKER};
use proc_macro2::Ident;
use quote::ToTokens;
use syn::{
    Attribute, DataStruct, DeriveInput, Fields, Lit, Meta, NestedMeta, Path, PathSegment, Result,
    Type,
};

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
        match &self.attributes {
            Attributes::Root(root) => root.name_override.clone(),
            Attributes::Struct(StructAttributes {
                hidden,
                name_override,
            }) => {
                if *hidden {
                    return None;
                }
                if let Some(name) = &name_override {
                    Some(name.clone())
                } else {
                    self.original.ident.as_ref().map(|x| x.to_string())
                }
            }
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

#[derive(Debug)]
pub enum Attributes {
    Root(RootAttributes),
    Struct(StructAttributes),
}

#[derive(Default, Debug)]
pub struct RootAttributes {
    pub separator: Option<String>,
    pub name_override: Option<String>,
}

#[derive(Default, Debug)]
pub struct StructAttributes {
    pub hidden: bool,
    pub name_override: Option<String>,
}

impl Attributes {
    pub fn is_hidden(&self) -> bool {
        match self {
            Self::Struct(s) => s.hidden,
            Self::Root(_) => false,
        }
    }

    pub fn from_node(attrs: &[Attribute]) -> Self {
        let mut root = None;
        let mut attributes = StructAttributes::default();
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
                                                root.get_or_insert(RootAttributes::default());
                                            }
                                            if let Meta::NameValue(val) = m {
                                                if val.path.is_ident(SEPARATOR_MARKER) {
                                                    if let Lit::Str(sep) = &val.lit {
                                                        root.as_mut().expect("Separator cannot be specified on a non-root element").separator = Some(sep.value());
                                                    } else {
                                                        panic!("Separator should be specified as a string")
                                                    }
                                                }
                                            }
                                        }
                                        NestedMeta::Lit(lit) => {
                                            if let Lit::Str(name) = lit {
                                                attributes.name_override = Some(name.value());
                                                if let Some(root) = &mut root {
                                                    root.name_override = Some(name.value());
                                                }
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

        if let Some(root) = root {
            Self::Root(root)
        } else {
            Self::Struct(attributes)
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypePath {
    pub path: String,
    pub args: Option<String>,
}

impl TypePath {
    pub fn full_path(&self) -> String {
        if let Some(args) = self.args.as_ref() {
            format!("{}::{}", self.path, args)
        } else {
            self.path.clone()
        }
    }
}

impl From<&Path> for TypePath {
    fn from(path: &Path) -> Self {
        let segment_count = path.segments.len();
        let mut segments = path
            .segments
            .iter()
            .take(segment_count - 1)
            .map(PathSegment::to_token_stream)
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let last_segment = path.segments.last().unwrap();
        segments.push(last_segment.ident.to_token_stream().to_string());
        TypePath {
            path: segments.join("::"),
            args: if last_segment.arguments.is_empty() {
                None
            } else {
                Some(last_segment.arguments.to_token_stream().to_string())
            },
        }
    }
}
