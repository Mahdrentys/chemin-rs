use super::router::*;
use proc_macro2::TokenStream;
use quote::quote;
use std::iter;
use syn::Ident;

pub fn parsing_method(routes: &[Route], chemin_crate: &TokenStream) -> TokenStream {
    let lazy_type = quote!(#chemin_crate::deps::once_cell::Lazy);
    let router_type = quote!(#chemin_crate::deps::route_recognizer::Router);
    let params_type = quote!(#chemin_crate::deps::route_recognizer::Params);

    let router_entries = router_entries(routes, chemin_crate);

    quote!(
        fn parse_with_accepted_locales(
            url: &::std::primitive::str,
            accepted_locales: &#chemin_crate::AcceptedLocales,
        ) -> ::std::option::Option<(Self, ::std::vec::Vec<#chemin_crate::Locale>)> {
            static ROUTER: #lazy_type<#router_type> = #lazy_type::new(|| {
                let mut router:
                    #router_type<
                        fn(
                            &#chemin_crate::AcceptedLocales,
                            &#params_type
                        ) -> ::std::option::Option<(Self, ::std::vec::Vec<#chemin_crate::Locale>)>
                    >
                    = #router_type::new();
                #(#router_entries)*
                router
            });

            match ROUTER.recognize(url) {
                Some(match_) => match_.handler()(accepted_locales, match_.params()),
                None => None,
            }
        }
    )
}

fn unnamed_param_name(i: usize) -> String {
    format!("p{}", i)
}

static UNNAMED_SUB_ROUTE_NAME: &str = "sub_route";

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

fn router_entries<'a>(
    routes: &'a [Route],
    chemin_crate: &TokenStream,
) -> impl Iterator<Item = TokenStream> + 'a {
    routes.iter().flat_map({
        let chemin_crate = chemin_crate.clone();

        move |route| {
            route.localized_routes.iter().map({
                let chemin_crate = chemin_crate.clone();
                move |localized_route| router_entry(route, localized_route, &chemin_crate)
            })
        }
    })
}

fn router_entry(
    route: &Route,
    localized_route: &LocalizedRoute,
    chemin_crate: &TokenStream,
) -> TokenStream {
    let route_recognizer_path = path_to_route_recognizer_path(&localized_route.path);

    let route_locales = if localized_route.locales.is_empty() {
        quote!(#chemin_crate::RouteLocales::Any)
    } else {
        let route_locales = localized_route.locales.iter();
        quote!(#chemin_crate::RouteLocales::Some(&[#(#route_locales),*]))
    };

    let sub_route_parsing = match &localized_route.path.sub_route {
        Some(sub_route) => sub_route_parsing(sub_route, chemin_crate),
        None => quote!(),
    };

    let route_variant_building = route_variant_building(route, localized_route);

    let resulting_locales = if localized_route.path.sub_route.is_some() {
        quote!(sub_route_resulting_locales)
    } else {
        quote!(accepted_locales.resulting_locales(ROUTE_LOCALES))
    };

    quote!(
        router.add(#route_recognizer_path, |accepted_locales, params| {
            static ROUTE_LOCALES: #chemin_crate::RouteLocales = #route_locales;

            if accepted_locales.accept(ROUTE_LOCALES) {
                #sub_route_parsing
                (#route_variant_building, #resulting_locales)
            } else {
                None
            }
        });
    )
}

fn sub_route_parsing(sub_route: &SubRoute, chemin_crate: &TokenStream) -> TokenStream {
    let sub_route_param_name = match sub_route {
        SubRoute::Unnamed => UNNAMED_SUB_ROUTE_NAME,
        SubRoute::Named(name) => name,
    };

    quote!(
        let sub_route_path = params.find(#sub_route_param_name).unwrap();
        let sub_route_accepted_locales = accepted_locales.accepted_locales_for_sub_route(ROUTE_LOCALES);
        let (sub_route, sub_route_resulting_locales) =
            match #chemin_crate::Chemin::parse_with_accepted_locales(sub_route_path, sub_route_accepted_locales) {
                Some(value) => value,
                None => return None,
            };
    )
}

fn route_variant_building(route: &Route, localized_route: &LocalizedRoute) -> TokenStream {
    fn parsing_code(str_exp: TokenStream) -> TokenStream {
        quote!(match ::std::primitive::str::parse(#str_exp) {
            Ok(value) => value,
            Err(_) => return None,
        })
    }

    if route.named_fields {
        let fields = localized_route
            .path
            .params()
            .map(|param| param.unwrap())
            .map(|param| {
                let field_ident = Ident::new(param, localized_route.path.span);
                let parsing_code = parsing_code(quote!(params.find(#param).unwrap()));
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
        let variant = &route.variant;
        quote!(Self::#variant { #(#fields),* })
    } else {
        let fields = localized_route
            .path
            .params()
            .enumerate()
            .map(|(i, _)| {
                let param_name = unnamed_param_name(i);
                parsing_code(quote!(param.find(#param_name).unwrap()))
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
        let variant = &route.variant;
        quote!(Self::#variant(#(#fields),*))
    }
}
