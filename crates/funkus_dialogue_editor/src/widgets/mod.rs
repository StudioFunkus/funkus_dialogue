//! Small, self-contained UI widgets for the editor panels.
//!
//! Widgets should keep their public surface minimal: take the data they need,
//! return small output structs, and avoid storing global editor state.

pub mod inspector;
pub mod left_panel;
pub mod node_canvas;
pub mod status_bar;
pub mod toolbar;

pub use inspector::*;
pub use left_panel::*;
pub use node_canvas::*;
pub use status_bar::*;
pub use toolbar::*;
