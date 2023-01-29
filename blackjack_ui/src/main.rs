// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Some useful re-exports.
pub mod prelude;

/// Extension methods for egui types
pub mod egui_ext;

/// The application window. This controls the lifecycle of the application:
/// Initialization and main loop.
pub mod app_window;

pub mod application;

/// The rendering context. Provides a layer of abstraction over rend3.
pub mod render_context;

/// A customized rend3 rendergraph for viewport display.
pub mod rendergraph;

/// The graph editor and compiler
pub mod graph;

/// Conversion from hexadecimal string to egui colors and vice-versa.
pub mod color_hex_utils;

/// Custom egui widgets.
pub mod custom_widgets;

/// Command line argument parsing.
pub mod cli_args;

fn main() {
    #[cfg(feature = "tracy")]
    let _client = profiling::tracy_client::Client::start();

    // Various setup calls
    env_logger::init();

    // Handle luadoc flag
    if let Some(ldoc_path) = &cli_args::CLI_ARGS.generate_ldoc {
        use blackjack_engine::lua_engine::lua_stdlib::lua_documentation;
        lua_documentation::generate_lua_documentation(ldoc_path).unwrap();
        println!("Wrote ldoc sources to {ldoc_path}");
        return; // Do nothing else when generating luadoc
    }

    let (app_window, event_loop) = app_window::AppWindow::new();
    app_window.run_app(event_loop);
}
