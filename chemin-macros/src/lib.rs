mod derive_chemin;
mod helpers;

use proc_macro::TokenStream;

fn chemin_crate() -> proc_macro2::TokenStream {
    use proc_macro2::{Ident, Span};
    use proc_macro_crate::FoundCrate;
    use quote::quote;

    match proc_macro_crate::crate_name("chemin").unwrap() {
        FoundCrate::Itself => quote!(crate),

        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
    }
}

#[proc_macro_derive(Chemin, attributes(route))]
pub fn derive_chemin(item: TokenStream) -> TokenStream {
    derive_chemin::derive_chemin(item.into(), &chemin_crate()).into()
}
