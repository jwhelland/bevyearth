//! UI state management

use bevy::prelude::*;

/// Main UI state resource
#[derive(Resource)]
pub struct UIState {
    pub show_axes: bool,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub show_top_panel: bool,
    pub show_bottom_panel: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            show_axes: false,
            show_left_panel: false,
            show_right_panel: true,
            show_top_panel: true,
            show_bottom_panel: true,
        }
    }
}

/// Right panel UI state
#[derive(Resource, Default)]
pub struct RightPanelUI {
    pub input: String,
    pub error: Option<String>,
    pub selected_group: Option<String>,
    pub group_loading: bool,
}
