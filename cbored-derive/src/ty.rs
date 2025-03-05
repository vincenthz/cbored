#![allow(unused)]
use syn::Ident;

pub enum Input {
    Struct {
        name: Ident,
        attr: StructAttr,
        content: Option<ProductType>,
    },
    Enum {
        name: Ident,
        attr: EnumAttr,
        variants: Vec<Variant>,
    },
}

pub struct StructAttr {
    attrs: Vec<syn::Meta>,
}

impl StructAttr {
    pub fn new<'a, I: Iterator<Item = &'a syn::Meta>>(metas: &mut I) -> Self {
        Self {
            attrs: metas.cloned().collect(),
        }
    }
}

pub struct EnumAttr {
    attrs: Vec<syn::Meta>,
}

impl EnumAttr {
    pub fn new<'a, I: Iterator<Item = &'a syn::Meta>>(metas: &mut I) -> Self {
        Self {
            attrs: metas.cloned().collect(),
        }
    }
}

pub struct Variant {
    name: Ident,
    attr: VariantAttr,
    inner: Option<ProductType>,
}

pub struct VariantAttr {
    attrs: Vec<syn::Meta>,
}

impl VariantAttr {
    pub fn new<'a, I: Iterator<Item = &'a syn::Meta>>(metas: &mut I) -> Self {
        Self {
            attrs: metas.cloned().collect(),
        }
    }
}

pub enum ProductType {
    Struct(Vec<(Ident, Field)>),
    Tuple(Vec<Field>),
}

pub struct Field {
    attr: FieldAttr,
    field: syn::Field,
}

pub struct FieldAttr {
    attrs: Vec<syn::Meta>,
}

impl FieldAttr {
    pub fn new<'a, I: Iterator<Item = &'a syn::Meta>>(metas: &mut I) -> Self {
        Self {
            attrs: metas.cloned().collect(),
        }
    }
}

use crate::attr::get_my_attributes;

pub fn parse(ast: syn::DeriveInput) -> Input {
    let has_generics = ast.generics.params.len() > 0;
    if has_generics {
        panic!("cannot handle types with generics")
    }

    // Gather the cborrepr attributes as Meta
    let mut attrs = get_my_attributes(&ast.attrs);

    // either do struct or enum handling
    match ast.data {
        syn::Data::Struct(st) => {
            let attr = StructAttr::new(&mut attrs);
            let content = parse_product_type(&st.fields);
            Input::Struct {
                name: ast.ident,
                attr,
                content,
            }
        }
        syn::Data::Enum(e) => {
            let attr = EnumAttr::new(&mut attrs);
            let variants = parse_sum_type(&e.variants);
            Input::Enum {
                name: ast.ident,
                attr,
                variants,
            }
        }
        syn::Data::Union(_) => panic!("Union not supported"),
    }
}

pub fn parse_sum_type(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> Vec<Variant> {
    variants
        .iter()
        .map(|variant| {
            let mut attrs = get_my_attributes(&variant.attrs);
            let attr = VariantAttr::new(&mut attrs);
            let inner = parse_product_type(&variant.fields);
            Variant {
                name: variant.ident.clone(),
                attr,
                inner,
            }
        })
        .collect::<Vec<_>>()
}

pub fn parse_product_type(fields: &syn::Fields) -> Option<ProductType> {
    if fields.is_empty() {
        None
    } else {
        match fields {
            syn::Fields::Named(fields_named) => Some(ProductType::Struct(
                fields_named
                    .named
                    .iter()
                    .map(|f| (f.ident.clone().unwrap(), parse_field(f)))
                    .collect::<Vec<_>>(),
            )),
            syn::Fields::Unnamed(_) => {
                let r = fields.iter().map(|f| parse_field(f)).collect::<Vec<_>>();
                Some(ProductType::Tuple(r))
            }
            syn::Fields::Unit => {
                panic!("unit not supported here")
            }
        }
    }
}

fn parse_field(f: &syn::Field) -> Field {
    let mut attrs = get_my_attributes(&f.attrs);
    let attr = FieldAttr::new(&mut attrs);
    Field {
        attr,
        field: f.clone(),
    }
}
