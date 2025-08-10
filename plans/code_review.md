# Code Review: src/main.rs

## Executive Summary

The [`src/main.rs`](src/main.rs:1) file is a monolithic 1030-line file that handles multiple concerns including satellite tracking, UI rendering, TLE fetching, orbital propagation, and visualization. This review focuses on architectural improvements and code organization to break down this monolith into smaller, focused modules.

## Critical Architectural Issues

### 1. Monolithic Structure
**Issue**: [`main.rs`](src/main.rs:1) contains 1030 lines mixing multiple responsibilities:
- Satellite data management (lines 52-115)
- TLE fetching and parsing (lines 241-333)
- UI rendering (lines 673-929)
- Orbital mechanics (lines 147-165)
- Arrow rendering (lines 349-431)
- System scheduling (lines 997-1029)

**Impact**: High coupling, difficult maintenance, poor testability

### 2. Mixed Abstraction Levels
**Issue**: Low-level orbital mechanics mixed with high-level UI code
- [`parse_tle_epoch_to_utc()`](src/main.rs:116) (TLE parsing) next to [`ui_example_system()`](src/main.rs:674) (UI rendering)
- Mathematical functions like [`gmst_rad()`](src/main.rs:153) alongside UI state management

## Recommended Module Structure

### Core Modules to Extract

#### 1. `src/satellite/mod.rs` - Satellite Management
Extract satellite-related code:
- [`SatelliteStore`](src/main.rs:52) resource
- [`SatEntry`](src/main.rs:58) struct
- [`Satellite`](src/main.rs:46) and [`SatelliteColor`](src/main.rs:49) components
- [`propagate_satellites_system()`](src/main.rs:456)
- [`update_satellite_ecef()`](src/main.rs:340)

```rust
// src/satellite/mod.rs
pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Satellite, SatelliteColor};
pub use resources::{SatelliteStore, SatEntry};
pub use systems::{propagate_satellites_system, update_satellite_ecef};
```

#### 2. `src/tle/mod.rs` - TLE Data Management
Extract TLE-related functionality:
- [`TleData`](src/main.rs:109) struct
- [`parse_tle_epoch_to_utc()`](src/main.rs:116)
- [`FetchChannels`](src/main.rs:102), [`FetchCommand`](src/main.rs:87), [`FetchResultMsg`](src/main.rs:91)
- [`start_tle_worker()`](src/main.rs:242)
- [`process_fetch_results_system()`](src/main.rs:932)

```rust
// src/tle/mod.rs
pub mod fetcher;
pub mod parser;
pub mod types;

pub use types::{TleData, FetchCommand, FetchResultMsg};
pub use fetcher::{start_tle_worker, FetchChannels};
pub use parser::parse_tle_epoch_to_utc;
```

#### 3. `src/orbital/mod.rs` - Orbital Mechanics
Extract orbital calculations:
- [`minutes_since_epoch()`](src/main.rs:147)
- [`gmst_rad()`](src/main.rs:153)
- [`eci_to_ecef_km()`](src/main.rs:160)
- [`SimulationTime`](src/main.rs:227)
- [`advance_simulation_clock()`](src/main.rs:443)

```rust
// src/orbital/mod.rs
pub mod time;
pub mod coordinates;
pub mod propagation;

pub use time::{SimulationTime, advance_simulation_clock};
pub use coordinates::{eci_to_ecef_km, gmst_rad};
pub use propagation::minutes_since_epoch;
```

#### 4. `src/ui/mod.rs` - User Interface
Extract UI-related code:
- [`UIState`](src/main.rs:217), [`RightPanelUI`](src/main.rs:75)
- [`ui_example_system()`](src/main.rs:674)
- UI configuration and state management

```rust
// src/ui/mod.rs
pub mod state;
pub mod panels;
pub mod systems;

pub use state::{UIState, RightPanelUI};
pub use systems::ui_example_system;
```

#### 5. `src/visualization/mod.rs` - Rendering & Visualization
Extract visualization systems:
- [`ArrowConfig`](src/main.rs:168)
- [`draw_city_to_satellite_arrows()`](src/main.rs:396)
- [`draw_arrow_segment()`](src/main.rs:349)
- [`draw_axes()`](src/main.rs:433)
- [`ShowAxes`](src/main.rs:336)

```rust
// src/visualization/mod.rs
pub mod arrows;
pub mod axes;
pub mod config;

pub use config::ArrowConfig;
pub use arrows::{draw_city_to_satellite_arrows, draw_arrow_segment};
pub use axes::{draw_axes, ShowAxes};
```

### Refactored `main.rs` Structure

After extraction, [`main.rs`](src/main.rs:997) should only contain:
- App initialization and plugin registration
- System scheduling and ordering
- High-level resource initialization

```rust
// Simplified main.rs (target: ~100 lines)
use bevy::prelude::*;

mod satellite;
mod tle;
mod orbital;
mod ui;
mod visualization;

use satellite::{Satellite, SatelliteColor, SatelliteStore};
use tle::{FetchChannels, start_tle_worker};
use orbital::SimulationTime;
use ui::{UIState, RightPanelUI};
use visualization::ArrowConfig;

fn main() {
    App::new()
        .init_resource::<UIState>()
        .init_resource::<ArrowConfig>()
        .init_resource::<SimulationTime>()
        .init_resource::<SatelliteStore>()
        .init_resource::<RightPanelUI>()
        .add_plugins(DefaultPlugins)
        .add_plugins(satellite::SatellitePlugin)
        .add_plugins(tle::TlePlugin)
        .add_plugins(orbital::OrbitalPlugin)
        .add_plugins(ui::UIPlugin)
        .add_plugins(visualization::VisualizationPlugin)
        .run();
}
```

## Specific Code Organization Issues

### 1. Resource Management
**Current Issues**:
- Resources scattered throughout file
- No clear ownership or lifecycle management
- [`SatEcef`](src/main.rs:214) resource only used in one place

**Recommendations**:
- Group related resources in dedicated modules
- Use plugin pattern for resource initialization
- Consider removing [`SatEcef`](src/main.rs:214) if only used for debugging

### 2. System Organization
**Current Issues**:
- Systems mixed with data structures and utilities
- Complex system dependencies not clearly expressed
- Manual system ordering in [`main()`](src/main.rs:997)

**Recommendations**:
- Group systems by domain (satellite, UI, visualization)
- Use Bevy's system sets for better organization
- Document system dependencies clearly

### 3. Error Handling
**Current Issues**:
- Inconsistent error handling patterns
- [`ui_example_system()`](src/main.rs:674) returns `Result` but errors not properly handled
- TLE parsing errors mixed with network errors

**Recommendations**:
- Standardize error handling with [`anyhow`](Cargo.toml:21) for application errors
- Use [`thiserror`](Cargo.toml:20) for domain-specific errors
- Implement proper error recovery strategies

### 4. Configuration Management
**Current Issues**:
- Configuration scattered across multiple resources
- No centralized configuration loading
- Hard-coded values mixed with configurable parameters

**Recommendations**:
- Create `src/config.rs` for centralized configuration
- Use [`serde`](Cargo.toml:19) for configuration serialization
- Separate runtime state from configuration

## Testing Strategy

### Current State
- No tests in [`main.rs`](src/main.rs:1)
- Other modules have good test coverage ([`coverage.rs`](src/coverage.rs:228), [`coord.rs`](src/coord.rs:152))

### Recommendations

#### 1. Unit Testing
After module extraction, add tests for:
- TLE parsing functions
- Orbital mechanics calculations
- Satellite state management
- UI state transitions

#### 2. Integration Testing
Create integration tests for:
- Satellite tracking workflow
- TLE fetching and processing
- UI interactions with satellite data

#### 3. System Testing
Add system-level tests for:
- Complete satellite addition workflow
- Error recovery scenarios
- Performance under load

## Performance Considerations

### Current Issues
1. **UI System Performance**: [`ui_example_system()`](src/main.rs:674) is complex and runs every frame
2. **String Allocations**: Frequent string formatting in UI code
3. **Vector Allocations**: [`draw_city_to_satellite_arrows()`](src/main.rs:396) creates vectors each frame

### Recommendations
1. **UI Optimization**: Split UI system into multiple smaller systems
2. **Caching**: Cache formatted strings and computed values
3. **Memory Pools**: Use object pools for frequently allocated objects

## Implementation Priority

### Phase 1: Core Extraction (High Priority)
1. Extract [`satellite`](src/main.rs:46) module
2. Extract [`tle`](src/main.rs:109) module
3. Extract [`orbital`](src/main.rs:227) module

### Phase 2: UI Separation (Medium Priority)
1. Extract [`ui`](src/main.rs:217) module
2. Extract [`visualization`](src/main.rs:168) module
3. Implement plugin architecture

### Phase 3: Polish (Low Priority)
1. Add comprehensive testing
2. Optimize performance
3. Improve error handling

## Migration Strategy

### Step 1: Create Module Structure
```bash
mkdir -p src/{satellite,tle,orbital,ui,visualization}
touch src/{satellite,tle,orbital,ui,visualization}/mod.rs
```

### Step 2: Extract One Module at a Time
Start with the [`satellite`](src/main.rs:46) module as it has the clearest boundaries:
1. Move [`Satellite`](src/main.rs:46), [`SatelliteColor`](src/main.rs:49) to `src/satellite/components.rs`
2. Move [`SatelliteStore`](src/main.rs:52), [`SatEntry`](src/main.rs:58) to `src/satellite/resources.rs`
3. Move related systems to `src/satellite/systems.rs`
4. Update imports in [`main.rs`](src/main.rs:1)

### Step 3: Test After Each Extraction
Ensure the application still compiles and runs correctly after each module extraction.

### Step 4: Implement Plugin Pattern
Convert each module to a Bevy plugin for better organization and reusability.

## Conclusion

The current [`main.rs`](src/main.rs:1) violates the Single Responsibility Principle and makes the codebase difficult to maintain and test. By extracting functionality into focused modules and implementing a plugin architecture, we can achieve:

- **Better Maintainability**: Each module has a clear purpose
- **Improved Testability**: Smaller, focused units are easier to test
- **Enhanced Reusability**: Modules can be reused in other contexts
- **Clearer Dependencies**: Module boundaries make dependencies explicit
- **Easier Collaboration**: Multiple developers can work on different modules

The recommended refactoring will transform a 1030-line monolith into a well-organized, modular architecture that follows Rust and Bevy best practices.