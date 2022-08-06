use std::collections::BTreeMap;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Attribute, ItemFn, ReturnType, Type};

/// The mini-language inside #[lua] annotations
mod fn_attr;
use fn_attr::*;

use crate::utils::{join_str, parse_doc_attr, unwrap_result};

/// Metadata to generate automatic Lua documentation
#[derive(Debug)]
struct LuaDocstring {
    /// Name of the module where this docstring will be placed
    module: String,
    /// A syntactically valid Lua string of a function definition plus any
    /// available comments.
    doc: String,
}

#[derive(Debug)]
enum LuaFnDefKind {
    Method { class: String },
    Global { table: String },
}

#[derive(Debug)]
struct LuaFnDef {
    kind: LuaFnDefKind,
    /// An item (fn definition) of a function that will register the annotated
    /// function into the lua bindings. The function can be called using the
    /// `register_fn_ident`.
    register_fn_item: TokenStream,
    /// The name of the function in the `register_fn_item`.
    register_fn_ident: Ident,
    /// Lua docstring metadata
    lua_docstr: LuaDocstring,
}

/// Generates the automatic Lua documentation for this `item_fn`, using LuaDoc
/// format. The generated function will have no body, only signature.
fn generate_lua_fn_documentation(item_fn: &ItemFn, attrs: &FunctionAttributes) -> LuaDocstring {
    use std::fmt::Write;
    let doc = (|| -> Result<String, Box<dyn std::error::Error>> {
        let mut docstr = String::new();
        let mut first = true;
        for line in &attrs.docstring_lines {
            if first {
                first = false;
                writeln!(docstr, "--- {line}")?;
            } else {
                writeln!(docstr, "-- {line}")?;
            }
        }
        writeln!(docstr, "--")?;

        let mut param_idents = vec![];
        for param in item_fn.sig.inputs.iter() {
            match param {
                syn::FnArg::Receiver(_) => {
                    writeln!(docstr, "-- @param self The current object")?;
                }
                syn::FnArg::Typed(tpd) => {
                    let name = tpd.pat.to_token_stream().to_string();
                    let typ = tpd.ty.to_token_stream().to_string();
                    writeln!(docstr, "-- @param {name} {typ}")?;
                    param_idents.push(name);
                }
            }
        }

        let fn_name = &item_fn.sig.ident;
        let param_list = join_str(param_idents.iter(), ", ");
        writeln!(docstr, "function {fn_name}({param_list})")?;
        writeln!(docstr, "    error('Documentation stub only')")?;
        writeln!(docstr, "end\n")?;

        Ok(docstr)
    })()
    .unwrap();

    LuaDocstring {
        module: attrs
            .lua_attr
            .under
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "Default".into()),
        doc,
    }
}

/// Some sanity checks for a function annotated as #[lua]
fn lua_fn_sanity_checks(item_fn: &ItemFn) -> syn::Result<()> {
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
    Ok(())
}

enum LuaFnArgKind {
    Owned,
    Ref,
    RefMut,
    SelfRef,
    SelfRefMut,
}

struct LuaFnArg {
    kind: LuaFnArgKind,
    typ: Type,
    name: Ident,
}

fn analyze_lua_fn_args(
    item_fn: &ItemFn,
    fn_def_kind: &LuaFnDefKind,
    attrs: &FunctionAttributes,
) -> syn::Result<Vec<LuaFnArg>> {
    let mut lua_fn_args = vec![];

    for arg in item_fn.sig.inputs.iter() {
        match arg {
            syn::FnArg::Receiver(r) => match fn_def_kind {
                LuaFnDefKind::Method { class } => {
                    let is_mut = r.mutability.is_some();
                    let maybe_mut = if is_mut {
                        quote! { mut }
                    } else {
                        quote! {}
                    };
                    lua_fn_args.push(LuaFnArg {
                        kind: match r.mutability {
                            Some(_mut) => LuaFnArgKind::SelfRefMut,
                            None => LuaFnArgKind::SelfRef,
                        },
                        typ: parse_quote!(& #maybe_mut #class),
                        name: format_ident!("self_ref"),
                    });
                }
                LuaFnDefKind::Global { table } => {
                    return Err(syn::Error::new(
                        item_fn.sig.ident.span(),
                        "Can't use self here.",
                    ));
                }
            },
            syn::FnArg::Typed(t) => {
                let arg_name = match &*t.pat {
                    syn::Pat::Ident(id) => id.clone(),
                    _ => todo!(),
                };
                match &*t.ty {
                    Type::Reference(inner) => {
                        lua_fn_args.push(LuaFnArg {
                            kind: if inner.mutability.is_some() {
                                LuaFnArgKind::RefMut
                            } else {
                                LuaFnArgKind::Ref
                            },
                            typ: *inner.elem.clone(),
                            name: arg_name.ident,
                        });
                    }
                    t => {
                        lua_fn_args.push(LuaFnArg {
                            kind: LuaFnArgKind::Owned,
                            typ: t.clone(),
                            name: arg_name.ident,
                        });
                    }
                }
            }
        }
    }

    Ok(lua_fn_args)
}

/// Given a global function (i.e. not a method) annotated with a #[lua] mark,
/// performs the analysis for that function and returns the collected metadata.
fn analyze_lua_global_fn(
    item_fn: &ItemFn,
    under_table: String,
    attrs: &FunctionAttributes,
) -> syn::Result<LuaFnDef> {
    lua_fn_sanity_checks(item_fn)?;

    let register_fn_ident = format_ident!("__blackjack_export_{}_to_lua", &item_fn.sig.ident);
    let original_fn_name = item_fn.sig.ident.to_string();
    let original_fn_ident = &item_fn.sig.ident;
    let fn_def_kind = LuaFnDefKind::Global {
        table: under_table.clone(),
    };
    let args = analyze_lua_fn_args(item_fn, &fn_def_kind, attrs)?;

    let signature = {
        let types = args.iter().map(|arg| match &arg.kind {
            LuaFnArgKind::Owned => arg.typ.to_token_stream(),
            LuaFnArgKind::Ref | LuaFnArgKind::RefMut => quote! { mlua::AnyUserData },
            LuaFnArgKind::SelfRef | LuaFnArgKind::SelfRefMut => {
                panic!("self is not allowed here.")
            }
        });
        let names = args.iter().map(|arg| &arg.name);

        quote! { (#(#names),*) : (#(#types),*) }
    };

    let borrows = args.iter().filter_map(|arg| {
        let name = &arg.name;
        let typ = &arg.typ;
        match arg.kind {
            LuaFnArgKind::Owned => None,
            LuaFnArgKind::Ref => Some(quote! {
                let #name = #name.borrow::<#typ>()?;
            }),
            LuaFnArgKind::RefMut => Some(quote! {
                let mut #name = #name.borrow_mut::<#typ>()?;
            }),
            LuaFnArgKind::SelfRef | LuaFnArgKind::SelfRefMut => {
                panic!("self is not allowed here.")
            }
        }
    });

    let invoke_args = args.iter().map(|LuaFnArg { kind, name, .. }| match kind {
        LuaFnArgKind::Owned => quote! { #name },
        LuaFnArgKind::Ref => quote! { &#name},
        LuaFnArgKind::RefMut => quote! { &mut #name },
        LuaFnArgKind::SelfRef | LuaFnArgKind::SelfRefMut => {
            panic!("self is not allowed here.")
        }
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

    Ok(LuaFnDef {
        kind: fn_def_kind,
        register_fn_item: quote! {
            pub fn #register_fn_ident(lua: &mlua::Lua) {
                fn __inner(lua: &mlua::Lua, #signature) -> mlua::Result<#ret_typ> {
                    #(#borrows)*
                    #call_fn_and_map_result
                }

                // TODO: This unwrap is not correct. If the table is not there it should be created.
                let table = lua.globals().get::<_, mlua::Table>(#under_table).unwrap();
                table.set(
                    #original_fn_name,
                    lua.create_function(__inner).unwrap()
                ).unwrap()

            }
        },
        register_fn_ident,
        lua_docstr: generate_lua_fn_documentation(item_fn, attrs),
    })
}

/// Given a global function (i.e. not a method) annotated with a #[lua] mark,
/// performs the analysis for that function and returns the collected metadata.
fn analyze_lua_method_fn(
    item_fn: &ItemFn,
    class_name: String,
    attrs: &FunctionAttributes,
) -> syn::Result<LuaFnDef> {
    lua_fn_sanity_checks(item_fn)?;

    let register_fn_ident = format_ident!("__blackjack_export_{}_to_lua", &item_fn.sig.ident);
    let original_fn_name = item_fn.sig.ident.to_string();
    let original_fn_ident = &item_fn.sig.ident;
    let class_ident = format_ident!("{class_name}");
    let fn_def_kind = LuaFnDefKind::Method { class: class_name };
    let args = analyze_lua_fn_args(item_fn, &fn_def_kind, attrs)?;

    let register_fn_item = quote! {
        pub fn #register_fn_ident<'lua, M: mlua::UserDataMethods<'lua, #class_ident>>(methods: &mut M) {
            methods.add_method(#original_fn_name, |lua, __this, (..args here..) : (..types here..)| {
                // WIP: Need to use call_fn_and_map_result here
                #original_fn_ident(..invoke args here..)
            })
        }
    };

    todo!()
}

/// Scans an attribute list, looking for attributes for which `parser_fn`
/// succeeds. Returns any values that matched. If `remove_matches` is true, the
/// matching values are removed from the attribute list.
fn collect_attrs<T>(
    attrs: &mut Vec<Attribute>,
    mut parser_fn: impl FnMut(&Attribute) -> Option<T>,
    remove_matches: bool,
) -> Vec<T> {
    let mut matches = vec![];
    let mut to_remove = vec![];

    for (i, attr) in attrs.iter().enumerate() {
        if let Some(m) = parser_fn(attr) {
            matches.push(m);
            if remove_matches {
                to_remove.push(i);
            }
        }
    }

    for tr in to_remove {
        attrs.remove(tr);
    }

    matches
}

/// If the attribute has a single ident (e.g. #[lua], #[doc]), returns Some(())
/// when the ident is equal to the given `name`, else None.
fn path_ident_is<'a>(attr: &'a Attribute, name: &str) -> Option<&'a Attribute> {
    if let Some(ident) = attr.path.get_ident() {
        if ident == name {
            Some(attr)
        } else {
            None
        }
    } else {
        None
    }
}

/// Collects the #[lua] attribute in a function and any other relevant metadata.
/// Also strips out any annotations that rustc cannot interpret.
fn collect_function_attributes(attrs: &mut Vec<Attribute>) -> Option<FunctionAttributes> {
    // #[lua] special annotations
    let lua_attrs = collect_attrs(
        attrs,
        |attr| {
            path_ident_is(attr, "lua").map(|attr| {
                if attr.tokens.is_empty() {
                    LuaFnAttr::default()
                } else {
                    attr.parse_args::<LuaFnAttr>().unwrap()
                }
            })
        },
        true,
    );
    // Each docstring line, function documentation
    let docstring_lines = collect_attrs(
        attrs,
        |attr| path_ident_is(attr, "doc").map(parse_doc_attr),
        false,
    );

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

fn collect_lua_impl_attrs(attrs: &mut Vec<Attribute>) -> bool {
    let lua_impl_attrs = collect_attrs(
        attrs,
        |attr| path_ident_is(attr, "lua_impl").map(|_| ()),
        true,
    );

    if lua_impl_attrs.len() > 1 {
        panic!("Only one #[lua_impl] annotation is supported per impl block.")
    }

    lua_impl_attrs.len() > 0
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
                    let function_attributes = collect_function_attributes(&mut item_fn.attrs);
                    if let Some(lua_attr) = function_attributes {
                        if let Some(under) = lua_attr.lua_attr.under.as_ref().cloned() {
                            fn_defs.push(analyze_lua_global_fn(item_fn, under, &lua_attr)?);
                        }
                    }
                }
                syn::Item::Impl(item_impl) => {
                    if collect_lua_impl_attrs(&mut item_impl.attrs) {
                        for impl_item in &mut item_impl.items {
                            match impl_item {
                                syn::ImplItem::Method(item_method) => {
                                    let method_attributes =
                                        collect_function_attributes(&mut item_method.attrs);
                                    if let Some(method_attrs) = method_attributes {
                                        dbg!(method_attrs);
                                    }
                                }
                                _ => { /* Ignore */ }
                            }
                        }
                    }
                }
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

    let mut docstrs_by_module = BTreeMap::new();
    for LuaFnDef { lua_docstr, .. } in fn_defs.iter() {
        let module = docstrs_by_module
            .entry(&lua_docstr.module)
            .or_insert_with(Vec::new);
        module.push(lua_docstr.doc.clone());
    }

    let static_docstrings = docstrs_by_module
        .iter()
        .flat_map(|(module, docstrs)| docstrs.iter().map(move |d| quote! { (#module, #d) }));

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

            inventory::submit! {
                blackjack_engine::lua_engine::lua_stdlib::LuaRegisterFn {
                    f: __blackjack_register_lua_fns,
                }
            }

            pub static __blackjack_lua_docstrings : &'static [(&'static str, &'static str)] = &[
                #(#static_docstrings),*
            ];
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


                #[lua_impl]
                impl HalfEdgeMesh {
                    // WIP:
                    // - [ ] Need to abstract the analyze_fn function so that it
                    // can take both a method and a plain fn (or, at least,
                    // figure out how to extract the common logic.)
                    //
                    #[lua]
                    fn set_channel(
                        &mut self,
                        lua: &mlua::Lua,
                        kty: ChannelKeyType,
                        vty: ChannelValueType,
                        name: String,
                        table: mlua::Table,
                    ) -> Result<()> {
                        use slotmap::Key;
                        let conn = self.read_connectivity();
                        let keys: Box<dyn Iterator<Item = u64>> = match kty {
                            ChannelKeyType::VertexId => {
                                Box::new(conn.iter_vertices().map(|(v_id, _)| v_id.data().as_ffi()))
                            }
                            ChannelKeyType::FaceId => {
                                Box::new(conn.iter_faces().map(|(f_id, _)| f_id.data().as_ffi()))
                            }
                            ChannelKeyType::HalfEdgeId => {
                                Box::new(conn.iter_halfedges().map(|(h_id, _)| h_id.data().as_ffi()))
                            }
                        };
                        self.channels
                            .dyn_write_channel_by_name(kty, vty, &name)?
                            .set_from_seq_table(keys, lua, table)
                    }
                }
            }
        };
        let module = syn::parse2(input).unwrap();
        write_and_fmt("/tmp/test.rs", blackjack_lua_module2(module).unwrap()).unwrap();
    }
}
