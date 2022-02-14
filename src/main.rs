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

async fn async_main() {
    // Setup logging
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Debug).unwrap();

    let (app_window, event_loop) = app_window::AppWindow::new().await;
    app_window.run_app(event_loop);
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async_main());
    }


    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(async_main());
    }
}
