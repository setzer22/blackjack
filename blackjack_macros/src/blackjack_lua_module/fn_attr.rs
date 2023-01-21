// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    Expr, Token,
};

use crate::utils::{ExprUtils, SynParseBufferExt};

#[derive(Default, Debug)]
pub struct LuaFnAttr {
    pub under: Option<String>,
    pub coerce: bool,
    pub map_this: Option<Expr>,
    pub map_result: Option<Expr>,
    pub hidden_fn: bool,
}

#[derive(Default, Debug)]
pub struct FunctionAttributes {
    pub lua_attr: LuaFnAttr,
    pub docstring_lines: Vec<String>,
}

impl Parse for LuaFnAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let properties = input.comma_separated_fn(|input| {
            let lhs: Ident = input.parse()?;
            let rhs: Option<Expr> = if input.peek(Token![=]) {
                let _eq_sign = input.expect_token::<Token![=]>();
                Some(input.parse()?)
            } else {
                None
            };
            Ok((lhs, rhs))
        })?;

        let mut lua_attr = LuaFnAttr::default();

        for (key, val) in properties.iter() {
            if key == "under" {
                lua_attr.under = Some(
                    val.as_ref()
                        .expect("'under' declaration should have an assigned value")
                        .assume_string_literal("Value for 'under' must be a string")?,
                );
            } else if key == "coerce" {
                lua_attr.coerce = true;
            } else if key == "this" {
                lua_attr.map_this = Some(syn::parse_str(
                    &val.as_ref()
                        .expect("'this' declaration should have an assigned value")
                        .assume_string_literal("Value for 'self' must be a string")?,
                )?);
            } else if key == "map" {
                lua_attr.map_result = Some(syn::parse_str(
                    &val.as_ref()
                        .expect("'this' declaration should have an assigned value")
                        .assume_string_literal("Value for 'self' must be a string")?,
                )?);
            } else if key == "hidden" {
                lua_attr.hidden_fn = true;
            } else {
                panic!("Unexpected annotation '{key}'");
            }
        }

        Ok(lua_attr)
    }
}
