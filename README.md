# Chemin

An enum-based router generator for rust, supporting query strings and i18n. It can be used on front-end or back-end, with any framework or library. It can be used both ways: to parse a url into a route, and to generate a url from a route you constructed.

It is not meant to be "blazingly fast": in this crate, code clarity is always privileged over optimization.

## Example

See the [API documentation](https://docs.rs/chemin) for more detailed explanations.

```rust
#[derive(Chemin)]
enum Route {
    #[route("/")]
    Home,

    #[route(en => "/about")]
    #[route(fr => "/a-propos")]
    About,

    #[route(en => "/hello/:name")]
    #[route(fr => "/bonjour/:name")]
    Hello {
        name: String,
        #[query_param(optional)]
        age: Option<u8>,
    },

    #[route("/sub-route/..")]
    SubRoute(SubRoute),
}

#[derive(Chemin)]
enum SubRoute {
    #[route("/a")]
    A,

    #[route("/b")]
    B,
}
```
