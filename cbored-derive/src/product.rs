use proc_macro::TokenStream;

use quote::quote;
use syn::{DataStruct, Fields, FieldsNamed, FieldsUnnamed, Ident, Meta};

use super::attr::*;
use super::common::*;

pub(crate) struct StructAttrs {
    structure_type: StructureType,
    tag: Option<u64>,
    skips: Vec<u64>,
}

impl Default for StructAttrs {
    fn default() -> Self {
        StructAttrs {
            structure_type: StructureType::Flat,
            tag: None,
            skips: Vec::new(),
        }
    }
}

impl StructAttrs {
    pub fn merge(mut self, attr: &Attr) -> Self {
        match attr {
            Attr::Tag(n) => self.tag = Some(*n),
            Attr::Structure(ty) => {
                self.structure_type = *ty;
            }
            Attr::EnumType(_) | Attr::VariantStartsAt(_) => {
                panic!("structure does not support enum type attribute")
            }
            Attr::SkipKey(skip) => self.skips.push(*skip),
        }
        self
    }
}

pub struct Field {
    index: usize,
    name: Ident,
    attrs: FieldAttrs,
}

pub(crate) enum StructOutput {
    /// contains the fields idents
    Unnamed(Vec<Field>),
    /// contains a made up fields idents, sequenced incrementally from field{n}
    Named(Vec<Field>),
}

impl StructOutput {
    pub fn len(&self) -> usize {
        match self {
            StructOutput::Unnamed(v) => v.len(),
            StructOutput::Named(v) => v.len(),
        }
    }
}

fn get_struct_naming(fields: &Fields) -> StructOutput {
    fn attrs(attrs: &Vec<syn::Attribute>) -> FieldAttrs {
        get_my_attributes(attrs)
            .map(|a| parse_field_attr(&a.parse_meta().expect("field attr")))
            .fold(FieldAttrs::default(), |acc, y| {
                y.iter().fold(acc, |acc, y| acc.merge(y))
            })
    }

    match fields {
        Fields::Named(FieldsNamed {
            brace_token: _,
            named,
        }) => {
            let names = named
                .iter()
                .enumerate()
                .map(|(index, field)| Field {
                    index,
                    name: field.ident.clone().unwrap(),
                    attrs: attrs(&field.attrs),
                })
                .collect::<Vec<_>>();
            StructOutput::Named(names)
        }
        Fields::Unnamed(FieldsUnnamed {
            paren_token: _,
            unnamed,
        }) => {
            let indexes = unnamed
                .iter()
                .enumerate()
                .map(|(i, fi)| Field {
                    index: i,
                    name: quote::format_ident!("field{}", i),
                    attrs: attrs(&fi.attrs),
                })
                .collect();
            StructOutput::Unnamed(indexes)
        }
        Fields::Unit => panic!("field unit not supported"),
    }
}

pub(crate) fn derive_struct_se(
    name: &Ident,
    attrs: &StructAttrs,
    st: &DataStruct,
) -> proc_macro2::TokenStream {
    let fields = &st.fields;

    let field_names = get_struct_naming(fields);
    let nb_items = field_names.len();

    let se_body = match &field_names {
        // Generate output for a standard record
        StructOutput::Named(fields) => {
            let mut field_bodies = Vec::new();
            let last_is_opt = if attrs.structure_type == StructureType::ArrayLastOpt {
                true
            } else {
                false
            };
            for field in fields.iter() {
                let Field {
                    index: field_idx,
                    name: field_name,
                    attrs: _,
                } = &field;
                let field_body = if last_is_opt && *field_idx == fields.len() - 1 {
                    quote! {
                        match &self.#field_name {
                            None => (),
                            Some(v) => writer.encode(v),
                        };
                    }
                } else {
                    quote! {
                        writer.encode(&self.#field_name);
                    }
                };
                field_bodies.push(field_body);
            }

            quote! {
                #( #field_bodies )*
            }
        }
        // Generate output for a N-tuple
        StructOutput::Unnamed(fields) => {
            let mut se_bodies = Vec::new();

            for (field_idx, _field_name) in fields.iter().enumerate() {
                let idx = syn::Index::from(field_idx);
                let se_body = quote! {
                    writer.encode(&self.#idx);
                };
                se_bodies.push(se_body);
            }

            quote! {
                #( #se_bodies )*
            }
        }
    };

    // wrap the body inside an array (or nothing if flat representation)
    let se_body = {
        match attrs.structure_type {
            StructureType::Flat => {
                quote! { #se_body }
            }
            StructureType::Array => {
                quote! {
                    writer.array_build(::cbored::StructureLength::from(#nb_items as u64), |writer| {
                        #se_body
                    });
                }
            }
            StructureType::ArrayLastOpt => {
                let last_field = match &field_names {
                    // Generate output for a standard record
                    StructOutput::Named(fields) => fields.last().unwrap().name.clone(),
                    StructOutput::Unnamed(_) => todo!(),
                };
                quote! {
                    let nb_actual_items = (#nb_items as u64 - 1) + match &self.#last_field { None => 0, Some(_) => 1 };
                    writer.array_build(::cbored::StructureLength::from(nb_actual_items), |writer| {
                        #se_body
                    });
                }
            }
            StructureType::MapInt => {
                let mut fixed = 0u64;
                let mut len_for_optionals = Vec::new();
                let mut fields_write_map = Vec::new();

                match field_names {
                    StructOutput::Unnamed(_) => {
                        panic!("map not supported with unnamed fields")
                    }
                    // Generate output for a standard record
                    StructOutput::Named(field_elements) => {
                        let mut rel_index = 0;
                        for field in field_elements.iter() {
                            let Field {
                                index: field_index,
                                name: field_name,
                                attrs: field_attrs,
                            } = &field;
                            loop {
                                let abs_index = *field_index as u64 + rel_index;
                                if attrs.skips.iter().any(|v| *v == abs_index) {
                                    rel_index += 1;
                                } else {
                                    break;
                                }
                            }
                            let abs_index = *field_index as u64 + rel_index;

                            if field_attrs.mandatory_map {
                                fields_write_map.push(quote! {
                                    writer.encode(&(#abs_index as u64));
                                    writer.encode(&self.#field_name);
                                });
                                fixed += 1;
                            } else {
                                fields_write_map.push(quote! {
                                    match &self.#field_name {
                                        None => {},
                                        Some(value) => {
                                            writer.encode(&(#abs_index as u64));
                                            writer.encode(value);
                                        }
                                    }
                                });
                                len_for_optionals.push(quote! {
                                    + match &self.#field_name {
                                        None => 0,
                                        Some(_) => 1,
                                    }
                                });
                            }
                        }
                    }
                };
                quote! {
                    let nb_values : u64 = #fixed #( #len_for_optionals )* ;
                    writer.map_build(::cbored::StructureLength::from(nb_values), |writer| {
                        #( #fields_write_map )*
                    })
                }
            }
        }
    };

    // wrap the body inside a tag if there's a tag attribute
    let se_body = match attrs.tag {
        None => quote! { #se_body },
        Some(n) => {
            if attrs.structure_type == StructureType::Flat && nb_items > 1 {
                panic!("cannot support tag on flat structure with more than 1 element")
            }
            quote! {
                writer.tag_build(::cbored::TagValue::from_u64(#n as u64), |writer| { #se_body });
            }
        }
    };

    token_impl_serializer(&name, se_body)
}

pub enum DeStructure {
    Flat,
    Array { last_optional: bool },
    MapInt,
}

// derive CBOR serializer and deserialize for a struct (either tuple or record)
pub(crate) fn derive_struct_de(
    name: &Ident,
    attrs: &StructAttrs,
    st: &DataStruct,
) -> proc_macro2::TokenStream {
    let fields = &st.fields;

    let field_names = get_struct_naming(fields);
    let nb_items = field_names.len();

    // If the structure has a tag, create a reader from the inside of the tag, otherwise use the original reader
    //
    // context:
    // * input/output:
    //   * 'reader' which is CBOR reader, and replace by another 'reader' conditionally (when it's tagged)
    let (tag_wrapper, tag_structure) = match attrs.tag {
        None => (quote! {}, false),
        Some(n) => (
            quote! {
                let tag = reader
                    .tag()
                    .map_err(::cbored::DecodeErrorKind::ReaderError)
                    .map_err(|e| e.context::<Self>())?;
                match tag.value() {
                    read_tag if read_tag == #n => {}
                    read_tag => {
                        return Err(::cbored::DecodeErrorKind::ReaderError(::cbored::ReaderError::WrongExpectedTag {
                            expected: #n,
                            got: read_tag,
                        }).context::<Self>());
                    }
                };
                //let reader = tag.data().reader();
            },
            true,
        ),
    };

    // Destructure the CBOR either not an array (StructureType::Flat),
    // from an array (StructureType::Array), or from a map (StructureType::Map).
    //
    // When from an array, check that the number of expected element match the number of element in the structure
    // (1 field or 1 tuple element == 1 CBOR element)
    //
    // context:
    // * input:
    //   * 'reader' which is CBOR reader
    // * output:
    //   * 'array' which is CBOR Array if StructureType::Array
    let (prelude_sty_de, structure) = match attrs.structure_type {
        StructureType::Flat => (quote! {}, DeStructure::Flat),
        StructureType::Array | StructureType::ArrayLastOpt => {
            let r = if tag_structure {
                quote! {
                    #tag_wrapper
                    let array = tag.read_data(|reader| reader.array()).map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?;
                }
            } else {
                quote! { let array = reader.array().map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?; }
            };
            if attrs.structure_type == StructureType::ArrayLastOpt {
                (
                    quote! {
                        #r
                        if array.len() != #nb_items && array.len() != #nb_items - 1 {
                            return Err(::cbored::DecodeErrorKind::Custom(format!("wrong number of items got {} expected {} or {}", array.len(), #nb_items, #nb_items - 1)).context::<Self>());
                        }
                    },
                    DeStructure::Array {
                        last_optional: true,
                    },
                )
            } else {
                (
                    quote! {
                        #r
                        if array.len() != #nb_items {
                            return Err(::cbored::DecodeErrorKind::Custom(format!("wrong number of items got {} expected {}", array.len(), #nb_items)).context::<Self>());
                        }
                    },
                    DeStructure::Array {
                        last_optional: false,
                    },
                )
            }
        }
        StructureType::MapInt => {
            let r = if tag_structure {
                quote! {
                    #tag_wrapper
                    let map = tag.read_data(|reader| reader.map()).map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?;
                }
            } else {
                quote! { let map = reader.map().map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.context::<Self>())?; }
            };
            (
                quote! {
                    #r
                },
                DeStructure::MapInt,
            )
        }
    };

    let prelude_deser = quote! { #prelude_sty_de };

    let mut de_bodies = Vec::new();

    // create the quote that deserialize all the elements one by one
    let de_body = match field_names {
        // Generate output for a standard record
        StructOutput::Named(fields) => {
            let field_names = fields
                .iter()
                .map(|x| x.name.clone())
                .collect::<Vec<Ident>>();
            match structure {
                DeStructure::Array { last_optional } => {
                    // deserialize each named field
                    for field in fields.iter() {
                        let Field {
                            index: field_index,
                            name: field_name,
                            attrs: field_attrs,
                        } = &field;
                        let field_index = *field_index;
                        let field_name_str = format!("{}", field_name);
                        let de_body = if field_attrs.variant == FieldVariantType::Vec {
                            quote! {
                                let #field_name = {
                                    let mut r = array[#field_index].reader();
                                    let vec = r.array().map_err(::cbored::DecodeErrorKind::ReaderError).map_err(|e| e.push::<Self>())?
                                        .iter()
                                        .map(|mut r| r.decode())
                                        .collect::<Result<Vec<_>, ::cbored::DecodeError>>()?;
                                    vec
                                };
                            }
                        } else {
                            if last_optional && field_index == fields.len() - 1 {
                                quote! {
                                    let #field_name = if array.len() == #field_index - 1 {
                                        Some(array[#field_index].decode().map_err(|e| e.push_str(#field_name_str).push::<Self>())?)
                                    } else {
                                        None
                                    };
                                }
                            } else {
                                quote! {
                                    let #field_name = array[#field_index].decode().map_err(|e| e.push_str(#field_name_str).push::<Self>())?;
                                }
                            }
                        };
                        de_bodies.push(de_body);
                    }

                    quote! {
                        #prelude_deser
                        #( #de_bodies )*
                        Ok(#name { #(#field_names),* })
                    }
                }
                DeStructure::MapInt => {
                    if nb_items > 64 {
                        panic!("cannot support structure with more than 64 fields");
                    }

                    let mut keydefs = Vec::new();
                    let mut keyfields = Vec::new();
                    let mut mandatory_keys = Vec::new();

                    let mut rel_index = 0;

                    for field in fields.iter() {
                        let Field {
                            index: field_index,
                            name: field_name,
                            attrs: field_attrs,
                        } = &field;
                        let field_index = *field_index;

                        loop {
                            let abs_index = field_index as u64 + rel_index;
                            if attrs.skips.iter().any(|v| *v == abs_index) {
                                rel_index += 1;
                            } else {
                                break;
                            }
                        }
                        let abs_index = field_index as u64 + rel_index;
                        let field_name_str = format!("{}", field_name);
                        let keydef = quote! {
                            let mut #field_name = None;
                        };
                        let keyfield = quote! {
                            #abs_index => {
                                #field_name = Some(v.decode().map_err(|e| e.push_str(#field_name_str).push::<Self>())?);
                            }
                        };
                        keydefs.push(keydef);
                        keyfields.push(keyfield);

                        let key_mandatory = field_attrs.mandatory_map;

                        if key_mandatory {
                            let mandatory_key = quote! {
                                let #field_name = match #field_name {
                                    None => {
                                        return Err(cbored::DecodeErrorKind::Custom(format!("missing {}", #field_name_str)).context::<Self>());
                                    }
                                    Some(value) => {
                                        value
                                    }
                                };
                            };
                            mandatory_keys.push(mandatory_key);
                        }
                    }

                    quote! {
                        #prelude_sty_de

                        #( #keydefs )*

                        let mut found_keys = 0;
                        for (mut k, mut v) in map.iter() {
                            let key: u64 = k.decode().map_err(|e| e.push::<Self>())?;

                            if (found_keys & (1 << key)) != 0 {
                                return Err(cbored::DecodeErrorKind::Custom(format!("duplicated key {}", key)).context::<Self>());
                            } else {
                                found_keys |= 1 << key;
                            }

                            match key {
                                #( #keyfields )*
                                // handle unknown keys
                                _ => {
                                    return Err(cbored::DecodeErrorKind::Custom(format!(
                                            "unknown key {}",
                                            key
                                        )).context::<Self>());
                                }
                            }
                        }

                        #( #mandatory_keys )*

                        Ok(#name { #(#field_names),*})
                    }
                }
                DeStructure::Flat => {
                    // deserialize each named field
                    for field in fields.iter() {
                        let Field {
                            index: _,
                            name: field_name,
                            attrs: _,
                        } = &field;
                        let field_name_str = format!("{}", field_name);
                        let de_body = quote! {
                            let #field_name = reader.decode().map_err(|e| e.push_str(#field_name_str))?;
                        };
                        de_bodies.push(de_body);
                    }

                    quote! {
                        #prelude_deser
                        #( #de_bodies )*
                        Ok(#name { #(#field_names),* })
                    }
                }
            }
        }
        // Generate output for a N-tuple
        StructOutput::Unnamed(fields) => {
            // deserialize each unnamed field
            for field in fields.iter() {
                let Field {
                    index: field_index,
                    name: field_name,
                    attrs: _,
                } = &field;
                let field_index = *field_index;
                let field_name_str = format!("{}", field_name);
                let de_body = match structure {
                    DeStructure::Array { last_optional: _ } => quote! {
                        let #field_name = array[#field_index].decode().map_err(|e| e.push_str(#field_name_str))?;
                    },
                    DeStructure::MapInt => {
                        todo!()
                    }
                    DeStructure::Flat => quote! {
                        let #field_name = reader.decode().map_err(|e| e.push_str(#field_name_str))?;
                    },
                };
                de_bodies.push(de_body);
            }

            let indexes = fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>();

            quote! {
                #prelude_deser
                #( #de_bodies )*
                Ok(#name ( #(#indexes),* ))
            }
        }
    };

    token_impl_deserializer(&name, de_body)
}

pub(crate) fn derive_struct(name: Ident, attrs: &[Meta], st: DataStruct) -> TokenStream {
    let attrs = attrs
        .iter()
        .map(|meta| parse_attr(meta))
        .fold(StructAttrs::default(), |acc, y| {
            y.iter().fold(acc, |x, y| x.merge(y))
        });

    let se = derive_struct_se(&name, &attrs, &st);
    let de = derive_struct_de(&name, &attrs, &st);
    TokenStream::from(quote! { #se #de })
}
