//! Reusable widgets for node-canvas presentation.
//!
//! These components intentionally keep rendering concerns separate from graph behavior in
//! `DialogueSnarlViewer` (connections, selection, menu actions).

mod body;
mod header;

pub use body::{NodeBodyData, NodeBodyWidget};
pub use header::{NodeHeaderData, NodeHeaderWidget};
