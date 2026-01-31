# Coordinate Frames and Units

This project uses a **two-frame model** to keep simulation math correct and rendering stable.

## Canonical World Frame (Authoritative)
- **Frame:** Standard ECEF (Earth-Centered, Earth-Fixed)
- **Axes:**  
  - +X: lat=0, lon=0 (prime meridian on equator)  
  - +Y: lat=0, lon=+90E  
  - +Z: north pole
- **Units:** kilometers
- **Type:** `DVec3`
- **Component:** `WorldEcefKm(DVec3)`

All simulation and long-lived data (satellites, ground points, LOS, ground tracks) must use this frame.

## Bevy Render Frame (Non-Authoritative)
- **Frame:** Bevy world space (`Transform.translation`)
- **Units:** kilometers (`Vec3`, f32)
- **Purpose:** Rendering only

Conversion boundary lives in `core::space`:
- `ecef_to_bevy_km(DVec3) -> Vec3`
- `bevy_to_ecef_km(Vec3) -> DVec3`

## Rule of Thumb
Never use `Transform.translation` as the source of truth for absolute world position.

## Naming Convention
- `*_ecef_km`: standard ECEF, `DVec3`
- `*_bevy_km`: Bevy render space, `Vec3`
