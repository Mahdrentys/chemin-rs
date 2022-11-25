mod router;
use router::*;
mod generate_url_parsing;

use proc_macro2::TokenStream;
use quote::quote;

pub fn derive_chemin(item: TokenStream, chemin_crate: &TokenStream) -> TokenStream {
    let Router { item_enum, routes } = match Router::parse(item) {
        Ok(router) => router,
        Err(error) => return error.into_compile_error(),
    };
    proc_macro_error::abort_if_dirty();

    let enum_ident = &item_enum.ident;
    let (impl_generics, ty_generics, where_clause) = item_enum.generics.split_for_impl();
    let parsing_method = generate_url_parsing::parsing_method(&routes, chemin_crate);

    quote!(
        impl #impl_generics #chemin_crate::Chemin for #enum_ident #ty_generics #where_clause {
            #parsing_method
        }
    )
}
