//! User interface module
//!
//! This module handles UI state management, panels, and systems for the
//! Bevy UI-based user interface.

use bevy::prelude::*;

pub mod groups;
pub mod skybox;
pub mod state;
pub mod systems;

pub use skybox::SkyboxPlugin;
pub use state::{CameraFocusState, RightPanelUI, UIState, UiLayoutState};
pub use systems::{MainCamera, UiConfigBundle};

/// Plugin for user interface management
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UIState>()
            .init_resource::<UiLayoutState>()
            .init_resource::<RightPanelUI>()
            .init_resource::<CameraFocusState>()
            .init_resource::<UiConfigBundle>()
            .add_plugins(systems::UiSystemsPlugin);
    }
}
