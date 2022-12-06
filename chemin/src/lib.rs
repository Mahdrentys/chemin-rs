pub use chemin_macros::Chemin;

use percent_encoding::AsciiSet;
use qstring::QString;
use smallvec::{SmallVec, ToSmallVec};
use std::borrow::Cow;
use std::fmt::Display;

#[doc(hidden)]
pub mod deps {
    pub use once_cell;
    pub use qstring;
    pub use route_recognizer;
}

pub trait Chemin: Sized {
    fn parse(url: &str, decode_params: bool) -> Option<(Self, Vec<Locale>)> {
        let mut split = url.split('?').peekable();
        let path = split.next()?;

        let qstring = if split.peek().is_none() {
            QString::default()
        } else {
            let qstring = split
                .fold(String::new(), |mut qstring, fragment| {
                    qstring.push('?');
                    qstring.push_str(fragment);
                    qstring
                })
                .replace('+', "%20");
            QString::from(&qstring[..])
        };

        Self::parse_with_accepted_locales(path, &AcceptedLocales::Any, decode_params, &qstring)
    }

    fn parse_with_accepted_locales(
        path: &str,
        accepted_locales: &AcceptedLocales,
        decode_params: bool,
        qstring: &QString,
    ) -> Option<(Self, Vec<Locale>)>;

    fn generate_url(&self, locale: Option<&str>, encode_params: bool) -> Option<String> {
        let mut qstring = QString::default();

        self.generate_url_and_build_qstring(locale, encode_params, &mut qstring)
            .map(|mut value| {
                if qstring.is_empty() {
                    value
                } else {
                    value.push('?');
                    value.push_str(&qstring.to_string().replace("%20", "+"));
                    value
                }
            })
    }

    fn generate_url_and_build_qstring(
        &self,
        locale: Option<&str>,
        encode_params: bool,
        qstring: &mut QString,
    ) -> Option<String>;
}

pub type Locale = &'static str;

#[doc(hidden)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub enum AcceptedLocales {
    Any,
    Some(SmallVec<[Locale; 1]>),
}

#[doc(hidden)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub enum RouteLocales {
    Any,
    Some(&'static [Locale]),
}

impl AcceptedLocales {
    pub fn accept(&self, route_locales: &RouteLocales) -> bool {
        match self {
            AcceptedLocales::Any => true,

            AcceptedLocales::Some(accepted_locales) => match route_locales {
                RouteLocales::Any => true,

                RouteLocales::Some(route_locales) => route_locales
                    .iter()
                    .any(|route_locale| accepted_locales.contains(route_locale)),
            },
        }
    }

    pub fn accepted_locales_for_sub_route(&self, route_locales: &RouteLocales) -> AcceptedLocales {
        match self {
            AcceptedLocales::Any => match route_locales {
                RouteLocales::Any => AcceptedLocales::Any,
                RouteLocales::Some(route_locales) => {
                    AcceptedLocales::Some(route_locales.to_smallvec())
                }
            },

            AcceptedLocales::Some(accepted_locales) => match route_locales {
                RouteLocales::Any => AcceptedLocales::Some(accepted_locales.clone()),

                RouteLocales::Some(route_locales) => AcceptedLocales::Some(
                    intersect_locales(accepted_locales, route_locales).collect(),
                ),
            },
        }
    }

    pub fn resulting_locales(&self, route_locales: &RouteLocales) -> Vec<Locale> {
        match route_locales {
            RouteLocales::Any => match self {
                AcceptedLocales::Any => Vec::new(),
                AcceptedLocales::Some(accepted_locales) => accepted_locales.to_vec(),
            },

            RouteLocales::Some(route_locales) => match self {
                AcceptedLocales::Any => route_locales.to_vec(),
                AcceptedLocales::Some(accepted_locales) => {
                    intersect_locales(accepted_locales, route_locales).collect()
                }
            },
        }
    }
}

fn intersect_locales<'a>(
    accepted_locales: &'a SmallVec<[Locale; 1]>,
    route_locales: &&'static [Locale],
) -> impl Iterator<Item = Locale> + 'a {
    route_locales
        .iter()
        .copied()
        .filter(|route_locale| accepted_locales.contains(route_locale))
}

#[doc(hidden)]
pub fn decode_param(param: &str) -> Option<Cow<str>> {
    percent_encoding::percent_decode_str(param)
        .decode_utf8()
        .ok()
}

#[doc(hidden)]
pub fn encode_param(param: impl Display) -> String {
    static ASCII_SET: &AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(&param.to_string(), ASCII_SET).to_string()
}

#[cfg(test)]
use smallvec::smallvec;

#[test]
fn test_accepted_locales_accept() {
    assert!(AcceptedLocales::Any.accept(&RouteLocales::Any));
    assert!(AcceptedLocales::Any.accept(&RouteLocales::Some(&["en", "fr"])));
    assert!(AcceptedLocales::Some(smallvec!["en", "fr"]).accept(&RouteLocales::Any));
    assert!(AcceptedLocales::Some(smallvec!["en", "fr"]).accept(&RouteLocales::Some(&["en", "fr"])));
    assert!(AcceptedLocales::Some(smallvec!["en", "fr"]).accept(&RouteLocales::Some(&["en"])));
    assert!(AcceptedLocales::Some(smallvec!["en", "fr"]).accept(&RouteLocales::Some(&["fr", "es"])));
    assert!(!AcceptedLocales::Some(smallvec!["en", "fr"]).accept(&RouteLocales::Some(&["es"])));
}

#[test]
fn test_accepted_locales_accepted_locales_for_sub_route() {
    assert_eq!(
        AcceptedLocales::Any.accepted_locales_for_sub_route(&RouteLocales::Any),
        AcceptedLocales::Any,
    );

    assert_eq!(
        AcceptedLocales::Any.accepted_locales_for_sub_route(&RouteLocales::Some(&["en", "fr"])),
        AcceptedLocales::Some(smallvec!["en", "fr"]),
    );

    assert_eq!(
        AcceptedLocales::Some(smallvec!["en", "fr"])
            .accepted_locales_for_sub_route(&RouteLocales::Any),
        AcceptedLocales::Some(smallvec!["en", "fr"]),
    );

    assert_eq!(
        AcceptedLocales::Some(smallvec!["en", "fr"])
            .accepted_locales_for_sub_route(&RouteLocales::Some(&["en", "fr"])),
        AcceptedLocales::Some(smallvec!["en", "fr"]),
    );

    assert_eq!(
        AcceptedLocales::Some(smallvec!["en", "fr"])
            .accepted_locales_for_sub_route(&RouteLocales::Some(&["en", "es"])),
        AcceptedLocales::Some(smallvec!["en"]),
    );
}

#[test]
fn test_accepted_locales_resulting_locales() {
    assert_eq!(
        AcceptedLocales::Any.resulting_locales(&RouteLocales::Any),
        Vec::<Locale>::new(),
    );

    assert_eq!(
        AcceptedLocales::Any.resulting_locales(&RouteLocales::Some(&["en", "fr"])),
        vec!["en", "fr"],
    );

    assert_eq!(
        AcceptedLocales::Some(smallvec!["en", "fr"]).resulting_locales(&RouteLocales::Any),
        vec!["en", "fr"],
    );

    assert_eq!(
        AcceptedLocales::Some(smallvec!["en", "fr"])
            .resulting_locales(&RouteLocales::Some(&["en", "es"])),
        vec!["en"],
    );
}

#[test]
fn test_derive() {
    use maplit::hashset;
    use std::collections::HashSet;

    fn with_locales_vec_to_hashset(
        value: Option<(Route, Vec<Locale>)>,
    ) -> Option<(Route, HashSet<Locale>)> {
        value.map(|(route, locales)| (route, HashSet::from_iter(locales)))
    }

    #[derive(Chemin, PartialEq, Eq, Debug)]
    enum Route {
        #[route("/")]
        Home,

        #[route("/hello")]
        Hello,

        #[route(en_US, en_UK => "/hello/:/")]
        #[route(fr => "/bonjour/:/")]
        HelloWithName(String),

        #[route("/hello/:name/:age")]
        HelloWithNameAndAge { name: String, age: u8 },

        #[route(en, fr => "/with-sub-route/..")]
        WithSubRoute(SubRoute),

        #[route("/with-named-sub-route/..sub_route")]
        WithNamedSubRoute {
            sub_route: SubRoute,
            #[query_param]
            mandatory_param: String,
        },
    }

    #[derive(Chemin, PartialEq, Eq, Debug)]
    enum SubRoute {
        #[route("/home")]
        Home,

        #[route(fr_FR, fr => "/bonjour")]
        Hello,

        #[route("/with-params")]
        WithParams {
            #[query_param(optional)]
            optional_param: Option<String>,
            #[query_param(default = String::from("default"))]
            param_with_default_value: String,
        },
    }

    // Test parsing
    assert_eq!(Route::parse("", false), Some((Route::Home, vec![])));
    assert_eq!(Route::parse("/", false), Some((Route::Home, vec![])));

    assert_eq!(Route::parse("/hello", false), Some((Route::Hello, vec![])));
    assert_eq!(Route::parse("/hello/", false), None);

    assert_eq!(Route::parse("/hello/john", false), None);
    assert_eq!(
        with_locales_vec_to_hashset(Route::parse("/hello/john/", false)),
        Some((
            Route::HelloWithName(String::from("john")),
            hashset!["en-US", "en-UK"],
        ))
    );
    assert_eq!(Route::parse("/bonjour/john", false), None);
    assert_eq!(
        Route::parse("/bonjour/john%20doe/", false),
        Some((Route::HelloWithName(String::from("john%20doe")), vec!["fr"])),
    );
    assert_eq!(
        Route::parse("/bonjour/john%20doe/", true),
        Some((Route::HelloWithName(String::from("john doe")), vec!["fr"]))
    );

    assert_eq!(Route::parse("/hello/john/invalid_age", false), None);
    assert_eq!(
        Route::parse("/hello/john/30", false),
        Some((
            Route::HelloWithNameAndAge {
                name: String::from("john"),
                age: 30,
            },
            vec![]
        )),
    );

    assert_eq!(
        with_locales_vec_to_hashset(Route::parse("/with-sub-route/home", false)),
        Some((Route::WithSubRoute(SubRoute::Home), hashset!["en", "fr"])),
    );
    assert_eq!(Route::parse("/with-sub-route/bonjour/", false), None);
    assert_eq!(
        Route::parse("/with-sub-route/bonjour", false),
        Some((Route::WithSubRoute(SubRoute::Hello), vec!["fr"])),
    );

    assert_eq!(
        Route::parse("/with-named-sub-route/with-params", false),
        None,
    );
    assert_eq!(
        Route::parse(
            "/with-named-sub-route/with-params?mandatory_param=value%20value+value",
            false
        ),
        Some((
            Route::WithNamedSubRoute {
                sub_route: SubRoute::WithParams {
                    optional_param: None,
                    param_with_default_value: String::from("default"),
                },
                mandatory_param: String::from("value value value"),
            },
            vec![]
        )),
    );
    assert_eq!(
        Route::parse(
            "/with-named-sub-route/with-params?optional_param=optional+value&mandatory_param=value&param_with_default_value=default+value",
            false
        ),
        Some((
            Route::WithNamedSubRoute {
                sub_route: SubRoute::WithParams {
                    optional_param: Some(String::from("optional value")),
                    param_with_default_value: String::from("default value"),
                },
                mandatory_param: String::from("value"),
            },
            vec![]
        )),
    );

    // Test url generation
    assert_eq!(
        Route::Home.generate_url(None, false),
        Some(String::from("/"))
    );
    assert_eq!(
        Route::Home.generate_url(Some("es"), false),
        Some(String::from("/")),
    );

    assert_eq!(
        Route::Hello.generate_url(None, false),
        Some(String::from("/hello")),
    );

    assert_eq!(
        Route::HelloWithName(String::from("John")).generate_url(Some("en-US"), false),
        Some(String::from("/hello/John/")),
    );
    assert_eq!(
        Route::HelloWithName(String::from("John")).generate_url(Some("en-UK"), false),
        Some(String::from("/hello/John/")),
    );
    assert_eq!(
        Route::HelloWithName(String::from("John Doe")).generate_url(Some("fr"), false),
        Some(String::from("/bonjour/John Doe/")),
    );
    assert_eq!(
        Route::HelloWithName(String::from("John Doe.")).generate_url(Some("fr"), true),
        Some(String::from("/bonjour/John%20Doe./"))
    );
    assert_eq!(
        Route::HelloWithName(String::from("John")).generate_url(Some("en"), false),
        None,
    );
    assert_eq!(
        Route::HelloWithName(String::from("John")).generate_url(None, false),
        None,
    );

    assert_eq!(
        Route::HelloWithNameAndAge {
            name: String::from("John"),
            age: 30,
        }
        .generate_url(None, false),
        Some(String::from("/hello/John/30")),
    );

    assert_eq!(
        Route::WithSubRoute(SubRoute::Home).generate_url(None, false),
        None
    );
    assert_eq!(
        Route::WithSubRoute(SubRoute::Home).generate_url(Some("en"), false),
        Some(String::from("/with-sub-route/home")),
    );
    assert_eq!(
        Route::WithSubRoute(SubRoute::Hello).generate_url(Some("fr-FR"), false),
        None,
    );
    assert_eq!(
        Route::WithSubRoute(SubRoute::Hello).generate_url(Some("fr"), false),
        Some(String::from("/with-sub-route/bonjour")),
    );
    assert_eq!(
        Route::WithSubRoute(SubRoute::Hello).generate_url(Some("en"), false),
        None,
    );
    assert_eq!(
        Route::WithSubRoute(SubRoute::Hello).generate_url(None, false),
        None,
    );

    assert_eq!(
        Route::WithNamedSubRoute {
            sub_route: SubRoute::WithParams {
                optional_param: None,
                param_with_default_value: String::from("default"),
            },
            mandatory_param: String::from("mandatory param"),
        }
        .generate_url(Some("en"), false),
        Some(String::from(
            "/with-named-sub-route/with-params?mandatory_param=mandatory+param"
        ))
    );
    assert_eq!(
        Route::WithNamedSubRoute {
            sub_route: SubRoute::WithParams {
                optional_param: Some(String::from("optional param")),
                param_with_default_value: String::from("default value"),
            },
            mandatory_param: String::from("mandatory param"),
        }
        .generate_url(Some("en"), false),
        Some(String::from(
            "/with-named-sub-route/with-params?mandatory_param=mandatory+param&optional_param=optional+param&param_with_default_value=default+value"
        ))
    );
}
