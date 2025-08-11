# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BevyEarth is a Rust application using the Bevy game engine to create an interactive 3D Earth model with satellite tracking capabilities. It visualizes satellites in real-time using TLE (Two-Line Element) data from external sources like Celestrak.

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

- **EarthPlugin** (`src/earth.rs`) - Generates the 3D Earth model using cube-to-sphere mapping with texture materials
- **SatellitePlugin** (`src/satellite/`) - Manages satellite entities, propagation, and rendering
- **TlePlugin** (`src/tle/`) - Handles TLE data fetching and parsing from external sources
- **OrbitalPlugin** (`src/orbital/`) - Orbital mechanics calculations and coordinate transformations
- **UiPlugin** (`src/ui/`) - egui-based user interface for satellite management
- **VisualizationPlugin** - Handles visual elements like axes and gizmos
- **CoveragePlugin** - Satellite coverage area calculations
- **CitiesPlugin** - City visualization on Earth

### Key Data Flow
1. **TLE Fetching**: Async worker fetches TLE data from Celestrak and other sources
2. **Satellite Propagation**: SGP4 algorithm propagates satellite positions in real-time
3. **Coordinate Conversion**: ECI coordinates converted to ECEF for Earth-relative positioning
4. **Rendering**: Satellites rendered as entities with position updates each frame

### Module Organization
- `src/satellite/` - Satellite components, resources, and systems
  - `resources.rs` - SatelliteStore (HashMap-based) for efficient satellite data management
  - `components.rs` - Satellite and SatelliteColor components
  - `systems.rs` - Propagation and entity spawning systems
- `src/orbital/` - Orbital mechanics utilities
  - `coordinates.rs` - ECI to ECEF transformations
  - `propagation.rs` - SGP4 integration
  - `time.rs` - Simulation time management
- `src/tle/` - TLE data management
  - `fetcher.rs` - Async HTTP client for TLE downloads
  - `parser.rs` - TLE format parsing
  - `types.rs` - Data structures for TLE handling
- `src/ui/` - User interface components using egui

### Key Resources
- **SatelliteStore**: HashMap-based storage for satellite data (changed from Vec for O(1) access)
- **SimulationTime**: Manages simulation clock and time acceleration
- **FetchChannels**: Communication between TLE worker and main thread

### Dependencies
- **Bevy 0.16.1** with dynamic linking for faster development builds
- **bevy_egui** for immediate mode GUI
- **bevy_panorbit_camera** for 3D camera controls  
- **sgp4 2.3.0** for satellite orbit propagation
- **reqwest** with rustls-tls for HTTP TLE fetching
- **chrono** for time handling

## Development Notes

### Performance Optimizations
- Development profile uses opt-level 1 for faster incremental builds
- Dependencies use opt-level 3 for runtime performance
- SatelliteStore uses HashMap instead of Vec for efficient satellite lookups

### Earth Rendering
The Earth is rendered using a cube-to-sphere mapping technique that generates 6 cube faces with 4 subdivisions each, creating a detailed sphere mesh with proper UV mapping for textures.

### Coordinate Systems
- **ECI** (Earth-Centered Inertial) - Used for orbital calculations
- **ECEF** (Earth-Centered Earth-Fixed) - Used for Earth-relative positioning
- **Geographic** (Lat/Lon) - Used for surface features and user interface