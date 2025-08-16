//! Axes visualization systems

use crate::ui::UIState;
use bevy::prelude::*;

/// Component marker for entities that should display axes
#[derive(Component)]
pub struct ShowAxes;

/// System to draw axes for entities with the ShowAxes component
pub fn draw_axes(
    mut gizmos: Gizmos,
    query: Query<&Transform, With<ShowAxes>>,
    state: Res<UIState>,
) {
    if !state.show_axes {
        return;
    }
    for &transform in &query {
        gizmos.axes(transform, 8000.0);
    }
}
