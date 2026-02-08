//! UI systems for the Bevy UI interface

use bevy::camera::Viewport;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::ecs::spawn::Spawn;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::EntityWorldMut;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::{ButtonInput, ButtonState};
use bevy::picking::Pickable;
use bevy::picking::events::{Click, Drag, DragEnd, DragStart, Pointer};
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
use bevy_feathers::tokens;
use bevy_input_focus::{FocusedInput, InputFocus, InputFocusVisible, tab_navigation::TabIndex};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraSystemSet};
use bevy_ui_widgets::{
    Activate, Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, ValueChange,
    checkbox_self_update, slider_self_update,
};
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

use crate::orbital::time::SimulationTime;
use crate::satellite::{
    OrbitTrailConfig, SatelliteRenderConfig, SatelliteStore, SelectedSatellite,
};
use crate::space_weather::{AuroraGrid, KpIndex, SolarWind, SpaceWeatherConfig, SpaceWeatherState};
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
    hidden_panels_hint: Entity,
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
struct HiddenPanelsHint;

#[derive(Resource, Default)]
struct HiddenPanelsHintState {
    last_shown: Option<Instant>,
    visible: bool,
}

#[derive(Component)]
struct RightPanelResizeHandle;

#[derive(Component)]
struct RightPanelScroll;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct HiddenPanelsHintCard;

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
struct SpaceWeatherKpText;

#[derive(Component)]
struct SpaceWeatherMagText;

#[derive(Component)]
struct SpaceWeatherPlasmaText;

#[derive(Component)]
struct SpaceWeatherUpdatedText;

#[derive(Component)]
struct SpaceWeatherErrorText;

#[derive(Component)]
struct AuroraStatusText;

#[derive(Component)]
struct TextInputField;

#[derive(Component)]
struct TextInputValueText;

#[derive(Component)]
struct TextInputPlaceholderText;

#[derive(Component)]
struct TooltipBubble;

#[derive(Component)]
struct TooltipTarget {
    bubble: Entity,
}

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
    AuroraOverlay,
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
    AuroraIntensity,
    AuroraAlpha,
    AuroraLongitudeOffset,
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
enum PanelToggleKind {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Component, Clone, Copy)]
struct PanelToggle {
    kind: PanelToggleKind,
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

#[derive(Component)]
struct SectionToggle {
    body: Entity,
}

#[derive(Component)]
struct UiFontBold;

#[derive(Resource, Clone)]
struct UiFontHandles {
    medium: Handle<Font>,
    bold: Handle<Font>,
}

const PANEL_BG: Color = Color::srgba(0.03, 0.05, 0.08, 0.78);
const PANEL_EDGE: Color = Color::srgba(0.1, 0.9, 0.95, 0.35);
const PANEL_DIVIDER: Color = Color::srgba(0.12, 0.3, 0.35, 0.9);
const PANEL_INNER_BG: Color = Color::srgba(0.02, 0.04, 0.06, 0.85);
const PANEL_TEXT_ACCENT: Color = Color::srgba(0.4, 1.0, 1.0, 1.0);
const TOOLTIP_BG: Color = Color::srgba(0.02, 0.08, 0.12, 0.95);
const TOOLTIP_TEXT: Color = Color::srgba(0.8, 0.9, 1.0, 0.95);
const TOOLTIP_MAX_WIDTH_PX: f32 = 220.0;
const UI_FONT_PATH: &str = "Orbitron-Medium.ttf";
const UI_FONT_BOLD_PATH: &str = "Orbitron-Bold.ttf";
const TOP_PANEL_HEIGHT_PX: f32 = 52.0;
const BOTTOM_PANEL_HEIGHT_PX: f32 = 32.0;
const GRID_LINE: Color = Color::srgba(0.1, 0.6, 0.7, 0.10);
const GRID_STEPS: [f32; 9] = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0];
const HIDDEN_PANELS_HINT_DURATION: Duration = Duration::from_secs(5);
const HIDDEN_PANELS_HINT_OFFSET_X_PX: f32 = 320.0;

#[derive(Clone, Copy)]
enum Edge {
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct LabelStyle {
    font_size: f32,
    color: Option<Color>,
    bold: bool,
}

impl LabelStyle {
    fn normal(font_size: f32) -> Self {
        Self {
            font_size,
            color: None,
            bold: false,
        }
    }

    fn accent(font_size: f32) -> Self {
        Self {
            font_size,
            color: Some(PANEL_TEXT_ACCENT),
            bold: true,
        }
    }
}

type UiCameraQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Camera,
        Option<&'static Camera2d>,
        Option<&'static Camera3d>,
        Option<&'static MainCamera>,
    ),
>;

type SliderVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SliderValue,
        &'static SliderRange,
        Option<&'static SliderPrecision>,
        Option<&'static mut BackgroundGradient>,
    ),
    With<Slider>,
>;

#[derive(SystemParam)]
struct SyncWidgetStateParams<'w, 's> {
    store: Res<'w, SatelliteStore>,
    ui_state: Res<'w, UIState>,
    arrows: Res<'w, ArrowConfig>,
    config_bundle: Res<'w, UiConfigBundle>,
    heatmap_cfg: Res<'w, HeatmapConfig>,
    space_weather_cfg: Res<'w, SpaceWeatherConfig>,
    selected: Res<'w, SelectedSatellite>,
    sim_time: Res<'w, crate::orbital::SimulationTime>,
    right_ui: Res<'w, RightPanelUI>,
    checkboxes: Query<'w, 's, (Entity, &'static CheckboxBinding, Option<&'static Checked>)>,
    range_modes: Query<'w, 's, (Entity, &'static RangeModeBinding, Option<&'static Checked>)>,
    group_choices: Query<'w, 's, (Entity, &'static GroupChoice, Option<&'static Checked>)>,
    sliders: Query<'w, 's, (Entity, &'static SliderBinding), With<SliderValue>>,
    slider_values: Query<'w, 's, &'static SliderValue>,
    satellite_toggles: Query<
        'w,
        's,
        (Entity, &'static SatelliteToggle, Option<&'static Checked>),
    >,
    commands: Commands<'w, 's>,
}

#[derive(SystemParam)]
struct ButtonActivateParams<'w, 's> {
    q_action: Query<'w, 's, &'static ButtonAction>,
    q_sat_action: Query<'w, 's, &'static SatelliteActionButton>,
    q_panel_toggle: Query<'w, 's, &'static PanelToggle>,
    right_ui: ResMut<'w, RightPanelUI>,
    store: ResMut<'w, SatelliteStore>,
    selected: ResMut<'w, SelectedSatellite>,
    sim_time: ResMut<'w, crate::orbital::SimulationTime>,
    ui_state: ResMut<'w, UIState>,
    commands: Commands<'w, 's>,
    fetch_channels: Option<Res<'w, FetchChannels>>,
}

#[derive(SystemParam)]
struct CheckboxChangeParams<'w, 's> {
    q_binding: Query<'w, 's, &'static CheckboxBinding>,
    q_sat_toggle: Query<'w, 's, &'static SatelliteToggle>,
    arrows: ResMut<'w, ArrowConfig>,
    ui_state: ResMut<'w, UIState>,
    config_bundle: ResMut<'w, UiConfigBundle>,
    heatmap_cfg: ResMut<'w, HeatmapConfig>,
    space_weather_cfg: ResMut<'w, SpaceWeatherConfig>,
    store: ResMut<'w, SatelliteStore>,
}

/// Plugin that registers UI systems and observers
pub struct UiSystemsPlugin;

impl Plugin for UiSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HiddenPanelsHintState>()
            .add_systems(
                Startup,
                (setup_ui_camera, setup_ui, apply_orbitron_font, load_ui_font).chain(),
            )
            .add_systems(
                Update,
                (
                    toggle_panels_keyboard,
                    apply_panel_visibility,
                    apply_panel_layout,
                    update_hidden_panels_hint,
                    update_tooltip_visibility,
                    sync_panel_toggle_buttons,
                    scroll_right_panel_on_wheel,
                    update_time_display,
                    update_status_texts,
                    update_space_weather_texts,
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
                    enforce_orbitron_text,
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
            .add_observer(handle_section_toggle)
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

fn enforce_ui_camera_settings(mut cameras: UiCameraQuery<'_, '_>) {
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

fn apply_orbitron_font(mut fonts: Query<&mut InheritableFont>) {
    for mut font in &mut fonts {
        font.font = HandleOrPath::Path(UI_FONT_PATH.to_owned());
    }
}

fn load_ui_font(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFontHandles {
        medium: assets.load(UI_FONT_PATH),
        bold: assets.load(UI_FONT_BOLD_PATH),
    });
}

fn enforce_orbitron_text(
    ui_font: Res<UiFontHandles>,
    mut q_text: Query<(&mut TextFont, Option<&UiFontBold>), With<ThemedText>>,
) {
    for (mut font, is_bold) in &mut q_text {
        let target = if is_bold.is_some() {
            &ui_font.bold
        } else {
            &ui_font.medium
        };
        if font.font != *target {
            font.font = target.clone();
        }
    }
}

fn setup_ui(
    mut commands: Commands,
    layout: Res<UiLayoutState>,
    arrows: Res<ArrowConfig>,
    config_bundle: Res<UiConfigBundle>,
    heatmap_cfg: Res<HeatmapConfig>,
    space_weather_cfg: Res<SpaceWeatherConfig>,
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
                font: HandleOrPath::Path(UI_FONT_PATH.to_owned()),
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
            BackgroundColor(PANEL_BG),
            InheritableFont {
                font: HandleOrPath::Path(UI_FONT_PATH.to_owned()),
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
            BackgroundColor(PANEL_BG),
            InheritableFont {
                font: HandleOrPath::Path(UI_FONT_PATH.to_owned()),
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
                height: Val::Px(TOP_PANEL_HEIGHT_PX),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(PANEL_BG),
            InheritableFont {
                font: HandleOrPath::Path(UI_FONT_PATH.to_owned()),
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
                height: Val::Px(BOTTOM_PANEL_HEIGHT_PX),
                padding: UiRect::horizontal(Val::Px(12.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            InheritableFont {
                font: HandleOrPath::Path(UI_FONT_PATH.to_owned()),
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

    let mut hidden_panels_hint = Entity::PLACEHOLDER;
    commands.entity(root).with_children(|parent| {
        hidden_panels_hint = spawn_hidden_panels_hint(parent);
    });

    commands.entity(left_panel).with_children(|panel| {
        spawn_edge_glow(panel, Edge::Right);
        spawn_grid_overlay(panel);
    });
    commands.entity(right_panel).with_children(|panel| {
        spawn_edge_glow(panel, Edge::Left);
        spawn_grid_overlay(panel);
    });
    commands.entity(top_panel).with_children(|panel| {
        spawn_edge_glow(panel, Edge::Top);
        spawn_grid_overlay(panel);
    });
    commands.entity(bottom_panel).with_children(|panel| {
        spawn_edge_glow(panel, Edge::Top);
        spawn_grid_overlay(panel);
    });

    // Left panel contents
    commands.entity(left_panel).with_children(|parent| {
        parent.spawn((
            bevy::ui::widget::Text::new("Space Weather"),
            ThemedText,
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(PANEL_TEXT_ACCENT),
            UiFontBold,
        ));

        spawn_section(parent, "Overview", true, |section| {
            spawn_space_weather_row(
                section,
                SpaceWeatherKpText,
                "Kp: --",
                "Kp is the planetary geomagnetic index (0-9). Higher values mean stronger geomagnetic activity.",
            );
            spawn_space_weather_row(
                section,
                SpaceWeatherMagText,
                "Bz: -- nT  Bt: -- nT",
                "Bz is the north-south IMF component (GSM) in nT. Negative values indicate southward fields.",
            );
            spawn_space_weather_row(
                section,
                SpaceWeatherPlasmaText,
                "Vsw: -- km/s  n: -- cm^-3",
                "Vsw is solar wind speed in km/s. Higher values indicate faster solar wind streams.",
            );
            section.spawn((
                SpaceWeatherUpdatedText,
                bevy::ui::widget::Text::new("Updated: --"),
                ThemedText,
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.85)),
            ));

            section.spawn((checkbox(
                (
                    CheckboxBinding::AuroraOverlay,
                    AutoDirectionalNavigation::default(),
                ),
                Spawn((bevy::ui::widget::Text::new("Aurora overlay"), ThemedText)),
            ),));

            section.spawn((
                AuroraStatusText,
                bevy::ui::widget::Text::new(""),
                ThemedText,
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.4, 0.85)),
            ));

            spawn_labeled_slider(
                section,
                "Intensity",
                SliderBinding::AuroraIntensity,
                0.1,
                3.0,
                space_weather_cfg.aurora_intensity_scale,
                0.1,
            );
            spawn_labeled_slider(
                section,
                "Alpha",
                SliderBinding::AuroraAlpha,
                0.0,
                1.0,
                space_weather_cfg.aurora_alpha,
                0.05,
            );
            spawn_labeled_slider(
                section,
                "Longitude offset",
                SliderBinding::AuroraLongitudeOffset,
                -180.0,
                180.0,
                space_weather_cfg.aurora_longitude_offset,
                5.0,
            );

            section.spawn((
                SpaceWeatherErrorText,
                bevy::ui::widget::Text::new(""),
                ThemedText,
                TextColor(Color::srgb(1.0, 0.35, 0.35)),
            ));
        });

        spawn_section(parent, "City → Sat Vis", true, |section| {
            section.spawn((checkbox(
                (
                    CheckboxBinding::ShowArrows,
                    AutoDirectionalNavigation::default(),
                ),
                Spawn((bevy::ui::widget::Text::new("Show arrows"), ThemedText)),
            ),));

            section.spawn((checkbox(
                (
                    CheckboxBinding::ArrowGradient,
                    AutoDirectionalNavigation::default(),
                ),
                Spawn((
                    bevy::ui::widget::Text::new("Distance color gradient"),
                    ThemedText,
                )),
            ),));

            section.spawn((checkbox(
                (
                    CheckboxBinding::ArrowGradientLog,
                    AutoDirectionalNavigation::default(),
                ),
                Spawn((bevy::ui::widget::Text::new("Log scale"), ThemedText)),
            ),));

            section.spawn((
                bevy::ui::widget::Text::new("Gradient range (km)"),
                ThemedText,
            ));
            spawn_labeled_slider(
                section,
                "Near",
                SliderBinding::GradientNear,
                10.0,
                200000.0,
                arrows.gradient_near_km,
                1000.0,
            );
            spawn_labeled_slider(
                section,
                "Far",
                SliderBinding::GradientFar,
                10.0,
                200000.0,
                arrows.gradient_far_km,
                1000.0,
            );

            section.spawn((checkbox(
                (
                    CheckboxBinding::ShowAxes,
                    AutoDirectionalNavigation::default(),
                ),
                Spawn((bevy::ui::widget::Text::new("Show axes"), ThemedText)),
            ),));
        });
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
                        parent.spawn((
                            bevy::ui::widget::Text::new("Satellites"),
                            ThemedText,
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(PANEL_TEXT_ACCENT),
                            UiFontBold,
                        ));
                        spawn_section(parent, "Satellite Groups", true, |section| {
                            section
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(8.0),
                                        width: Val::Percent(100.0),
                                        min_width: Val::Px(0.0),
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
                                                width: Val::Percent(100.0),
                                                min_width: Val::Px(0.0),
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
                                                        GroupChoice(group_key),
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

                            section.spawn((button(
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

                            section.spawn((button(
                                ButtonProps::default(),
                                (ButtonAction::ClearAll, AutoDirectionalNavigation::default()),
                                Spawn((
                                    bevy::ui::widget::Text::new("Clear All Satellites"),
                                    ThemedText,
                                )),
                            ),));

                            section.spawn((
                                GroupLoadingText,
                                bevy::ui::widget::Text::new(""),
                                ThemedText,
                            ));
                        });

                        spawn_section(parent, "Add Satellite", false, |section| {
                            section
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

                            section.spawn((
                                ErrorText,
                                bevy::ui::widget::Text::new(""),
                                ThemedText,
                                TextColor(Color::srgb(1.0, 0.2, 0.2)),
                            ));
                        });

                        spawn_section(parent, "Ground Tracks", false, |section| {
                            section.spawn((checkbox(
                                (
                                    CheckboxBinding::TracksAll,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((bevy::ui::widget::Text::new("All Tracks"), ThemedText)),
                            ),));
                            section.spawn((checkbox(
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
                                section,
                                "Track radius (km)",
                                SliderBinding::GroundTrackRadius,
                                10.0,
                                500.0,
                                config_bundle.ground_track_cfg.radius_km,
                                5.0,
                            );

                            section
                                .spawn((bevy::ui::widget::Text::new("Gizmo Settings"), ThemedText));
                            section.spawn((checkbox(
                                (
                                    CheckboxBinding::GizmoEnabled,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((
                                    bevy::ui::widget::Text::new("Use gizmo circles"),
                                    ThemedText,
                                )),
                            ),));
                            spawn_labeled_slider(
                                section,
                                "Circle segments",
                                SliderBinding::GizmoSegments,
                                16.0,
                                128.0,
                                config_bundle.gizmo_cfg.circle_segments as f32,
                                1.0,
                            );
                            section.spawn((checkbox(
                                (
                                    CheckboxBinding::GizmoShowCenterDot,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((bevy::ui::widget::Text::new("Show center dot"), ThemedText)),
                            ),));
                            spawn_labeled_slider(
                                section,
                                "Center dot size (km)",
                                SliderBinding::GizmoCenterDotSize,
                                50.0,
                                500.0,
                                config_bundle.gizmo_cfg.center_dot_size,
                                10.0,
                            );
                        });

                        spawn_section(parent, "Orbit Trails", false, |section| {
                            section.spawn((checkbox(
                                (
                                    CheckboxBinding::TrailsAll,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((bevy::ui::widget::Text::new("All Trails"), ThemedText)),
                            ),));
                            spawn_labeled_slider(
                                section,
                                "Max history points",
                                SliderBinding::TrailMaxPoints,
                                100.0,
                                10000.0,
                                config_bundle.trail_cfg.max_points as f32,
                                50.0,
                            );
                            spawn_labeled_slider(
                                section,
                                "Update interval (s)",
                                SliderBinding::TrailUpdateInterval,
                                0.5,
                                10.0,
                                config_bundle.trail_cfg.update_interval_seconds,
                                0.1,
                            );
                        });

                        spawn_section(parent, "Heatmap", false, |section| {
                            section.spawn((checkbox(
                                (
                                    CheckboxBinding::HeatmapEnabled,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((bevy::ui::widget::Text::new("Enable heatmap"), ThemedText)),
                            ),));
                            spawn_labeled_slider(
                                section,
                                "Update period (s)",
                                SliderBinding::HeatmapUpdatePeriod,
                                0.1,
                                2.0,
                                heatmap_cfg.update_period_s,
                                0.1,
                            );
                            spawn_labeled_slider(
                                section,
                                "Opacity",
                                SliderBinding::HeatmapOpacity,
                                0.0,
                                1.0,
                                heatmap_cfg.color_alpha,
                                0.05,
                            );

                            section.spawn((bevy::ui::widget::Text::new("Range mode"), ThemedText));
                            section.spawn((radio(
                                (RangeModeBinding::Auto, AutoDirectionalNavigation::default()),
                                Spawn((bevy::ui::widget::Text::new("Auto"), ThemedText)),
                            ),));
                            section.spawn((radio(
                                (
                                    RangeModeBinding::Fixed,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((bevy::ui::widget::Text::new("Fixed"), ThemedText)),
                            ),));

                            spawn_labeled_slider(
                                section,
                                "Fixed max",
                                SliderBinding::HeatmapFixedMax,
                                1.0,
                                100.0,
                                heatmap_cfg.fixed_max.unwrap_or(20) as f32,
                                1.0,
                            );

                            spawn_labeled_slider(
                                section,
                                "Chunk size",
                                SliderBinding::HeatmapChunkSize,
                                500.0,
                                5000.0,
                                heatmap_cfg.chunk_size as f32,
                                100.0,
                            );
                            spawn_labeled_slider(
                                section,
                                "Chunks/frame",
                                SliderBinding::HeatmapChunksPerFrame,
                                1.0,
                                5.0,
                                heatmap_cfg.chunks_per_frame as f32,
                                1.0,
                            );
                        });

                        spawn_section(parent, "Satellite Rendering", false, |section| {
                            spawn_labeled_slider(
                                section,
                                "Sphere size (km)",
                                SliderBinding::SatelliteSphereRadius,
                                1.0,
                                200.0,
                                config_bundle.render_cfg.sphere_radius,
                                1.0,
                            );
                            spawn_labeled_slider(
                                section,
                                "Emissive intensity",
                                SliderBinding::SatelliteEmissiveIntensity,
                                10.0,
                                500.0,
                                config_bundle.render_cfg.emissive_intensity,
                                5.0,
                            );
                        });

                        // Atmosphere controls removed for now (feature disabled).

                        spawn_section(parent, "Camera Tracking", false, |section| {
                            section.spawn((
                                TrackingStatusText,
                                bevy::ui::widget::Text::new("Tracking: None"),
                                ThemedText,
                            ));
                            section.spawn((button(
                                ButtonProps::default(),
                                (
                                    ButtonAction::StopTracking,
                                    AutoDirectionalNavigation::default(),
                                ),
                                Spawn((bevy::ui::widget::Text::new("Stop Tracking"), ThemedText)),
                            ),));
                            spawn_labeled_slider(
                                section,
                                "Tracking distance (km)",
                                SliderBinding::TrackingDistance,
                                1000.0,
                                20000.0,
                                selected.tracking_offset,
                                100.0,
                            );
                            spawn_labeled_slider(
                                section,
                                "Tracking smoothness",
                                SliderBinding::TrackingSmoothness,
                                0.01,
                                1.0,
                                selected.smooth_factor,
                                0.01,
                            );
                        });

                        spawn_section(parent, "Satellites List", false, |section| {
                            // Header Row
                            section
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(6.0),
                                        padding: UiRect::horizontal(Val::Px(4.0)),
                                        width: Val::Percent(100.0),
                                        min_width: Val::Px(0.0),
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

                            section
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(6.0),
                                        height: Val::Px(240.0),
                                        width: Val::Percent(100.0),
                                        min_width: Val::Px(0.0),
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
                                                width: Val::Percent(100.0),
                                                min_width: Val::Px(0.0),
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
                        });
                    })
                    .id();

                spawn_scrollbar_fill(row, scroll_entity);
            });
    });

    // Top panel contents
    commands.entity(top_panel).with_children(|parent| {
        spawn_top_time_row(parent);
        spawn_top_speed_row(parent, sim_time.time_scale);
        spawn_top_panel_toggles_row(parent);
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
        hidden_panels_hint,
    });
}

struct LabeledSliderRowProps<'a> {
    label: &'a str,
    label_style: LabelStyle,
    binding: SliderBinding,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
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
    spawn_labeled_slider_row(
        parent,
        LabeledSliderRowProps {
            label,
            label_style: LabelStyle::normal(12.0),
            binding,
            min,
            max,
            value,
            step,
        },
    );
}

fn spawn_styled_text<B: Bundle>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    style: LabelStyle,
    extra: B,
) {
    let text = bevy::ui::widget::Text::new(label);
    let font = TextFont {
        font_size: style.font_size,
        ..default()
    };

    match (style.color, style.bold) {
        (Some(color), true) => {
            parent.spawn((extra, text, ThemedText, font, TextColor(color), UiFontBold));
        }
        (Some(color), false) => {
            parent.spawn((extra, text, ThemedText, font, TextColor(color)));
        }
        (None, true) => {
            parent.spawn((extra, text, ThemedText, font, UiFontBold));
        }
        (None, false) => {
            parent.spawn((extra, text, ThemedText, font));
        }
    }
}

fn spawn_pill_chip<B: Bundle>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    style: LabelStyle,
    extra: B,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_INNER_BG),
            Outline::new(Val::Px(1.0), Val::Px(0.0), PANEL_EDGE),
            Pickable::IGNORE,
            ThemedText,
        ))
        .with_children(|chip| {
            spawn_styled_text(chip, label, style, extra);
        });
}

fn spawn_labeled_slider_row(parent: &mut ChildSpawnerCommands, props: LabeledSliderRowProps<'_>) {
    let precision = slider_precision_from_step(props.step);
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
            spawn_styled_text(row, props.label, props.label_style, ());
            row.spawn((slider(
                SliderProps {
                    value: props.value,
                    min: props.min,
                    max: props.max,
                },
                (
                    props.binding,
                    SliderStep(props.step),
                    SliderPrecision(precision),
                    AutoDirectionalNavigation::default(),
                ),
            ),));
        });
}

fn spawn_tooltip_bubble(parent: &mut ChildSpawnerCommands, text: &str) -> Entity {
    parent
        .spawn((
            Node {
                max_width: Val::Px(TOOLTIP_MAX_WIDTH_PX),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::top(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(TOOLTIP_BG),
            Outline::new(Val::Px(1.0), Val::Px(0.0), PANEL_EDGE),
            Pickable::IGNORE,
            ThemedText,
            TooltipBubble,
        ))
        .with_children(|bubble| {
            bubble.spawn((
                bevy::ui::widget::Text::new(text),
                ThemedText,
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(TOOLTIP_TEXT),
            ));
        })
        .id()
}

fn spawn_space_weather_row<B: Bundle>(
    parent: &mut ChildSpawnerCommands,
    marker: B,
    label: &str,
    tooltip: &str,
) {
    let row_entity = parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(4.0),
                ..default()
            },
            RelativeCursorPosition::default(),
            Pickable::IGNORE,
            ThemedText,
        ))
        .id();

    parent.commands().entity(row_entity).with_children(|row| {
        row.spawn((marker, bevy::ui::widget::Text::new(label), ThemedText));
        let bubble = spawn_tooltip_bubble(row, tooltip);
        row.commands()
            .entity(row_entity)
            .insert(TooltipTarget { bubble });
    });
}

fn spawn_fixed_button<B: Bundle>(
    parent: &mut ChildSpawnerCommands,
    width_px: f32,
    props: ButtonProps,
    overrides: B,
    label: &str,
) {
    parent
        .spawn(Node {
            width: Val::Px(width_px),
            flex_grow: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|container| {
            container
                .spawn(button(
                    props,
                    overrides,
                    Spawn((
                        bevy::ui::widget::Text::new(label),
                        ThemedText,
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                    )),
                ))
                .insert(Outline::new(Val::Px(1.0), Val::Px(0.0), PANEL_EDGE));
        });
}

fn spawn_top_time_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                width: Val::Px(220.0),
                flex_grow: 0.0,
                ..default()
            },
            Pickable::IGNORE,
            ThemedText,
        ))
        .with_children(|left| {
            spawn_pill_chip(left, "UTC: --", LabelStyle::normal(15.0), TimeText);
        });
}

fn spawn_top_speed_row(parent: &mut ChildSpawnerCommands, time_scale: f32) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                width: Val::Px(240.0),
                flex_grow: 0.0,
                ..default()
            },
            Pickable::IGNORE,
            ThemedText,
        ))
        .with_children(|middle| {
            spawn_labeled_slider_row(
                middle,
                LabeledSliderRowProps {
                    label: "Speed",
                    label_style: LabelStyle::accent(12.0),
                    binding: SliderBinding::TimeScale,
                    min: 1.0,
                    max: 1000.0,
                    value: time_scale,
                    step: 1.0,
                },
            );
            spawn_fixed_button(
                middle,
                56.0,
                ButtonProps::default(),
                (
                    ButtonAction::TimeScale1x,
                    AutoDirectionalNavigation::default(),
                ),
                "1x",
            );
            spawn_fixed_button(
                middle,
                64.0,
                ButtonProps::default(),
                (ButtonAction::TimeNow, AutoDirectionalNavigation::default()),
                "Now",
            );
        });
}

fn spawn_top_panel_toggles_row(parent: &mut ChildSpawnerCommands) {
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
            spawn_fixed_button(
                right,
                64.0,
                ButtonProps::default(),
                (
                    PanelToggle {
                        kind: PanelToggleKind::Left,
                    },
                    AutoDirectionalNavigation::default(),
                ),
                "Vis",
            );
            spawn_fixed_button(
                right,
                64.0,
                ButtonProps::default(),
                (
                    PanelToggle {
                        kind: PanelToggleKind::Right,
                    },
                    AutoDirectionalNavigation::default(),
                ),
                "Sat",
            );
            spawn_fixed_button(
                right,
                64.0,
                ButtonProps::default(),
                (
                    PanelToggle {
                        kind: PanelToggleKind::Top,
                    },
                    AutoDirectionalNavigation::default(),
                ),
                "Time",
            );
            spawn_fixed_button(
                right,
                64.0,
                ButtonProps::default(),
                (
                    PanelToggle {
                        kind: PanelToggleKind::Bottom,
                    },
                    AutoDirectionalNavigation::default(),
                ),
                "Status",
            );
        });
}

fn spawn_hidden_panels_hint(parent: &mut ChildSpawnerCommands) -> Entity {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Auto,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::top(Val::Px(12.0)),
                display: Display::None,
                ..default()
            },
            Pickable::IGNORE,
            ThemedText,
            HiddenPanelsHint,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        margin: UiRect::left(Val::Px(HIDDEN_PANELS_HINT_OFFSET_X_PX)),
                        max_width: Val::Px(360.0),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL_INNER_BG),
                    Outline::new(Val::Px(1.0), Val::Px(0.0), PANEL_EDGE),
                    Pickable::IGNORE,
                    ThemedText,
                    HiddenPanelsHintCard,
                ))
                .with_children(|card| {
                    spawn_styled_text(card, "Panels hidden", LabelStyle::accent(15.0), ());
                    spawn_styled_text(
                        card,
                        "H=Vis, J=Sat, K=Time, L=Status",
                        LabelStyle::normal(12.0),
                        (),
                    );
                    spawn_styled_text(
                        card,
                        "V = Toggle viewport crop",
                        LabelStyle::normal(11.0),
                        (),
                    );
                });
        })
        .id()
}

fn spawn_section(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    initially_expanded: bool,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_INNER_BG),
            Pickable::IGNORE,
            ThemedText,
        ))
        .with_children(|section| {
            spawn_corner_brackets(section);
            let header_entity = section
                .spawn(button(
                    ButtonProps::default(),
                    AutoDirectionalNavigation::default(),
                    Spawn((
                        bevy::ui::widget::Text::new(title),
                        ThemedText,
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(PANEL_TEXT_ACCENT),
                        UiFontBold,
                    )),
                ))
                .id();

            let mut body_node = Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                ..default()
            };
            if !initially_expanded {
                body_node.display = Display::None;
            }

            let body = section
                .spawn((body_node, Pickable::IGNORE, ThemedText))
                .with_children(|body| {
                    body.spawn((
                        Node {
                            height: Val::Px(1.0),
                            width: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(PANEL_DIVIDER),
                        Pickable::IGNORE,
                    ));
                    build(body);
                })
                .id();

            section.commands().entity(header_entity).insert((
                SectionToggle { body },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(Val::Px(10.0), Val::Px(6.0), Val::Px(0.0), Val::Px(0.0)),
                    flex_grow: 0.0,
                    ..default()
                },
            ));
        });
}

fn spawn_edge_glow(parent: &mut ChildSpawnerCommands, edge: Edge) {
    let node = match edge {
        Edge::Top => Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Px(2.0),
            ..default()
        },
        Edge::Left => Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            width: Val::Px(2.0),
            ..default()
        },
        Edge::Right => Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            width: Val::Px(2.0),
            ..default()
        },
    };

    parent.spawn((node, BackgroundColor(PANEL_EDGE), Pickable::IGNORE));
}

fn spawn_grid_overlay(parent: &mut ChildSpawnerCommands) {
    for step in GRID_STEPS {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(step),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(GRID_LINE),
            Pickable::IGNORE,
        ));

        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(step),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(GRID_LINE),
            Pickable::IGNORE,
        ));
    }
}

fn spawn_corner_brackets(parent: &mut ChildSpawnerCommands) {
    let offset = 6.0;
    let length = 12.0;
    let thickness = 2.0;

    let corners = [
        (Val::Px(offset), Val::Px(offset), false, false), // top-left
        (Val::Px(offset), Val::Px(offset), true, false),  // top-right
        (Val::Px(offset), Val::Px(offset), false, true),  // bottom-left
        (Val::Px(offset), Val::Px(offset), true, true),   // bottom-right
    ];

    for (x_off, y_off, right, bottom) in corners {
        let horiz = Node {
            position_type: PositionType::Absolute,
            width: Val::Px(length),
            height: Val::Px(thickness),
            ..default()
        };
        let vert = Node {
            position_type: PositionType::Absolute,
            width: Val::Px(thickness),
            height: Val::Px(length),
            ..default()
        };

        let mut horiz_node = horiz;
        let mut vert_node = vert;

        if right {
            horiz_node.right = x_off;
            vert_node.right = x_off;
        } else {
            horiz_node.left = x_off;
            vert_node.left = x_off;
        }

        if bottom {
            horiz_node.bottom = y_off;
            vert_node.bottom = y_off;
        } else {
            horiz_node.top = y_off;
            vert_node.top = y_off;
        }

        parent.spawn((horiz_node, BackgroundColor(PANEL_EDGE), Pickable::IGNORE));
        parent.spawn((vert_node, BackgroundColor(PANEL_EDGE), Pickable::IGNORE));
    }
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
    ui_state: Res<UIState>,
    mut panels: Query<&mut Node>,
) {
    if !layout.is_changed() && !ui_state.is_changed() {
        return;
    }

    let top = if ui_state.show_top_panel {
        Val::Px(TOP_PANEL_HEIGHT_PX)
    } else {
        Val::Px(0.0)
    };
    let bottom = if ui_state.show_bottom_panel {
        Val::Px(BOTTOM_PANEL_HEIGHT_PX)
    } else {
        Val::Px(0.0)
    };

    if let Ok(mut node) = panels.get_mut(ui_entities.left_panel) {
        node.top = top;
        node.bottom = bottom;
    }

    if let Ok(mut node) = panels.get_mut(ui_entities.right_panel) {
        node.top = top;
        node.bottom = bottom;
        node.width = Val::Px(layout.right_panel_width_px);
    }
}

fn update_hidden_panels_hint(
    ui_state: Res<UIState>,
    ui_entities: Res<UiEntities>,
    mut hint_state: ResMut<HiddenPanelsHintState>,
    mut nodes: Query<&mut Node, With<HiddenPanelsHint>>,
) {
    let show_hint = !ui_state.show_top_panel;

    let Ok(mut node) = nodes.get_mut(ui_entities.hidden_panels_hint) else {
        return;
    };

    if show_hint {
        if !hint_state.visible {
            hint_state.visible = true;
            hint_state.last_shown = Some(Instant::now());
            node.display = Display::Flex;
        } else if let Some(shown_at) = hint_state.last_shown
            && shown_at.elapsed() > HIDDEN_PANELS_HINT_DURATION
        {
            hint_state.visible = false;
            node.display = Display::None;
        }
    } else if hint_state.visible || hint_state.last_shown.is_some() {
        hint_state.visible = false;
        hint_state.last_shown = None;
        node.display = Display::None;
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

    if state.show_left_panel
        && let Ok(pos) = panels.get(ui_entities.left_panel)
    {
        hovered |= pos.cursor_over;
    }
    if state.show_right_panel
        && let Ok(pos) = panels.get(ui_entities.right_panel)
    {
        hovered |= pos.cursor_over;
    }
    if state.show_top_panel
        && let Ok(pos) = panels.get(ui_entities.top_panel)
    {
        hovered |= pos.cursor_over;
    }
    if state.show_bottom_panel
        && let Ok(pos) = panels.get(ui_entities.bottom_panel)
    {
        hovered |= pos.cursor_over;
    }

    for mut pan_orbit in cameras.iter_mut() {
        pan_orbit.enabled = !hovered;
    }
}

fn update_tooltip_visibility(
    mut bubbles: Query<&mut Node, With<TooltipBubble>>,
    targets: Query<(&RelativeCursorPosition, &TooltipTarget)>,
) {
    for (cursor, target) in targets.iter() {
        let Ok(mut node) = bubbles.get_mut(target.bubble) else {
            continue;
        };
        node.display = if cursor.cursor_over {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn update_time_display(
    mut texts: ParamSet<(Query<&mut bevy::ui::widget::Text, With<TimeText>>,)>,
    sim_time: Res<crate::orbital::SimulationTime>,
) {
    for mut text in texts.p0().iter_mut() {
        text.0 = format!("UTC: {}", sim_time.current_utc.format("%Y-%m-%d %H:%M:%S"));
    }
}

#[allow(clippy::type_complexity)]
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

fn update_space_weather_texts(
    kp: Res<KpIndex>,
    solar_wind: Res<SolarWind>,
    aurora: Res<AuroraGrid>,
    state: Res<SpaceWeatherState>,
    sim_time: Res<SimulationTime>,
    mut texts: ParamSet<(
        Query<&mut bevy::ui::widget::Text, With<SpaceWeatherKpText>>,
        Query<&mut bevy::ui::widget::Text, With<SpaceWeatherMagText>>,
        Query<&mut bevy::ui::widget::Text, With<SpaceWeatherPlasmaText>>,
        Query<&mut bevy::ui::widget::Text, With<SpaceWeatherUpdatedText>>,
        Query<&mut bevy::ui::widget::Text, With<SpaceWeatherErrorText>>,
        Query<(&mut bevy::ui::widget::Text, &mut TextColor), With<AuroraStatusText>>,
    )>,
) {
    if !kp.is_changed()
        && !solar_wind.is_changed()
        && !aurora.is_changed()
        && !state.is_changed()
        && !sim_time.is_changed()
    {
        return;
    }

    for mut text in texts.p0().iter_mut() {
        text.0 = match (kp.value, kp.timestamp) {
            (Some(value), timestamp) => {
                let time = format_time(timestamp);
                format!("Kp: {:.1} ({})", value, time)
            }
            _ => "Kp: --".to_string(),
        };
    }

    for mut text in texts.p1().iter_mut() {
        let bz = solar_wind
            .bz
            .map(|v| format!("{:+.1}", v))
            .unwrap_or_else(|| "--".to_string());
        let bt = solar_wind
            .bt
            .map(|v| format!("{:.1}", v))
            .unwrap_or_else(|| "--".to_string());
        text.0 = format!("Bz: {} nT  Bt: {} nT", bz, bt);
    }

    for mut text in texts.p2().iter_mut() {
        let speed = solar_wind
            .speed
            .map(|v| format!("{:.0}", v))
            .unwrap_or_else(|| "--".to_string());
        let density = solar_wind
            .density
            .map(|v| format!("{:.1}", v))
            .unwrap_or_else(|| "--".to_string());
        text.0 = format!("Vsw: {} km/s  n: {} cm^-3", speed, density);
    }

    for mut text in texts.p3().iter_mut() {
        let updated = latest_time([kp.timestamp, solar_wind.timestamp, aurora.updated_utc]);
        text.0 = format!("Updated: {}", format_time(updated));
    }

    for mut text in texts.p4().iter_mut() {
        let err = state
            .ovation_error
            .as_deref()
            .or(state.kp_error.as_deref())
            .or(state.mag_error.as_deref())
            .or(state.plasma_error.as_deref());
        text.0 = err
            .map(|e| format!("Data error: {}", e))
            .unwrap_or_default();
    }

    for (mut text, mut color) in texts.p5().iter_mut() {
        if let Some(forecast_time) = aurora.updated_utc {
            let age = sim_time.current_utc.signed_duration_since(forecast_time);
            let age_mins = age.num_minutes();

            if age_mins < 0 {
                // Simulation time is behind forecast time
                text.0 = format!("Forecast: {} min ahead", -age_mins);
                color.0 = Color::srgba(0.5, 0.8, 1.0, 0.85);
            } else if age_mins > 60 {
                text.0 = "⚠ Forecast expired".to_string();
                color.0 = Color::srgba(1.0, 0.6, 0.0, 0.95);
            } else if age_mins > 45 {
                text.0 = "⚠ Forecast expiring soon".to_string();
                color.0 = Color::srgba(1.0, 0.9, 0.3, 0.9);
            } else {
                text.0 = format!("Forecast age: {} min", age_mins);
                color.0 = Color::srgba(0.6, 0.9, 0.6, 0.85);
            }
        } else {
            text.0.clear();
        }
    }
}

fn format_time(timestamp: Option<DateTime<Utc>>) -> String {
    timestamp
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "--".to_string())
}

fn latest_time(times: [Option<DateTime<Utc>>; 3]) -> Option<DateTime<Utc>> {
    times.into_iter().flatten().max()
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

#[allow(clippy::type_complexity)]
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

fn sync_panel_toggle_buttons(
    ui_state: Res<UIState>,
    mut buttons: Query<(&PanelToggle, &mut ButtonVariant)>,
) {
    for (toggle, mut variant) in buttons.iter_mut() {
        let is_on = match toggle.kind {
            PanelToggleKind::Left => ui_state.show_left_panel,
            PanelToggleKind::Right => ui_state.show_right_panel,
            PanelToggleKind::Top => ui_state.show_top_panel,
            PanelToggleKind::Bottom => ui_state.show_bottom_panel,
        };
        let target = if is_on {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        };
        if *variant != target {
            *variant = target;
        }
    }
}

fn process_pending_add(
    mut right_ui: ResMut<RightPanelUI>,
    mut store: ResMut<SatelliteStore>,
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

    // Insert entry; spawn_missing_satellite_entities_system will create the entity
    store.items.insert(
        norad,
        crate::satellite::SatEntry {
            name: None,
            color,
            entity: None,
            tle: None,
            propagator: None,
            error: None,
            show_ground_track: false,
            show_trail: false,
            is_clicked: false,
        },
    );
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

fn sync_widget_states(mut params: SyncWidgetStateParams<'_, '_>) {
    if params.ui_state.is_changed()
        || params.arrows.is_changed()
        || params.config_bundle.is_changed()
        || params.heatmap_cfg.is_changed()
        || params.space_weather_cfg.is_changed()
        || params.store.is_changed()
        || params.selected.is_changed()
        || params.sim_time.is_changed()
        || params.right_ui.is_changed()
    {
        for (entity, binding, checked) in params.checkboxes.iter_mut() {
            let should_check = match binding {
                CheckboxBinding::ShowAxes => params.ui_state.show_axes,
                CheckboxBinding::ShowArrows => params.arrows.enabled,
                CheckboxBinding::ArrowGradient => params.arrows.gradient_enabled,
                CheckboxBinding::ArrowGradientLog => params.arrows.gradient_log_scale,
                CheckboxBinding::GroundTracksEnabled => {
                    params.config_bundle.ground_track_cfg.enabled
                }
                CheckboxBinding::GizmoEnabled => params.config_bundle.gizmo_cfg.enabled,
                CheckboxBinding::GizmoShowCenterDot => {
                    params.config_bundle.gizmo_cfg.show_center_dot
                }
                CheckboxBinding::TrailsAll => {
                    !params.store.items.is_empty()
                        && params
                            .store
                            .items
                            .values()
                            .filter(|s| s.propagator.is_some())
                            .all(|s| s.show_trail)
                }
                CheckboxBinding::TracksAll => {
                    !params.store.items.is_empty()
                        && params
                            .store
                            .items
                            .values()
                            .filter(|s| s.propagator.is_some())
                            .all(|s| s.show_ground_track)
                }
                CheckboxBinding::HeatmapEnabled => params.heatmap_cfg.enabled,
                CheckboxBinding::AuroraOverlay => params.space_weather_cfg.aurora_enabled,
            };

            match (should_check, checked.is_some()) {
                (true, false) => {
                    queue_set_checked(&mut params.commands, entity, true);
                }
                (false, true) => {
                    queue_set_checked(&mut params.commands, entity, false);
                }
                _ => {}
            }
        }

        for (entity, binding, checked) in params.range_modes.iter_mut() {
            let should_check = match binding {
                RangeModeBinding::Auto => params.heatmap_cfg.range_mode == RangeMode::Auto,
                RangeModeBinding::Fixed => params.heatmap_cfg.range_mode == RangeMode::Fixed,
            };
            match (should_check, checked.is_some()) {
                (true, false) => {
                    queue_set_checked(&mut params.commands, entity, true);
                }
                (false, true) => {
                    queue_set_checked(&mut params.commands, entity, false);
                }
                _ => {}
            }
        }

        if let Some(selected_group) = params.right_ui.selected_group.as_deref() {
            for (entity, choice, checked) in params.group_choices.iter_mut() {
                let should_check = choice.0 == selected_group;
                match (should_check, checked.is_some()) {
                    (true, false) => {
                        queue_set_checked(&mut params.commands, entity, true);
                    }
                    (false, true) => {
                        queue_set_checked(&mut params.commands, entity, false);
                    }
                    _ => {}
                }
            }
        }

        for (entity, binding) in params.sliders.iter() {
            let value = match binding {
                SliderBinding::GradientNear => params.arrows.gradient_near_km,
                SliderBinding::GradientFar => params.arrows.gradient_far_km,
                SliderBinding::GroundTrackRadius => params.config_bundle.ground_track_cfg.radius_km,
                SliderBinding::GizmoSegments => {
                    params.config_bundle.gizmo_cfg.circle_segments as f32
                }
                SliderBinding::GizmoCenterDotSize => params.config_bundle.gizmo_cfg.center_dot_size,
                SliderBinding::TrailMaxPoints => params.config_bundle.trail_cfg.max_points as f32,
                SliderBinding::TrailUpdateInterval => {
                    params.config_bundle.trail_cfg.update_interval_seconds
                }
                SliderBinding::HeatmapUpdatePeriod => params.heatmap_cfg.update_period_s,
                SliderBinding::HeatmapOpacity => params.heatmap_cfg.color_alpha,
                SliderBinding::HeatmapFixedMax => {
                    params.heatmap_cfg.fixed_max.unwrap_or(20) as f32
                }
                SliderBinding::HeatmapChunkSize => params.heatmap_cfg.chunk_size as f32,
                SliderBinding::HeatmapChunksPerFrame => params.heatmap_cfg.chunks_per_frame as f32,
                SliderBinding::AuroraIntensity => params.space_weather_cfg.aurora_intensity_scale,
                SliderBinding::AuroraAlpha => params.space_weather_cfg.aurora_alpha,
                SliderBinding::AuroraLongitudeOffset => {
                    params.space_weather_cfg.aurora_longitude_offset
                }
                SliderBinding::SatelliteSphereRadius => params.config_bundle.render_cfg.sphere_radius,
                SliderBinding::SatelliteEmissiveIntensity => {
                    params.config_bundle.render_cfg.emissive_intensity
                }
                SliderBinding::TrackingDistance => params.selected.tracking_offset,
                SliderBinding::TrackingSmoothness => params.selected.smooth_factor,
                SliderBinding::TimeScale => params.sim_time.time_scale,
            };
            if let Ok(current) = params.slider_values.get(entity) {
                if (current.0 - value).abs() > f32::EPSILON {
                    params.commands.entity(entity).insert(SliderValue(value));
                }
            } else {
                params.commands.entity(entity).insert(SliderValue(value));
            }
        }

        for (entity, toggle, checked) in params.satellite_toggles.iter_mut() {
            if let Some(entry) = params.store.items.get(&toggle.norad) {
                let should_check = match toggle.kind {
                    SatelliteToggleKind::GroundTrack => entry.show_ground_track,
                    SatelliteToggleKind::Trail => entry.show_trail,
                };
                match (should_check, checked.is_some()) {
                    (true, false) => {
                        queue_set_checked(&mut params.commands, entity, true);
                    }
                    (false, true) => {
                        queue_set_checked(&mut params.commands, entity, false);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn sync_slider_visuals(
    mut sliders: SliderVisualQuery<'_, '_>,
    children: Query<&Children>,
    mut texts: Query<&mut bevy::ui::widget::Text>,
) {
    for (entity, value, range, precision, gradient) in sliders.iter_mut() {
        if let Some(mut gradient) = gradient
            && let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..]
        {
            let percent_value = (range.thumb_position(value.0) * 100.0).clamp(0.0, 100.0);
            linear_gradient.stops[1].point = Val::Percent(percent_value);
            linear_gradient.stops[2].point = Val::Percent(percent_value);
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

fn handle_button_activate(ev: On<Activate>, mut params: ButtonActivateParams<'_, '_>) {
    if let Ok(action) = params.q_action.get(ev.entity) {
        match action {
            ButtonAction::LoadGroup => {
                if params.right_ui.group_loading {
                    return;
                }
                if let Some(group) = &params.right_ui.selected_group {
                    if let Some(fetch) = &params.fetch_channels {
                        if let Err(e) = fetch.cmd_tx.send(FetchCommand::FetchGroup {
                            group: group.clone(),
                        }) {
                            params.right_ui.error =
                                Some(format!("Failed to request group: {}", e));
                            params.right_ui.group_loading = false;
                        } else {
                            params.right_ui.group_loading = true;
                            params.right_ui.error = None;
                        }
                    } else {
                        params.right_ui.error =
                            Some("Fetch service not available".to_string());
                    }
                } else {
                    params.right_ui.error = Some("Please select a group first".to_string());
                }
            }
            ButtonAction::ClearAll => {
                for entry in params.store.items.values_mut() {
                    if let Some(entity) = entry.entity.take() {
                        params.commands.entity(entity).despawn_children();
                        params.commands.entity(entity).despawn();
                    }
                }
                params.store.items.clear();
                params.right_ui.error = None;
                params.selected.tracking = None;
            }
            ButtonAction::AddSatellite => {
                params.right_ui.pending_add = true;
            }
            ButtonAction::StopTracking => {
                params.selected.tracking = None;
            }
            ButtonAction::TimeScale1x => {
                params.sim_time.time_scale = 1.0;
            }
            ButtonAction::TimeNow => {
                params.sim_time.current_utc = chrono::Utc::now();
                params.sim_time.time_scale = 1.0;
            }
        }
    }

    if let Ok(action) = params.q_sat_action.get(ev.entity) {
        match action.action {
            SatelliteAction::Track => {
                if params.selected.tracking == Some(action.norad) {
                    params.selected.tracking = None;
                } else {
                    params.selected.selected = Some(action.norad);
                    params.selected.tracking = Some(action.norad);
                }
            }
            SatelliteAction::Remove => {
                if let Some(entry) = params.store.items.remove(&action.norad)
                    && let Some(entity) = entry.entity
                {
                    params.commands.entity(entity).despawn_children();
                    params.commands.entity(entity).despawn();
                }
                if params.selected.tracking == Some(action.norad) {
                    params.selected.tracking = None;
                }
                if params.selected.selected == Some(action.norad) {
                    params.selected.selected = None;
                }
            }
        }
    }

    if let Ok(toggle) = params.q_panel_toggle.get(ev.entity) {
        match toggle.kind {
            PanelToggleKind::Left => {
                params.ui_state.show_left_panel = !params.ui_state.show_left_panel;
            }
            PanelToggleKind::Right => {
                params.ui_state.show_right_panel = !params.ui_state.show_right_panel;
            }
            PanelToggleKind::Top => {
                params.ui_state.show_top_panel = !params.ui_state.show_top_panel;
            }
            PanelToggleKind::Bottom => {
                params.ui_state.show_bottom_panel = !params.ui_state.show_bottom_panel;
            }
        }
    }
}

fn handle_section_toggle(
    ev: On<Activate>,
    q_toggle: Query<&SectionToggle>,
    mut nodes: Query<&mut Node>,
) {
    let Ok(toggle) = q_toggle.get(ev.entity) else {
        return;
    };

    if let Ok(mut node) = nodes.get_mut(toggle.body) {
        node.display = match node.display {
            Display::None => Display::Flex,
            _ => Display::None,
        };
    }
}

fn handle_checkbox_change(ev: On<ValueChange<bool>>, mut params: CheckboxChangeParams<'_, '_>) {
    if let Ok(binding) = params.q_binding.get(ev.source) {
        match binding {
            CheckboxBinding::ShowAxes => params.ui_state.show_axes = ev.value,
            CheckboxBinding::ShowArrows => params.arrows.enabled = ev.value,
            CheckboxBinding::ArrowGradient => params.arrows.gradient_enabled = ev.value,
            CheckboxBinding::ArrowGradientLog => params.arrows.gradient_log_scale = ev.value,
            CheckboxBinding::GroundTracksEnabled => {
                params.config_bundle.ground_track_cfg.enabled = ev.value
            }
            CheckboxBinding::GizmoEnabled => params.config_bundle.gizmo_cfg.enabled = ev.value,
            CheckboxBinding::GizmoShowCenterDot => {
                params.config_bundle.gizmo_cfg.show_center_dot = ev.value
            }
            CheckboxBinding::TrailsAll => {
                for entry in params.store.items.values_mut() {
                    if entry.propagator.is_some() {
                        entry.show_trail = ev.value;
                    }
                }
            }
            CheckboxBinding::TracksAll => {
                for entry in params.store.items.values_mut() {
                    if entry.propagator.is_some() {
                        entry.show_ground_track = ev.value;
                    }
                }
            }
            CheckboxBinding::HeatmapEnabled => params.heatmap_cfg.enabled = ev.value,
            CheckboxBinding::AuroraOverlay => params.space_weather_cfg.aurora_enabled = ev.value,
        }
        return;
    }

    if let Ok(toggle) = params.q_sat_toggle.get(ev.source)
        && let Some(entry) = params.store.items.get_mut(&toggle.norad)
    {
        match toggle.kind {
            SatelliteToggleKind::GroundTrack => entry.show_ground_track = ev.value,
            SatelliteToggleKind::Trail => entry.show_trail = ev.value,
        }
    }
}

fn handle_slider_change(
    ev: On<ValueChange<f32>>,
    q_binding: Query<&SliderBinding>,
    mut arrows: ResMut<ArrowConfig>,
    mut config_bundle: ResMut<UiConfigBundle>,
    mut heatmap_cfg: ResMut<HeatmapConfig>,
    mut space_weather_cfg: ResMut<SpaceWeatherConfig>,
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
        SliderBinding::AuroraIntensity => {
            space_weather_cfg.aurora_intensity_scale = ev.value;
        }
        SliderBinding::AuroraAlpha => {
            space_weather_cfg.aurora_alpha = ev.value;
        }
        SliderBinding::AuroraLongitudeOffset => {
            space_weather_cfg.aurora_longitude_offset = ev.value;
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
