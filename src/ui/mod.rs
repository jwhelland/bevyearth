//! User interface module
//! 
//! This module handles UI state management, panels, and systems for the
//! egui-based user interface.

pub mod panels;
pub mod state;
pub mod systems;

pub use state::{UIState, RightPanelUI};
pub use systems::ui_example_system;