use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) fn token_impl_deserializer(class_name: &Ident, body: TokenStream) -> TokenStream {
    let stringified_class_name = format!("{}", class_name);
    quote! {
        impl ::cbored::Decode for #class_name {
            #[tracing::instrument(skip_all, err, name = #stringified_class_name)]
            fn decode<'a>(reader: &mut ::cbored::Reader<'a>) -> Result<Self, ::cbored::DecodeError> {
                #body
            }
        }
    }
}

pub(crate) fn token_impl_serializer(class_name: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::cbored::Encode for #class_name {
            fn encode(&self, writer: &mut ::cbored::Writer) {
                #body
            }
        }
    }
}
