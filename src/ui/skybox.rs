use bevy::core_pipeline::Skybox;
use bevy::prelude::Plugin;
use bevy::{
    asset::LoadState,
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};

use crate::orbital::{Dut1, SimulationTime, gmst_rad_with_dut1};
use crate::ui::systems::MainCamera;

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, (asset_loaded, update_skybox_rotation));
    }
}

#[derive(Resource)]
pub struct Cubemap {
    pub activated: bool,
    pub is_loaded: bool,
    pub image_handle: Handle<Image>,
}

const SKYBOX_YAW_OFFSET_DEG: f32 = 0.0;
// Approximate tilt of the Milky Way's galactic plane relative to Earth's equator.
const SKYBOX_PITCH_OFFSET_DEG: f32 = 62.6;
const SKYBOX_ROLL_OFFSET_DEG: f32 = 0.0;

fn asset_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut cubemap: ResMut<Cubemap>,
    mut camera_query: Query<(Entity, Option<&Skybox>), With<MainCamera>>,
) {
    if cubemap.activated
        && !cubemap.is_loaded
        && asset_server
            .get_load_state(cubemap.image_handle.id())
            .unwrap_or(LoadState::NotLoaded)
            .is_loaded()
    {
        let image = images.get_mut(&cubemap.image_handle).unwrap();
        // NOTE: PNGs do not have any metadata that could indicate they contain a cubemap texture,
        // so they appear as one texture. The following code reconfigures the texture as necessary.
        if image.texture_descriptor.array_layer_count() == 1 {
            if let Err(err) = image.reinterpret_stacked_2d_as_array(image.height() / image.width())
            {
                warn!("Failed to reinterpret skybox image as cubemap: {}", err);
                cubemap.is_loaded = true;
                return;
            }
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }

        cubemap.is_loaded = true;
    }

    if cubemap.activated && cubemap.is_loaded {
        if let Ok((camera_entity, skybox)) = camera_query.single_mut() {
            if skybox.is_none() {
                commands.entity(camera_entity).insert(Skybox {
                    image: cubemap.image_handle.clone(),
                    brightness: 500.0,
                    ..default()
                });
            }
        }
    }
}

fn update_skybox_rotation(
    sim_time: Res<SimulationTime>,
    dut1: Res<Dut1>,
    mut query: Query<&mut Skybox, With<MainCamera>>,
) {
    if query.is_empty() {
        return;
    }

    // Calculate GMST rotation
    let gmst = gmst_rad_with_dut1(sim_time.current_utc, **dut1);
    
    // Rotate around Y axis (North).
    // Earth rotates East (CCW from North).
    // Stars appear to rotate West (CW).
    // ECEF is fixed. We need to rotate the Skybox by -GMST to match the Stars' ECI position relative to ECEF.
    // Wait, ECI = RotZ(-GMST) * ECEF?
    // r_eci = [cos -t, -sin -t... ] * r_ecef?
    // No, r_ecef = RotZ(GMST) * r_eci (Frame rotation).
    // So Vector rotation: v_ecef = R_z(GMST) * v_eci.
    // If v_eci is fixed (1,0,0), then v_ecef rotates.
    // v_ecef(t) = (cos t, -sin t, 0).
    // So the skybox should rotate by -t?
    // Let's try -gmst.

    let yaw = -gmst as f32 + SKYBOX_YAW_OFFSET_DEG.to_radians();
    let pitch = SKYBOX_PITCH_OFFSET_DEG.to_radians();
    let roll = SKYBOX_ROLL_OFFSET_DEG.to_radians();
    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    for mut skybox in &mut query {
        skybox.rotation = rotation;
    }
}
