use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Expr, ItemFn, Token,
};

use crate::utils::{ExprUtils, SynParseBufferExt};

#[derive(Default, Debug)]
struct LuaFnAttrs {
    under: Option<String>,
}

impl Parse for LuaFnAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let properties = input.comma_separated_fn(|input| {
            let lhs: Ident = input.parse()?;
            let _eq_sign = input.expect_token::<Token![=]>();
            let rhs: Expr = input.parse()?;
            Ok((lhs, rhs))
        })?;

        let mut lua_attr = LuaFnAttrs::default();

        for (key, val) in properties.iter() {
            if key == "under" {
                lua_attr.under =
                    Some(val.assume_string_literal("Value for 'under' must be a string")?);
            }
        }

        Ok(lua_attr)
    }
}

#[derive(Debug)]
struct LuaFnDef {
    register_fn_ident: Ident,
    register_fn_item: TokenStream,
}

fn analyze_lua_fn(item_fn: &ItemFn, attrs: &LuaFnAttrs) -> syn::Result<LuaFnDef> {
    if item_fn.sig.generics.params.iter().count() > 0 {
        return Err(syn::Error::new(
            item_fn.sig.ident.span(),
            "Functions exported to lua can't have generic parameters.",
        ));
    } else if item_fn.sig.asyncness.is_some() {
        return Err(syn::Error::new(
            item_fn.sig.ident.span(),
            "Functions exported to lua can't be marked async.",
        ));
    }

    enum ArgKind {
        Owned,
        Ref,
        RefMut,
    }

    struct WrapperArg {
        kind: ArgKind,
        typ: syn::Type,
        name: Ident,
    }

    let mut wrapper_fn_args = vec![];

    for arg in item_fn.sig.inputs.iter() {
        match arg {
            syn::FnArg::Receiver(_) => {
                return Err(syn::Error::new(
                    item_fn.sig.ident.span(),
                    "Can't use self here.",
                ));
            }
            syn::FnArg::Typed(t) => {
                let arg_name = match &*t.pat {
                    syn::Pat::Ident(id) => id.clone(),
                    _ => todo!(),
                };
                match &*t.ty {
                    syn::Type::Reference(inner) => {
                        wrapper_fn_args.push(WrapperArg {
                            kind: if inner.mutability.is_some() {
                                ArgKind::RefMut
                            } else {
                                ArgKind::Ref
                            },
                            typ: *inner.elem.clone(),
                            name: arg_name.ident,
                        });
                    }
                    t => {
                        wrapper_fn_args.push(WrapperArg {
                            kind: ArgKind::Owned,
                            typ: t.clone(),
                            name: arg_name.ident,
                        });
                    }
                }
            }
        }
    }

    let register_fn_ident = format_ident!("export_{}_to_lua", &item_fn.sig.ident);
    let original_fn_name = item_fn.sig.ident.to_string();
    let original_fn_ident = &item_fn.sig.ident;

    let signature = {
        let types = wrapper_fn_args.iter().map(|arg| match &arg.kind {
            ArgKind::Owned => arg.typ.to_token_stream(),
            ArgKind::Ref | ArgKind::RefMut => quote! { mlua::AnyUserData },
        });
        let names = wrapper_fn_args.iter().map(|arg| &arg.name);

        quote! { (#(#names),*) : (#(#types),*) }
    };

    let borrows = wrapper_fn_args.iter().filter_map(|arg| {
        let name = &arg.name;
        let typ = &arg.name;
        match arg.kind {
            ArgKind::Owned => None,
            ArgKind::Ref => Some(quote! {
                let #name = #name.borrow::<#typ>()?;
            }),
            ArgKind::RefMut => Some(quote! {
                let #name = #name.borrow_mut::<#typ>()?;
            }),
        }
    });

    let invoke_args = wrapper_fn_args.iter().map(|arg| &arg.name);

    Ok(LuaFnDef {
        register_fn_item: quote! {
            fn #register_fn_ident(lua: &mlua::Lua) {
                fn __inner(lua: &mlua::Lua, #signature) {
                    #(#borrows)*
                    #original_fn_ident(#(#invoke_args),*)
                }

                let table = lua.globals.get("Ops");
                table.set(
                    #original_fn_name,
                    lua.create_function(__inner).unwrap()
                ).unwrap()

            }
        },
        register_fn_ident,
    })
}

pub(crate) fn blackjack_lua_module2(
    mut module: syn::ItemMod,
) -> Result<TokenStream, Box<dyn std::error::Error>> {
    // Any new items that will be appended at the end of the module are stored here.
    let mut new_items = vec![];

    if let Some((_, items)) = module.content.as_mut() {
        for item in items.iter() {
            match item {
                syn::Item::Fn(item_fn) => {
                    let lua_attr = item_fn.attrs.iter().find_map(|attr| {
                        attr.path.get_ident().and_then(|ident| {
                            if ident == "lua" {
                                let lua_attr: LuaFnAttrs = attr.parse_args().unwrap();
                                Some(lua_attr)
                            } else {
                                None
                            }
                        })
                    });

                    if let Some(lua_attr) = lua_attr {
                        new_items.push(analyze_lua_fn(item_fn, &lua_attr)?);
                    }
                }
                syn::Item::Impl(_) => todo!(),
                _ => { /* Ignore */ }
            }
        }
    } else {
        panic!("This macro only supports inline modules")
    }

    let original_items = module.content.as_ref().unwrap().1.iter();
    let new_items = new_items.iter().map(|n| &n.register_fn_item);
    let mod_name = module.ident;

    Ok(quote! {
        mod #mod_name {
            #(#original_items)*
            #(#new_items)*
        }
    })
}

#[cfg(test)]
mod test {

    use crate::utils::write_and_fmt;
    use super::*;

    #[test]
    fn test() {
        let input = quote! {
            mod lua_functions {
                /// Modifies the given `mesh`, beveling all selected `edges` in
                /// a given distance `amount`.
                #[lua(under = "Ops")]
                fn bevel(mesh: &mut HalfEdgeMesh, edges: SelectionExpression, amount: f32) {

                }
            }
        };
        let module = syn::parse2(input).unwrap();
        write_and_fmt("/tmp/test.rs", blackjack_lua_module2(module).unwrap()).unwrap();
    }
}
