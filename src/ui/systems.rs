//! UI systems for the Bevy UI interface

use bevy::camera::Viewport;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::ecs::spawn::Spawn;
use bevy::ecs::world::EntityWorldMut;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::{ButtonInput, ButtonState};
use bevy::picking::events::{Click, Drag, DragEnd, DragStart, Pointer};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::text::TextColor;
use bevy::ui::UiSystems;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};
use bevy::ui::{BackgroundGradient, Checked, Gradient, IsDefaultUiCamera, RelativeCursorPosition};
use bevy::window::{PrimaryWindow, SystemCursorIcon};
use bevy_feathers::controls::{
    ButtonProps, ButtonVariant, SliderProps, button, checkbox, radio, slider,
};
use bevy_feathers::cursor::EntityCursor;
use bevy_feathers::font_styles::InheritableFont;
use bevy_feathers::handle_or_path::HandleOrPath;
use bevy_feathers::theme::ThemeFontColor;
use bevy_feathers::theme::ThemedText;
use bevy_feathers::{constants::fonts, tokens};
use bevy_input_focus::{FocusedInput, InputFocus, InputFocusVisible, tab_navigation::TabIndex};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraSystemSet};
use bevy_ui_widgets::{
    Activate, Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, ValueChange,
    checkbox_self_update, slider_self_update,
};

use crate::satellite::{
    NoradId, OrbitTrailConfig, Satellite, SatelliteRenderAssets, SatelliteRenderConfig,
    SatelliteStore, SelectedSatellite,
};
use crate::tle::{FetchChannels, FetchCommand};
use crate::ui::groups::SATELLITE_GROUPS;
use crate::ui::state::{RightPanelUI, UIState, UiLayoutState};
use crate::visualization::{
    ArrowConfig, GroundTrackConfig, GroundTrackGizmoConfig, HeatmapConfig, RangeMode,
};

/// Configuration bundle to reduce parameter count
#[derive(Resource, Default)]
pub struct UiConfigBundle {
    pub ground_track_cfg: GroundTrackConfig,
    pub gizmo_cfg: GroundTrackGizmoConfig,
    pub trail_cfg: OrbitTrailConfig,
    pub render_cfg: SatelliteRenderConfig,
}

#[derive(Resource)]
struct UiEntities {
    left_panel: Entity,
    right_panel: Entity,
    top_panel: Entity,
    bottom_panel: Entity,
    satellite_list: Entity,
}

#[derive(Component)]
struct LeftPanel;

#[derive(Component)]
struct RightPanel;

#[derive(Component)]
struct TopPanel;

#[derive(Component)]
struct BottomPanel;

#[derive(Component)]
struct SatelliteList;

#[derive(Component)]
struct GroupList;

#[derive(Component)]
struct RightPanelResizeHandle;

#[derive(Component)]
struct RightPanelScroll;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
struct TimeText;

fn queue_set_checked(commands: &mut Commands, entity: Entity, checked: bool) {
    commands
        .entity(entity)
        .queue_silenced(move |mut e: EntityWorldMut| {
            if checked {
                e.insert(Checked);
            } else {
                e.remove::<Checked>();
            }
        });
}

#[derive(Component)]
struct TimeScaleText;

#[derive(Component)]
struct SatelliteCountText;

#[derive(Component)]
struct FetchStatusText;

#[derive(Component)]
struct SelectedSatelliteText;

#[derive(Component)]
struct ErrorText;

#[derive(Component)]
struct GroupLoadingText;

#[derive(Component)]
struct TrackingStatusText;

#[derive(Component)]
struct TextInputField;

#[derive(Component)]
struct TextInputValueText;

#[derive(Component)]
struct TextInputPlaceholderText;

#[derive(Component, Clone, Copy)]
enum CheckboxBinding {
    ShowAxes,
    ShowArrows,
    ArrowGradient,
    ArrowGradientLog,
    GroundTracksEnabled,
    GizmoEnabled,
    GizmoShowCenterDot,
    TrailsAll,
    TracksAll,
    HeatmapEnabled,
    ShowLeftPanel,
    ShowRightPanel,
    ShowTopPanel,
    ShowBottomPanel,
}

#[derive(Component, Clone, Copy)]
enum SliderBinding {
    GradientNear,
    GradientFar,
    GroundTrackRadius,
    GizmoSegments,
    GizmoCenterDotSize,
    TrailMaxPoints,
    TrailUpdateInterval,
    HeatmapUpdatePeriod,
    HeatmapOpacity,
    HeatmapFixedMax,
    HeatmapChunkSize,
    HeatmapChunksPerFrame,
    SatelliteSphereRadius,
    SatelliteEmissiveIntensity,
    TrackingDistance,
    TrackingSmoothness,
    TimeScale,
}

#[derive(Component, Clone, Copy)]
enum RangeModeBinding {
    Auto,
    Fixed,
}

#[derive(Component, Clone, Copy)]
enum ButtonAction {
    LoadGroup,
    ClearAll,
    AddSatellite,
    StopTracking,
    TimeScale1x,
    TimeNow,
}

#[derive(Component, Clone, Copy)]
enum SatelliteAction {
    Track,
    Remove,
}

#[derive(Component, Clone, Copy)]
enum SatelliteToggleKind {
    GroundTrack,
    Trail,
}

#[derive(Component, Clone, Copy)]
struct SatelliteActionButton {
    norad: u32,
    action: SatelliteAction,
}

#[derive(Component, Clone, Copy)]
struct SatelliteToggle {
    norad: u32,
    kind: SatelliteToggleKind,
}

#[derive(Component)]
struct GroupChoice(&'static str);

/// Plugin that registers UI systems and observers
pub struct UiSystemsPlugin;

impl Plugin for UiSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_ui_camera, setup_ui))
            .add_systems(
                Update,
                (
                    toggle_panels_keyboard,
                    apply_panel_visibility,
                    apply_panel_layout,
                    scroll_right_panel_on_wheel,
                    update_time_display,
                    update_status_texts,
                    update_text_input_display,
                ),
            )
            .add_systems(
                Update,
                (
                    process_pending_add,
                    sync_widget_states,
                    sync_slider_visuals,
                    handle_group_loading_text,
                    navigate_focus_with_arrows,
                    // IMPORTANT: this despawns/spawns UI entities; it must run after any system
                    // that might queue commands targeting the current list rows.
                    update_satellite_list,
                )
                    .chain(),
            )
            // Camera + viewport must be finalized after UI layout so sizes are up-to-date,
            // and late enough that no later system overwrites our settings before extraction.
            .add_systems(
                PostUpdate,
                (enforce_ui_camera_settings, update_camera_viewport_from_ui)
                    .chain()
                    .after(UiSystems::Layout),
            )
            .add_systems(
                PostUpdate,
                update_camera_input_from_ui_hover.before(PanOrbitCameraSystemSet),
            )
            .add_observer(checkbox_self_update)
            .add_observer(slider_self_update)
            .add_observer(handle_button_activate)
            .add_observer(handle_checkbox_change)
            .add_observer(handle_slider_change)
            .add_observer(handle_range_mode_change)
            .add_observer(text_input_on_click)
            .add_observer(text_input_on_key_input)
            .add_observer(handle_group_choice)
            .add_observer(handle_right_panel_resize_start)
            .add_observer(handle_right_panel_resize_drag)
            .add_observer(handle_right_panel_resize_end);
    }
}

fn enforce_ui_camera_settings(
    mut cameras: Query<(
        &mut Camera,
        Option<&Camera2d>,
        Option<&Camera3d>,
        Option<&MainCamera>,
    )>,
) {
    // Ensure UI cameras never clear after 3D, and prevent non-main cameras from wiping the frame.
    for (mut camera, is_2d, is_3d, is_main) in cameras.iter_mut() {
        if is_main.is_some() {
            camera.order = 0;
            camera.is_active = true;
            continue;
        }

        if is_2d.is_some() {
            camera.order = 10;
            camera.clear_color = ClearColorConfig::None;
            continue;
        }

        if is_3d.is_some() {
            camera.clear_color = ClearColorConfig::None;
        }
    }
}

fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
    ));
}

fn setup_ui(
    mut commands: Commands,
    layout: Res<UiLayoutState>,
    arrows: Res<ArrowConfig>,
    config_bundle: Res<UiConfigBundle>,
    heatmap_cfg: Res<HeatmapConfig>,
    selected: Res<SelectedSatellite>,
    sim_time: Res<crate::orbital::SimulationTime>,
) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
            // Provide a default font + text color for all `ThemedText` descendants.
            InheritableFont {
                font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
                font_size: 12.0,
            },
            ThemeFontColor(tokens::TEXT_MAIN),
            // Feathers propagation with `With<ThemedText>` requires all nodes in the ancestry chain
            // to opt-in, not just the Text entity itself.
            ThemedText,
        ))
        .id();

    let left_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(280.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.1, 0.92)),
            InheritableFont {
                font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
                font_size: 12.0,
            },
            ThemeFontColor(tokens::TEXT_MAIN),
            ThemedText,
            RelativeCursorPosition::default(),
            Pickable::IGNORE,
            LeftPanel,
        ))
        .id();

    let right_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(layout.right_panel_width_px),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.1, 0.92)),
            InheritableFont {
                font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
                font_size: 12.0,
            },
            ThemeFontColor(tokens::TEXT_MAIN),
            ThemedText,
            RelativeCursorPosition::default(),
            Pickable::IGNORE,
            RightPanel,
        ))
        .id();

    let top_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                height: Val::Px(52.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.95)),
            InheritableFont {
                font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
                font_size: 12.0,
            },
            ThemeFontColor(tokens::TEXT_MAIN),
            ThemedText,
            RelativeCursorPosition::default(),
            Pickable::IGNORE,
            TopPanel,
        ))
        .id();

    let bottom_panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(32.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.95)),
            InheritableFont {
                font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
                font_size: 11.0,
            },
            ThemeFontColor(tokens::TEXT_DIM),
            ThemedText,
            RelativeCursorPosition::default(),
            Pickable::IGNORE,
            BottomPanel,
        ))
        .id();

    commands.entity(root).add_child(left_panel);
    commands.entity(root).add_child(right_panel);
    commands.entity(root).add_child(top_panel);
    commands.entity(root).add_child(bottom_panel);

    // Left panel contents
    commands.entity(left_panel).with_children(|parent| {
        parent.spawn((bevy::ui::widget::Text::new("City → Sat Vis"), ThemedText));

        parent.spawn((checkbox(
            (
                CheckboxBinding::ShowArrows,
                AutoDirectionalNavigation::default(),
            ),
            Spawn((bevy::ui::widget::Text::new("Show arrows"), ThemedText)),
        ),));

        parent.spawn((checkbox(
            (
                CheckboxBinding::ArrowGradient,
                AutoDirectionalNavigation::default(),
            ),
            Spawn((
                bevy::ui::widget::Text::new("Distance color gradient"),
                ThemedText,
            )),
        ),));

        parent.spawn((checkbox(
            (
                CheckboxBinding::ArrowGradientLog,
                AutoDirectionalNavigation::default(),
            ),
            Spawn((bevy::ui::widget::Text::new("Log scale"), ThemedText)),
        ),));

        parent.spawn((
            bevy::ui::widget::Text::new("Gradient range (km)"),
            ThemedText,
        ));
        spawn_labeled_slider(
            parent,
            "Near",
            SliderBinding::GradientNear,
            10.0,
            200000.0,
            arrows.gradient_near_km,
            1000.0,
        );
        spawn_labeled_slider(
            parent,
            "Far",
            SliderBinding::GradientFar,
            10.0,
            200000.0,
            arrows.gradient_far_km,
            1000.0,
        );

        parent.spawn((checkbox(
            (
                CheckboxBinding::ShowAxes,
                AutoDirectionalNavigation::default(),
            ),
            Spawn((bevy::ui::widget::Text::new("Show axes"), ThemedText)),
        ),));
    });

    // Right panel contents
    let mut satellite_list = Entity::PLACEHOLDER;

    commands.entity(right_panel).with_children(|parent| {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-4.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.14, 0.9)),
            EntityCursor::System(SystemCursorIcon::EwResize),
            RelativeCursorPosition::default(),
            RightPanelResizeHandle,
        ));
        parent
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    min_height: Val::Px(0.0),
                    ..default()
                },
                Pickable::IGNORE,
                ThemedText,
            ))
            .with_children(|row| {
                let scroll_entity = row
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            width: Val::Auto,
                            height: Val::Percent(100.0),
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            min_height: Val::Px(0.0),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        ScrollPosition::default(),
                        RelativeCursorPosition::default(),
                        Pickable::IGNORE,
                        ThemedText,
                        RightPanelScroll,
                    ))
                    .with_children(|parent| {
                        parent.spawn((bevy::ui::widget::Text::new("Satellites"), ThemedText));

                        parent.spawn((bevy::ui::widget::Text::new("Satellite Groups"), ThemedText));

                        parent
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                Pickable::IGNORE,
                                ThemedText,
                            ))
                            .with_children(|container| {
                                let group_list_entity = container
                                    .spawn((
                                        Node {
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(4.0),
                                            width: Val::Px(320.0),
                                            height: Val::Px(180.0),
                                            overflow: Overflow::scroll_y(),
                                            padding: UiRect::all(Val::Px(4.0)),
                                            ..default()
                                        },
                                        ScrollPosition::default(),
                                        ThemedText,
                                        GroupList,
                                    ))
                                    .with_children(|groups| {
                                        for (group_key, group_name) in SATELLITE_GROUPS {
                                            groups.spawn((radio(
                                                (
                                                    GroupChoice(*group_key),
                                                    AutoDirectionalNavigation::default(),
                                                ),
                                                Spawn((
                                                    bevy::ui::widget::Text::new(*group_name),
                                                    ThemedText,
                                                )),
                                            ),));
                                        }
                                    })
                                    .id();

                                spawn_scrollbar(container, group_list_entity, 180.0);
                            });

                        parent.spawn((button(
                            ButtonProps {
                                variant: ButtonVariant::Primary,
                                ..default()
                            },
                            (
                                ButtonAction::LoadGroup,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("Load Group"), ThemedText)),
                        ),));

                        parent.spawn((
                            GroupLoadingText,
                            bevy::ui::widget::Text::new(""),
                            ThemedText,
                        ));

                        parent.spawn((bevy::ui::widget::Text::new("Add Satellite"), ThemedText));

                        parent
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                Pickable::IGNORE,
                                ThemedText,
                            ))
                            .with_children(|row| {
                                row.spawn((
                                    Node {
                                        width: Val::Px(180.0),
                                        height: Val::Px(28.0),
                                        padding: UiRect::horizontal(Val::Px(6.0)),
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.08, 0.1, 0.14, 1.0)),
                                    ThemedText,
                                    AutoDirectionalNavigation::default(),
                                    TabIndex(0),
                                    TextInputField,
                                ))
                                .with_children(|field| {
                                    field.spawn((
                                        TextInputValueText,
                                        bevy::ui::widget::Text::new(""),
                                        ThemedText,
                                    ));
                                    field.spawn((
                                        TextInputPlaceholderText,
                                        bevy::ui::widget::Text::new("NORAD ID"),
                                        ThemedText,
                                    ));
                                });

                                row.spawn((button(
                                    ButtonProps::default(),
                                    (
                                        ButtonAction::AddSatellite,
                                        AutoDirectionalNavigation::default(),
                                    ),
                                    Spawn((bevy::ui::widget::Text::new("Add"), ThemedText)),
                                ),));
                            });

                        parent.spawn((
                            ErrorText,
                            bevy::ui::widget::Text::new(""),
                            ThemedText,
                            TextColor(Color::srgb(1.0, 0.2, 0.2)),
                        ));

                        parent.spawn((button(
                            ButtonProps::default(),
                            (ButtonAction::ClearAll, AutoDirectionalNavigation::default()),
                            Spawn((
                                bevy::ui::widget::Text::new("Clear All Satellites"),
                                ThemedText,
                            )),
                        ),));

                        parent.spawn((
                            bevy::ui::widget::Text::new("Ground Track Settings"),
                            ThemedText,
                        ));

                        parent.spawn((checkbox(
                            (
                                CheckboxBinding::TracksAll,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("All Tracks"), ThemedText)),
                        ),));
                        parent.spawn((checkbox(
                            (
                                CheckboxBinding::GroundTracksEnabled,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((
                                bevy::ui::widget::Text::new("Show ground tracks"),
                                ThemedText,
                            )),
                        ),));
                spawn_labeled_slider(
                    parent,
                    "Track radius (km)",
                    SliderBinding::GroundTrackRadius,
                    10.0,
                    500.0,
                    config_bundle.ground_track_cfg.radius_km,
                    5.0,
                );

                        parent.spawn((bevy::ui::widget::Text::new("Gizmo Settings"), ThemedText));
                        parent.spawn((checkbox(
                            (
                                CheckboxBinding::GizmoEnabled,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("Use gizmo circles"), ThemedText)),
                        ),));
                spawn_labeled_slider(
                    parent,
                    "Circle segments",
                    SliderBinding::GizmoSegments,
                    16.0,
                    128.0,
                    config_bundle.gizmo_cfg.circle_segments as f32,
                    1.0,
                );
                        parent.spawn((checkbox(
                            (
                                CheckboxBinding::GizmoShowCenterDot,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("Show center dot"), ThemedText)),
                        ),));
                spawn_labeled_slider(
                    parent,
                    "Center dot size (km)",
                    SliderBinding::GizmoCenterDotSize,
                    50.0,
                    500.0,
                    config_bundle.gizmo_cfg.center_dot_size,
                    10.0,
                );

                        parent.spawn((
                            bevy::ui::widget::Text::new("Orbit Trail Settings"),
                            ThemedText,
                        ));
                        parent.spawn((checkbox(
                            (
                                CheckboxBinding::TrailsAll,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("All Trails"), ThemedText)),
                        ),));
                spawn_labeled_slider(
                    parent,
                    "Max history points",
                    SliderBinding::TrailMaxPoints,
                    100.0,
                    10000.0,
                    config_bundle.trail_cfg.max_points as f32,
                    50.0,
                );
                spawn_labeled_slider(
                    parent,
                    "Update interval (s)",
                    SliderBinding::TrailUpdateInterval,
                    0.5,
                    10.0,
                    config_bundle.trail_cfg.update_interval_seconds,
                    0.1,
                );

                        parent.spawn((bevy::ui::widget::Text::new("Heatmap Settings"), ThemedText));
                        parent.spawn((checkbox(
                            (
                                CheckboxBinding::HeatmapEnabled,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("Enable heatmap"), ThemedText)),
                        ),));
                        spawn_labeled_slider(
                            parent,
                            "Update period (s)",
                            SliderBinding::HeatmapUpdatePeriod,
                            0.1,
                            2.0,
                            heatmap_cfg.update_period_s,
                            0.1,
                        );
                        spawn_labeled_slider(
                            parent,
                            "Opacity",
                            SliderBinding::HeatmapOpacity,
                            0.0,
                            1.0,
                            heatmap_cfg.color_alpha,
                            0.05,
                        );

                        parent.spawn((bevy::ui::widget::Text::new("Range mode"), ThemedText));
                        parent.spawn((radio(
                            (RangeModeBinding::Auto, AutoDirectionalNavigation::default()),
                            Spawn((bevy::ui::widget::Text::new("Auto"), ThemedText)),
                        ),));
                        parent.spawn((radio(
                            (
                                RangeModeBinding::Fixed,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("Fixed"), ThemedText)),
                        ),));

                        spawn_labeled_slider(
                            parent,
                            "Fixed max",
                            SliderBinding::HeatmapFixedMax,
                            1.0,
                            100.0,
                            heatmap_cfg.fixed_max.unwrap_or(20) as f32,
                            1.0,
                        );

                        spawn_labeled_slider(
                            parent,
                            "Chunk size",
                            SliderBinding::HeatmapChunkSize,
                            500.0,
                            5000.0,
                            heatmap_cfg.chunk_size as f32,
                            100.0,
                        );
                        spawn_labeled_slider(
                            parent,
                            "Chunks/frame",
                            SliderBinding::HeatmapChunksPerFrame,
                            1.0,
                            5.0,
                            heatmap_cfg.chunks_per_frame as f32,
                            1.0,
                        );

                        parent.spawn((
                            bevy::ui::widget::Text::new("Satellite Rendering"),
                            ThemedText,
                        ));
                        spawn_labeled_slider(
                            parent,
                            "Sphere size (km)",
                            SliderBinding::SatelliteSphereRadius,
                            1.0,
                            200.0,
                            config_bundle.render_cfg.sphere_radius,
                            1.0,
                        );
                        spawn_labeled_slider(
                            parent,
                            "Emissive intensity",
                            SliderBinding::SatelliteEmissiveIntensity,
                            10.0,
                            500.0,
                            config_bundle.render_cfg.emissive_intensity,
                            5.0,
                        );

                // Atmosphere controls removed for now (feature disabled).

                        parent.spawn((bevy::ui::widget::Text::new("Camera Tracking"), ThemedText));
                        parent.spawn((
                            TrackingStatusText,
                            bevy::ui::widget::Text::new("Tracking: None"),
                            ThemedText,
                        ));
                        parent.spawn((button(
                            ButtonProps::default(),
                            (
                                ButtonAction::StopTracking,
                                AutoDirectionalNavigation::default(),
                            ),
                            Spawn((bevy::ui::widget::Text::new("Stop Tracking"), ThemedText)),
                        ),));
                        spawn_labeled_slider(
                            parent,
                            "Tracking distance (km)",
                            SliderBinding::TrackingDistance,
                            1000.0,
                            20000.0,
                            selected.tracking_offset,
                            100.0,
                        );
                        spawn_labeled_slider(
                            parent,
                            "Tracking smoothness",
                            SliderBinding::TrackingSmoothness,
                            0.01,
                            1.0,
                            selected.smooth_factor,
                            0.01,
                        );

                        parent.spawn((bevy::ui::widget::Text::new("Satellites List"), ThemedText));

                        // Header Row
                        parent
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(6.0),
                                    padding: UiRect::horizontal(Val::Px(4.0)),
                                    width: Val::Px(380.0),
                                    ..default()
                                },
                                ThemedText,
                            ))
                            .with_children(|header| {
                                header.spawn((
                                    bevy::ui::widget::Text::new("NORAD"),
                                    ThemedText,
                                    Node {
                                        width: Val::Px(90.0),
                                        ..default()
                                    },
                                    TextFont {
                                        font_size: 11.0,
                                        ..default()
                                    },
                                ));
                                header.spawn((
                                    bevy::ui::widget::Text::new("Name"),
                                    ThemedText,
                                    Node {
                                        flex_grow: 1.0,
                                        min_width: Val::Px(0.0),
                                        ..default()
                                    },
                                    TextFont {
                                        font_size: 11.0,
                                        ..default()
                                    },
                                ));
                                header.spawn((
                                    bevy::ui::widget::Text::new("Status"),
                                    ThemedText,
                                    Node {
                                        width: Val::Px(60.0),
                                        ..default()
                                    },
                                    TextFont {
                                        font_size: 11.0,
                                        ..default()
                                    },
                                ));
                                header.spawn((
                                    bevy::ui::widget::Text::new("G.T."),
                                    ThemedText,
                                    Node {
                                        width: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    TextFont {
                                        font_size: 11.0,
                                        ..default()
                                    },
                                ));
                                header.spawn((
                                    bevy::ui::widget::Text::new("Trail"),
                                    ThemedText,
                                    Node {
                                        width: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    TextFont {
                                        font_size: 11.0,
                                        ..default()
                                    },
                                ));
                                // Spacer for X button
                                header.spawn(Node {
                                    width: Val::Px(28.0),
                                    ..default()
                                });
                            });

                        parent
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(6.0),
                                    height: Val::Px(240.0),
                                    ..default()
                                },
                                ThemedText,
                            ))
                            .with_children(|container| {
                                let list_entity = container
                                    .spawn((
                                        Node {
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(2.0),
                                            width: Val::Px(380.0),
                                            height: Val::Px(240.0),
                                            overflow: Overflow::scroll_y(),
                                            padding: UiRect::all(Val::Px(4.0)),
                                            ..default()
                                        },
                                        ScrollPosition::default(),
                                        ThemedText,
                                        SatelliteList,
                                    ))
                                    .id();

                                spawn_scrollbar(container, list_entity, 240.0);
                                satellite_list = list_entity;
                            });
                    })
                    .id();

                spawn_scrollbar_fill(row, scroll_entity);
            });
    });

    // Top panel contents
    commands.entity(top_panel).with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            ThemedText,
                        ))
            .with_children(|left| {
                left.spawn((
                    TimeText,
                    bevy::ui::widget::Text::new("UTC: --"),
                    ThemedText,
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                ));
                left.spawn((
                    TimeScaleText,
                    bevy::ui::widget::Text::new("1.0x"),
                    ThemedText,
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                ));
            });

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                flex_grow: 1.0,
                                min_width: Val::Px(220.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            ThemedText,
                        ))
            .with_children(|middle| {
                spawn_labeled_slider(
                    middle,
                    "Speed",
                    SliderBinding::TimeScale,
                    1.0,
                    1000.0,
                    sim_time.time_scale,
                    1.0,
                );
            });

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            ThemedText,
                        ))
            .with_children(|right| {
                right
                    .spawn(button(
                        ButtonProps::default(),
                        (
                            ButtonAction::TimeScale1x,
                            AutoDirectionalNavigation::default(),
                        ),
                        Spawn((
                            bevy::ui::widget::Text::new("1x"),
                            ThemedText,
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                        )),
                    ))
                    .insert(Node {
                        width: Val::Px(72.0),
                        flex_grow: 0.0,
                        ..default()
                    });

                right
                    .spawn(button(
                        ButtonProps::default(),
                        (ButtonAction::TimeNow, AutoDirectionalNavigation::default()),
                        Spawn((
                            bevy::ui::widget::Text::new("Now"),
                            ThemedText,
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                        )),
                    ))
                    .insert(Node {
                        width: Val::Px(84.0),
                        flex_grow: 0.0,
                        ..default()
                    });

                right.spawn((
                    bevy::ui::widget::Text::new("Panels:"),
                    ThemedText,
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                ));

                right.spawn((checkbox(
                    (
                        CheckboxBinding::ShowLeftPanel,
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((
                        bevy::ui::widget::Text::new("Left"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    )),
                ),));

                right.spawn((checkbox(
                    (
                        CheckboxBinding::ShowRightPanel,
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((
                        bevy::ui::widget::Text::new("Right"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    )),
                ),));

                right.spawn((checkbox(
                    (
                        CheckboxBinding::ShowTopPanel,
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((
                        bevy::ui::widget::Text::new("Top"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    )),
                ),));

                right.spawn((checkbox(
                    (
                        CheckboxBinding::ShowBottomPanel,
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((
                        bevy::ui::widget::Text::new("Bottom"),
                        ThemedText,
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    )),
                ),));
            });
    });

    // Bottom panel contents
    commands.entity(bottom_panel).with_children(|parent| {
        parent.spawn((
            SatelliteCountText,
            bevy::ui::widget::Text::new("Satellites: 0"),
            ThemedText,
        ));
        parent.spawn((
            FetchStatusText,
            bevy::ui::widget::Text::new("TLE Fetcher: --"),
            ThemedText,
        ));
        parent.spawn((
            SelectedSatelliteText,
            bevy::ui::widget::Text::new("Selected: None"),
            ThemedText,
        ));
    });

    commands.insert_resource(UiEntities {
        left_panel,
        right_panel,
        top_panel,
        bottom_panel,
        satellite_list,
    });
}

fn spawn_labeled_slider(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    binding: SliderBinding,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
) {
    let precision = slider_precision_from_step(step);
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
            ThemedText,
        ))
        .with_children(|row| {
            row.spawn((
                bevy::ui::widget::Text::new(label),
                ThemedText,
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));
            row.spawn((slider(
                SliderProps { value, min, max },
                (
                    binding,
                    SliderStep(step),
                    SliderPrecision(precision),
                    AutoDirectionalNavigation::default(),
                ),
            ),));
        });
}

fn slider_precision_from_step(step: f32) -> i32 {
    let mut precision = 0;
    let mut value = step.abs();
    while precision < 4 && (value - value.round()).abs() > 1e-6 {
        value *= 10.0;
        precision += 1;
    }
    precision
}

fn spawn_scrollbar(parent: &mut ChildSpawnerCommands, target: Entity, height: f32) -> Entity {
    use bevy_ui_widgets::{ControlOrientation, CoreScrollbarThumb, Scrollbar};

    parent
        .spawn((
            Node {
                width: Val::Px(12.0),
                height: Val::Px(height),
                ..default()
            },
            ThemedText,
            BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 1.0)),
            Scrollbar::new(target, ControlOrientation::Vertical, 24.0),
        ))
        .with_children(|thumb| {
            thumb.spawn((
                Node {
                    width: Val::Px(8.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                ThemedText,
                BackgroundColor(Color::srgba(0.2, 0.3, 0.4, 1.0)),
                CoreScrollbarThumb,
            ));
        })
        .id()
}

fn spawn_scrollbar_fill(parent: &mut ChildSpawnerCommands, target: Entity) -> Entity {
    use bevy_ui_widgets::{ControlOrientation, CoreScrollbarThumb, Scrollbar};

    parent
        .spawn((
            Node {
                width: Val::Px(12.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ThemedText,
            BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 1.0)),
            Scrollbar::new(target, ControlOrientation::Vertical, 24.0),
        ))
        .with_children(|thumb| {
            thumb.spawn((
                Node {
                    width: Val::Px(8.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                ThemedText,
                BackgroundColor(Color::srgba(0.2, 0.3, 0.4, 1.0)),
                CoreScrollbarThumb,
            ));
        })
        .id()
}

fn toggle_panels_keyboard(input: Res<ButtonInput<KeyCode>>, mut state: ResMut<UIState>) {
    if input.just_pressed(KeyCode::KeyH) {
        state.show_left_panel = !state.show_left_panel;
    }
    if input.just_pressed(KeyCode::KeyJ) {
        state.show_right_panel = !state.show_right_panel;
    }
    if input.just_pressed(KeyCode::KeyK) {
        state.show_top_panel = !state.show_top_panel;
    }
    if input.just_pressed(KeyCode::KeyL) {
        state.show_bottom_panel = !state.show_bottom_panel;
    }
    if input.just_pressed(KeyCode::KeyV) {
        state.crop_3d_viewport_to_ui = !state.crop_3d_viewport_to_ui;
        info!(
            "3D viewport cropping: {} (press V to toggle)",
            if state.crop_3d_viewport_to_ui {
                "ON"
            } else {
                "OFF"
            }
        );
    }
}

fn apply_panel_visibility(
    state: Res<UIState>,
    ui_entities: Res<UiEntities>,
    mut panels: Query<&mut Node>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut node) = panels.get_mut(ui_entities.left_panel) {
        node.display = if state.show_left_panel {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut node) = panels.get_mut(ui_entities.right_panel) {
        node.display = if state.show_right_panel {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut node) = panels.get_mut(ui_entities.top_panel) {
        node.display = if state.show_top_panel {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut node) = panels.get_mut(ui_entities.bottom_panel) {
        node.display = if state.show_bottom_panel {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn apply_panel_layout(
    layout: Res<UiLayoutState>,
    ui_entities: Res<UiEntities>,
    mut panels: Query<&mut Node>,
) {
    if !layout.is_changed() {
        return;
    }

    if let Ok(mut node) = panels.get_mut(ui_entities.right_panel) {
        node.width = Val::Px(layout.right_panel_width_px);
    }
}

fn scroll_right_panel_on_wheel(
    mut wheel_events: MessageReader<MouseWheel>,
    q_scroll: Query<&RelativeCursorPosition, With<RightPanelScroll>>,
    mut scroll_positions: Query<&mut ScrollPosition, With<RightPanelScroll>>,
) {
    let mut delta: f32 = 0.0;
    for ev in wheel_events.read() {
        let step = match ev.unit {
            MouseScrollUnit::Line => ev.y * 20.0,
            MouseScrollUnit::Pixel => ev.y,
        };
        delta += step;
    }

    if delta.abs() < f32::EPSILON {
        return;
    }

    let Ok(cursor) = q_scroll.single() else {
        return;
    };

    if !cursor.cursor_over {
        return;
    }

    if let Ok(mut scroll) = scroll_positions.single_mut() {
        scroll.0.y = (scroll.0.y - delta).max(0.0);
    }
}

fn update_camera_viewport_from_ui(
    ui_entities: Res<UiEntities>,
    panels: Query<&ComputedNode>,
    state: Res<UIState>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };

    if !state.crop_3d_viewport_to_ui {
        camera.viewport = None;
        return;
    }

    let left = if state.show_left_panel {
        panels
            .get(ui_entities.left_panel)
            .map(|n| n.size.x)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let right = if state.show_right_panel {
        panels
            .get(ui_entities.right_panel)
            .map(|n| n.size.x)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let top = if state.show_top_panel {
        panels
            .get(ui_entities.top_panel)
            .map(|n| n.size.y)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let bottom = if state.show_bottom_panel {
        panels
            .get(ui_entities.bottom_panel)
            .map(|n| n.size.y)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let width = window.physical_width() as f32;
    let height = window.physical_height() as f32;

    // `ComputedNode` sizes are already in physical pixels.
    let left_px = left.round();
    let right_px = right.round();
    let top_px = top.round();
    let bottom_px = bottom.round();

    if !left_px.is_finite()
        || !right_px.is_finite()
        || !top_px.is_finite()
        || !bottom_px.is_finite()
        || !width.is_finite()
        || !height.is_finite()
    {
        camera.viewport = None;
        return;
    }

    // If layout hasn't settled yet or calculations are invalid, render full-screen.
    if left_px + right_px >= width - 1.0 || top_px + bottom_px >= height - 1.0 {
        camera.viewport = None;
        return;
    }

    let avail_w = (width - left_px - right_px).max(1.0);
    let avail_h = (height - top_px - bottom_px).max(1.0);

    // Avoid accidentally rendering to a 0-1px viewport (which looks like "no 3D").
    if avail_w < 32.0 || avail_h < 32.0 {
        camera.viewport = None;
        return;
    }

    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(left_px as u32, top_px as u32),
        physical_size: UVec2::new(avail_w as u32, avail_h as u32),
        ..default()
    });
}

fn update_camera_input_from_ui_hover(
    ui_entities: Res<UiEntities>,
    state: Res<UIState>,
    panels: Query<&RelativeCursorPosition>,
    mut cameras: Query<&mut PanOrbitCamera, With<MainCamera>>,
) {
    let mut hovered = false;

    if state.show_left_panel {
        if let Ok(pos) = panels.get(ui_entities.left_panel) {
            hovered |= pos.cursor_over;
        }
    }
    if state.show_right_panel {
        if let Ok(pos) = panels.get(ui_entities.right_panel) {
            hovered |= pos.cursor_over;
        }
    }
    if state.show_top_panel {
        if let Ok(pos) = panels.get(ui_entities.top_panel) {
            hovered |= pos.cursor_over;
        }
    }
    if state.show_bottom_panel {
        if let Ok(pos) = panels.get(ui_entities.bottom_panel) {
            hovered |= pos.cursor_over;
        }
    }

    for mut pan_orbit in cameras.iter_mut() {
        pan_orbit.enabled = !hovered;
    }
}

fn update_time_display(
    mut texts: ParamSet<(
        Query<&mut bevy::ui::widget::Text, With<TimeText>>,
        Query<&mut bevy::ui::widget::Text, With<TimeScaleText>>,
    )>,
    sim_time: Res<crate::orbital::SimulationTime>,
) {
    for mut text in texts.p0().iter_mut() {
        text.0 = format!(
            "UTC: {}",
            sim_time
                .current_utc
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        );
    }
    for mut text in texts.p1().iter_mut() {
        text.0 = format!("{:.2}x", sim_time.time_scale);
    }
}

fn update_status_texts(
    store: Res<SatelliteStore>,
    mut texts: ParamSet<(
        Query<&mut bevy::ui::widget::Text, With<SatelliteCountText>>,
        Query<&mut bevy::ui::widget::Text, With<FetchStatusText>>,
        Query<&mut bevy::ui::widget::Text, With<SelectedSatelliteText>>,
        Query<&mut bevy::ui::widget::Text, With<TrackingStatusText>>,
    )>,
    selected: Res<SelectedSatellite>,
    fetch: Option<Res<FetchChannels>>,
) {
    for mut text in texts.p0().iter_mut() {
        text.0 = format!("Satellites: {}", store.items.len());
    }
    for mut text in texts.p1().iter_mut() {
        text.0 = if fetch.is_some() {
            "TLE Fetcher: Active".to_string()
        } else {
            "TLE Fetcher: Inactive".to_string()
        };
    }
    for mut text in texts.p2().iter_mut() {
        if let Some((norad, entry)) = store.items.iter().find(|(_, e)| e.is_clicked) {
            let name = entry.name.as_deref().unwrap_or("Unnamed");
            text.0 = format!("Selected: {} ({})", name, norad);
        } else {
            text.0 = "Selected: None".to_string();
        }
    }

    for mut text in texts.p3().iter_mut() {
        if let Some(norad) = selected.tracking {
            if let Some(entry) = store.items.get(&norad) {
                let name = entry.name.as_deref().unwrap_or("Unnamed");
                text.0 = format!("Tracking: {} ({})", name, norad);
            } else {
                text.0 = format!("Tracking: {}", norad);
            }
        } else {
            text.0 = "Tracking: None".to_string();
        }
    }
}

fn handle_group_loading_text(
    right_ui: Res<RightPanelUI>,
    mut group_text: Query<&mut bevy::ui::widget::Text, With<GroupLoadingText>>,
) {
    if !right_ui.is_changed() {
        return;
    }
    if let Ok(mut text) = group_text.single_mut() {
        if right_ui.group_loading {
            text.0 = "Loading group...".to_string();
        } else {
            text.0.clear();
        }
    }
}

fn update_text_input_display(
    right_ui: Res<RightPanelUI>,
    mut texts: ParamSet<(
        Query<&mut bevy::ui::widget::Text, With<TextInputValueText>>,
        Query<&mut bevy::ui::widget::Text, With<TextInputPlaceholderText>>,
        Query<&mut bevy::ui::widget::Text, With<ErrorText>>,
    )>,
) {
    for mut text in texts.p0().iter_mut() {
        text.0 = right_ui.input.clone();
    }
    for mut text in texts.p1().iter_mut() {
        if right_ui.input.is_empty() {
            text.0 = "NORAD ID".to_string();
        } else {
            text.0.clear();
        }
    }
    for mut text in texts.p2().iter_mut() {
        if let Some(err) = &right_ui.error {
            text.0 = err.clone();
        } else {
            text.0.clear();
        }
    }
}

fn process_pending_add(
    mut right_ui: ResMut<RightPanelUI>,
    mut store: ResMut<SatelliteStore>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config_bundle: Res<UiConfigBundle>,
    render_assets: Res<SatelliteRenderAssets>,
    fetch_channels: Option<Res<FetchChannels>>,
) {
    if !right_ui.pending_add {
        return;
    }
    right_ui.pending_add = false;

    let input = right_ui.input.trim();
    let norad = match input.parse::<u32>() {
        Ok(value) => value,
        Err(_) => {
            right_ui.error = Some("Invalid NORAD ID".to_string());
            return;
        }
    };

    if store.items.contains_key(&norad) {
        right_ui.error = Some("Satellite already added".to_string());
        return;
    }

    let seed = norad.wrapping_mul(1664525).wrapping_add(1013904223);
    let hue = (seed as f32 / u32::MAX as f32).fract();
    let sat = (0.65 + ((norad % 7) as f32) * 0.035).clamp(0.6, 0.9);
    let light = (0.55 + ((norad % 11) as f32) * 0.02).clamp(0.5, 0.8);
    let color = Color::hsl(hue, sat, light);

    let entity = commands
        .spawn((
            Mesh3d(render_assets.sphere_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                emissive: color.to_linear() * config_bundle.render_cfg.emissive_intensity,
                ..default()
            })),
            Satellite,
            NoradId(norad),
            crate::satellite::SatelliteColor(color),
            Transform::from_xyz(crate::core::coordinates::EARTH_RADIUS_KM + 5000.0, 0.0, 0.0)
                .with_scale(Vec3::splat(config_bundle.render_cfg.sphere_radius)),
        ))
        .id();

    store.items.insert(
        norad,
        crate::satellite::SatEntry {
            norad,
            name: None,
            color,
            entity: Some(entity),
            tle: None,
            propagator: None,
            error: None,
            show_ground_track: false,
            show_trail: false,
            is_clicked: false,
        },
    );
    store.entity_by_norad.insert(norad, entity);

    right_ui.error = None;
    if let Some(fetch) = fetch_channels {
        if let Err(e) = fetch.cmd_tx.send(FetchCommand::Fetch(norad)) {
            right_ui.error = Some(format!("Failed to fetch NORAD {}: {}", norad, e));
        }
    } else {
        right_ui.error = Some("Fetch service not available".to_string());
    }

    right_ui.input.clear();
}

#[derive(Component)]
struct SatelliteRow {
    norad: u32,
}

#[derive(Component)]
struct SatelliteRowRefs {
    track_btn: Entity,
    name_text: Entity,
    status_text: Entity,
    ground_track_chk: Entity,
    trail_chk: Entity,
}

fn update_satellite_list(
    store: Res<SatelliteStore>,
    selected: Res<SelectedSatellite>,
    ui_entities: Res<UiEntities>,
    row_query: Query<(Entity, &SatelliteRow, &SatelliteRowRefs)>,
    mut texts: Query<(&mut bevy::ui::widget::Text, Option<&mut TextColor>)>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    if !store.is_changed() && !selected.is_changed() {
        return;
    }

    let mut existing_rows: std::collections::HashMap<u32, (Entity, &SatelliteRowRefs)> = row_query
        .iter()
        .map(|(e, r, refs)| (r.norad, (e, refs)))
        .collect();

    let mut keys: Vec<u32> = store.items.keys().copied().collect();
    keys.sort_unstable();

    let parent = ui_entities.satellite_list;

    for norad in keys {
        if let Some(entry) = store.items.get(&norad) {
            let is_tracking = selected.tracking == Some(norad);
            let (status_text, status_color) = if entry.error.is_some() {
                ("Error", Color::srgb(1.0, 0.2, 0.2))
            } else if entry.propagator.is_some() {
                ("Ready", Color::srgb(0.2, 0.9, 0.2))
            } else if entry.tle.is_some() {
                ("TLE", Color::srgb(0.9, 0.9, 0.2))
            } else {
                ("Fetching", Color::srgb(0.7, 0.7, 0.7))
            };

            if let Some((_, refs)) = existing_rows.remove(&norad) {
                // Find and update track button text
                if let Ok(btn_children) = children.get(refs.track_btn) {
                    for child in btn_children.iter() {
                        if let Ok((mut text, _)) = texts.get_mut(child) {
                            let label = if is_tracking {
                                format!("> {norad}")
                            } else {
                                norad.to_string()
                            };
                            if text.0 != label {
                                text.0 = label;
                            }
                        }
                    }
                }

                if let Ok((mut text, _)) = texts.get_mut(refs.name_text) {
                    let name = entry.name.as_deref().unwrap_or("Unnamed");
                    if text.0 != name {
                        text.0 = name.to_string();
                    }
                }

                if let Ok((mut text, mut color_opt)) = texts.get_mut(refs.status_text) {
                    if text.0 != status_text {
                        text.0 = status_text.to_string();
                    }
                    if let Some(ref mut color) = color_opt {
                        color.0 = status_color;
                    }
                }

                if entry.show_ground_track {
                    commands.entity(refs.ground_track_chk).insert(Checked);
                } else {
                    commands.entity(refs.ground_track_chk).remove::<Checked>();
                }

                if entry.show_trail {
                    commands.entity(refs.trail_chk).insert(Checked);
                } else {
                    commands.entity(refs.trail_chk).remove::<Checked>();
                }
            } else {
                commands.entity(parent).with_children(|parent| {
                    spawn_satellite_row(
                        parent,
                        norad,
                        entry,
                        is_tracking,
                        status_text,
                        status_color,
                    );
                });
            }
        }
    }

    // Remove remaining rows
    for (_, (entity, _)) in existing_rows {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }
}

fn spawn_satellite_row(
    parent: &mut ChildSpawnerCommands,
    norad: u32,
    entry: &crate::satellite::SatEntry,
    is_tracking: bool,
    status_text: &str,
    status_color: Color,
) {
    let mut track_btn = Entity::PLACEHOLDER;
    let mut name_text = Entity::PLACEHOLDER;
    let mut status_text_entity = Entity::PLACEHOLDER;
    let mut ground_track_chk = Entity::PLACEHOLDER;
    let mut trail_chk = Entity::PLACEHOLDER;

    parent
        .spawn((
            SatelliteRow { norad },
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
            ThemedText,
        ))
        .with_children(|row| {
            let label = if is_tracking {
                format!("> {norad}")
            } else {
                norad.to_string()
            };

            track_btn = row
                .spawn(button(
                    ButtonProps {
                        variant: if is_tracking {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Normal
                        },
                        ..default()
                    },
                    (
                        SatelliteActionButton {
                            norad,
                            action: SatelliteAction::Track,
                        },
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((
                        bevy::ui::widget::Text::new(label),
                        ThemedText,
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                    )),
                ))
                .insert(Node {
                    width: Val::Px(90.0),
                    flex_grow: 0.0,
                    ..default()
                })
                .id();

            name_text = row
                .spawn((
                    bevy::ui::widget::Text::new(entry.name.as_deref().unwrap_or("Unnamed")),
                    ThemedText,
                    Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        ..default()
                    },
                ))
                .id();

            status_text_entity = row
                .spawn((
                    bevy::ui::widget::Text::new(status_text),
                    ThemedText,
                    TextColor(status_color),
                    Node {
                        min_width: Val::Px(60.0),
                        ..default()
                    },
                ))
                .id();

            // Ground Track Checkbox Container
            row.spawn(Node {
                width: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|container| {
                let mut cb = container.spawn((checkbox(
                    (
                        SatelliteToggle {
                            norad,
                            kind: SatelliteToggleKind::GroundTrack,
                        },
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((bevy::ui::widget::Text::new(""), ThemedText)),
                ),));
                if entry.show_ground_track {
                    cb.insert(Checked);
                }
                ground_track_chk = cb.id();
            });

            // Trail Checkbox Container
            row.spawn(Node {
                width: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|container| {
                let mut cb = container.spawn((checkbox(
                    (
                        SatelliteToggle {
                            norad,
                            kind: SatelliteToggleKind::Trail,
                        },
                        AutoDirectionalNavigation::default(),
                    ),
                    Spawn((bevy::ui::widget::Text::new(""), ThemedText)),
                ),));
                if entry.show_trail {
                    cb.insert(Checked);
                }
                trail_chk = cb.id();
            });

            row.spawn(button(
                ButtonProps::default(),
                (
                    SatelliteActionButton {
                        norad,
                        action: SatelliteAction::Remove,
                    },
                    AutoDirectionalNavigation::default(),
                ),
                Spawn((
                    bevy::ui::widget::Text::new("x"),
                    ThemedText,
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                )),
            ))
            .insert(Node {
                width: Val::Px(28.0),
                flex_grow: 0.0,
                ..default()
            });
        })
        .insert(SatelliteRowRefs {
            track_btn,
            name_text,
            status_text: status_text_entity,
            ground_track_chk,
            trail_chk,
        });
}

fn sync_widget_states(
    store: Res<SatelliteStore>,
    ui_state: Res<UIState>,
    arrows: Res<ArrowConfig>,
    config_bundle: Res<UiConfigBundle>,
    heatmap_cfg: Res<HeatmapConfig>,
    selected: Res<SelectedSatellite>,
    sim_time: Res<crate::orbital::SimulationTime>,
    right_ui: Res<RightPanelUI>,
    mut checkboxes: Query<(Entity, &CheckboxBinding, Option<&Checked>)>,
    mut range_modes: Query<(Entity, &RangeModeBinding, Option<&Checked>)>,
    mut group_choices: Query<(Entity, &GroupChoice, Option<&Checked>)>,
    sliders: Query<(Entity, &SliderBinding), With<SliderValue>>,
    slider_values: Query<&SliderValue>,
    mut satellite_toggles: Query<(Entity, &SatelliteToggle, Option<&Checked>)>,
    mut commands: Commands,
) {
    if ui_state.is_changed()
        || arrows.is_changed()
        || config_bundle.is_changed()
        || heatmap_cfg.is_changed()
        || store.is_changed()
        || selected.is_changed()
        || sim_time.is_changed()
        || right_ui.is_changed()
    {
        for (entity, binding, checked) in checkboxes.iter_mut() {
            let should_check = match binding {
                CheckboxBinding::ShowAxes => ui_state.show_axes,
                CheckboxBinding::ShowArrows => arrows.enabled,
                CheckboxBinding::ArrowGradient => arrows.gradient_enabled,
                CheckboxBinding::ArrowGradientLog => arrows.gradient_log_scale,
                CheckboxBinding::GroundTracksEnabled => config_bundle.ground_track_cfg.enabled,
                CheckboxBinding::GizmoEnabled => config_bundle.gizmo_cfg.enabled,
                CheckboxBinding::GizmoShowCenterDot => config_bundle.gizmo_cfg.show_center_dot,
                CheckboxBinding::TrailsAll => {
                    !store.items.is_empty()
                        && store
                            .items
                            .values()
                            .filter(|s| s.propagator.is_some())
                            .all(|s| s.show_trail)
                }
                CheckboxBinding::TracksAll => {
                    !store.items.is_empty()
                        && store
                            .items
                            .values()
                            .filter(|s| s.propagator.is_some())
                            .all(|s| s.show_ground_track)
                }
                CheckboxBinding::HeatmapEnabled => heatmap_cfg.enabled,
                CheckboxBinding::ShowLeftPanel => ui_state.show_left_panel,
                CheckboxBinding::ShowRightPanel => ui_state.show_right_panel,
                CheckboxBinding::ShowTopPanel => ui_state.show_top_panel,
                CheckboxBinding::ShowBottomPanel => ui_state.show_bottom_panel,
            };

            match (should_check, checked.is_some()) {
                (true, false) => {
                    queue_set_checked(&mut commands, entity, true);
                }
                (false, true) => {
                    queue_set_checked(&mut commands, entity, false);
                }
                _ => {}
            }
        }

        for (entity, binding, checked) in range_modes.iter_mut() {
            let should_check = match binding {
                RangeModeBinding::Auto => heatmap_cfg.range_mode == RangeMode::Auto,
                RangeModeBinding::Fixed => heatmap_cfg.range_mode == RangeMode::Fixed,
            };
            match (should_check, checked.is_some()) {
                (true, false) => {
                    queue_set_checked(&mut commands, entity, true);
                }
                (false, true) => {
                    queue_set_checked(&mut commands, entity, false);
                }
                _ => {}
            }
        }

        if let Some(selected_group) = right_ui.selected_group.as_deref() {
            for (entity, choice, checked) in group_choices.iter_mut() {
                let should_check = choice.0 == selected_group;
                match (should_check, checked.is_some()) {
                    (true, false) => {
                        queue_set_checked(&mut commands, entity, true);
                    }
                    (false, true) => {
                        queue_set_checked(&mut commands, entity, false);
                    }
                    _ => {}
                }
            }
        }

        for (entity, binding) in sliders.iter() {
            let value = match binding {
                SliderBinding::GradientNear => arrows.gradient_near_km,
                SliderBinding::GradientFar => arrows.gradient_far_km,
                SliderBinding::GroundTrackRadius => config_bundle.ground_track_cfg.radius_km,
                SliderBinding::GizmoSegments => config_bundle.gizmo_cfg.circle_segments as f32,
                SliderBinding::GizmoCenterDotSize => config_bundle.gizmo_cfg.center_dot_size,
                SliderBinding::TrailMaxPoints => config_bundle.trail_cfg.max_points as f32,
                SliderBinding::TrailUpdateInterval => {
                    config_bundle.trail_cfg.update_interval_seconds
                }
                SliderBinding::HeatmapUpdatePeriod => heatmap_cfg.update_period_s,
                SliderBinding::HeatmapOpacity => heatmap_cfg.color_alpha,
                SliderBinding::HeatmapFixedMax => heatmap_cfg.fixed_max.unwrap_or(20) as f32,
                SliderBinding::HeatmapChunkSize => heatmap_cfg.chunk_size as f32,
                SliderBinding::HeatmapChunksPerFrame => heatmap_cfg.chunks_per_frame as f32,
                SliderBinding::SatelliteSphereRadius => config_bundle.render_cfg.sphere_radius,
                SliderBinding::SatelliteEmissiveIntensity => {
                    config_bundle.render_cfg.emissive_intensity
                }
                SliderBinding::TrackingDistance => selected.tracking_offset,
                SliderBinding::TrackingSmoothness => selected.smooth_factor,
                SliderBinding::TimeScale => sim_time.time_scale,
            };
            if let Ok(current) = slider_values.get(entity) {
                if (current.0 - value).abs() > f32::EPSILON {
                    commands.entity(entity).insert(SliderValue(value));
                }
            } else {
                commands.entity(entity).insert(SliderValue(value));
            }
        }

        for (entity, toggle, checked) in satellite_toggles.iter_mut() {
            if let Some(entry) = store.items.get(&toggle.norad) {
                let should_check = match toggle.kind {
                    SatelliteToggleKind::GroundTrack => entry.show_ground_track,
                    SatelliteToggleKind::Trail => entry.show_trail,
                };
                match (should_check, checked.is_some()) {
                    (true, false) => {
                        queue_set_checked(&mut commands, entity, true);
                    }
                    (false, true) => {
                        queue_set_checked(&mut commands, entity, false);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn sync_slider_visuals(
    mut sliders: Query<(
        Entity,
        &SliderValue,
        &SliderRange,
        Option<&SliderPrecision>,
        Option<&mut BackgroundGradient>,
    ), With<Slider>>,
    children: Query<&Children>,
    mut texts: Query<&mut bevy::ui::widget::Text>,
) {
    for (entity, value, range, precision, gradient) in sliders.iter_mut() {
        if let Some(mut gradient) = gradient {
            if let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..] {
                let percent_value = (range.thumb_position(value.0) * 100.0).clamp(0.0, 100.0);
                linear_gradient.stops[1].point = Val::Percent(percent_value);
                linear_gradient.stops[2].point = Val::Percent(percent_value);
            }
        }

        let precision = precision.map(|p| p.0).unwrap_or(0);
        let label = format!("{}", value.0);
        let decimals_len = label
            .split_once('.')
            .map(|(_, decimals)| decimals.len() as i32)
            .unwrap_or(precision);
        let formatted = if precision >= 0 && decimals_len <= precision {
            format!("{:.precision$}", value.0, precision = precision as usize)
        } else {
            label
        };

        for child in children.iter_descendants(entity) {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = formatted.clone();
            }
        }
    }
}

fn handle_button_activate(
    ev: On<Activate>,
    q_action: Query<&ButtonAction>,
    q_sat_action: Query<&SatelliteActionButton>,
    mut right_ui: ResMut<RightPanelUI>,
    mut store: ResMut<SatelliteStore>,
    mut selected: ResMut<SelectedSatellite>,
    mut sim_time: ResMut<crate::orbital::SimulationTime>,
    mut commands: Commands,
    fetch_channels: Option<Res<FetchChannels>>,
) {
    if let Ok(action) = q_action.get(ev.entity) {
        match action {
            ButtonAction::LoadGroup => {
                if right_ui.group_loading {
                    return;
                }
                if let Some(group) = &right_ui.selected_group {
                    if let Some(fetch) = fetch_channels {
                        if let Err(e) = fetch.cmd_tx.send(FetchCommand::FetchGroup {
                            group: group.clone(),
                        }) {
                            right_ui.error = Some(format!("Failed to request group: {}", e));
                            right_ui.group_loading = false;
                        } else {
                            right_ui.group_loading = true;
                            right_ui.error = None;
                        }
                    } else {
                        right_ui.error = Some("Fetch service not available".to_string());
                    }
                } else {
                    right_ui.error = Some("Please select a group first".to_string());
                }
            }
            ButtonAction::ClearAll => {
                for entry in store.items.values_mut() {
                    if let Some(entity) = entry.entity.take() {
                        commands.entity(entity).despawn_children();
                        commands.entity(entity).despawn();
                    }
                }
                store.items.clear();
                store.entity_by_norad.clear();
                right_ui.error = None;
                selected.tracking = None;
            }
            ButtonAction::AddSatellite => {
                right_ui.pending_add = true;
            }
            ButtonAction::StopTracking => {
                selected.tracking = None;
            }
            ButtonAction::TimeScale1x => {
                sim_time.time_scale = 1.0;
            }
            ButtonAction::TimeNow => {
                sim_time.current_utc = chrono::Utc::now();
                sim_time.time_scale = 1.0;
            }
        }
    }

    if let Ok(action) = q_sat_action.get(ev.entity) {
        match action.action {
            SatelliteAction::Track => {
                if selected.tracking == Some(action.norad) {
                    selected.tracking = None;
                } else {
                    selected.selected = Some(action.norad);
                    selected.tracking = Some(action.norad);
                }
            }
            SatelliteAction::Remove => {
                if let Some(entry) = store.items.remove(&action.norad) {
                    if let Some(entity) = entry.entity {
                        commands.entity(entity).despawn_children();
                        commands.entity(entity).despawn();
                    }
                }
                store.entity_by_norad.remove(&action.norad);
                if selected.tracking == Some(action.norad) {
                    selected.tracking = None;
                }
                if selected.selected == Some(action.norad) {
                    selected.selected = None;
                }
            }
        }
    }
}

fn handle_checkbox_change(
    ev: On<ValueChange<bool>>,
    q_binding: Query<&CheckboxBinding>,
    q_sat_toggle: Query<&SatelliteToggle>,
    mut arrows: ResMut<ArrowConfig>,
    mut ui_state: ResMut<UIState>,
    mut config_bundle: ResMut<UiConfigBundle>,
    mut heatmap_cfg: ResMut<HeatmapConfig>,
    mut store: ResMut<SatelliteStore>,
) {
    if let Ok(binding) = q_binding.get(ev.source) {
        match binding {
            CheckboxBinding::ShowAxes => ui_state.show_axes = ev.value,
            CheckboxBinding::ShowArrows => arrows.enabled = ev.value,
            CheckboxBinding::ArrowGradient => arrows.gradient_enabled = ev.value,
            CheckboxBinding::ArrowGradientLog => arrows.gradient_log_scale = ev.value,
            CheckboxBinding::GroundTracksEnabled => {
                config_bundle.ground_track_cfg.enabled = ev.value
            }
            CheckboxBinding::GizmoEnabled => config_bundle.gizmo_cfg.enabled = ev.value,
            CheckboxBinding::GizmoShowCenterDot => {
                config_bundle.gizmo_cfg.show_center_dot = ev.value
            }
            CheckboxBinding::TrailsAll => {
                for entry in store.items.values_mut() {
                    if entry.propagator.is_some() {
                        entry.show_trail = ev.value;
                    }
                }
            }
            CheckboxBinding::TracksAll => {
                for entry in store.items.values_mut() {
                    if entry.propagator.is_some() {
                        entry.show_ground_track = ev.value;
                    }
                }
            }
            CheckboxBinding::HeatmapEnabled => heatmap_cfg.enabled = ev.value,
            CheckboxBinding::ShowLeftPanel => ui_state.show_left_panel = ev.value,
            CheckboxBinding::ShowRightPanel => ui_state.show_right_panel = ev.value,
            CheckboxBinding::ShowTopPanel => ui_state.show_top_panel = ev.value,
            CheckboxBinding::ShowBottomPanel => ui_state.show_bottom_panel = ev.value,
        }
        return;
    }

    if let Ok(toggle) = q_sat_toggle.get(ev.source) {
        if let Some(entry) = store.items.get_mut(&toggle.norad) {
            match toggle.kind {
                SatelliteToggleKind::GroundTrack => entry.show_ground_track = ev.value,
                SatelliteToggleKind::Trail => entry.show_trail = ev.value,
            }
        }
    }
}

fn handle_slider_change(
    ev: On<ValueChange<f32>>,
    q_binding: Query<&SliderBinding>,
    mut arrows: ResMut<ArrowConfig>,
    mut config_bundle: ResMut<UiConfigBundle>,
    mut heatmap_cfg: ResMut<HeatmapConfig>,
    mut selected: ResMut<SelectedSatellite>,
    mut sim_time: ResMut<crate::orbital::SimulationTime>,
) {
    let Ok(binding) = q_binding.get(ev.source) else {
        return;
    };

    match binding {
        SliderBinding::GradientNear => arrows.gradient_near_km = ev.value,
        SliderBinding::GradientFar => arrows.gradient_far_km = ev.value,
        SliderBinding::GroundTrackRadius => config_bundle.ground_track_cfg.radius_km = ev.value,
        SliderBinding::GizmoSegments => {
            config_bundle.gizmo_cfg.circle_segments = ev.value.round().clamp(16.0, 128.0) as u32
        }
        SliderBinding::GizmoCenterDotSize => config_bundle.gizmo_cfg.center_dot_size = ev.value,
        SliderBinding::TrailMaxPoints => {
            config_bundle.trail_cfg.max_points = ev.value.round().clamp(100.0, 10000.0) as usize
        }
        SliderBinding::TrailUpdateInterval => {
            config_bundle.trail_cfg.update_interval_seconds = ev.value
        }
        SliderBinding::HeatmapUpdatePeriod => heatmap_cfg.update_period_s = ev.value,
        SliderBinding::HeatmapOpacity => heatmap_cfg.color_alpha = ev.value,
        SliderBinding::HeatmapFixedMax => {
            heatmap_cfg.fixed_max = Some(ev.value.round().clamp(1.0, 100.0) as u32)
        }
        SliderBinding::HeatmapChunkSize => {
            heatmap_cfg.chunk_size = ev.value.round().clamp(500.0, 5000.0) as usize
        }
        SliderBinding::HeatmapChunksPerFrame => {
            heatmap_cfg.chunks_per_frame = ev.value.round().clamp(1.0, 5.0) as usize
        }
        SliderBinding::SatelliteSphereRadius => config_bundle.render_cfg.sphere_radius = ev.value,
        SliderBinding::SatelliteEmissiveIntensity => {
            config_bundle.render_cfg.emissive_intensity = ev.value
        }
        SliderBinding::TrackingDistance => selected.tracking_offset = ev.value,
        SliderBinding::TrackingSmoothness => selected.smooth_factor = ev.value,
        SliderBinding::TimeScale => sim_time.time_scale = ev.value,
    }
}

fn handle_range_mode_change(
    ev: On<ValueChange<bool>>,
    q_binding: Query<&RangeModeBinding>,
    mut heatmap_cfg: ResMut<HeatmapConfig>,
) {
    let Ok(binding) = q_binding.get(ev.source) else {
        return;
    };
    if !ev.value {
        return;
    }
    heatmap_cfg.range_mode = match binding {
        RangeModeBinding::Auto => RangeMode::Auto,
        RangeModeBinding::Fixed => RangeMode::Fixed,
    };
}

fn handle_group_choice(
    ev: On<ValueChange<bool>>,
    q_choice: Query<&GroupChoice>,
    q_all_choices: Query<(Entity, &GroupChoice, Option<&Checked>)>,
    mut commands: Commands,
    mut right_ui: ResMut<RightPanelUI>,
) {
    let Ok(choice) = q_choice.get(ev.source) else {
        return;
    };
    if ev.value {
        right_ui.selected_group = Some(choice.0.to_string());
        right_ui.error = None;

        for (entity, group, checked) in q_all_choices.iter() {
            let should_check = group.0 == choice.0;
            match (should_check, checked.is_some()) {
                (true, false) => {
                    queue_set_checked(&mut commands, entity, true);
                }
                (false, true) => {
                    queue_set_checked(&mut commands, entity, false);
                }
                _ => {}
            }
        }
    }
}

fn text_input_on_click(
    ev: On<Pointer<Click>>,
    q_input: Query<(), With<TextInputField>>,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
) {
    if q_input.contains(ev.entity) {
        if let Some(mut focus) = focus {
            focus.0 = Some(ev.entity);
        }
        if let Some(mut focus_visible) = focus_visible {
            focus_visible.0 = true;
        }
    }
}

fn text_input_on_key_input(
    ev: On<FocusedInput<bevy::input::keyboard::KeyboardInput>>,
    q_input: Query<(), With<TextInputField>>,
    mut right_ui: ResMut<RightPanelUI>,
) {
    if !q_input.contains(ev.focused_entity) {
        return;
    }

    let event = &ev.event().input;
    if event.state != ButtonState::Pressed || event.repeat {
        return;
    }

    match event.key_code {
        KeyCode::Backspace => {
            right_ui.input.pop();
        }
        KeyCode::Enter => {
            right_ui.pending_add = true;
        }
        KeyCode::Digit0 => right_ui.input.push('0'),
        KeyCode::Digit1 => right_ui.input.push('1'),
        KeyCode::Digit2 => right_ui.input.push('2'),
        KeyCode::Digit3 => right_ui.input.push('3'),
        KeyCode::Digit4 => right_ui.input.push('4'),
        KeyCode::Digit5 => right_ui.input.push('5'),
        KeyCode::Digit6 => right_ui.input.push('6'),
        KeyCode::Digit7 => right_ui.input.push('7'),
        KeyCode::Digit8 => right_ui.input.push('8'),
        KeyCode::Digit9 => right_ui.input.push('9'),
        KeyCode::Numpad0 => right_ui.input.push('0'),
        KeyCode::Numpad1 => right_ui.input.push('1'),
        KeyCode::Numpad2 => right_ui.input.push('2'),
        KeyCode::Numpad3 => right_ui.input.push('3'),
        KeyCode::Numpad4 => right_ui.input.push('4'),
        KeyCode::Numpad5 => right_ui.input.push('5'),
        KeyCode::Numpad6 => right_ui.input.push('6'),
        KeyCode::Numpad7 => right_ui.input.push('7'),
        KeyCode::Numpad8 => right_ui.input.push('8'),
        KeyCode::Numpad9 => right_ui.input.push('9'),
        _ => {}
    }
}

fn handle_right_panel_resize_start(
    ev: On<Pointer<DragStart>>,
    q_handle: Query<(), With<RightPanelResizeHandle>>,
    mut layout: ResMut<UiLayoutState>,
) {
    if !q_handle.contains(ev.entity) {
        return;
    }

    layout.resizing_right_panel = true;
    layout.resize_start_width_px = layout.right_panel_width_px;
}

fn handle_right_panel_resize_drag(
    ev: On<Pointer<Drag>>,
    q_handle: Query<(), With<RightPanelResizeHandle>>,
    mut layout: ResMut<UiLayoutState>,
) {
    if !q_handle.contains(ev.entity) || !layout.resizing_right_panel {
        return;
    }

    let drag = ev.event();
    let width = (layout.resize_start_width_px - drag.distance.x)
        .clamp(layout.right_panel_min_px, layout.right_panel_max_px);
    layout.right_panel_width_px = width;
}

fn handle_right_panel_resize_end(
    ev: On<Pointer<DragEnd>>,
    q_handle: Query<(), With<RightPanelResizeHandle>>,
    mut layout: ResMut<UiLayoutState>,
) {
    if !q_handle.contains(ev.entity) {
        return;
    }

    layout.resizing_right_panel = false;
}

fn navigate_focus_with_arrows(
    input: Res<ButtonInput<KeyCode>>,
    mut navigator: AutoDirectionalNavigator,
) {
    let direction = if input.just_pressed(KeyCode::ArrowUp) {
        Some(bevy::math::CompassOctant::North)
    } else if input.just_pressed(KeyCode::ArrowDown) {
        Some(bevy::math::CompassOctant::South)
    } else if input.just_pressed(KeyCode::ArrowLeft) {
        Some(bevy::math::CompassOctant::West)
    } else if input.just_pressed(KeyCode::ArrowRight) {
        Some(bevy::math::CompassOctant::East)
    } else {
        None
    };

    if let Some(direction) = direction {
        let _ = navigator.navigate(direction);
    }
}
