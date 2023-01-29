// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::BTreeMap;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Attribute, ReturnType, Signature, Type};

/// The mini-language inside #[lua] annotations
mod fn_attr;
use fn_attr::*;

use crate::utils::{join_str, parse_doc_attr, unwrap_result};

/// Metadata to generate automatic Lua documentation
#[derive(Debug)]
struct LuaDocstring {
    /// The module, or class this docstring refers to.
    def_kind: LuaFnDefKind,
    /// A syntactically valid Lua string of a function definition plus any
    /// available comments.
    doc: String,
}

#[derive(Debug, Clone)]
enum LuaFnDefKind {
    Method { class: String },
    Global { table: String },
    GlobalConstant { table: String },
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

#[derive(Debug)]
struct LuaConst {
    register_const_fn_item: TokenStream,
    register_const_fn_ident: Ident,
    lua_docstr: LuaDocstring,
}

/// Generates the automatic Lua documentation for this `item_fn`, using LuaDoc
/// format. The generated function will have no body, only signature.
fn generate_lua_fn_documentation(
    item_fn: &GlobalFnOrMethod,
    attrs: &FunctionAttributes,
    fn_def_kind: &LuaFnDefKind,
) -> LuaDocstring {
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
        def_kind: fn_def_kind.clone(),
        doc,
    }
}

/// Generates the automatic Lua documentation for this `item_const`, using
/// LuaDoc format. The generated constant will be set to null
fn generate_lua_const_documentation(
    item_const: &syn::ItemConst,
    attrs: &FunctionAttributes,
) -> LuaDocstring {
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

        let const_name = &item_const.ident;
        writeln!(docstr, "local {const_name} = null")?;

        Ok(docstr)
    })()
    .unwrap();

    LuaDocstring {
        def_kind: LuaFnDefKind::GlobalConstant {
            table: attrs
                .lua_attr
                .under
                .clone()
                .unwrap_or_else(|| "Default".into()),
        },
        doc,
    }
}

/// Some sanity checks for a function annotated as #[lua]
fn lua_fn_sanity_checks(item_fn: &GlobalFnOrMethod) -> syn::Result<()> {
    // Lifetime parameters are allowed, but not other kinds
    item_fn
        .sig
        .generics
        .params
        .iter()
        .try_for_each(|x| match x {
            syn::GenericParam::Lifetime(_) => Ok(()),
            _ => Err(syn::Error::new(
                item_fn.sig.ident.span(),
                "Functions exported to lua can't have generic parameters.",
            )),
        })?;

    // Async functions are not allowed
    if item_fn.sig.asyncness.is_some() {
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
    LuaRef,
}

struct LuaFnArg {
    kind: LuaFnArgKind,
    typ: Type,
    name: Ident,
}

struct LuaFnReturn {
    inner_type: TokenStream,
    is_result: bool,
}

struct LuaFnSignature {
    inputs: Vec<LuaFnArg>,
    output: LuaFnReturn,
}

fn analyze_lua_fn_args(
    item_fn: &GlobalFnOrMethod,
    fn_def_kind: &LuaFnDefKind,
) -> syn::Result<LuaFnSignature> {
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

                    let class_ident = format_ident!("{class}");
                    lua_fn_args.push(LuaFnArg {
                        kind: match r.mutability {
                            Some(_mut) => LuaFnArgKind::SelfRefMut,
                            None => LuaFnArgKind::SelfRef,
                        },
                        typ: parse_quote!(& #maybe_mut #class_ident),
                        name: format_ident!("self_ref"),
                    });
                }
                LuaFnDefKind::Global { .. } => {
                    return Err(syn::Error::new(
                        item_fn.sig.ident.span(),
                        "Can't use self here.",
                    ));
                }
                LuaFnDefKind::GlobalConstant { .. } => {
                    panic!("Not a function. This should never happen.");
                }
            },
            syn::FnArg::Typed(t) => {
                let arg_name = match &*t.pat {
                    syn::Pat::Ident(id) => id.clone(),
                    _ => todo!(),
                };
                match &*t.ty {
                    Type::Reference(inner) => {
                        if inner.elem.to_token_stream().to_string() == "Lua" {
                            lua_fn_args.push(LuaFnArg {
                                kind: LuaFnArgKind::LuaRef,
                                typ: *inner.elem.clone(),
                                name: arg_name.ident,
                            });
                        } else {
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

    let (ret_typ, ret_is_result) = match &item_fn.sig.output {
        ReturnType::Default => (quote! { () }, false),
        ReturnType::Type(_, t) => match unwrap_result(t) {
            Some(inner) => (quote! { #inner }, true),
            None => (quote! { #t }, false),
        },
    };

    Ok(LuaFnSignature {
        inputs: lua_fn_args,
        output: LuaFnReturn {
            inner_type: ret_typ,
            is_result: ret_is_result,
        },
    })
}

/// Syn distinguishes between an fn and a method. This struct stores the common
/// bits we care about so our code can be agnostic over that without introducing
/// traits.
struct GlobalFnOrMethod<'a> {
    sig: &'a mut Signature,
}

/// Given a global function (i.e. not a method) annotated with a #[lua] mark,
/// performs the analysis for that function and returns the collected metadata.
fn analyze_lua_global_fn(
    item_fn: &GlobalFnOrMethod,
    under_table: String,
    attrs: &FunctionAttributes,
) -> syn::Result<LuaFnDef> {
    lua_fn_sanity_checks(item_fn)?;

    let register_fn_ident =
        format_ident!("__blackjack_export_global_fn_{}_to_lua", &item_fn.sig.ident);
    let original_fn_name = item_fn.sig.ident.to_string();
    let original_fn_ident = &item_fn.sig.ident;
    let fn_def_kind = LuaFnDefKind::Global {
        table: under_table.clone(),
    };

    let signature = analyze_lua_fn_args(item_fn, &fn_def_kind)?;
    let fn_sig_args_code = signature.code_for_fn_signature();
    let fn_borrows_code = signature.code_for_fn_borrows();
    let fn_invoke_args_code = signature.code_for_fn_invoke_args();
    let call_fn_and_map_result_code = signature.code_for_call_fn_and_map_result(
        quote! { #original_fn_ident },
        fn_invoke_args_code,
        None,
        None,
    );
    let ret_typ_code = &signature.output.inner_type;

    Ok(LuaFnDef {
        lua_docstr: generate_lua_fn_documentation(item_fn, attrs, &fn_def_kind),
        kind: fn_def_kind,
        register_fn_item: quote! {
            pub fn #register_fn_ident(lua: &mlua::Lua) -> mlua::Result<()> {
                fn __inner(lua: &mlua::Lua, #fn_sig_args_code) -> mlua::Result<#ret_typ_code> {
                    #(#fn_borrows_code)*
                    #call_fn_and_map_result_code
                }

                // Ensure the table exists before accessing
                if !lua.globals().contains_key(#under_table)? {
                    lua.globals().set(#under_table, lua.create_table()?)?;
                }
                let table = lua.globals().get::<_, mlua::Table>(#under_table).unwrap();


                table.set(
                    #original_fn_name,
                    lua.create_function(__inner)?
                )?;

                Ok(())
            }
        },
        register_fn_ident,
    })
}

/// Given a method inside an impl block annotated with a #[lua] mark, performs
/// the analysis for that method and returns the collected metadata.
fn analyze_lua_method_fn(
    item_fn: &mut GlobalFnOrMethod,
    class_name: String,
    attrs: &FunctionAttributes,
) -> syn::Result<LuaFnDef> {
    lua_fn_sanity_checks(item_fn)?;

    let fn_def_kind = LuaFnDefKind::Method {
        class: class_name.clone(),
    };

    // It's important to generate this now before we (maybe) mutate the function
    // below. See the NOTE.
    let docstring = generate_lua_fn_documentation(item_fn, attrs, &fn_def_kind);

    let register_fn_ident =
        format_ident!("__blackjack_export_method_{}_to_lua", &item_fn.sig.ident);
    // NOTE: The original_fn_name stores the name of the function as it's going
    // to be bound to lua. OTOH, the original_fn_ident stores the name of the
    // function as it's going to be bound to rust. This difference is important
    // when the 'hidden' attribute is set to true, because in that case we
    // mutate the fn identifier to add a __lua_hidden_ prefix. The zzzz is
    // just a hack so these functions will appear at the bottom of
    // auto-completion list, because Rust language servers don't take function
    // visibility into account for autocompletion.
    let original_fn_name = item_fn.sig.ident.to_string();
    if attrs.lua_attr.hidden_fn {
        let new_ident = format_ident!("__lua_hidden_{}", item_fn.sig.ident);
        item_fn.sig.ident = new_ident;
    }
    let original_fn_ident = &item_fn.sig.ident;
    let class_ident = format_ident!("{class_name}");

    let signature = analyze_lua_fn_args(item_fn, &fn_def_kind)?;
    let fn_sig_args_code = signature.code_for_fn_signature();
    let fn_borrows_code = signature.code_for_fn_borrows();
    let fn_invoke_args_code = signature.code_for_fn_invoke_args();

    let call_fn_and_map_result_code = signature.code_for_call_fn_and_map_result(
        if let Some(self_expr) = attrs.lua_attr.map_this.as_ref() {
            quote! { this.#self_expr.#original_fn_ident }
        } else {
            quote! { this.#original_fn_ident }
        },
        fn_invoke_args_code,
        attrs
            .lua_attr
            .coerce
            .then(|| signature.output.inner_type.clone()),
        attrs.lua_attr.map_result.as_ref(),
    );

    let add_method_maybe_mut_code = if let Some(rcv) = signature.inputs.first() {
        match rcv.kind {
            LuaFnArgKind::SelfRef => quote! { add_method },
            LuaFnArgKind::SelfRefMut => quote! { add_method_mut },
            _ => {
                panic!("Method should take &self or &mut self");
            }
        }
    } else {
        panic!("Method should take &self or &mut self");
    };

    let register_fn_item = quote! {
        pub fn #register_fn_ident<'lua, M: mlua::UserDataMethods<'lua, #class_ident>>(methods: &mut M) {
            methods.#add_method_maybe_mut_code(#original_fn_name, |lua, this, #fn_sig_args_code| {
                #(#fn_borrows_code)*
                #call_fn_and_map_result_code
            })
        }
    };

    Ok(LuaFnDef {
        lua_docstr: docstring,
        kind: fn_def_kind,
        register_fn_item,
        register_fn_ident,
    })
}

/// Given a constant declaration inside the lua module with a #[lua] mark, the
/// analysis for it and returns the collected metadata.
fn analyze_lua_const(
    item_const: &mut syn::ItemConst,
    attributes: &FunctionAttributes,
) -> syn::Result<LuaConst> {
    let register_const_fn_ident =
        format_ident!("__blackjack_export_const_{}_to_lua", item_const.ident);
    let original_const_ident = &item_const.ident;
    let under_table = attributes
        .lua_attr
        .under
        .clone()
        .unwrap_or_else(|| "Default".into());
    let register_const_fn_item = quote! {
        #[allow(non_snake_case)]
        pub fn #register_const_fn_ident(lua: &mlua::Lua) -> mlua::Result<()> {
            // Ensure the table exists before accessing
            if !lua.globals().contains_key(#under_table)? {
                lua.globals().set(#under_table, lua.create_table()?)?;
            }
            let table = lua.globals().get::<_, mlua::Table>(#under_table).unwrap();


            table.set(
                stringify!(#original_const_ident),
                #original_const_ident,
            )?;

            Ok(())
        }
    };
    Ok(LuaConst {
        register_const_fn_item,
        register_const_fn_ident,
        lua_docstr: generate_lua_const_documentation(item_const, attributes),
    })
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

/// Returns whether `m` corresponds to an empty method inside an impl block, e.g.:
/// ```ignore
/// fn foo(a: T, b: U);
/// ```
fn is_empty_method(m: &syn::ImplItemMethod) -> bool {
    if let &[syn::Stmt::Item(syn::Item::Verbatim(semi))] = &m.block.stmts.as_slice() {
        semi.to_string() == ";"
    } else {
        false
    }
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

    !lua_impl_attrs.is_empty()
}

pub(crate) fn blackjack_lua_module2(
    mut module: syn::ItemMod,
) -> Result<TokenStream, Box<dyn std::error::Error>> {
    // Any new function definitions that will be exported to Lua at the end of
    // the module are stored here.
    let mut fn_defs = vec![];

    // Similarly Const defs are stored here.
    let mut const_defs = vec![];

    if let Some((_, items)) = module.content.as_mut() {
        for item in items.iter_mut() {
            match item {
                syn::Item::Fn(item_fn) => {
                    let function_attributes = collect_function_attributes(&mut item_fn.attrs);
                    if let Some(lua_attr) = function_attributes {
                        if let Some(under) = lua_attr.lua_attr.under.as_ref().cloned() {
                            let item_fn = GlobalFnOrMethod {
                                sig: &mut item_fn.sig,
                            };
                            fn_defs.push(analyze_lua_global_fn(&item_fn, under, &lua_attr)?);
                        }
                    }
                }
                syn::Item::Impl(item_impl) => {
                    if collect_lua_impl_attrs(&mut item_impl.attrs) {
                        let mut to_delete = vec![];
                        for (i, impl_item) in item_impl.items.iter_mut().enumerate() {
                            if let syn::ImplItem::Method(item_method) = impl_item {
                                // Empty methods are only used to tell the
                                // macro to forward this method which is
                                // declared somewhere else, so remove the
                                // declaration to avoid rustc complaining
                                // about an empty conflicting method.
                                if is_empty_method(item_method) {
                                    to_delete.push(i);
                                }

                                let method_attributes =
                                    collect_function_attributes(&mut item_method.attrs);
                                let class_name = item_impl.self_ty.to_token_stream().to_string();
                                if let Some(method_attrs) = method_attributes {
                                    let mut item_fn = GlobalFnOrMethod {
                                        sig: &mut item_method.sig,
                                    };
                                    fn_defs.push(analyze_lua_method_fn(
                                        &mut item_fn,
                                        class_name,
                                        &method_attrs,
                                    )?);
                                }
                            }
                        }
                        // Execute the deferred deletes
                        for idx in to_delete.iter().rev() {
                            item_impl.items.remove(*idx);
                        }
                    }
                }
                syn::Item::Const(item_const) => {
                    if let Some(attributes) = collect_function_attributes(&mut item_const.attrs) {
                        const_defs.push(analyze_lua_const(item_const, &attributes)?);
                    }
                }
                _ => { /* Ignore */ }
            }
        }
    } else {
        panic!("This macro only supports inline modules")
    }

    let register_global_fn_calls_code = {
        let calls = fn_defs
            .iter()
            .filter(|f| matches!(f.kind, LuaFnDefKind::Global { .. }))
            .map(
                |LuaFnDef {
                     register_fn_ident, ..
                 }| {
                    quote! { #register_fn_ident(lua)?; }
                },
            );

        let const_calls = const_defs.iter().map(
            |LuaConst {
                 register_const_fn_ident,
                 ..
             }| {
                quote! { #register_const_fn_ident(lua)?; }
            },
        );

        quote! {
            pub fn __blackjack_register_lua_fns(lua: &mlua::Lua) -> mlua::Result<()> {
                #(#calls)*
                #(#const_calls)*
                Ok(())
            }

            inventory::submit! {
                blackjack_engine::lua_engine::lua_stdlib::LuaRegisterFn {
                    f: __blackjack_register_lua_fns,
                }
            }
        }
    };

    let register_method_fn_calls_code = {
        let mut calls_by_class = BTreeMap::<String, Vec<TokenStream>>::new();
        fn_defs.iter().for_each(
            |LuaFnDef {
                 kind,
                 register_fn_ident,
                 ..
             }| {
                if let LuaFnDefKind::Method { class } = kind {
                    let code = quote! {
                        #register_fn_ident(methods);
                    };
                    calls_by_class.entry(class.clone()).or_default().push(code);
                }
            },
        );

        calls_by_class.into_iter().map(|(class, calls)| {
            let class_ident = format_ident!("{class}");
            quote! {
                impl mlua::UserData for #class_ident {
                    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(
                        fields: &mut F
                    ) {
                        /* Nothing, for now */
                    }

                    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(
                        methods: &mut M
                    ) {
                        #(#calls)*
                    }
                }
            }
        })
    };

    let static_docstrings_code = fn_defs
        .iter()
        .map(|x| &x.lua_docstr)
        .chain(const_defs.iter().map(|x| &x.lua_docstr))
        .map(|lua_docstr| {
            let (typ, name) = match &lua_docstr.def_kind {
                LuaFnDefKind::Method { class } => ("class", class),
                LuaFnDefKind::Global { table } => ("module", table),
                LuaFnDefKind::GlobalConstant { table } => ("module", table),
            };
            let doc = &lua_docstr.doc;

            quote! { (#typ, #name, #doc) }
        });

    let original_items_code = module.content.as_ref().unwrap().1.iter();
    let register_fns_code = fn_defs.iter().map(|n| &n.register_fn_item);
    let register_consts_code = const_defs.iter().map(|n| &n.register_const_fn_item);
    let mod_name = module.ident;
    let visibility = module.vis;
    let mod_attrs = module.attrs.iter().filter(|attr| {
        attr.path.to_token_stream().to_string() != "blackjack_macros::blackjack_lua_module"
    });

    Ok(quote! {
        #(#mod_attrs)*
        #visibility mod #mod_name {
            #(#original_items_code)*

            // One function for each function-like object (method, global fn)
            // that needs to be registered here..
            #(#register_fns_code)*

            // One function for constant that needs to be registered here..
            #(#register_consts_code)*

            // A single function to register all global functions in this module
            #register_global_fn_calls_code

            // Functions to register all methods, grouped by class name
            #(#register_method_fn_calls_code)*

            // Docstrings are grouped at the end, ldoc sorts them out
            #[allow(non_upper_case_globals)]
            pub static __blackjack_lua_docstrings : &'static [(&'static str, &'static str, &'static str)] = &[
                #(#static_docstrings_code),*
            ];

            inventory::submit! {
                blackjack_engine::lua_engine::lua_stdlib::LuaDocstringData {
                    data: __blackjack_lua_docstrings,
                }
            }
        }
    })
}

impl LuaFnSignature {
    /// Returns generated code to correctly borrow each of the arguments inside
    /// a Lua fn, assuming the arguments were taken as AnyUserData instead of
    /// the user specified values such as &T.
    ///
    /// E.g. for a signature of
    /// ```ignore
    /// fn foo(a: &HalfEdgeMesh, b: u32, c: &mut PerlinNoise) { }
    /// ```
    ///
    /// Would generate two lines of code:
    /// ```ignore
    /// let a = a.borrow::<HalfEdgeMesh>()?;
    /// let mut c = c.borrow_mut::<PerlinNoise>()?;
    /// ```
    fn code_for_fn_borrows(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.inputs.iter().filter_map(|arg| {
            let name = &arg.name;
            let typ = &arg.typ;
            match arg.kind {
                LuaFnArgKind::Ref => Some(quote! {
                    let #name = #name.borrow::<#typ>()?;
                }),
                LuaFnArgKind::RefMut => Some(quote! {
                    let mut #name = #name.borrow_mut::<#typ>()?;
                }),
                LuaFnArgKind::SelfRef
                | LuaFnArgKind::SelfRefMut
                | LuaFnArgKind::LuaRef
                | LuaFnArgKind::Owned => None,
            }
        })
    }

    /// Returns generated code to specify the fn signature of a function when
    /// exposing it to Lua, wrapping it as a tuple of arguments. Any references
    /// are mapped to AnyUserData. This mapping is then undone by shadowing
    /// those variables in `code_for_fn_borrows`.
    ///
    /// For methods, the self argument is ommitted.
    ///
    /// E.g. for a signature of
    /// ```ignore
    /// fn foo(a: &HalfEdgeMesh, b: u32, c: &mut PerlinNoise) { }
    /// ```
    ///
    /// Would generate
    /// ```ignore
    /// (a, b, c): (AnyUserData, u32, AnyUserData)
    /// ```
    fn code_for_fn_signature(&self) -> TokenStream {
        let types = self.inputs.iter().filter_map(|arg| match &arg.kind {
            LuaFnArgKind::Owned => Some(arg.typ.to_token_stream()),
            LuaFnArgKind::Ref | LuaFnArgKind::RefMut => Some(quote! { mlua::AnyUserData }),
            LuaFnArgKind::SelfRef | LuaFnArgKind::SelfRefMut | LuaFnArgKind::LuaRef => {
                // We can safely ignore self values here, because when they
                // occur, they don't go inside the tuple. Same for the lua
                // reference, which is always a separate argument from the
                // tuple.
                None
            }
        });
        let names = self.inputs.iter().filter_map(|arg| match &arg.kind {
            LuaFnArgKind::Owned | LuaFnArgKind::Ref | LuaFnArgKind::RefMut => Some(&arg.name),
            LuaFnArgKind::SelfRef | LuaFnArgKind::SelfRefMut | LuaFnArgKind::LuaRef => None,
        });

        quote! { (#(#names),*) : (#(#types),*) }
    }

    /// Emits code for the function arguments as they need to be specified at
    /// the calling site. For methods, the self argument is ommitted.
    ///
    /// E.g. for a signature of
    /// ```ignore
    /// fn foo(a: &HalfEdgeMesh, b: u32, c: &mut PerlinNoise) { }
    /// ```
    ///
    /// Would generate
    /// ```ignore
    /// &a, b, &mut c
    /// ```
    fn code_for_fn_invoke_args(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.inputs
            .iter()
            .filter_map(|LuaFnArg { kind, name, .. }| match kind {
                LuaFnArgKind::Owned => Some(quote! { #name }),
                LuaFnArgKind::Ref => Some(quote! { &#name}),
                LuaFnArgKind::RefMut => Some(quote! { &mut #name }),
                LuaFnArgKind::LuaRef => Some(quote! { lua }),
                LuaFnArgKind::SelfRef | LuaFnArgKind::SelfRefMut => None,
            })
    }

    fn code_for_call_fn_and_map_result(
        &self,
        fn_expr: TokenStream,
        fn_invoke_args_code: impl Iterator<Item = TokenStream>,
        coerce_return_type: Option<TokenStream>,
        map_result: Option<&syn::Expr>,
    ) -> TokenStream {
        let maybe_coercion = if let Some(coerce_ret) = coerce_return_type {
            quote! { .map(|x| <#coerce_ret>::from(x)) }
        } else {
            quote! {}
        };

        let maybe_map_result = if let Some(map_result) = map_result {
            quote! { .map(|x| #map_result ) }
        } else {
            quote! {}
        };

        if self.output.is_result {
            quote! {
                match #fn_expr(#(#fn_invoke_args_code),*) {
                    Ok(val) => { mlua::Result::Ok(val) },
                    Err(err) => {
                        mlua::Result::Err(mlua::Error::RuntimeError(format!("{:?}", err)))
                    }
                }
                #maybe_coercion
                #maybe_map_result
            }
        } else {
            quote! {
                mlua::Result::Ok(#fn_expr(#(#fn_invoke_args_code),*))
                    #maybe_coercion
                    #maybe_map_result
            }
        }
    }
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

                #[lua_impl]
                impl HalfEdgeMesh {
                    #[lua(map = "x.to_vec()")]
                    fn set_channel(&mut self, lua: &mlua::Lua) -> Vec<Potato>;
                }
            }
        };
        let module = syn::parse2(input).unwrap();
        write_and_fmt("/tmp/test.rs", blackjack_lua_module2(module).unwrap()).unwrap();
    }
}
