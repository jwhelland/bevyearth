# Satellite Camera Tracking Feature

## Overview

The satellite tracking feature allows the camera to continuously follow a selected satellite as it moves through its orbit, providing a dynamic view of the satellite's movement around Earth.

## How to Use

### Starting Tracking
1. **Load Satellites**: Use the right panel to load a satellite group (e.g., "DMC") or add individual satellites by NORAD ID
2. **Click to Track**: Click on any satellite's NORAD ID button in the satellite list
3. **Camera Movement**: The camera will immediately move to the satellite and begin continuous tracking

### Visual Indicators
- **Tracking Icon**: Satellites being tracked show a 📹 camera icon next to their NORAD ID
- **Button Highlight**: The NORAD ID button of the tracked satellite has a dark green background
- **Status Display**: The "Camera Tracking" section shows which satellite is currently being tracked

### Stopping Tracking
- **Stop Button**: Click the "Stop Tracking" button in the "Camera Tracking" section
- **Switch Satellites**: Click on a different satellite's NORAD ID to switch tracking targets
- **Manual Override**: The tracking will continue until explicitly stopped or switched

### Configuration Options

#### Tracking Distance
- **Range**: 1,000 - 20,000 km from the satellite
- **Default**: 5,000 km
- **Adjustment**: Use the "Distance (km)" slider in the Camera Tracking section

#### Smoothness
- **Range**: 0.01 - 1.0 (higher = more responsive, lower = smoother)
- **Default**: 0.1 (smooth movement)
- **Adjustment**: Use the "Smoothness" slider in the Camera Tracking section

## Technical Implementation

### Architecture
- **SelectedSatellite Resource**: Extended to support both one-time camera movement (`selected`) and continuous tracking (`tracking`)
- **Tracking System**: `track_satellite_continuously` system runs every frame to update camera position
- **Smooth Interpolation**: Uses frame-rate independent interpolation for smooth camera movement

### System Execution Order
```
propagate_satellites_system → track_satellite_continuously → move_camera_to_satellite
```

### Camera Behavior
- **Focus Point**: Camera always looks at Earth center (Vec3::ZERO)
- **Orbit Tracking**: Camera maintains orbital distance while following the satellite
- **Smooth Movement**: Uses exponential interpolation to avoid jittery camera movement
- **Yaw Wrapping**: Handles 360° yaw transitions smoothly

## Keyboard Shortcuts

- **H**: Toggle left panel
- **J**: Toggle right panel (where tracking controls are located)
- **K**: Toggle top panel
- **L**: Toggle bottom panel

## Tips

1. **Best Viewing**: Use a tracking distance of 3,000-8,000 km for optimal satellite visibility
2. **Smooth Tracking**: Lower smoothness values (0.05-0.15) provide cinematic camera movement
3. **Fast Tracking**: Higher smoothness values (0.3-1.0) provide more responsive camera movement
4. **Multiple Satellites**: You can switch between tracking different satellites instantly
5. **Performance**: Tracking has minimal performance impact as it only updates one camera per frame

## Troubleshooting

### Camera Not Moving
- Ensure the satellite has a "Ready" status (green indicator)
- Check that tracking is enabled in the Camera Tracking section
- Verify the satellite entity exists and has valid position data

### Jerky Movement
- Increase the smoothness factor for more responsive movement
- Check that the simulation time scale isn't too high
- Ensure stable frame rate for smooth interpolation

### Tracking Lost
- Satellite tracking automatically stops if the satellite entity is removed
- Re-click the satellite NORAD ID to restart tracking
- Check the satellite status for any errors