//! UI systems for the egui interface

use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, egui};

use crate::orbital::SimulationTime;
use crate::satellite::{
    OrbitTrailConfig, SatelliteRenderConfig, SatelliteStore, SelectedSatellite,
};
use crate::tle::FetchChannels;
use crate::ui::panels::{
    render_bottom_panel_with_clicked_satellite, render_left_panel, render_right_panel,
    render_top_panel,
};
use crate::ui::state::{RightPanelUI, UIState};
use crate::visualization::ArrowConfig;
use crate::visualization::GroundTrackConfig;
use crate::visualization::GroundTrackGizmoConfig;

/// Configuration bundle to reduce parameter count
#[derive(Resource)]
pub struct UiConfigBundle {
    pub ground_track_cfg: GroundTrackConfig,
    pub gizmo_cfg: GroundTrackGizmoConfig,
    pub trail_cfg: OrbitTrailConfig,
    pub render_cfg: SatelliteRenderConfig,
}

impl Default for UiConfigBundle {
    fn default() -> Self {
        Self {
            ground_track_cfg: GroundTrackConfig::default(),
            gizmo_cfg: GroundTrackGizmoConfig::default(),
            trail_cfg: OrbitTrailConfig::default(),
            render_cfg: SatelliteRenderConfig::default(),
        }
    }
}

/// Main UI system that renders all the egui panels
#[allow(clippy::too_many_arguments)]
pub fn ui_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<bevy_egui::EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut state: ResMut<UIState>,
    mut arrows_cfg: ResMut<ArrowConfig>,
    mut config_bundle: ResMut<UiConfigBundle>,
    mut heatmap_config: ResMut<crate::visualization::HeatmapConfig>,
    mut sim_time: ResMut<SimulationTime>,
    mut store: ResMut<SatelliteStore>,
    mut right_ui: ResMut<RightPanelUI>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut selected_sat: ResMut<SelectedSatellite>,
    fetch_channels: Option<Res<FetchChannels>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Handle keyboard shortcuts for panel toggles
    ctx.input(|i| {
        if i.key_pressed(egui::Key::H) {
            state.show_left_panel = !state.show_left_panel;
        }
        if i.key_pressed(egui::Key::J) {
            state.show_right_panel = !state.show_right_panel;
        }
        if i.key_pressed(egui::Key::K) {
            state.show_top_panel = !state.show_top_panel;
        }
        if i.key_pressed(egui::Key::L) {
            state.show_bottom_panel = !state.show_bottom_panel;
        }
    });
    let mut left = 0.0;
    if state.show_left_panel {
        left = egui::SidePanel::left("left_panel")
            .resizable(true)
            .show(ctx, |ui| {
                render_left_panel(ui, &mut arrows_cfg, &mut sim_time);
            })
            .response
            .rect
            .width();
    }

    let mut right = 0.0;
    if state.show_right_panel {
        right = egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ctx, |ui| {
                render_right_panel(
                    ui,
                    &mut store,
                    &mut right_ui,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut selected_sat,
                    &mut config_bundle,
                    &mut heatmap_config,
                    &fetch_channels,
                );
            })
            .response
            .rect
            .width();
    }

    let mut top = 0.0;
    if state.show_top_panel {
        top = egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ctx, |ui| {
                render_top_panel(ui, &mut state, &sim_time);
            })
            .response
            .rect
            .height();
    }

    let mut bottom = 0.0;
    if state.show_bottom_panel {
        bottom = egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .show(ctx, |ui| {
                render_bottom_panel_with_clicked_satellite(ui, &store, &fetch_channels);
            })
            .response
            .rect
            .height();
    }

    // Scale from logical units to physical units.
    left *= window.scale_factor();
    right *= window.scale_factor();
    top *= window.scale_factor();
    bottom *= window.scale_factor();

    let pos = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height())
        - pos
        - UVec2::new(right as u32, bottom as u32);

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });

    // System completed successfully
}
