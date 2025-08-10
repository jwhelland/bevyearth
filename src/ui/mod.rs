//! User interface module
//!
//! This module handles UI state management, panels, and systems for the
//! egui-based user interface.

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub mod panels;
pub mod state;
pub mod systems;

pub use state::{UIState, RightPanelUI};
pub use systems::ui_example_system;

/// Plugin for user interface management
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UIState>()
            .init_resource::<RightPanelUI>()
            .add_systems(EguiPrimaryContextPass, ui_example_system);
    }
}