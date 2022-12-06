mod localized_route;
pub use localized_route::*;
use quote::ToTokens;

use crate::helpers;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseBuffer};
use syn::spanned::Spanned;
use syn::{parenthesized, Error, Expr, Fields, Ident, ItemEnum, Token, Variant};

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
    pub query_params: Vec<QueryParam>,
}

impl Route {
    fn from_variant(variant: &Variant) -> syn::Result<Self> {
        let mut route = Route {
            variant: variant.clone(),
            localized_routes: Vec::new(),
            query_params: Vec::new(),
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

        for field in &variant.fields {
            if let Some(attr) = field
                .attrs
                .iter()
                .find(|attr| attr.path.is_ident("query_param"))
            {
                match &field.ident {
                    Some(field_ident) => {
                        let mut token_stream_to_parse = field_ident.into_token_stream();
                        token_stream_to_parse.extend(attr.tokens.clone().into_iter());
                        route.query_params.push(syn::parse2(token_stream_to_parse)?);
                    }

                    None => {
                        return Err(Error::new(
                            attr.path.span(),
                            "Only named fields can be query string parameters",
                        ))
                    }
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

#[derive(PartialEq, Eq, Debug)]
pub enum QueryParam {
    Mandatory(Ident),
    Optional(Ident),
    WithDefaultValue(Ident, Expr),
}

impl Parse for QueryParam {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let field_ident: Ident = input.parse()?;

        if input.is_empty() {
            Ok(Self::Mandatory(field_ident))
        } else {
            let content;
            parenthesized!(content in input);
            helpers::parse_eos(input)?;
            let ident: Ident = content.parse()?;

            if ident == "optional" {
                helpers::parse_eos(&content)?;
                Ok(Self::Optional(field_ident))
            } else if ident == "default" {
                content.parse::<Token![=]>()?;
                let default_value = content.parse()?;
                helpers::parse_eos(&content)?;
                Ok(Self::WithDefaultValue(field_ident, default_value))
            } else {
                Err(Error::new(
                    ident.span(),
                    "Expected `optional` or `default = ...`",
                ))
            }
        }
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
                HelloWithNamedFields {
                    name: String,
                    age: u8,
                    #[query_param]
                    param: String,
                },

                #[route("/hello/:/..")]
                HelloSubRoute(String, SubRoute),

                #[route("/hello/:name/..sub_route")]
                HelloSubRouteWithNamedFields {
                    #[query_param(default = String::from("default"))]
                    name: String,
                    sub_route: SubRoute,
                    #[query_param(optional)]
                    param: Option<String>,
                },
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
                query_params: vec![],
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
                query_params: vec![],
            },
            Route {
                variant: syn::parse2(quote!(
                    #[route("/hello/:name/:age")]
                    HelloWithNamedFields {
                        name: String,
                        age: u8,
                        #[query_param]
                        param: String,
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
                query_params: vec![QueryParam::Mandatory(Ident::new(
                    "param",
                    Span::call_site()
                ))],
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
                query_params: vec![],
            },
            Route {
                variant: syn::parse2(quote!(
                    #[route("/hello/:name/..sub_route")]
                    HelloSubRouteWithNamedFields {
                        #[query_param(default = String::from("default"))]
                        name: String,
                        sub_route: SubRoute,
                        #[query_param(optional)]
                        param: Option<String>,
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
                query_params: vec![
                    QueryParam::WithDefaultValue(
                        Ident::new("name", Span::call_site()),
                        syn::parse2(quote!(String::from("default"))).unwrap()
                    ),
                    QueryParam::Optional(Ident::new("param", Span::call_site()))
                ],
            },
        ]
    );
}
