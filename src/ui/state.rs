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
    pub crop_3d_viewport_to_ui: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            show_axes: false,
            show_left_panel: false,
            show_right_panel: false,
            show_top_panel: true,
            show_bottom_panel: true,
            // Default off until we're confident viewport cropping math is stable across DPI / UI scale.
            crop_3d_viewport_to_ui: false,
        }
    }
}

/// Layout state for resizable UI panels
#[derive(Resource)]
pub struct UiLayoutState {
    pub left_panel_width_px: f32,
    pub left_panel_min_px: f32,
    pub left_panel_max_px: f32,
    pub left_resize_start_width_px: f32,
    pub resizing_left_panel: bool,
    pub right_panel_width_px: f32,
    pub right_panel_min_px: f32,
    pub right_panel_max_px: f32,
    pub resize_start_width_px: f32,
    pub resizing_right_panel: bool,
}

impl Default for UiLayoutState {
    fn default() -> Self {
        Self {
            left_panel_width_px: 280.0,
            left_panel_min_px: 220.0,
            left_panel_max_px: 420.0,
            left_resize_start_width_px: 280.0,
            resizing_left_panel: false,
            right_panel_width_px: 360.0,
            right_panel_min_px: 280.0,
            right_panel_max_px: 520.0,
            resize_start_width_px: 360.0,
            resizing_right_panel: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchLibraryItemKind {
    Launch,
    Event,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchLibrarySelection {
    pub kind: LaunchLibraryItemKind,
    pub index: usize,
}

#[derive(Resource, Default)]
pub struct LaunchLibraryUiState {
    pub selection: Option<LaunchLibrarySelection>,
    pub camera_target: Option<CameraPose>,
}

/// Right panel UI state
#[derive(Resource, Default)]
pub struct RightPanelUI {
    pub input: String,
    pub error: Option<String>,
    pub selected_group: Option<String>,
    pub group_loading: bool,
    pub pending_add: bool,
    /// URL of the group whose color is currently being edited (for color picker UI)
    pub editing_group_color: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct CameraPose {
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraFocusTarget {
    Earth,
    Moon,
}

#[derive(Resource, Debug)]
pub struct CameraFocusState {
    pub target: CameraFocusTarget,
    pub last_earth_pose: Option<CameraPose>,
}

impl Default for CameraFocusState {
    fn default() -> Self {
        Self {
            target: CameraFocusTarget::Earth,
            last_earth_pose: None,
        }
    }
}
