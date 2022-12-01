use super::router::*;
use super::unnamed_param_name;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::iter;
use syn::{Fields, Ident};

static UNNAMED_SUB_ROUTE_NAME: &str = "sub_route";

pub fn parsing_method(routes: &[Route], chemin_crate: &TokenStream) -> TokenStream {
    let lazy_type = quote!(#chemin_crate::deps::once_cell::sync::Lazy);
    let router_type = quote!(#chemin_crate::deps::route_recognizer::Router);

    let router_entries = router_entries(routes);
    let route_handlers = route_handlers(routes, chemin_crate);

    quote!(
        fn parse_with_accepted_locales(
            url: &::std::primitive::str,
            accepted_locales: &#chemin_crate::AcceptedLocales,
            decode_params: ::std::primitive::bool,
        ) -> ::std::option::Option<(Self, ::std::vec::Vec<#chemin_crate::Locale>)> {
            static ROUTER: #lazy_type<#router_type<u32>> = #lazy_type::new(|| {
                let mut router = #router_type::new();
                #router_entries
                router
            });

            match ROUTER.recognize(url) {
                ::std::result::Result::Ok(match_) => {
                    let params = match_.params();
                    match *match_.handler() {
                        #route_handlers
                        _ => ::std::option::Option::None
                    }
                },

                ::std::result::Result::Err(_) => ::std::option::Option::None,
            }
        }
    )
}

fn router_entries(routes: &[Route]) -> TokenStream {
    let mut router_entries = quote!();
    let mut i = 0u32;

    for route in routes {
        for localized_route in &route.localized_routes {
            let route_recognizer_path = path_to_route_recognizer_path(&localized_route.path);
            router_entries = quote!(
                #router_entries
                router.add(#route_recognizer_path, #i);
            );
            i += 1;
        }
    }

    router_entries
}

fn path_to_route_recognizer_path(path: &Path) -> String {
    let mut route_recognizer_path = String::new();
    let mut param_i = 0usize;

    for component in &path.components {
        match component {
            PathComponent::Static(value) => {
                route_recognizer_path.push('/');
                route_recognizer_path.push_str(value);
            }

            PathComponent::Param(name) => {
                route_recognizer_path.push_str("/:");
                match name {
                    Some(name) => route_recognizer_path.push_str(name),
                    None => route_recognizer_path.push_str(&unnamed_param_name(param_i)),
                }
                param_i += 1;
            }
        }
    }

    if let Some(sub_route) = &path.sub_route {
        route_recognizer_path.push_str("/*");

        match sub_route {
            SubRoute::Unnamed => route_recognizer_path.push_str(UNNAMED_SUB_ROUTE_NAME),
            SubRoute::Named(name) => route_recognizer_path.push_str(name),
        }
    }

    if path.trailing_slash {
        route_recognizer_path.push('/');
    }

    route_recognizer_path
}

fn route_handlers(routes: &[Route], chemin_crate: &TokenStream) -> TokenStream {
    let mut route_handlers = quote!();
    let mut i = 0u32;

    for route in routes {
        for localized_route in &route.localized_routes {
            let route_handler = route_handler(route, localized_route, chemin_crate);
            route_handlers = quote!(#route_handlers #i => #route_handler,);
            i += 1;
        }
    }

    route_handlers
}

fn route_handler(
    route: &Route,
    localized_route: &LocalizedRoute,
    chemin_crate: &TokenStream,
) -> TokenStream {
    let route_locales = if localized_route.locales.is_empty() {
        quote!(#chemin_crate::RouteLocales::Any)
    } else {
        let route_locales = localized_route.locales.iter();
        quote!(#chemin_crate::RouteLocales::Some(&[#(#route_locales),*]))
    };

    let sub_route_parsing = match &localized_route.path.sub_route {
        Some(sub_route) => sub_route_parsing(localized_route, sub_route, chemin_crate),
        None => quote!(),
    };

    let route_variant_building = route_variant_building(route, localized_route, chemin_crate);

    let resulting_locales = if localized_route.path.sub_route.is_some() {
        quote!(sub_route_resulting_locales)
    } else {
        quote!(accepted_locales.resulting_locales(&ROUTE_LOCALES))
    };

    quote!({
        static ROUTE_LOCALES: #chemin_crate::RouteLocales = #route_locales;

        if accepted_locales.accept(&ROUTE_LOCALES) {
            #sub_route_parsing
            ::std::option::Option::Some((#route_variant_building, #resulting_locales))
        } else {
            ::std::option::Option::None
        }
    })
}

fn sub_route_parsing(
    localized_route: &LocalizedRoute,
    sub_route: &SubRoute,
    chemin_crate: &TokenStream,
) -> TokenStream {
    let sub_route_param_name = match sub_route {
        SubRoute::Unnamed => UNNAMED_SUB_ROUTE_NAME,
        SubRoute::Named(name) => name,
    };

    quote_spanned!(localized_route.path.span=>
        let sub_route_path = params.find(#sub_route_param_name).unwrap();
        let sub_route_accepted_locales = accepted_locales.accepted_locales_for_sub_route(&ROUTE_LOCALES);
        let (sub_route, sub_route_resulting_locales) =
            match #chemin_crate::Chemin::parse_with_accepted_locales(sub_route_path, &sub_route_accepted_locales, decode_params) {
                ::std::option::Option::Some(value) => value,
                ::std::option::Option::None => return ::std::option::Option::None,
            };
    )
}

fn route_variant_building(
    route: &Route,
    localized_route: &LocalizedRoute,
    chemin_crate: &TokenStream,
) -> TokenStream {
    fn parsing_code(str_exp: TokenStream, span: Span, chemin_crate: &TokenStream) -> TokenStream {
        quote_spanned!(span=> {
            let value = if decode_params {
                match #chemin_crate::decode_param(#str_exp) {
                    Some(value) => value,
                    None => return None,
                }
            } else {
                ::std::borrow::Cow::Borrowed(#str_exp)
            };

            match ::std::primitive::str::parse(&value) {
                Ok(value) => value,
                Err(_) => return ::std::option::Option::None,
            }
        })
    }

    match route.variant.fields {
        Fields::Named(_) => {
            let fields = localized_route
                .path
                .params()
                .map(|param| param.unwrap())
                .map(|param| {
                    let field_ident = Ident::new(param, localized_route.path.span);
                    let parsing_code = parsing_code(
                        quote!(params.find(#param).unwrap()),
                        localized_route.path.span,
                        chemin_crate,
                    );
                    quote!(#field_ident: #parsing_code)
                })
                .chain(match &localized_route.path.sub_route {
                    Some(sub_route) => match sub_route {
                        SubRoute::Unnamed => unreachable!(),
                        SubRoute::Named(name) => {
                            let field_ident = Ident::new(name, localized_route.path.span);
                            Box::new(iter::once(quote!(#field_ident: sub_route)))
                                as Box<dyn Iterator<Item = _>>
                        }
                    },

                    None => Box::new(iter::empty()) as Box<dyn Iterator<Item = _>>,
                });
            let variant_ident = &route.variant.ident;
            quote_spanned!(localized_route.path.span=> Self::#variant_ident { #(#fields),* })
        }

        Fields::Unnamed(_) => {
            let fields = localized_route
                .path
                .params()
                .enumerate()
                .map(|(i, _)| {
                    let param_name = unnamed_param_name(i);
                    parsing_code(
                        quote!(params.find(#param_name).unwrap()),
                        localized_route.path.span,
                        chemin_crate,
                    )
                })
                .chain(match &localized_route.path.sub_route {
                    Some(sub_route) => match sub_route {
                        SubRoute::Unnamed => {
                            Box::new(iter::once(quote!(sub_route))) as Box<dyn Iterator<Item = _>>
                        }

                        SubRoute::Named(_) => unreachable!(),
                    },

                    None => Box::new(iter::empty()) as Box<dyn Iterator<Item = _>>,
                });
            let variant_ident = &route.variant.ident;
            quote_spanned!(localized_route.path.span=> Self::#variant_ident(#(#fields),*))
        }

        Fields::Unit => {
            let variant_ident = &route.variant.ident;
            quote!(Self::#variant_ident)
        }
    }
}
