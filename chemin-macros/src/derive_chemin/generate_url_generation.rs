use super::router::*;
use super::unnamed_param_name;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, Ident};

pub fn url_generation_method(routes: &[Route], chemin_crate: &TokenStream) -> TokenStream {
    let route_match_arms = routes
        .iter()
        .map(|route| route_match_arm(route, chemin_crate));

    quote!(
        fn generate_url(&self, __chemin_locale: &::std::primitive::str) -> ::std::option::Option<::std::string::String> {
            match self { #(#route_match_arms),* }
        }
    )
}

fn route_match_arm(route: &Route, chemin_crate: &TokenStream) -> TokenStream {
    let route_variant_pat = route_variant_pat(route);
    let locale_match_arms = route
        .localized_routes
        .iter()
        .map(|localized_route| locale_match_arm(localized_route, chemin_crate));

    quote!(#route_variant_pat => match __chemin_locale {
        #(#locale_match_arms,)*
        _ => ::std::option::Option::None,
    })
}

fn route_variant_pat(route: &Route) -> TokenStream {
    match &route.variant.fields {
        Fields::Named(fields_named) => {
            let variant_ident = &route.variant.ident;
            let fields = fields_named
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap());
            quote!(Self::#variant_ident { #(#fields),* })
        }

        Fields::Unnamed(fields_unnamed) => {
            let variant_ident = &route.variant.ident;
            let fields = fields_unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| Ident::new(&unnamed_param_name(i), Span::call_site()));
            quote!(Self::#variant_ident(#(#fields),*))
        }

        Fields::Unit => {
            let variant_ident = &route.variant.ident;
            quote!(Self::#variant_ident)
        }
    }
}

fn locale_match_arm(localized_route: &LocalizedRoute, chemin_crate: &TokenStream) -> TokenStream {
    let mut fmt_str = String::new();
    let mut fmt_args = quote!();
    let mut param_i = 0usize;

    for path_component in &localized_route.path.components {
        fmt_str.push('/');

        match path_component {
            PathComponent::Static(value) => fmt_str.push_str(value),

            PathComponent::Param(Some(name)) => {
                fmt_str.push_str("{}");
                let field_ident = Ident::new(name, localized_route.path.span);
                fmt_args = quote!(#fmt_args #field_ident,);
            }

            PathComponent::Param(None) => {
                fmt_str.push_str("{}");
                let field_ident =
                    Ident::new(&unnamed_param_name(param_i), localized_route.path.span);
                fmt_args = quote!(#fmt_args #field_ident,);
                param_i += 1;
            }
        }
    }

    if let Some(sub_route) = &localized_route.path.sub_route {
        fmt_str.push('/');

        let sub_route_ident = match sub_route {
            SubRoute::Unnamed => Ident::new(&param_i.to_string(), localized_route.path.span),
            SubRoute::Named(name) => Ident::new(name, localized_route.path.span),
        };

        fmt_str.push_str("{}");
        fmt_args = quote!(
            #fmt_args
            match #chemin_crate::Chemin::generate_url(#sub_route_ident, __chemin_locale) {
                ::std::option::Option::Some(sub_url) => sub_url,
                ::std::option::Option::None => return ::std::option::Option::None,
            }
        );
    }

    if localized_route.path.trailing_slash {
        fmt_str.push('/');
    }

    let route_locales = localized_route.locales.iter();

    quote!(#(#route_locales)|* => ::std::option::Option::Some(format!(#fmt_str, #fmt_args)))
}