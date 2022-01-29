/// Some useful re-exports.
mod prelude;

/// The application window. This controls the lifecycle of the application:
/// Initialization and main loop.
mod app_window;

mod application;

/// The rendering context. Provides a layer of abstraction over rend3.
mod render_context;

/// A customized rend3 rendergraph for viewport display.
mod rendergraph;

/// Conversion from hexadecimal string to egui colors and vice-versa.
mod color_hex_utils;

/// The graph editor and compiler
mod graph;

/// The halfedge graph data structure and main edit operations
mod mesh;

/// Some utility math types and conversions
mod math;

fn main() {
    // Setup logging
    env_logger::init();

    let (app_window, event_loop) = app_window::AppWindow::new();
    app_window.run_app(event_loop);
}

