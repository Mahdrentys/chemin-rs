mod router;
use router::*;

use proc_macro2::TokenStream;

pub fn derive_chemin(item: TokenStream) -> TokenStream {
    let _router = match Router::parse(item) {
        Ok(router) => router,
        Err(error) => return error.into_compile_error(),
    };
    proc_macro_error::abort_if_dirty();
    todo!()
}
