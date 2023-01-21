// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro2::Span;
use syn::parse::ParseBuffer;
use syn::token::Token;
use syn::{Attribute, Ident, PathArguments, Token, Type};

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

/// When `typ` is of the form `Result<Something>`, returns the inner `Something`.
pub fn unwrap_result(typ: &Type) -> Option<&Type> {
    if let Type::Path(typepath) = typ {
        if let Some(seg) = typepath.path.segments.first() {
            if seg.ident == "Result" {
                if let PathArguments::AngleBracketed(bracketed) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(t)) = bracketed.args.iter().next() {
                        return Some(t);
                    }
                }
            }
        }
    }
    None
}

/// Assuming `attr` is of the form `#[doc = r"Some docstring line"]`, returns
/// the inner string. Panics otherwise
pub fn parse_doc_attr(attr: &Attribute) -> String {
    let meta = attr.parse_meta().unwrap();

    match meta {
        syn::Meta::NameValue(nameval) => match nameval.lit {
            syn::Lit::Str(s) => s.value(),
            _ => panic!("Unexpected docstring attribute form"),
        },
        _ => panic!("Unexpected docstring attribute form"),
    }
}

pub fn join_str<S>(it: impl Iterator<Item = S>, sep: &str) -> String
where
    S: AsRef<str>,
{
    let mut s = String::new();
    let mut first = true;
    for i in it {
        if !first {
            s += sep;
        }
        first = false;
        s += i.as_ref();
    }
    s
}
