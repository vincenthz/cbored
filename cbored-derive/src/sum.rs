use proc_macro::TokenStream;

use quote::quote;
use syn::{DataEnum, Ident, Meta, Variant};

use crate::product::get_struct_naming;
use crate::product::StructOutput;

use super::attr::*;
use super::common::*;

pub(crate) struct EnumAttrs {
    enumtype: EnumType,
    variant_starts_at: usize,
    variant_skip: Vec<usize>,
}

impl EnumAttrs {
    pub fn from_metas(attrs: &[&Meta]) -> Self {
        let mut enumtype = EnumType::TagVariant;
        let mut variant_starts_at = 0;
        let mut variant_skip = Vec::new();

        for attr in attrs {
            for attr in parse_attr(&attr) {
                match attr {
                    Attr::Tag(_) | Attr::Structure(_) => {
                        panic!("enum does not support struct type attribute")
                    }
                    Attr::MapStartsAt(_) => {
                        panic!("enum does not support map_starts_at key attribute")
                    }
                    Attr::EnumType(ty) => enumtype = ty,
                    Attr::VariantStartsAt(v) => variant_starts_at = v,
                    Attr::SkipKey(v) => variant_skip.push(v as usize),
                }
            }
        }
        Self {
            enumtype,
            variant_starts_at,
            variant_skip,
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct VariantDef {
    cbor_type: Option<FieldCborType>,
    ty: Option<StructOutput>,
}

// get whether the variant is of the form `A { a: ... , b: ... }` or `A(... , ...)` or `A`
fn variant_field(attrs: &EnumAttrs, variant: &Variant) -> VariantDef {
    let all_named = variant.fields.iter().all(|f| f.ident.is_some());
    let all_unnamed = variant.fields.iter().all(|f| f.ident.is_none());
    let nb_items = variant.fields.len();

    if nb_items > 0 && (!all_named && !all_unnamed) {
        panic!("fields should be all named or unnamed");
    }

    let variant_attrs = get_my_attributes(&variant.attrs)
        .map(|a| parse_field_attr(a))
        .fold(FieldAttrs::default(), |acc, y| {
            y.iter().fold(acc, |acc, y| acc.merge(y))
        });

    match attrs.enumtype {
        EnumType::EnumInt => assert_eq!(nb_items, 0),
        EnumType::EnumType => {
            if variant_attrs.cbor_type.is_none() {
                panic!("enum type needs cbor-repr cbor-type attributes")
            }
        }
        EnumType::TagVariant => {}
    };

    let FieldAttrs {
        variant_type: _,
        mandatory_map: _,
        optional_vec: _,
        cbor_type,
    } = variant_attrs;

    let ty = if variant.fields.is_empty() {
        None
    } else {
        Some(get_struct_naming(&variant.fields))
    };
    VariantDef { ty, cbor_type }
}

pub fn enumerate_variant_indices<'a, T: Clone, I: Iterator<Item = T>>(
    attr: &EnumAttrs,
    it: &mut I,
) -> Vec<(usize, T)> {
    let elements = it.map(|v| v.clone()).collect::<Vec<_>>();

    let mut index = attr.variant_starts_at;

    let mut indices = Vec::new();
    for _ in 0..elements.len() {
        while attr.variant_skip.contains(&index) {
            index += 1;
        }
        indices.push(index);
        index += 1;
    }

    assert_eq!(elements.len(), indices.len());
    indices.into_iter().zip(elements).collect()
}

pub(crate) fn derive_enum_se(
    name: &Ident,
    attrs: &[&Meta],
    st: &DataEnum,
) -> proc_macro2::TokenStream {
    let mut se_branches = Vec::new();

    let attrs = EnumAttrs::from_metas(attrs);

    if attrs.enumtype == EnumType::EnumType {
        for (_variant_index, variant) in
            enumerate_variant_indices(&attrs, &mut st.variants.iter()).iter()
        {
            let ident = &variant.ident;

            let variant_def = variant_field(&attrs, &variant);
            let variant_type = &variant_def.ty;

            let (parameters, se_branch_body) = {
                match &variant_type {
                    Some(StructOutput::Named(field_names)) => {
                        if field_names.len() != 1 {
                            panic!("cannot have enumtype with more than 1 argument")
                        }
                        let field_name = &field_names[0].name;
                        (
                            quote! {
                                { #field_name }
                            },
                            quote! { #field_name.encode(writer); },
                        )
                    }
                    Some(StructOutput::Unnamed(field_names)) => {
                        if field_names.len() != 1 {
                            panic!("cannot have enumtype with more than 1 argument")
                        }
                        let field_name = &field_names[0].name;
                        (
                            quote! {
                                ( #field_name )
                            },
                            quote! { #field_name.encode(writer); },
                        )
                    }
                    None => match variant_def.cbor_type {
                        None => panic!("cannot have no cbor_type"),
                        Some(FieldCborType::Null) => (quote! {}, quote! { writer.null(); }),
                        Some(_) => {
                            panic!("cannot have a cbor-type that is not null without argument")
                        }
                    },
                }
            };

            let se_branch = quote! {
                Self::#ident #parameters => { #se_branch_body }
            };

            se_branches.push(se_branch);
        }
    } else {
        for (variant_index, variant) in
            enumerate_variant_indices(&attrs, &mut st.variants.iter()).iter()
        {
            let ident = &variant.ident;

            let nb_items = variant.fields.len();

            let variant_def = variant_field(&attrs, &variant);
            let variant_type = &variant_def.ty;

            let variant_number = attrs.variant_starts_at + variant_index;

            let (parameters, se_fields) = {
                match &variant_type {
                    Some(StructOutput::Named(field_names)) => {
                        let de_field_names = field_names
                            .iter()
                            .map(|field| &field.name)
                            .map(|ident| quote! { #ident })
                            .collect::<Vec<_>>();
                        let se_fields = field_names
                            .iter()
                            .map(|field| &field.name)
                            .map(|ident| {
                                quote! { writer.encode(#ident); }
                            })
                            .collect::<Vec<_>>();
                        let parameters = quote! { { #( #de_field_names ),* } };
                        (parameters, se_fields)
                    }
                    Some(StructOutput::Unnamed(field_names)) => {
                        let de_field_names = field_names
                            .iter()
                            .map(|field| &field.name)
                            .map(|ident| quote! { #ident })
                            .collect::<Vec<_>>();
                        let se_fields = field_names
                            .iter()
                            .map(|field| {
                                let ident = &field.name;
                                if field.is_vec || field.attrs.variant_type == FieldVariantType::Vec {
                                    quote! {
                                        writer.array_build(::cbored::StructureLength::from(#ident.len() as u64), |writer| {
                                            for v in #ident.iter() {
                                                writer.encode(v);
                                            }
                                        });
                                    }
                                } else {
                                    quote! { writer.encode(#ident); }
                                }
                            })
                            .collect::<Vec<_>>();
                        let parameters = quote! { ( #( #de_field_names ),* ) };
                        (parameters, se_fields)
                    }
                    None => (quote! {}, vec![]),
                }
            };

            // skip writing array in a case of enumint mode and no params
            let se_branch_body = if variant_type == &None && attrs.enumtype == EnumType::EnumInt {
                quote! {
                    writer.encode(&(#variant_number as u64));
                    #(#se_fields)*
                }
            } else {
                quote! {
                    let len = ::cbored::StructureLength::from(1 + #nb_items as u64);
                    writer.array_build(len, |writer| {
                        writer.encode(&(#variant_number as u64));
                        #(#se_fields)*
                    })
                }
            };
            let se_branch = quote! {
                Self::#ident #parameters => { #se_branch_body }
            };

            se_branches.push(se_branch);
        }
    }

    let se_body = quote! {
        match self {
            #( #se_branches )*
        }
    };
    token_impl_serializer(&name, se_body)
}

pub(crate) fn derive_enum_de(
    name: &Ident,
    attrs: &[&Meta],
    st: &DataEnum,
) -> proc_macro2::TokenStream {
    let name_type = format!("{}", name);

    let attrs = EnumAttrs::from_metas(attrs);

    let mut field_matches = Vec::new();

    match attrs.enumtype {
        EnumType::EnumInt => {
            // each branch of deserialization is of the form
            //     X => {
            //          get field 0..n;
            //          Ok(Constructor field 0..n)
            //     }
            for (variant_index, variant) in
                enumerate_variant_indices(&attrs, &mut st.variants.iter()).iter()
            {
                let ident = &variant.ident;
                let variant_number = attrs.variant_starts_at + variant_index;

                let de_branch = quote! {
                    #variant_number => {
                        Ok(Self::#ident)
                    }
                };
                field_matches.push(de_branch);
            }
        }
        EnumType::EnumType => {
            // each branch of deserialization is of the form
            //     X => {
            //          get field 1;
            //          Ok(Constructor field 0..n)
            //     }
            for (_variant_index, variant) in
                enumerate_variant_indices(&attrs, &mut st.variants.iter()).iter()
            {
                let ident = &variant.ident;
                let variant_name = format!("{}", ident);
                let variant_def = variant_field(&attrs, &variant);
                let variant_type = &variant_def.ty;

                //let mut variant_fields_deser = Vec::new();
                //let mut parameters = quote! {};

                let cbor_type = variant_def
                    .cbor_type
                    .expect("variant is missing a cbor type");

                fn cborty(s: &str) -> syn::Ident {
                    quote::format_ident!("{}", s)
                }
                let eqval = match cbor_type {
                    FieldCborType::Positive => cborty("Positive"),
                    FieldCborType::Negative => cborty("Negative"),
                    FieldCborType::Array => cborty("Array"),
                    FieldCborType::Map => cborty("Map"),
                    FieldCborType::Tag => cborty("Tag"),
                    FieldCborType::Bytes => cborty("Bytes"),
                    FieldCborType::Text => cborty("Text"),
                    FieldCborType::Null => cborty("Null"),
                };

                let (field_parameter, variant_field_deser) = match variant_type {
                    None => {
                        if cbor_type != FieldCborType::Null {
                            panic!("no arguemnt cannot be anything else than cbor null")
                        }
                        (
                            quote! {},
                            quote! {
                                reader.null().map_err(DecodeErrorKind::ReaderError).map_err(|e| e.context_str(#variant_name).push::<Self>());
                            },
                        )
                    }
                    Some(StructOutput::Unnamed(field_names)) => {
                        if field_names.len() != 1 {
                            panic!("cannot have enumtype with more than 1 argument")
                        }
                        let field_name = &field_names[0].name;
                        (
                            quote! {
                                ( #field_name )
                            },
                            quote! {
                                let #field_name = reader.decode().map_err(|e| e.push_str(#variant_name).push::<Self>())?;
                            },
                        )
                    }
                    Some(StructOutput::Named(field_names)) => {
                        if field_names.len() != 1 {
                            panic!("cannot have enumtype with more than 1 argument")
                        }
                        let field_name = &field_names[0].name;
                        (
                            quote! {
                                { #field_name }
                            },
                            quote! {
                                let #field_name = reader.decode().map_err(|e| e.push_str(#variant_name).push::<Self>())?;
                            },
                        )
                    }
                };

                let variant_match = quote! {
                    ::cbored::Type::#eqval => {
                        #variant_field_deser
                        Ok(Self::#ident #field_parameter)
                    }
                };
                field_matches.push(variant_match);
            }
        }
        EnumType::TagVariant => {
            for (variant_index, variant) in
                enumerate_variant_indices(&attrs, &mut st.variants.iter()).iter()
            {
                let ident = &variant.ident;
                let variant_name = format!("{}", ident);
                let variant_number = attrs.variant_starts_at + variant_index;

                let nb_items = variant.fields.len();

                let variant_def = variant_field(&attrs, &variant);
                let variant_type = &variant_def.ty;

                // skip array length check in a case of enumint mode
                let de_array_lencheck = quote! {
                    if array.len() != #nb_items + 1 {
                        return Err(::cbored::DecodeErrorKind::Custom(
                            format!("wrong number of items for {}::{} got {} expected {}",
                                #name_type,
                                #variant_name,
                                array.len(),
                                #nb_items + 1)
                            ).context::<Self>()
                        );
                    }
                };

                let (parameters, de_fields) = {
                    match variant_type {
                        Some(StructOutput::Named(field_names)) => {
                            let de_field_names = field_names
                                .iter()
                                .map(|field| &field.name)
                                .map(|ident| quote! { #ident })
                                .collect::<Vec<_>>();
                            let de_fields = de_field_names
                                .iter()
                                .enumerate()
                                .map(|(fidx, fname)| {
                                    let fname_str = format!("{}", fname);
                                    quote! {
                                        let #fname = array[#fidx + 1].decode().map_err(|e| e.push_str(#fname_str).push_str(#variant_name).push::<Self>())?;
                                    }
                                })
                                .collect::<Vec<_>>();
                            let parameters = quote! { { #( #de_field_names ),* } };
                            (parameters, de_fields)
                        }
                        Some(StructOutput::Unnamed(field_names)) => {
                            let de_field_names = field_names
                                .iter()
                                .map(|field| &field.name)
                                .map(|ident| quote! { #ident })
                                .collect::<Vec<_>>();
                            let de_fields = field_names
                                .iter()
                                .map(|field| {
                                    let fidx = field.index;
                                    let ident = &field.name;
                                    if field.is_vec || field.attrs.variant_type == FieldVariantType::Vec {
                                        quote! {
                                            let #ident = {
                                                let mut r = array[#fidx + 1].reader();
                                                let vec = r.array().map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?
                                                    .iter()
                                                    .map(|mut r| r.decode())
                                                    .collect::<Result<Vec<_>, ::cbored::DecodeError>>()?;
                                                vec
                                            };
                                        }
                                    } else {
                                        let fname_str = format!("{}", ident);
                                        quote! {
                                            let #ident = array[#fidx + 1].decode().map_err(|e| e.push_str(#fname_str).push_str(#variant_name).push::<Self>())?;
                                        }
                                    }
                                })
                                .collect::<Vec<_>>();
                            let parameters = quote! { ( #( #de_field_names ),* ) };
                            (parameters, de_fields)
                        }
                        None => (quote! {}, vec![]),
                    }
                };

                // each branch of deserialization is of the form
                //     X => {
                //          check_len();
                //          get field 0..n;
                //          Ok(Constructor field 0..n)
                //     }
                let de_branch = quote! {
                    #variant_number => {
                        #de_array_lencheck
                        #( #de_fields )*
                        Ok(Self::#ident #parameters)
                    }
                };
                field_matches.push(de_branch)
            }
        }
    }

    let body = match attrs.enumtype {
        EnumType::EnumInt => quote! {
            let variant: u64 = reader.decode()?;
            let variant: usize = variant as usize;
            match variant {
                #( #field_matches )*
                _ => {
                    return Err(::cbored::DecodeErrorKind::Custom(format!("{} variant number {} is not known", #name_type, variant)).context::<Self>());
                }
            }
        },
        EnumType::EnumType => {
            quote! {
                let cbor_type = reader.peek_type().map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?;
                match cbor_type {
                    #( #field_matches )*
                    _ => {
                        return Err(::cbored::DecodeErrorKind::Custom(format!("{} unknown type {:?} is not known", #name_type, cbor_type)).context::<Self>());
                    }
                }
            }
        }
        EnumType::TagVariant => {
            quote! {
                let array = reader.array().map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?;
                match array.len() {
                    0 => {
                        return Err(::cbored::DecodeErrorKind::Custom(format!("expecting at least 1 item in variant encoding of {}", #name_type)).context::<Self>());
                    }
                    _ => {}
                };
                let variant: u64 = array[0].decode()?;
                let variant: usize = variant as usize;
                match variant {
                    #( #field_matches )*
                    _ => {
                        return Err(::cbored::DecodeErrorKind::Custom(format!("{} variant number {} is not known", #name_type, variant)).context::<Self>());
                    }
                }
            }
        }
    };

    /*
    for (variant_index, variant) in st.variants.iter().enumerate() {
        let ident = &variant.ident;
        let variant_name = format!("{}", ident);
        let variant_number = attrs.variant_starts_at + variant_index;

        let nb_items = variant.fields.len();

        let variant_type = variant_field(&attrs, &variant);

        // skip array length check in a case of enumint mode
        let de_array_lencheck = if use_array {
            quote! {
                if array.len() != #nb_items + 1 {
                    return Err(::cbored::DecodeErrorKind::Custom(
                        format!("wrong number of items for {}::{} got {} expected {}",
                            #name_type,
                            #variant_name,
                            array.len(),
                            #nb_items + 1)
                        ).context::<Self>()
                    );
                }
            }
        } else {
            quote! {}
        };

        let (parameters, de_fields) = {
            match variant_type {
                VariantType::StructParams { field_names } => {
                    let de_field_names = field_names
                        .iter()
                        .map(|ident| quote! { #ident })
                        .collect::<Vec<_>>();
                    let de_fields = de_field_names
                        .iter()
                        .enumerate()
                        .map(|(fidx, fname)| {
                            let fname_str = format!("{}", fname);
                            if use_array {
                                quote! {
                                    let #fname = array[#fidx + 1].decode().map_err(|e| e.push_str(#fname_str).push_str(#variant_name).push::<Self>())?;
                                }
                            } else {
                                quote! {
                                    let #fname = reader.decode().decode().map_err(|e| e.push_str(#fname_str).push_str(#variant_name).push::<Self>())?;
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                    let parameters = quote! { { #( #de_field_names ),* } };
                    (parameters, de_fields)
                }
                VariantType::AnonParams { field_names } => {
                    let de_field_names = field_names
                        .iter()
                        .map(|(_, ident)| quote! { #ident })
                        .collect::<Vec<_>>();
                    let de_fields = field_names
                        .iter()
                        .map(|(fidx, ident)| {
                            let fname_str = format!("{}", ident);
                            if use_array {
                                quote! {
                                    let #ident = array[#fidx + 1].decode().map_err(|e| e.push_str(#fname_str).push_str(#variant_name).push::<Self>())?;
                                }
                            } else {
                                quote! {
                                    let #ident = reader.decode().map_err(|e| e.push_str(#fname_str).push_str(#variant_name).push::<Self>())?;
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                    let parameters = quote! { ( #( #de_field_names ),* ) };
                    (parameters, de_fields)
                }
                VariantType::NoParams => (quote! {}, vec![]),
            }
        };

        // each branch of deserialization is of the form
        //     X => {
        //          check_len();
        //          get field 0..n;
        //          Ok(Constructor field 0..n)
        //     }
        let de_branch = quote! {
            #variant_number => {
                #de_array_lencheck
                #( #de_fields )*
                Ok(Self::#ident #parameters)
            }
        };
        de_branches.push(de_branch);
    }

    let de_body = quote! {
        #de_array
        match variant {
            #( #de_branches )*
            _ => {
                return Err(::cbored::DecodeErrorKind::Custom(format!("{} variant number {} is not known", #name_type, variant)).context::<Self>());
            }
        }
    };
    */

    token_impl_deserializer(&name, body)
}

pub(crate) fn derive_enum(name: Ident, attrs: &[&Meta], st: DataEnum) -> TokenStream {
    let de = derive_enum_de(&name, attrs, &st);
    let se = derive_enum_se(&name, attrs, &st);
    TokenStream::from(quote! { #de #se })
}
