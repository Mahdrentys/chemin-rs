use crate::helpers;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use proc_macro2::Span;
use std::collections::HashSet;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseBuffer};
use syn::punctuated::Punctuated;
use syn::{parenthesized, Error, Ident, LitStr, Token};

#[derive(PartialEq, Eq, Debug)]
pub struct LocalizedRoute {
    pub path: Path,
    pub locales: HashSet<String>,
}

impl Parse for LocalizedRoute {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let input_inner;
        parenthesized!(input_inner in input);

        if input_inner.peek(LitStr) {
            let path: Path = input_inner.parse()?;
            input_inner.call(helpers::parse_eos)?;
            Ok(Self {
                path,
                locales: HashSet::new(),
            })
        } else {
            let locales: Punctuated<Ident, Token![,]> =
                Punctuated::parse_separated_nonempty_with(&input_inner, Ident::parse_any)?;
            input_inner.parse::<Token![=>]>()?;
            let path: Path = input_inner.parse()?;
            input_inner.call(helpers::parse_eos)?;
            Ok(Self {
                path,
                locales: locales
                    .into_iter()
                    .map(|locale_ident| locale_ident.to_string().replace('_', "-"))
                    .collect(),
            })
        }
    }
}

#[derive(Debug)]
pub struct Path {
    pub components: Vec<PathComponent>,
    /// `None` if there is no sub-route, `Some(None)` if there is a unnamed sub-route, `Some(Some)` if there is a named sub-route.
    pub sub_route: Option<SubRoute>,
    pub trailing_slash: bool,
    pub span: Span,
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
            && self.sub_route == other.sub_route
            && self.trailing_slash == other.trailing_slash
    }
}

impl Eq for Path {}

impl Path {
    pub fn contains_unnamed_params_and_sub_routes(&self) -> bool {
        matches!(self.sub_route, Some(SubRoute::Unnamed))
            || self
                .components
                .iter()
                .any(|path_component| match path_component {
                    PathComponent::Static(_) => false,
                    PathComponent::Param(name) => name.is_none(),
                })
    }

    pub fn contains_named_params_and_sub_routes(&self) -> bool {
        matches!(self.sub_route, Some(SubRoute::Named(_)))
            || self
                .components
                .iter()
                .any(|path_component| match path_component {
                    PathComponent::Static(_) => false,
                    PathComponent::Param(name) => name.is_some(),
                })
    }

    pub fn params(&self) -> impl Iterator<Item = Option<&String>> {
        self.components
            .iter()
            .filter_map(|path_component| match path_component {
                PathComponent::Static(_) => None,
                PathComponent::Param(name) => Some(name),
            })
            .map(|param| param.as_ref())
    }

    pub fn has_named_param(&self, expected_name: &str) -> bool {
        self.components
            .iter()
            .any(|path_component| match path_component {
                PathComponent::Static(_) => false,
                PathComponent::Param(None) => false,
                PathComponent::Param(Some(name)) => name == expected_name,
            })
    }
}

impl Parse for Path {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let path_lit: LitStr = input.parse()?;

        match Path::parse_str(&path_lit.value()) {
            Ok(mut path) => {
                path.span = path_lit.span();
                Ok(path)
            }
            Err(error) => Err(Error::new(path_lit.span(), error)),
        }
    }
}

#[derive(Parser)]
#[grammar = "derive_chemin/router/path.pest"]
struct PathParser;

impl Path {
    fn parse_str(input: &str) -> Result<Self, Box<pest::error::Error<Rule>>> {
        match PathParser::parse(Rule::path, input) {
            Ok(mut pairs) => {
                let path_pair = pairs.next().unwrap();
                assert_eq!(path_pair.as_rule(), Rule::path);

                let mut path = Self {
                    components: Vec::new(),
                    sub_route: None,
                    trailing_slash: false,
                    span: Span::call_site(),
                };

                for pair in path_pair.into_inner() {
                    match pair.as_rule() {
                        Rule::static_path | Rule::param => path.components.push(pair.into()),
                        Rule::sub_route => path.sub_route = Some(pair.into()),
                        Rule::trailing_slash => path.trailing_slash = true,
                        Rule::EOI => break,
                        _ => unreachable!(),
                    }
                }

                Ok(path)
            }

            Err(error) => Err(Box::new(error)),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum PathComponent {
    Static(String),
    Param(Option<String>),
}

impl From<Pair<'_, Rule>> for PathComponent {
    fn from(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::static_path => Self::Static(pair.as_str().to_owned()),

            Rule::param => match pair.into_inner().next() {
                Some(field_pair) => {
                    assert_eq!(field_pair.as_rule(), Rule::field);
                    Self::Param(Some(validate_ident(field_pair.as_str()).to_owned()))
                }

                None => Self::Param(None),
            },

            _ => unreachable!(),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum SubRoute {
    Unnamed,
    Named(String),
}

impl From<Pair<'_, Rule>> for SubRoute {
    fn from(pair: Pair<'_, Rule>) -> Self {
        match pair.into_inner().next() {
            Some(field_pair) => {
                assert_eq!(field_pair.as_rule(), Rule::field);
                Self::Named(validate_ident(field_pair.as_str()).to_owned())
            }

            None => Self::Unnamed,
        }
    }
}

/// Panics if ident is invalid.
fn validate_ident(ident: &str) -> &str {
    Ident::new(ident, Span::call_site());
    ident
}

#[test]
fn test_path_parsing() {
    assert_eq!(
        Path::parse_str("/home"),
        Ok(Path {
            components: vec![PathComponent::Static(String::from("home"))],
            sub_route: None,
            trailing_slash: false,
            span: Span::call_site(),
        })
    );

    assert_eq!(
        Path::parse_str("/home/"),
        Ok(Path {
            components: vec![PathComponent::Static(String::from("home"))],
            sub_route: None,
            trailing_slash: true,
            span: Span::call_site(),
        })
    );

    assert_eq!(
        Path::parse_str("/hello/:"),
        Ok(Path {
            components: vec![
                PathComponent::Static(String::from("hello")),
                PathComponent::Param(None),
            ],
            sub_route: None,
            trailing_slash: false,
            span: Span::call_site(),
        })
    );

    assert_eq!(
        Path::parse_str("/hello/:name/:age/aaa/..rest"),
        Ok(Path {
            components: vec![
                PathComponent::Static(String::from("hello")),
                PathComponent::Param(Some(String::from("name"))),
                PathComponent::Param(Some(String::from("age"))),
                PathComponent::Static(String::from("aaa")),
            ],
            sub_route: Some(SubRoute::Named(String::from("rest"))),
            trailing_slash: false,
            span: Span::call_site(),
        })
    );

    assert_eq!(
        Path::parse_str("/hello/:/.."),
        Ok(Path {
            components: vec![
                PathComponent::Static(String::from("hello")),
                PathComponent::Param(None),
            ],
            sub_route: Some(SubRoute::Unnamed),
            trailing_slash: false,
            span: Span::call_site(),
        })
    );
}
