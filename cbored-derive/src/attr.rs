use std::str::FromStr;
use syn::{Attribute, Meta};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldVariantType {
    Simple,
    Vec,
}

impl FromStr for FieldVariantType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "simple" => Ok(FieldVariantType::Simple),
            "vec" => Ok(FieldVariantType::Vec),
            _ => Err(format!("unrecognized field variant type {}", s)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldCborType {
    Positive,
    Negative,
    Array,
    Map,
    Tag,
    Bytes,
    Text,
    Null,
}

impl FromStr for FieldCborType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "positive" => Ok(FieldCborType::Positive),
            "negative" => Ok(FieldCborType::Negative),
            "array" => Ok(FieldCborType::Array),
            "map" => Ok(FieldCborType::Map),
            "tag" => Ok(FieldCborType::Tag),
            "text" => Ok(FieldCborType::Text),
            "bytes" => Ok(FieldCborType::Bytes),
            "null" => Ok(FieldCborType::Null),
            _ => Err(format!("unrecognized field variant type {}", s)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructureType {
    Flat,
    Array,
    ArrayLastOpt,
    MapInt,
}

impl FromStr for StructureType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "flat" => Ok(StructureType::Flat),
            "array" => Ok(StructureType::Array),
            "array_lastopt" => Ok(StructureType::ArrayLastOpt),
            "mapint" => Ok(StructureType::MapInt),
            _ => Err(format!("unrecognized structure type {}", s)),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum EnumType {
    TagVariant,
    EnumInt,
    EnumType,
}

impl FromStr for EnumType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tagvariant" => Ok(EnumType::TagVariant),
            "enumint" => Ok(EnumType::EnumInt),
            "enumtype" => Ok(EnumType::EnumType),
            _ => Err(format!("unrecognized enum type {}", s)),
        }
    }
}

#[derive(Clone)]
pub(crate) enum Attr {
    Structure(StructureType),
    EnumType(EnumType),
    Tag(u64),
    VariantStartsAt(usize),
    SkipKey(u64),
}

fn parse_meta_list(meta: &Meta) -> &syn::MetaList {
    match meta {
        Meta::List(meta_list) => &meta_list,
        Meta::NameValue(_meta_name_val) => {
            panic!("attribute name value not supported")
        }
        Meta::Path(_path) => {
            panic!("attribute path not supported")
        }
    }
}

pub(crate) fn parse_attr(meta: &Meta) -> Vec<Attr> {
    let mut output = Vec::new();
    let meta_list = parse_meta_list(meta);

    meta_list
        .parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let value = meta.value()?;
                let lit: syn::LitInt = value.parse()?;
                output.push(Attr::Tag(parse_int(&lit)));
                Ok(())
            } else if meta.path.is_ident("enumtype") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                let enum_type = EnumType::from_str(&lit.value()).expect("Valid enum type");
                output.push(Attr::EnumType(enum_type));
                Ok(())
            } else if meta.path.is_ident("structure") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                let struct_type = StructureType::from_str(&lit.value()).expect("Valid struct type");
                output.push(Attr::Structure(struct_type));
                Ok(())
            } else if meta.path.is_ident("variant_starts_at") {
                let value = meta.value()?;
                let lit: syn::LitInt = value.parse()?;
                output.push(Attr::VariantStartsAt(parse_int(&lit) as usize));
                Ok(())
            } else if meta.path.is_ident("skipkey") {
                let value = meta.value()?;
                let lit: syn::LitInt = value.parse()?;
                output.push(Attr::SkipKey(parse_int(&lit)));
                Ok(())
            } else {
                Err(meta.error("unsupported attribute"))
            }
        })
        .unwrap();
    output
}

#[derive(Clone)]
pub(crate) enum FieldAttr {
    Variant(FieldVariantType),
    Optional,
    Mandatory,
    CborType(FieldCborType),
}

#[derive(Clone)]
pub(crate) struct FieldAttrs {
    pub(crate) variant: FieldVariantType,
    pub(crate) mandatory_map: bool,
    pub(crate) optional_vec: bool,
    pub(crate) cbor_type: Option<FieldCborType>,
}

impl Default for FieldAttrs {
    fn default() -> Self {
        FieldAttrs {
            variant: FieldVariantType::Simple,
            mandatory_map: false,
            optional_vec: false,
            cbor_type: None,
        }
    }
}

impl FieldAttrs {
    pub fn merge(mut self, attr: &FieldAttr) -> Self {
        match attr {
            FieldAttr::Variant(vty) => self.variant = *vty,
            FieldAttr::Mandatory => self.mandatory_map = true,
            FieldAttr::Optional => self.optional_vec = true,
            FieldAttr::CborType(ty) => self.cbor_type = Some(*ty),
        }
        self
    }
}

pub(crate) fn parse_field_attr(meta: &Meta) -> Vec<FieldAttr> {
    let mut output = Vec::new();
    let meta_list = parse_meta_list(meta);
    meta_list
        .parse_nested_meta(|meta| {
            if meta.path.is_ident("variant") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                let variant_type = FieldVariantType::from_str(&s.value()).expect("Valid enum type");
                output.push(FieldAttr::Variant(variant_type));

                Ok(())
            } else if meta.path.is_ident("cbortype") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                let variant_type = FieldCborType::from_str(&s.value()).expect("Valid enum type");
                output.push(FieldAttr::CborType(variant_type));

                Ok(())
            } else if meta.path.is_ident("mandatory") {
                output.push(FieldAttr::Mandatory);
                Ok(())
            } else if meta.path.is_ident("optional") {
                output.push(FieldAttr::Optional);
                Ok(())
            } else {
                Err(meta.error("unsupported attribute"))
            }
        })
        .unwrap();
    output
}

fn parse_int(lit: &syn::LitInt) -> u64 {
    lit.base10_parse().unwrap()
}

pub(crate) fn get_my_attributes<'a>(attrs: &'a Vec<Attribute>) -> impl Iterator<Item = &'a Meta> {
    attrs.iter().filter_map(|a| {
        if a.path().is_ident("cborrepr") {
            Some(&a.meta)
        } else {
            None
        }
    })
}
