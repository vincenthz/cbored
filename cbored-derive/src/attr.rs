use std::str::FromStr;
use syn::{Attribute, Lit, Meta, NestedMeta};

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

pub(crate) fn parse_attr(meta: &Meta) -> Vec<Attr> {
    let mut output = Vec::new();
    match meta {
        Meta::List(meta_list) => {
            for attr in meta_list.nested.iter() {
                match attr {
                    NestedMeta::Meta(v) => match v {
                        Meta::NameValue(v) => {
                            let keys = v
                                .path
                                .segments
                                .iter()
                                .map(|p| p.ident.to_string())
                                .collect::<Vec<_>>();
                            match keys[0].as_str() {
                                "tag" => {
                                    let i = parse_int(&v.lit);
                                    output.push(Attr::Tag(i));
                                }
                                "enumtype" => {
                                    let s = parse_string(&v.lit);
                                    let enum_type =
                                        EnumType::from_str(s.as_str()).expect("Valid enum type");
                                    output.push(Attr::EnumType(enum_type));
                                }
                                "structure" => {
                                    let s = parse_string(&v.lit);
                                    let struct_type = StructureType::from_str(s.as_str())
                                        .expect("Valid struct type");
                                    output.push(Attr::Structure(struct_type));
                                }
                                "variant_starts_at" => {
                                    let i = parse_int(&v.lit);
                                    output.push(Attr::VariantStartsAt(i as usize));
                                }
                                "skipkey" => {
                                    let i = parse_int(&v.lit);
                                    output.push(Attr::SkipKey(i));
                                }
                                _ => {
                                    panic!("unknown key \"{:?}\"", keys[0])
                                }
                            }
                        }
                        Meta::Path(p) => {
                            panic!("uugh meta::path {:?}", p.get_ident().map(|p| p.to_string()))
                        }
                        Meta::List(_) => {
                            panic!("uugh meta::list")
                        }
                    },
                    _ => {
                        panic!("attribute list not supported")
                    }
                }
            }
        }
        Meta::NameValue(_meta_name_val) => {
            panic!("attribute name value not supported")
        }
        Meta::Path(_path) => {
            panic!("attribute path not supported")
        }
    };
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
    match meta {
        Meta::List(meta_list) => {
            for attr in meta_list.nested.iter() {
                match attr {
                    NestedMeta::Meta(v) => match v {
                        Meta::NameValue(v) => {
                            let keys = v
                                .path
                                .segments
                                .iter()
                                .map(|p| p.ident.to_string())
                                .collect::<Vec<_>>();
                            match keys[0].as_str() {
                                "variant" => {
                                    let s = parse_string(&v.lit);
                                    let variant_type = FieldVariantType::from_str(s.as_str())
                                        .expect("Valid enum type");
                                    output.push(FieldAttr::Variant(variant_type));
                                }
                                "cbortype" => {
                                    let s = parse_string(&v.lit);
                                    let variant_type = FieldCborType::from_str(s.as_str())
                                        .expect("Valid enum type");
                                    output.push(FieldAttr::CborType(variant_type));
                                }
                                _ => {
                                    panic!("unknown field attribute key \"{:?}\"", keys[0])
                                }
                            }
                        }
                        Meta::List(_) => {
                            panic!("uugh list")
                        }
                        Meta::Path(p) => match p.get_ident().map(|p| p.to_string()) {
                            None => panic!("field attribute path empty"),
                            Some(s) => match s.as_str() {
                                "mandatory" => {
                                    output.push(FieldAttr::Mandatory);
                                }
                                "optional" => {
                                    output.push(FieldAttr::Optional);
                                }
                                _ => {
                                    panic!("unknown field attribute path \"{:?}\"", s)
                                }
                            },
                        },
                    },
                    _ => {
                        panic!("attribute list not supported")
                    }
                }
            }
        }
        Meta::NameValue(_meta_name_val) => {
            panic!("attribute name value not supported")
        }
        Meta::Path(_path) => {
            panic!("attribute path not supported")
        }
    }
    output
}

fn parse_string(lit: &Lit) -> String {
    match &lit {
        Lit::Str(s) => s.value(),
        _ => {
            panic!("expecting literal string but got another type of literal");
        }
    }
}

fn parse_int(lit: &Lit) -> u64 {
    match &lit {
        Lit::Int(s) => s.base10_parse().unwrap(),
        _ => {
            panic!("expecting literal int but got another type of literal");
        }
    }
}

pub(crate) fn get_my_attributes<'a>(
    attrs: &'a Vec<Attribute>,
) -> impl Iterator<Item = &'a Attribute> {
    attrs.iter().filter(|a| a.path.is_ident("cborrepr"))
}
