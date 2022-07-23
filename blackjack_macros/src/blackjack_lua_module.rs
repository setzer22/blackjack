use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{Attribute, ItemFn, ReturnType, Type};

/// The mini-language inside #[lua] annotations
mod fn_attr;
use fn_attr::*;

use crate::utils::{parse_doc_attr, unwrap_result};

#[derive(Debug)]
struct LuaFnDef {
    /// An item (fn definition) of a function that will register the annotated
    /// function into the lua bindings. The function can be called using the
    /// `register_fn_ident`.
    register_fn_item: TokenStream,
    /// The name of the function in the `register_fn_item`.
    register_fn_ident: Ident,
    /// A syntactically valid Lua string of a function definition compatible
    /// with the annotated function. This is used to generate automatic
    /// documentation for the Lua API, the code is not meant to be called by the
    /// Lua interpreter.
    lua_docstr: String,
}

/// Given a function annotated with a #[lua] mark, performs the analysis for
/// that function and returns the collected metadata.
fn analyze_lua_fn(item_fn: &ItemFn, attrs: &FunctionAttributes) -> syn::Result<LuaFnDef> {
    // Some sanity checks
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

    #[rustfmt::skip]
    enum ArgKind { Owned, Ref, RefMut }
    struct WrapperArg {
        kind: ArgKind,
        typ: Type,
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
                    Type::Reference(inner) => {
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

    let register_fn_ident = format_ident!("__blackjack_export_{}_to_lua", &item_fn.sig.ident);
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
        let typ = &arg.typ;
        match arg.kind {
            ArgKind::Owned => None,
            ArgKind::Ref => Some(quote! {
                let #name = #name.borrow::<#typ>()?;
            }),
            ArgKind::RefMut => Some(quote! {
                let mut #name = #name.borrow_mut::<#typ>()?;
            }),
        }
    });

    let invoke_args = wrapper_fn_args
        .iter()
        .map(|WrapperArg { kind, name, .. }| match kind {
            ArgKind::Owned => quote! { #name },
            ArgKind::Ref => quote! { &#name},
            ArgKind::RefMut => quote! { &mut #name },
        });

    let (ret_typ, ret_is_result) = match &item_fn.sig.output {
        ReturnType::Default => (quote! { () }, false),
        ReturnType::Type(_, t) => match unwrap_result(t) {
            Some(inner) => (quote! { #inner }, true),
            None => (quote! { #t }, false),
        },
    };

    let call_fn_and_map_result = if ret_is_result {
        quote! {
            match #original_fn_ident(#(#invoke_args),*) {
                Ok(val) => { mlua::Result::Ok(val) },
                Err(err) => {
                    mlua::Result::Err(mlua::Error::RuntimeError(format!("{:?}", err)))
                }
            }
        }
    } else {
        quote! {
            mlua::Result::Ok(#original_fn_ident(#(#invoke_args),*))
        }
    };

    use std::fmt::Write;
    let lua_docstr = (|| -> Result<String, Box<dyn std::error::Error>> {
        let mut docstr = String::new();

        // Look for docstring comments
        for line in &attrs.docstring_lines {
            writeln!(docstr, "-- {line}")?;
        }
        writeln!(docstr, "function {original_fn_name}(TODO PARAMS)")?;
        writeln!(docstr, "    error('Documentation stub only')")?;
        writeln!(docstr, "end")?;

        Ok(docstr)
    })()
    .unwrap();

    Ok(LuaFnDef {
        register_fn_item: quote! {
            pub fn #register_fn_ident(lua: &mlua::Lua) {
                fn __inner(lua: &mlua::Lua, #signature) -> mlua::Result<#ret_typ> {
                    #(#borrows)*
                    #call_fn_and_map_result
                }

                // TODO: This unwrap is not correct. If the table is not there it should be created.
                let table = lua.globals().get::<_, mlua::Table>("Ops").unwrap();
                table.set(
                    #original_fn_name,
                    lua.create_function(__inner).unwrap()
                ).unwrap()

            }
        },
        register_fn_ident,
        lua_docstr,
    })
}

/// Collects the #[lua] attribute in a function and any other relevant metadata.
/// Also strips out any annotations that rustc cannot interpret.
fn collect_lua_attr(attrs: &mut Vec<Attribute>) -> Option<FunctionAttributes> {
    let mut lua_attrs = vec![];
    let mut to_remove = vec![];
    let mut docstring_lines = vec![];
    for (i, attr) in attrs.iter().enumerate() {
        if let Some(ident) = attr.path.get_ident() {
            // A #[lua] special annotation
            if ident == "lua" {
                let lua_attr: LuaFnAttr = attr.parse_args().unwrap();
                lua_attrs.push(lua_attr);
                to_remove.push(i);
            }
            // A docstring
            else if ident == "doc" {
                docstring_lines.push(parse_doc_attr(attr));
            }
        }
    }

    for i in to_remove.into_iter() {
        attrs.remove(i);
    }

    if lua_attrs.len() > 1 {
        panic!("Only one #[lua(...)] annotation is supported per function.")
    }

    lua_attrs
        .into_iter()
        .next()
        .map(|lua_attr| FunctionAttributes {
            lua_attr,
            docstring_lines,
        })
}

pub(crate) fn blackjack_lua_module2(
    mut module: syn::ItemMod,
) -> Result<TokenStream, Box<dyn std::error::Error>> {
    // Any new items that will be appended at the end of the module are stored here.
    let mut fn_defs = vec![];

    if let Some((_, items)) = module.content.as_mut() {
        for item in items.iter_mut() {
            match item {
                syn::Item::Fn(item_fn) => {
                    let lua_attr = collect_lua_attr(&mut item_fn.attrs);
                    if let Some(lua_attr) = lua_attr {
                        fn_defs.push(analyze_lua_fn(item_fn, &lua_attr)?);
                    }
                }
                syn::Item::Impl(_) => todo!(),
                _ => { /* Ignore */ }
            }
        }
    } else {
        panic!("This macro only supports inline modules")
    }

    let global_register_fn_calls = fn_defs.iter().map(
        |LuaFnDef {
             register_fn_ident, ..
         }| {
            quote! { #register_fn_ident(lua); }
        },
    );

    let print_docstrings = fn_defs
        .iter()
        .map(|LuaFnDef { lua_docstr, .. }| quote! { println!(#lua_docstr); });

    let original_items = module.content.as_ref().unwrap().1.iter();
    let register_fns = fn_defs.iter().map(|n| &n.register_fn_item);
    let mod_name = module.ident;
    let visibility = module.vis;

    Ok(quote! {
        #visibility mod #mod_name {
            #(#original_items)*
            #(#register_fns)*

            pub fn __blackjack_register_lua_fns(lua: &mlua::Lua) {
                #(#global_register_fn_calls)*
            }

            pub fn __blackjack_print_lua_docstrings(lua: &mlua::Lua) {
                #(#print_docstrings)*
            }
        }
    })
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::utils::write_and_fmt;

    #[test]
    fn test() {
        let input = quote! {
            pub mod lua_fns {
                use super::*;

                #[lua(under = "Ops")]
                pub fn test_exported_fn(
                    mesh: &mut HalfEdgeMesh,
                ) -> Result<i32> {
                    let mut conn = mesh.write_connectivity();
                    let f = conn.iter_faces().next().unwrap().0;
                    conn.remove_face(f);
                    Ok(42)
                }
            }
        };
        let module = syn::parse2(input).unwrap();
        write_and_fmt("/tmp/test.rs", blackjack_lua_module2(module).unwrap()).unwrap();
    }

    #[test]
    fn repl() {
        let attr: syn::ItemMod = syn::parse_quote! {
            #[doc = r" Single line doc comments"]
            #[doc = r" We write so many!"]
            #[doc = r"* Multi-line comments...
                      * May span many lines
            "]
            mod example {
                #![doc = r" Of course, they can be inner too"]
                #![doc = r" And fit in a single line "]
            }
        };

        dbg!(attr.attrs);
    }
}
