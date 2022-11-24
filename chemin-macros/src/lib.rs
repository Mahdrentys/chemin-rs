mod derive_chemin;
mod helpers;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_derive(Chemin, attributes(route))]
#[proc_macro_error]
pub fn derive_chemin(item: TokenStream) -> TokenStream {
    derive_chemin::derive_chemin(item.into()).into()
}
