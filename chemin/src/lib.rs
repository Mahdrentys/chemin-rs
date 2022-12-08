//! Chemin is an enum-based router generator, supporting query strings and i18n. It can be used on front-end or back-end, with any
//! framework or library. It can be used both ways: to parse a url into a route, and to generate a url from a route you constructed.
//!
//! It is not meant to be "blazingly fast": in this crate, code clarity is always privileged over optimization.
//!
//! ## Basic usage
//!
//! You just have to define your routes as different variant of an enum, derive the [Chemin] trait and that's it.
//!
//! ```
//! use chemin::Chemin;
//!
//! // `PartialEq`, `Eq` and `Debug` are not necessary to derive `Chemin`, but here they are used to be able to use `assert_eq`.
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum Route {
//!     ##[route("/")]
//!     Home,
//!
//!     /// If there is a trailing slash at the end (example: #[route("/about/")]), it is considered
//!     /// a different route than without the trailing slash.
//!     ##[route("/about")]
//!     About,
//!
//!     /// The character ":" is used for dynamic parameters.
//!     /// The type of the parameter (in this case `String`), must implement `FromStr` and `Display`.
//!     ##[route("/hello/:")]
//!     Hello(String),
//!
//!     /// You can use named fields by giving a name to the parameters (after ":").
//!     ##[route("/hello/:name/:age")]
//!     HelloWithAge {
//!         name: String,
//!         age: u8
//!     }
//! }
//!
//! // Url parsing:
//! let decode_params = true; // Whether or not to percent-decode url parameters (see `Chemin::parse` for documentation).
//! // `vec![]` is the list of the locales for this route. As we don't use i18n for this router yet, it is therefore empty.
//! assert_eq!(Route::parse("/", decode_params), Some((Route::Home, vec![])));
//! assert_eq!(Route::parse("/about", decode_params), Some((Route::About, vec![])));
//! assert_eq!(Route::parse("/about/", decode_params), None); // Route not found because of the trailing slash
//! assert_eq!(Route::parse("/hello/John", decode_params), Some((Route::Hello(String::from("John")), vec![])));
//! assert_eq!(
//!     Route::parse("/hello/John%20Doe/30", decode_params),
//!     Some((
//!         Route::HelloWithAge {
//!             name: String::from("John Doe"),
//!             age: 30,
//!         },
//!         vec![],
//!     ))
//! );
//!
//! // Url generation
//! let encode_params = true; // Whether or not to percent-encode url parameters (see `Chemin::generate_url` for documentation).
//! let locale = None; // The locale for which to generate the url. For now, we don't use i18n yet, so it is `None`.
//! assert_eq!(Route::Home.generate_url(locale, encode_params), Some(String::from("/"))); // The result is guaranteed to be `Some` if we don't use i18n.
//! assert_eq!(Route::About.generate_url(locale, encode_params), Some(String::from("/about")));
//! assert_eq!(Route::Hello(String::from("John")).generate_url(locale, encode_params), Some(String::from("/hello/John")));
//! assert_eq!(
//!     Route::HelloWithAge {
//!         name: String::from("John Doe"),
//!         age: 30,
//!     }.generate_url(locale, encode_params),
//!     Some(String::from("/hello/John%20Doe/30")),
//! );
//! ```
//!
//! ## Sub-routes
//!
//! But for more complex routers, you're not gonna put everything into a single enum. You can break it up with sub-routes:
//!
//! ```
//! use chemin::Chemin;
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum Route {
//!     /// You can use a sub-route by using ".." (only at the end of the path). The corresponding type must also implement `Chemin`.
//!     ///
//!     /// If you want a route to access "/sub-route" or "/sub-route/", it can't possibly be defined inside the sub-route, so it would
//!     /// have to be a different additional route here.
//!     ##[route("/sub-route/..")]
//!     WithSubRoute(SubRoute),
//!
//!     /// You can also combine sub-route with url parameters, and use named sub-routes, by adding the name after "..".
//!     ##[route("/hello/:name/..sub_route")]
//!     HelloWithSubRoute {
//!         name: String,
//!         sub_route: SubRoute,
//!     },
//! }
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum SubRoute {
//!     ##[route("/a")]
//!     A,
//!
//!     ##[route("/b")]
//!     B,
//! }
//!
//! // Url parsing:
//! assert_eq!(Route::parse("/sub-route/a", true), Some((Route::WithSubRoute(SubRoute::A), vec![])));
//! assert_eq!(
//!     Route::parse("/hello/John/b", true),
//!     Some((
//!         Route::HelloWithSubRoute {
//!             name: String::from("John"),
//!             sub_route: SubRoute::B,
//!         },
//!         vec![],
//!     )),
//! );
//!
//! // Url generation:
//! assert_eq!(Route::WithSubRoute(SubRoute::A).generate_url(None, true), Some(String::from("/sub-route/a")));
//! ```
//!
//! ## Query strings parameters
//!
//! Query strings are supported:
//!
//! ```
//! use chemin::Chemin;
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum Route {
//!     ##[route("/hello/:name")]
//!     Hello {
//!         name: String,
//!
//!         /// This attribute can only be used on named fields
//!         ##[query_param]
//!         age: u8,
//!     }
//! }
//!
//! // Url parsing:
//! assert_eq!(Route::parse("/hello/John", true), None); // Route not found because the "age" query parameter wasn't provided
//! assert_eq!(
//!     Route::parse("/hello/John?age=30", true),
//!     Some((
//!         Route::Hello {
//!             name: String::from("John"),
//!             age: 30,
//!         },
//!         vec![],
//!     ))
//! );
//!
//! // Url generation:
//! assert_eq!(
//!     Route::Hello {
//!         name: String::from("John"),
//!         age: 30,
//!     }.generate_url(None, true),
//!     Some(String::from("/hello/John?age=30")),
//! );
//! ```
//!
//! Query parameters can also be optional:
//!
//! ```
//! use chemin::Chemin;
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum Route {
//!     ##[route("/hello/:name")]
//!     Hello {
//!         name: String,
//!         ##[query_param(optional)]
//!         age: Option<u8>,
//!     }
//! }
//!
//! // Url parsing:
//! assert_eq!(
//!     Route::parse("/hello/John", true),
//!     Some((
//!         Route::Hello {
//!             name: String::from("John"),
//!             age: None,
//!         },
//!         vec![],
//!     )),
//! );
//! assert_eq!(
//!     Route::parse("/hello/John?age=30", true),
//!     Some((
//!         Route::Hello {
//!             name: String::from("John"),
//!             age: Some(30),
//!         },
//!         vec![],
//!     )),
//! );
//!
//! // Url generation:
//! assert_eq!(
//!     Route::Hello {
//!         name: String::from("John"),
//!         age: None,
//!     }.generate_url(None, true),
//!     Some(String::from("/hello/John")),
//! );
//! assert_eq!(
//!     Route::Hello {
//!         name: String::from("John"),
//!         age: Some(30),
//!     }.generate_url(None, true),
//!     Some(String::from("/hello/John?age=30")),
//! );
//! ```
//!
//! Query parameters can have a default value:
//!
//! ```
//! use chemin::Chemin;
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum Route {
//!     ##[route("/hello/:name")]
//!     Hello {
//!         name: String,
//!         ##[query_param(default = 20)]
//!         age: u8,
//!     }
//! }
//!
//! // Url parsing:
//! assert_eq!(
//!     Route::parse("/hello/John", true),
//!     Some((
//!         Route::Hello {
//!             name: String::from("John"),
//!             age: 20,
//!         },
//!         vec![],
//!     )),
//! );
//! assert_eq!(
//!     Route::parse("/hello/John?age=30", true),
//!     Some((
//!         Route::Hello {
//!             name: String::from("John"),
//!             age: 30,
//!         },
//!         vec![],
//!     )),
//! );
//!
//! // Url generation:
//! assert_eq!(
//!     Route::Hello {
//!         name: String::from("John"),
//!         age: 20,
//!     }.generate_url(None, true),
//!     Some(String::from("/hello/John")),
//! );
//! assert_eq!(
//!     Route::Hello {
//!         name: String::from("John"),
//!         age: 30,
//!     }.generate_url(None, true),
//!     Some(String::from("/hello/John?age=30")),
//! );
//! ```
//!
//! If you use sub-routes, you can have query parameters defined at any level of the "route tree", and they will all share the same
//! query string.
//!
//! ## Internationalization (i18n)
//!
//! This crate allows you to have translations of your routes for different languages, by defining multiple paths on each enum variant
//! and associating each with one or multiple locale codes
//! (as used with <https://developer.mozilla.org/en-US/docs/Web/API/Navigator/language>):
//!
//! ```
//! use chemin::Chemin;
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum Route {
//!     ##[route("/")]
//!     Home,
//!
//!     // Notice that the hyphens normally used in locale codes are here replaced by an underscore, to be valid rust identifiers
//!     ##[route(en, en_US, en_UK => "/about")]
//!     ##[route(fr, fr_FR => "/a-propos")]
//!     About,
//!
//!     ##[route(en, en_US, en_UK => "/select/..")]
//!     ##[route(fr, fr_FR => "/selectionner/..")]
//!     Select(SelectRoute),
//! }
//!
//! ##[derive(Chemin, PartialEq, Eq, Debug)]
//! enum SelectRoute {
//!     ##[route(en, en_US => "/color/:/:/:")]
//!     ##[route(en_UK => "/colour/:/:/:")]
//!     ##[route(fr, fr_FR => "/couleur/:/:/:")]
//!     RgbColor(u8, u8, u8),
//! }
//!
//! // Url parsing:
//! assert_eq!(Route::parse("/", true), Some((Route::Home, vec![])));
//!
//! let about_english = Route::parse("/about", true).unwrap();
//! assert_eq!(about_english.0, Route::About);
//! // The `Vec<String>` of locales has to be asserted that way, because the order isn't guaranteed
//! assert_eq!(about_english.1.len(), 3);
//! assert!(about_english.1.contains(&"en"));
//! assert!(about_english.1.contains(&"en-US")); // Notice that returned locale codes use hyphens and not underscores
//! assert!(about_english.1.contains(&"en-UK"));
//!
//! let about_french = Route::parse("/a-propos", true).unwrap();
//! assert_eq!(about_french.0, Route::About);
//! assert_eq!(about_french.1.len(), 2);
//! assert!(about_french.1.contains(&"fr"));
//! assert!(about_french.1.contains(&"fr-FR"));
//!
//! let select_color_us_english = Route::parse("/select/color/0/255/0", true).unwrap();
//! assert_eq!(select_color_us_english.0, Route::Select(SelectRoute::RgbColor(0, 255, 0)));
//! assert_eq!(select_color_us_english.1.len(), 2); // The `Vec<String>` has to be asserted that way, because the order isn't guaranteed
//! assert!(select_color_us_english.1.contains(&"en"));
//! assert!(select_color_us_english.1.contains(&"en-US"));
//!
//! assert_eq!(
//!     Route::parse("/select/colour/0/255/0", true),
//!     Some((Route::Select(SelectRoute::RgbColor(0, 255, 0)), vec!["en-UK"])),
//! );
//!
//! let select_color_french = Route::parse("/selectionner/couleur/0/255/0", true).unwrap();
//! assert_eq!(select_color_french.0, Route::Select(SelectRoute::RgbColor(0, 255, 0)));
//! assert_eq!(select_color_french.1.len(), 2); // The `Vec<String>` has to be asserted that way, because the order isn't guaranteed
//! assert!(select_color_french.1.contains(&"fr"));
//! assert!(select_color_french.1.contains(&"fr-FR"));
//!
//! assert_eq!(Route::parse("/select/couleur/0/255/0", true), None);
//!
//! // Url generation:
//! assert_eq!(Route::Home.generate_url(Some("es"), true), Some(String::from("/")));
//! assert_eq!(Route::Home.generate_url(None, true), Some(String::from("/")));
//!
//! // Notice that you have to use hyphens and not underscores in locale codes
//! assert_eq!(Route::About.generate_url(Some("en"), true), Some(String::from("/about")));
//! assert_eq!(Route::About.generate_url(Some("fr-FR"), true), Some(String::from("/a-propos")));
//! assert_eq!(Route::About.generate_url(Some("es"), true), None);
//! assert_eq!(Route::About.generate_url(None, true), None);
//!
//! assert_eq!(
//!     Route::Select(SelectRoute::RgbColor(0, 255, 0)).generate_url(Some("en-UK"), true),
//!     Some(String::from("/select/colour/0/255/0")),
//! );
//! assert_eq!(
//!     Route::Select(SelectRoute::RgbColor(0, 255, 0)).generate_url(Some("fr-FR"), true),
//!     Some(String::from("/selectionner/couleur/0/255/0")),
//! );
//! ```

extern crate self as chemin;

/// To derive the [Chemin] trait.
///
/// To learn how to use it, see [the root of the documentation](index.html).
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

/// Trait to derive to build a enum-based router.
///
/// This trait is not meant to be implemented directly (although you can). To learn how to derive it, see
/// [the root of the documentation](index.html).
pub trait Chemin: Sized {
    /// Parses an url to obtain a route.
    ///
    /// The `url` can contain a query string.
    ///
    /// If the `decode_params` argument is `true`, url parameters will be percent-decoded
    /// (see <https://www.w3schools.com/tags/ref_urlencode.ASP>). However, the query string parameters will always be percent-decoded,
    /// regardless of the `decode_params` argument. Additionally, the character "+" will be converted to a space (" ") for query string
    /// parameters.
    ///
    /// If the provided url doesn't correspond to any route, or if some parameter or query string argument failed to parse, this
    /// function returns [None]. If not, this function returns a tuple wrapped in [Some], whose first field is the obtained route, and
    /// whose second field is a list of the locales corresponding to this route. Most of the time, it is only one locale, or zero if
    /// no locale was defined for this route.
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

    /// This function is not meant to be called directly. It is used internally by [Chemin::parse].
    fn parse_with_accepted_locales(
        path: &str,
        accepted_locales: &AcceptedLocales,
        decode_params: bool,
        qstring: &QString,
    ) -> Option<(Self, Vec<Locale>)>;

    /// Generates a url from a route.
    ///
    /// The `locale` argument has to be [Some] when using i18n, because the locale for which the url is generated has to be known. If
    /// this route is not specific to a locale, it can be [None]. It is a standard locale code, such as used with
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Navigator/language>.
    ///
    /// If the `encode_params` argument is `true`, url parameters will be percent-encoded
    /// (see <https://www.w3schools.com/tags/ref_urlencode.ASP>). All non-alphanumeric characters except "-", "_", "." and "~" will be
    /// encoded. However, the query string parameters will always be percent-encoded, regardless of the `encode_params` argument.
    /// Additionally, the space character (" ") will be displayed as a "+" in query string parameters.
    ///
    /// If this route is not defined for the provided `locale`, then this method will return [None].
    fn generate_url(&self, locale: Option<&str>, encode_params: bool) -> Option<String> {
        let mut qstring = QString::default();

        self.generate_url_and_build_qstring(locale, encode_params, &mut qstring)
            .map(|mut value| {
                if qstring.is_empty() {
                    value
                } else {
                    value.push('?');
                    value.push_str(&qstring.to_string().replace('+', "%2B").replace("%20", "+"));
                    value
                }
            })
    }

    /// This method is not meant to be called directly. It is used internally by [Chemin::generate_url].
    fn generate_url_and_build_qstring(
        &self,
        locale: Option<&str>,
        encode_params: bool,
        qstring: &mut QString,
    ) -> Option<String>;
}

/// A standard locale code, such as used with <https://developer.mozilla.org/en-US/docs/Web/API/Navigator/language>.
///
/// Examples: `"en"`, `"en-US"`, `"fr"`, `"fr-FR"`, `"es-ES"`.
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
            "/with-named-sub-route/with-params?optional_param=optional%2Bvalue&mandatory_param=value&param_with_default_value=default+value",
            false
        ),
        Some((
            Route::WithNamedSubRoute {
                sub_route: SubRoute::WithParams {
                    optional_param: Some(String::from("optional+value")),
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
                optional_param: Some(String::from("optional+param")),
                param_with_default_value: String::from("default&value"),
            },
            mandatory_param: String::from("mandatory param"),
        }
        .generate_url(Some("en"), false),
        Some(String::from(
            "/with-named-sub-route/with-params?mandatory_param=mandatory+param&optional_param=optional%2Bparam&param_with_default_value=default%26value"
        ))
    );
}
