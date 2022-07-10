use proc_macro2::Span;
use syn::parse::ParseBuffer;
use syn::token::Token;
use syn::{Ident, Token};

#[cfg(test)]
use std::{fs, io, path::Path, process::Command};

pub trait SynParseBufferExt {
    fn as_stream(&self) -> &ParseBuffer<'_>;

    fn comma_separated_fn<T>(
        &self,
        item_parser: impl Fn(&ParseBuffer) -> syn::Result<T>,
    ) -> syn::Result<Vec<T>> {
        let input = self.as_stream();
        let mut items = vec![item_parser(input)?];
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            items.push(item_parser(input)?);
        }
        Ok(items)
    }

    fn expect_ident(&self, name: &str) -> syn::Result<()> {
        let input = self.as_stream();
        let id: Ident = input.parse()?;
        if id != name {
            panic!("Expected {name}, found {id}")
        } else {
            Ok(())
        }
    }

    fn expect_token<T: Token + syn::parse::Parse>(&self) -> syn::Result<()> {
        let input = self.as_stream();
        let _: T = input.parse()?;
        Ok(())
    }
}

pub trait ExprUtils {
    fn as_expr(&self) -> &syn::Expr;
    fn assume_string_literal(&self, err_msg: &str) -> syn::Result<String> {
        let expr = self.as_expr();
        if let syn::Expr::Lit(lit) = expr {
            if let syn::Lit::Str(s) = &lit.lit {
                return Ok(s.value());
            }
        }
        Err(syn::Error::new(Span::call_site(), err_msg.to_owned()))
    }
}

impl<'a> SynParseBufferExt for ParseBuffer<'a> {
    fn as_stream(&self) -> &ParseBuffer {
        self
    }
}

impl ExprUtils for syn::Expr {
    fn as_expr(&self) -> &syn::Expr {
        self
    }
}

#[cfg(test)]
pub fn write_and_fmt<P: AsRef<Path>, S: ToString>(path: P, code: S) -> io::Result<()> {
    fs::write(&path, code.to_string())?;
    Command::new("rustfmt").arg(path.as_ref()).spawn()?.wait()?;
    Ok(())
}
