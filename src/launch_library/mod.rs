//! Launch Library 2 integration (launches + events).

use bevy::prelude::*;

pub mod fetcher;
pub mod systems;
pub mod types;

pub use systems::{apply_launch_library_results, poll_launch_library};
pub use types::{EventSummary, LaunchLibraryConfig, LaunchLibraryData, LaunchLibraryState, LaunchSummary};

/// Plugin for Launch Library data management.
pub struct LaunchLibraryPlugin;

impl Plugin for LaunchLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LaunchLibraryConfig>()
            .init_resource::<LaunchLibraryState>()
            .init_resource::<LaunchLibraryData>()
            .add_systems(Startup, systems::setup_launch_library_worker)
            .add_systems(Update, (poll_launch_library, apply_launch_library_results).chain());
    }
}
