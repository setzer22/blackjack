use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    Expr, Token,
};

use crate::utils::{ExprUtils, SynParseBufferExt};

#[derive(Default, Debug)]
pub struct LuaFnAttr {
    pub under: Option<String>,
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
            let _eq_sign = input.expect_token::<Token![=]>();
            let rhs: Expr = input.parse()?;
            Ok((lhs, rhs))
        })?;

        let mut lua_attr = LuaFnAttr::default();

        for (key, val) in properties.iter() {
            if key == "under" {
                lua_attr.under =
                    Some(val.assume_string_literal("Value for 'under' must be a string")?);
            }
        }

        Ok(lua_attr)
    }
}
