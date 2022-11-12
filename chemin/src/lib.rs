pub type Locale<'a> = &'a str;

pub trait Chemin: Sized {
    fn parse(url: &str, accepted_locales: Option<&[Locale]>) -> Option<(Self, Locale<'static>)>;
    fn generate_url(&self, locale: Locale) -> String;
}
