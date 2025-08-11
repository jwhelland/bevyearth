//! UI state management

use bevy::prelude::*;

/// Main UI state resource
#[derive(Resource, Default)]
pub struct UIState {
    pub show_axes: bool,
}

/// Right panel UI state
#[derive(Resource, Default)]
pub struct RightPanelUI {
    pub input: String,
    pub error: Option<String>,
    pub selected_group: Option<String>,
    pub group_loading: bool,
}