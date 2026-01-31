//! Axes visualization systems

use crate::core::big_space::{BigSpaceRoot, cell_local_to_render, render_origin_from_grid};
use crate::ui::UIState;
use bevy::prelude::*;
use big_space::prelude::{CellCoord, Grid};

/// Component marker for entities that should display axes
#[derive(Component)]
pub struct ShowAxes;

/// System to draw axes for entities with the ShowAxes component
pub fn draw_axes(
    mut gizmos: Gizmos,
    query: Query<(&CellCoord, &Transform), With<ShowAxes>>,
    state: Res<UIState>,
    big_space_root: Res<BigSpaceRoot>,
    grid_query: Query<&Grid>,
) {
    if !state.show_axes {
        return;
    }
    let Ok(grid) = grid_query.get(big_space_root.0) else {
        return;
    };
    let (origin_cell, origin_local) = render_origin_from_grid(grid);
    for (cell, transform) in query.iter() {
        let render_pos = cell_local_to_render(
            grid,
            *cell,
            transform.translation,
            origin_cell,
            origin_local,
        );
        let render_transform = Transform {
            translation: render_pos,
            rotation: transform.rotation,
            scale: transform.scale,
        };
        gizmos.axes(render_transform, 8000.0);
    }
}
