/// Some useful re-exports.
pub mod prelude;

/// The application window. This controls the lifecycle of the application:
/// Initialization and main loop.
pub mod app_window;

pub mod application;

/// The rendering context. Provides a layer of abstraction over rend3.
pub mod render_context;

/// A customized rend3 rendergraph for viewport display.
pub mod rendergraph;

/// Conversion from hexadecimal string to egui colors and vice-versa.
pub mod color_hex_utils;

/// The graph editor and compiler
pub mod graph;

/// The halfedge graph data structure and main edit operations
pub mod mesh;

/// Some utility math types and conversions
pub mod math;

/// General utility methods and helper traits
pub mod utils;

/// The blackjack engine core types
pub mod lua_engine;

fn main() {
    #[cfg(feature = "tracy")]
    let _client = tracy_client::Client::start();

    // Setup logging
    env_logger::init();

    let (app_window, event_loop) = app_window::AppWindow::new();
    app_window.run_app(event_loop);
}
