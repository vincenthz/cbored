use proc_macro::TokenStream;
use syn::{Data, DeriveInput};

mod attr;
mod common;
mod product;
mod sum;

use attr::get_my_attributes;
use product::derive_struct;
use sum::derive_enum;

#[proc_macro_derive(CborRepr, attributes(cborrepr))]
pub fn derive_cbor_repr(input: TokenStream) -> TokenStream {
    // Parse type (struct/enum)
    let ast = syn::parse_macro_input!(input as DeriveInput);

    // Gather the cborrepr attributes as Meta
    let attrs = get_my_attributes(&ast.attrs).collect::<Vec<_>>();

    // either do struct or enum handling
    match ast.data {
        Data::Struct(st) => derive_struct(ast.ident, &attrs, st),
        Data::Enum(e) => derive_enum(ast.ident, &attrs, e),
        Data::Union(_) => panic!("Union not supported"),
    }
}
