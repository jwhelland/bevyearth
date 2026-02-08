# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BevyEarth is a Rust application using the Bevy game engine to create an interactive 3D Earth model with real-time satellite tracking using TLE (Two-Line Element) data from Celestrak. Features include SGP4 orbital propagation, ground track visualization, satellite visibility heatmaps, and city-to-satellite connectivity.

## Common Commands

### Building and Running
- `cargo run` - Build and run in development mode
- `cargo build --release` - Build optimized release version
- `cargo check` - Quick syntax and type checking

### Development
- `cargo clippy` - Run linting
- `cargo fmt` - Format code

### Feature Flags
- `cargo run --features dev` - Enable FPS overlay and dev tools
- `cargo run --features dev_camera` - Add free-fly camera (toggle with F2)
- `cargo run --features debug_basic_scene` - Minimal test scene
- `cargo run --features debug_scene_camera` - Debug scene with controls

**Note**: `debug_basic_scene` and `debug_scene_camera` are mutually exclusive.

## Architecture

### Plugin-Based ECS Structure
The application uses Bevy's Entity Component System with specialized plugins:

- **EarthPlugin** - 3D Earth model using cube-to-sphere mapping
- **SatellitePlugin** - Satellite entities, SGP4 propagation, orbit trails
- **TlePlugin** - Async TLE data fetching and parsing
- **OrbitalPlugin** - Orbital mechanics, coordinate transformations, simulation time
- **UiPlugin** - bevy_feathers-based interface with grouped satellite management
- **VisualizationPlugin** - Axes, arrows, gizmos
- **CitiesPlugin** - City markers and connectivity visualization
- **GroundTrackPlugin** / **GroundTrackGizmoPlugin** - Ground track rendering
- **HeatmapPlugin** - Real-time satellite visibility heatmap
- **SkyboxPlugin** - Space background management
- **SpaceWeatherPlugin** - Space weather data integration

### Key Data Flow
1. Async worker fetches TLE data from Celestrak
2. SGP4 algorithm propagates satellite positions in real-time
3. ECI coordinates converted to ECEF for Earth-relative positioning
4. Satellites rendered as entities with per-frame position updates

### Key Resources
- **SatelliteStore** - HashMap-based satellite data storage (O(1) NORAD ID lookup)
- **SimulationTime** - Simulation clock and time acceleration
- **SelectedSatellite** - Currently tracked satellite for camera following
- **OrbitTrailConfig** / **SatelliteRenderConfig** / **HeatmapConfig** / **GroundTrackConfig** - Visualization configs
- **UIState** / **RightPanelUI** - UI state management
- **UiConfigBundle** - Bundled UI configuration resources

### Coordinate Systems
- **ECI** (Earth-Centered Inertial) - SGP4 propagation and orbital calculations
- **ECEF** (Earth-Centered Earth-Fixed) - Earth-relative positioning and rendering
- **Geographic** (Lat/Lon) - Surface features, cities, user interface
- **Bevy World** - Final rendering coordinates with scaling

### Major Dependencies
- **Bevy 0.18.0** - Game engine with experimental feathers UI
- **bevy_feathers 0.18.0** - Modern UI widgets with dark theme
- **bevy_panorbit_camera 0.34.0** - 3D camera controls
- **sgp4 2.3.0** - Satellite orbit propagation
- **reqwest 0.12** - HTTP TLE fetching with rustls-tls
- **chrono 0.4** / **tokio 1.0** - Time handling and async runtime

## Performance Notes

- Development profile: opt-level 1 (incremental builds), dependencies opt-level 3 (runtime performance)
- SatelliteStore uses HashMap for O(1) lookups by NORAD ID
- Heatmap uses chunked vertex processing (configurable chunks per frame)
- Hemisphere pre-filtering for line-of-sight calculations
- Bevy ECS system ordering optimizes dependent calculations
