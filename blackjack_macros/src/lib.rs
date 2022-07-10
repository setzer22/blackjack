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
        Err(err) => panic!("Error in Blackjack Lua module definition: {:?}", err),
    }
}
