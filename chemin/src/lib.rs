use smallvec::{SmallVec, ToSmallVec};

#[doc(hidden)]
pub mod deps {
    pub use once_cell;
    pub use route_recognizer;
}

pub trait Chemin: Sized {
    fn parse(url: &str) -> Option<(Self, Vec<Locale>)> {
        Self::parse_with_accepted_locales(url, &AcceptedLocales::Any)
    }

    fn parse_with_accepted_locales(
        url: &str,
        accepted_locales: &AcceptedLocales,
    ) -> Option<(Self, Vec<Locale>)>;

    fn generate_url(&self, locale: &str) -> Option<String>;
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
