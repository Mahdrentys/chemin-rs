use syn::parse::ParseBuffer;
use syn::{Error, Result};

pub fn parse_eos(input: &ParseBuffer) -> Result<()> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(Error::new(input.span(), "Syntax Error: unexpected token"))
    }
}
