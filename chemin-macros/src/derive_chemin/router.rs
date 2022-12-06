mod localized_route;
pub use localized_route::*;

use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::{Error, Fields, ItemEnum, Variant};

pub struct Router {
    pub item_enum: ItemEnum,
    pub routes: Vec<Route>,
}

impl Router {
    pub fn parse(item: TokenStream) -> syn::Result<Self> {
        let item_enum: ItemEnum = syn::parse2(item)?;
        Ok(Self {
            routes: item_enum
                .variants
                .iter()
                .map(Route::from_variant)
                .collect::<syn::Result<Vec<Route>>>()?,
            item_enum,
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Route {
    pub variant: Variant,
    pub localized_routes: Vec<LocalizedRoute>,
}

impl Route {
    fn from_variant(variant: &Variant) -> syn::Result<Self> {
        fn validate_localized_route(
            localized_route: &LocalizedRoute,
            variant: &Variant,
            span: Span,
        ) -> syn::Result<()> {
            match variant.fields {
                Fields::Named(_) => {
                    if localized_route
                        .path
                        .contains_unnamed_params_and_sub_routes()
                    {
                        Err(Error::new(
                            span,
                            "This route can only have named params and sub-routes, because this enum variant has named fields",
                        ))
                    } else {
                        Ok(())
                    }
                }

                Fields::Unit | Fields::Unnamed(_) => {
                    if localized_route.path.contains_named_params_and_sub_routes() {
                        Err(Error::new(
                            span,
                            "This route can only have unnamed params and sub-routes, because this enum variant has unnamed fields",
                        ))
                    } else {
                        let number_of_params_and_sub_routes = localized_route.path.params().count()
                            + localized_route.path.sub_route.is_some() as usize;

                        if number_of_params_and_sub_routes == variant.fields.len() {
                            Ok(())
                        } else {
                            Err(Error::new(
                                span,
                                format!(
                                    "This route has {} unnamed params and sub-routes, but this enum variant has {} fields",
                                    number_of_params_and_sub_routes,
                                    variant.fields.len(),
                                ),
                            ))
                        }
                    }
                }
            }
        }

        let mut route = Route {
            variant: variant.clone(),
            localized_routes: Vec::new(),
        };

        for attr in &variant.attrs {
            if attr.path.is_ident("route") {
                let new_localized_route: LocalizedRoute = syn::parse2(attr.tokens.clone())?;
                validate_localized_route(&new_localized_route, variant, attr.tokens.span())?;

                if new_localized_route
                    .locales
                    .iter()
                    .any(|locale| route.accepts_locale(locale))
                {
                    return Err(Error::new(
                        attr.tokens.span(),
                        "You cannot define multiple routes for the same locale",
                    ));
                }

                match route
                    .localized_routes
                    .iter_mut()
                    .find(|localized_route| localized_route.path == new_localized_route.path)
                {
                    Some(localized_route) => localized_route
                        .locales
                        .extend(new_localized_route.locales.into_iter()),

                    None => route.localized_routes.push(new_localized_route),
                }
            }
        }

        if route.localized_routes.is_empty() {
            return Err(Error::new(
                variant.span(),
                "Every variant must have at least one route",
            ));
        }

        Ok(route)
    }

    fn accepts_locale(&self, locale: &str) -> bool {
        self.localized_routes
            .iter()
            .any(|localized_route| localized_route.locales.contains(locale))
    }
}

#[test]
fn test_parsing() {
    use maplit::hashset;
    use quote::quote;

    assert_eq!(
        Router::parse(quote!(
            enum Router {
                #[route("/")]
                Home,

                #[route(en => "/hello/:")]
                #[route(fr => "/bonjour/:")]
                #[route(another_one, yet_another_one => "/hello/:")]
                #[route(en_US => "/hello/:/")]
                Hello(String),

                #[route("/hello/:name/:age")]
                HelloWithNamedFields { name: String, age: u8 },

                #[route("/hello/:/..")]
                HelloSubRoute(String, SubRoute),

                #[route("/hello/:name/..sub_route")]
                HelloSubRouteWithNamedFields { name: String, sub_route: SubRoute },
            }
        ))
        .unwrap()
        .routes,
        vec![
            Route {
                variant: syn::parse2(quote!(
                    #[route("/")]
                    Home
                ))
                .unwrap(),
                localized_routes: vec![LocalizedRoute {
                    path: Path {
                        components: vec![],
                        sub_route: None,
                        trailing_slash: true,
                        span: Span::call_site(),
                    },
                    locales: hashset![],
                }],
            },
            Route {
                variant: syn::parse2(quote!(
                    #[route(en => "/hello/:")]
                    #[route(fr => "/bonjour/:")]
                    #[route(another_one, yet_another_one => "/hello/:")]
                    #[route(en_US => "/hello/:/")]
                    Hello(String)
                ))
                .unwrap(),
                localized_routes: vec![
                    LocalizedRoute {
                        path: Path {
                            components: vec![
                                PathComponent::Static(String::from("hello")),
                                PathComponent::Param(None),
                            ],
                            sub_route: None,
                            trailing_slash: false,
                            span: Span::call_site(),
                        },
                        locales: hashset![
                            String::from("another-one"),
                            String::from("yet-another-one"),
                            String::from("en"),
                        ],
                    },
                    LocalizedRoute {
                        path: Path {
                            components: vec![
                                PathComponent::Static(String::from("bonjour")),
                                PathComponent::Param(None),
                            ],
                            sub_route: None,
                            trailing_slash: false,
                            span: Span::call_site(),
                        },
                        locales: hashset![String::from("fr")],
                    },
                    LocalizedRoute {
                        path: Path {
                            components: vec![
                                PathComponent::Static(String::from("hello")),
                                PathComponent::Param(None),
                            ],
                            sub_route: None,
                            trailing_slash: true,
                            span: Span::call_site(),
                        },
                        locales: hashset![String::from("en-US"),],
                    },
                ],
            },
            Route {
                variant: syn::parse2(quote!(
                    #[route("/hello/:name/:age")]
                    HelloWithNamedFields {
                        name: String,
                        age: u8
                    }
                ))
                .unwrap(),
                localized_routes: vec![LocalizedRoute {
                    path: Path {
                        components: vec![
                            PathComponent::Static(String::from("hello")),
                            PathComponent::Param(Some(String::from("name"))),
                            PathComponent::Param(Some(String::from("age"))),
                        ],
                        sub_route: None,
                        trailing_slash: false,
                        span: Span::call_site(),
                    },
                    locales: hashset![],
                }],
            },
            Route {
                variant: syn::parse2(quote!(
                    #[route("/hello/:/..")]
                    HelloSubRoute(String, SubRoute)
                ))
                .unwrap(),
                localized_routes: vec![LocalizedRoute {
                    path: Path {
                        components: vec![
                            PathComponent::Static(String::from("hello")),
                            PathComponent::Param(None),
                        ],
                        sub_route: Some(SubRoute::Unnamed),
                        trailing_slash: false,
                        span: Span::call_site(),
                    },
                    locales: hashset![],
                }],
            },
            Route {
                variant: syn::parse2(quote!(
                    #[route("/hello/:name/..sub_route")]
                    HelloSubRouteWithNamedFields {
                        name: String,
                        sub_route: SubRoute
                    }
                ))
                .unwrap(),
                localized_routes: vec![LocalizedRoute {
                    path: Path {
                        components: vec![
                            PathComponent::Static(String::from("hello")),
                            PathComponent::Param(Some(String::from("name"))),
                        ],
                        sub_route: Some(SubRoute::Named(String::from("sub_route"))),
                        trailing_slash: false,
                        span: Span::call_site(),
                    },
                    locales: hashset![],
                }],
            },
        ]
    );
}
