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
        fn generate_url(
            &self,
            __chemin_locale: ::std::option::Option<&::std::primitive::str>,
            __chemin_encode_params: ::std::primitive::bool,
        ) -> ::std::option::Option<::std::string::String> {
            match self {
                #(#route_match_arms),*
                _ => ::std::option::Option::None,
            }
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
    let mut non_encoded_fmt_args = quote!();
    let mut encoded_fmt_args = quote!();
    let mut param_i = 0usize;

    for path_component in &localized_route.path.components {
        fmt_str.push('/');

        match path_component {
            PathComponent::Static(value) => fmt_str.push_str(value),

            PathComponent::Param(optional_name) => {
                fmt_str.push_str("{}");

                let field_ident = match optional_name {
                    Some(name) => Ident::new(name, localized_route.path.span),
                    None => Ident::new(&unnamed_param_name(param_i), localized_route.path.span),
                };

                non_encoded_fmt_args = quote!(#non_encoded_fmt_args #field_ident,);
                encoded_fmt_args =
                    quote!(#encoded_fmt_args #chemin_crate::encode_param(#field_ident),);

                param_i += 1;
            }
        }
    }

    if let Some(sub_route) = &localized_route.path.sub_route {
        // We don't do `fmt_str.push('/');` because the url generated by the sub_route is guaranteed to contain a "/" at the beginning.

        let sub_route_ident = match sub_route {
            SubRoute::Unnamed => {
                Ident::new(&unnamed_param_name(param_i), localized_route.path.span)
            }
            SubRoute::Named(name) => Ident::new(name, localized_route.path.span),
        };

        fmt_str.push_str("{}");

        let sub_route_url_generation = quote!(
            match #chemin_crate::Chemin::generate_url(#sub_route_ident, __chemin_locale, __chemin_encode_params) {
                ::std::option::Option::Some(sub_url) => sub_url,
                ::std::option::Option::None => return ::std::option::Option::None,
            }
        );
        non_encoded_fmt_args = quote!(#non_encoded_fmt_args #sub_route_url_generation);
        encoded_fmt_args = quote!(#encoded_fmt_args #sub_route_url_generation);
    }

    if localized_route.path.trailing_slash {
        fmt_str.push('/');
    }

    let match_arm_pat = if localized_route.locales.is_empty() {
        quote!(_)
    } else {
        let route_locales = localized_route.locales.iter();
        quote!(#(::std::option::Option::Some(#route_locales))|*)
    };

    quote!(#match_arm_pat => ::std::option::Option::Some(
        if __chemin_encode_params {
            format!(#fmt_str, #encoded_fmt_args)
        } else {
            format!(#fmt_str, #non_encoded_fmt_args)
        }
    ))
}
