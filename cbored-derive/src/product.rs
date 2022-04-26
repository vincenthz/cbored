use proc_macro::TokenStream;

use quote::quote;
use syn::{DataStruct, Fields, FieldsNamed, FieldsUnnamed, Ident, Meta};

use super::attr::*;
use super::common::*;

pub(crate) struct StructAttrs {
    structure_type: StructureType,
    tag: Option<u64>,
}

impl Default for StructAttrs {
    fn default() -> Self {
        StructAttrs {
            structure_type: StructureType::Flat,
            tag: None,
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
        }
        self
    }
}

pub(crate) enum StructOutput {
    /// contains the fields idents
    Unnamed(Vec<Ident>),
    /// contains a made up fields idents, sequenced incrementally from field{n}
    Named(Vec<(Ident, FieldAttrs)>),
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
    match fields {
        Fields::Named(FieldsNamed {
            brace_token: _,
            named,
        }) => {
            let names = named
                .iter()
                .map(|field| {
                    (
                        field.ident.clone().unwrap(),
                        get_my_attributes(&field.attrs)
                            .map(|a| parse_field_attr(&a.parse_meta().expect("field attr")))
                            .fold(FieldAttrs::default(), |acc, y| {
                                y.iter().fold(acc, |acc, y| acc.merge(y))
                            }),
                    )
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
                .map(|(i, _)| quote::format_ident!("field{}", i))
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

    let se_body = match field_names {
        // Generate output for a standard record
        StructOutput::Named(field_names) => {
            let mut se_bodies = Vec::new();

            for (field_name, field_attrs) in field_names.iter() {
                let se_body = if field_attrs.variant == FieldVariantType::Vec {
                    quote! {
                        writer.array_build(::cbored::StructureLength::from(self.#field_name.len() as u64), |iwriter| {
                            for e in self.#field_name.iter() {
                                iwriter.encode(e);
                            }
                        });
                        //writer.encode(&self.#field_name);
                    }
                } else {
                    quote! {
                        writer.encode(&self.#field_name);
                    }
                };
                se_bodies.push(se_body);
            }

            quote! {
                #( #se_bodies )*
            }
        }
        // Generate output for a N-tuple
        StructOutput::Unnamed(field_indexes) => {
            let mut se_bodies = Vec::new();

            for (field_idx, _field_name) in field_indexes.iter().enumerate() {
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
            _ => {
                panic!("map not supported")
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
                let tag_val = ::cbored::StructureLength::from(#n as u64);
                writer.tag_build(tag_val, |writer| { #se_body });
            }
        }
    };

    token_impl_serializer(&name, se_body)
}

pub enum DeStructure {
    Flat,
    Array,
}

// derive CBOR serializer and deserialize for a struct (either tuple or record)
pub(crate) fn derive_struct_de(
    name: &Ident,
    attrs: &StructAttrs,
    st: &DataStruct,
) -> proc_macro2::TokenStream {
    let name_type = format!("{}", name);
    let fields = &st.fields;

    //let attrs = StructAttrs::from_metas(attrs);

    let field_names = get_struct_naming(fields);
    let nb_items = field_names.len();

    let (tag_wrapper, tag_structure) = match attrs.tag {
        None => (quote! {}, None),
        Some(n) => (
            quote! {
                let tag = reader.tag()?;
                match tag.value() {
                    read_tag if read_tag == #n => {}
                    read_tag => {
                        return Err(::cbored::DecodeError::Custom(format!("{} expecting tag {} but got {}", #name_type, #n, read_tag)));
                    }
                };
                let reader = tag.data().reader();
            },
            Some(quote::format_ident!("tag")),
        ),
    };

    let (prelude_sty_de, structure) = match attrs.structure_type {
        StructureType::Flat => (quote! {}, DeStructure::Flat),
        StructureType::Array => {
            let r = match tag_structure {
                None => quote! { let array = reader.array()?; },
                Some(tag_name) => {
                    quote! { let array = #tag_name.read_data(|reader| reader.array())?; }
                }
            };
            (
                quote! {
                    #r
                    if array.len() != #nb_items {
                        return Err(::cbored::DecodeError::Custom(format!("wrong number of items for {} got {} expected {}", #name_type, array.len(), #nb_items)));
                    }
                },
                DeStructure::Array,
            )
        }
        StructureType::Map => {
            panic!("map not supported yet")
        }
    };

    let prelude_deser = quote! { #tag_wrapper #prelude_sty_de };

    let mut de_bodies = Vec::new();

    let de_body = match field_names {
        // Generate output for a standard record
        StructOutput::Named(field_elements) => {
            let field_names = field_elements
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<Ident>>();
            for (field_index, (field_name, field_attrs)) in field_elements.iter().enumerate() {
                let de_body = match structure {
                    DeStructure::Array => {
                        if field_attrs.variant == FieldVariantType::Vec {
                            quote! {
                                let #field_name = {
                                    let mut r = array[#field_index].reader();
                                    let vec = r.array()?
                                        .iter()
                                        .map(|mut r| r.decode())
                                        .collect::<Result<Vec<_>, ::cbored::DecodeError>>()?;
                                    r.expect_finished()?;
                                    vec
                                };
                            }
                        } else {
                            quote! {
                                let #field_name = array[#field_index].decode()?;
                            }
                        }
                    }
                    DeStructure::Flat => quote! {
                        let #field_name = reader.decode()?;
                    },
                };
                de_bodies.push(de_body);
            }

            quote! {
                #prelude_deser
                #( #de_bodies )*
                Ok(#name { #(#field_names),* })
            }
        }
        // Generate output for a N-tuple
        StructOutput::Unnamed(field_indexes) => {
            for (field_index, field_name) in field_indexes.iter().enumerate() {
                let de_body = match structure {
                    DeStructure::Array => quote! {
                        let #field_name = array[#field_index].decode()?;
                    },
                    DeStructure::Flat => quote! {
                        let #field_name = reader.decode()?;
                    },
                };
                de_bodies.push(de_body);
            }

            quote! {
                #prelude_deser
                #( #de_bodies )*
                Ok(#name ( #(#field_indexes),* ))
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
