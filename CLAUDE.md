# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BevyEarth is a Rust application using the Bevy game engine to create an interactive 3D Earth model with advanced satellite tracking and visualization capabilities. It features real-time satellite tracking, orbital propagation, ground track visualization, satellite visibility heatmaps, and city-to-satellite connectivity visualization using TLE (Two-Line Element) data from external sources like Celestrak.

## Common Commands

### Building and Running
- `cargo run` - Build and run the application in development mode
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release version
- `cargo check` - Quick syntax and type checking without building

### Development
- `cargo test` - Run tests (if any exist)
- `cargo clippy` - Run linting
- `cargo fmt` - Format code

## Architecture Overview

The application is built using a plugin-based architecture with Bevy's Entity Component System (ECS):

### Core Plugins Structure
The main application (`src/main.rs`) orchestrates multiple specialized plugins:

- **EarthPlugin** (`src/visualization/earth.rs`) - Generates the 3D Earth model using cube-to-sphere mapping with texture materials
- **SatellitePlugin** (`src/satellite/`) - Manages satellite entities, propagation, orbit trails, and rendering
- **TlePlugin** (`src/tle/`) - Handles TLE data fetching and parsing from external sources
- **OrbitalPlugin** (`src/orbital/`) - Orbital mechanics calculations, coordinate transformations, and simulation time
- **UiPlugin** (`src/ui/`) - egui-based user interface with grouped satellite management
- **VisualizationPlugin** (`src/visualization/`) - Handles visual elements like axes, arrows, and gizmos
- **CitiesPlugin** (`src/visualization/cities.rs`) - City visualization and city-to-satellite connectivity arrows
- **GroundTrackPlugin** (`src/visualization/ground_track.rs`) - Satellite ground track visualization
- **GroundTrackGizmoPlugin** (`src/visualization/ground_track_gizmo.rs`) - Gizmo-based ground track rendering
- **HeatmapPlugin** (`src/visualization/heatmap.rs`) - Real-time satellite visibility heatmap overlay
- **SkyboxPlugin** (`src/ui/skybox.rs`) - Space background and skybox management

### Key Data Flow
1. **TLE Fetching**: Async worker fetches TLE data from Celestrak and other sources
2. **Satellite Propagation**: SGP4 algorithm propagates satellite positions in real-time
3. **Coordinate Conversion**: ECI coordinates converted to ECEF for Earth-relative positioning
4. **Rendering**: Satellites rendered as entities with position updates each frame

### Module Organization
- `src/core/` - Core foundational types and utilities
  - `coordinates.rs` - Coordinate system utilities and transformations
- `src/satellite/` - Satellite components, resources, and systems
  - `resources.rs` - SatelliteStore (HashMap-based), rendering configs, and orbit trail management
  - `components.rs` - Satellite and SatelliteColor components
  - `systems.rs` - Propagation, entity spawning, orbit trails, and satellite interaction systems
- `src/orbital/` - Orbital mechanics utilities
  - `coordinates.rs` - ECI to ECEF transformations
  - `propagation.rs` - SGP4 integration
  - `time.rs` - Simulation time management
- `src/tle/` - TLE data management
  - `fetcher.rs` - Async HTTP client for TLE downloads
  - `parser.rs` - TLE format parsing
  - `types.rs` - Data structures for TLE handling
  - `systems.rs` - TLE processing systems
  - `mock_data.rs` - Mock data for development
- `src/ui/` - User interface components using egui
  - `panels.rs` - Left, top, and right panel implementations
  - `groups.rs` - Satellite grouping and management
  - `state.rs` - UI state management
  - `systems.rs` - UI system implementations and configuration bundles
  - `skybox.rs` - Skybox management UI and systems
- `src/visualization/` - Advanced visualization systems
  - `earth.rs` - Earth mesh generation and rendering
  - `cities.rs` - City visualization and markers
  - `arrows.rs` - City-to-satellite connectivity arrows
  - `axes.rs` - Coordinate axes visualization
  - `ground_track.rs` - Satellite ground track visualization
  - `ground_track_gizmo.rs` - Alternative gizmo-based ground tracks
  - `heatmap.rs` - Real-time satellite visibility heatmap with chunked updates
  - `colormaps.rs` - Color mapping utilities for visualizations
  - `config.rs` - Configuration structures for visual elements

### Key Resources
- **SatelliteStore**: HashMap-based storage for satellite data with O(1) access
- **SatWorldKm**: Satellite positions in world coordinates
- **SimulationTime**: Manages simulation clock and time acceleration
- **SelectedSatellite**: Tracks currently selected satellite for camera tracking
- **OrbitTrailConfig**: Configuration for satellite orbit trail visualization
- **SatelliteRenderConfig**: Rendering settings for satellite entities
- **HeatmapConfig**: Configuration for satellite visibility heatmap with performance tuning
- **GroundTrackConfig**: Settings for ground track visualization
- **ArrowConfig**: Configuration for city-to-satellite connectivity arrows
- **UIState**: Central UI state management
- **RightPanelUI**: Right panel specific state
- **UiConfigBundle**: Bundled configuration resources for UI systems

### Dependencies
- **Bevy 0.16.1** with dynamic linking and mesh picking for faster development builds
- **bevy_egui 0.35.1** for immediate mode GUI with extended features
- **bevy_panorbit_camera 0.27.1** for 3D camera controls  
- **sgp4 2.3.0** for satellite orbit propagation
- **reqwest 0.12** with rustls-tls for HTTP TLE fetching
- **chrono 0.4** for time handling
- **tokio 1.0** with rt-multi-thread for async runtime
- **glam 0.27** for mathematical operations
- **serde 1.0** with derive features for serialization
- **egui_extras 0.31** for additional UI widgets
- **thiserror** and **anyhow** for error handling

## Development Notes

### Performance Optimizations
- Development profile uses opt-level 1 for faster incremental builds
- Dependencies use opt-level 3 for runtime performance
- SatelliteStore uses HashMap for O(1) satellite lookups by NORAD ID
- Heatmap uses chunked vertex processing (configurable chunk size and chunks per frame)
- Orbit trails use efficient position history buffers
- Hemisphere pre-filtering for line-of-sight calculations to reduce computation
- Bevy's ECS system ordering optimizes dependent calculations

### Earth Rendering
The Earth is rendered using a cube-to-sphere mapping technique that generates 6 cube faces with 4 subdivisions each, creating a detailed sphere mesh with proper UV mapping for textures.

### Key Features
- **Real-time Satellite Tracking**: Live satellite position updates using SGP4 propagation
- **Interactive 3D Earth**: High-resolution textured Earth model with city markers
- **Satellite Groups**: Organized satellite management with predefined groups (Starlink, GPS, etc.)
- **Ground Track Visualization**: Historical and future satellite ground tracks
- **Visibility Heatmap**: Real-time satellite visibility overlay with color-coded intensity
- **Orbit Trails**: Configurable satellite orbit trail visualization
- **City Connectivity**: Visual arrows showing city-to-satellite line-of-sight connections
- **Camera Tracking**: Automatic camera following of selected satellites
- **Time Control**: Simulation time acceleration and control
- **Space Environment**: Realistic skybox and bloom effects

### Coordinate Systems
- **ECI** (Earth-Centered Inertial) - Used for orbital calculations and SGP4 propagation
- **ECEF** (Earth-Centered Earth-Fixed) - Used for Earth-relative positioning and rendering
- **Geographic** (Lat/Lon) - Used for surface features, cities, and user interface
- **Bevy World** - Final rendering coordinates with proper scaling