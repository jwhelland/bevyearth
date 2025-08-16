//! User interface module
//!
//! This module handles UI state management, panels, and systems for the
//! egui-based user interface.

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub mod groups;
pub mod panels;
pub mod skybox;
pub mod state;
pub mod systems;

pub use skybox::SkyboxPlugin;
pub use state::{RightPanelUI, UIState};
pub use systems::ui_system;

/// Plugin for user interface management
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UIState>()
            .init_resource::<RightPanelUI>()
            .add_systems(EguiPrimaryContextPass, ui_system);
    }
}
