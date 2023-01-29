// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use syn::{parse_macro_input, ItemMod};

mod blackjack_lua_module;
mod utils;

#[proc_macro_attribute]
pub fn blackjack_lua_module(
    _attr: proc_macro::TokenStream,
    tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let module = parse_macro_input!(tokens as ItemMod);
    match blackjack_lua_module::blackjack_lua_module2(module) {
        Ok(result) => result.into(),
        Err(err) => panic!("Error in Blackjack Lua module definition: {err:?}"),
    }
}
