# Bevy 0.20 upgrade validation

Validated October 24, 2026 on macOS 27.0.1, Apple M4 Pro / Metal, 24 GB RAM.

## Changes

- Bevy and the first-party camera controller use 0.20.0. The external orbit-camera dependency is removed. Rust 1.97.1 is pinned for builds, formatting, and linting.
- The application camera plugin supplies the input adapter required by the new controller. Left drag orbits and wheel/trackpad scroll zooms around the selected body center; translation panning is disabled. Input is blocked over UI containers, during resizing, on inactive cameras, and during automatic camera moves.
- Camera movement now has one application owner after simulation updates. Satellite framing uses canonical ECEF positions. Moon viewing transports the user-adjusted pose between successive body frames, preserving the viewed surface direction during accelerated simulation.
- Focus changes, satellite tracking, launch fly-to, target removal, and saved Earth poses have explicit transition handling. Programmatic moves clear the controller's momentum. F2 copies the current view to the development camera and disables input on its inactive free-camera controller.
- Pointer events and cursor imports use the 0.20 APIs. Feathers slider initialization uses `SliderValue`; the `dev` feature enables rendered development tools explicitly. Font enforcement has strict ordering before UI content measurement. The UI root has a tab group; application sections synchronize display and inherited visibility so hidden controls cannot receive keyboard navigation.
- The application palette uses distinct semantic tokens for each custom override, preserving existing colors without recoloring other widgets through shared default tokens. The color-plane widget owns its upward Y direction; application values are not inverted again.
- The custom sky material uses WESL. Its diagnostic uses Bevy's reverse-Z far plane and depth comparison so sky fragments cannot cover foreground geometry. Normal application rendering continues to use the built-in skybox.

No application data formats, cache formats, or network endpoints changed.

## Automated validation

Passed:

```sh
cargo test --locked --offline
cargo test --locked --offline --all-features
cargo check --locked --offline --all-targets
cargo check --locked --offline --all-targets --all-features
cargo check --locked --offline --all-targets --features debug_basic_scene
cargo check --locked --offline --all-targets --features debug_scene_camera
cargo check --locked --offline --all-targets --features dev
cargo check --locked --offline --all-targets --features dev_camera
cargo build --locked --offline --release
cargo build --locked --offline --release --features dev,dev_camera
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets --all-features -- -D warnings
```

The default suite contains **112 tests**; all features add the F2 regression test for **113 tests**. All original 100 tests pass. New tests cover pose round-trips, elapsed-time smoothing and yaw wrapping, moving Moon frames, ECEF-based tracking, removal/cancellation, focus restoration, fly-to completion without repeated restarts, input gating, momentum cancellation, theme resolution, color-channel mapping, hidden-section visibility for keyboard navigation, and development-camera activation.

The dependency graph contains one `bevy` version, 0.20.0, and no 0.19 engine crates or external orbit-camera plugin. All unrelated direct dependency versions were retained. The independently versioned `bevy_mikktspace` remains at 1.0.0.

The earlier toolchain/cache permission failures were resolved by installing the required toolchain and fetching dependencies. Standalone diagnostic configurations compile unused full-application modules and therefore emit unused-code warnings; the full all-features strict Clippy check is clean.

## Runtime checks

Both diagnostic scenes and the full application started on Metal. The full application loaded its fonts, textures, cached ISS TLE, space-weather data, and launch data without asset, shader, or scheduling errors.

Verified interactively:

- Earth/Moon focus switching and restoration, Moon orbit/zoom, and body-centered viewing at **929x** simulation speed.
- Live ISS tracking, followed by removing the tracked satellite while retaining the last camera pose.
- Launch selection, fly-to, and the launch popup.
- Panel visibility, scrolling, resizing, viewport cropping, the speed slider, and color selection (blue increases upward).
- Per-satellite ground tracks and trails, heatmap rendering, and translucent UI layering.
- F2 activation/deactivation, confirmed in the application log, plus the automated state/pose regression test.
- Built-in sky rendering and the custom WESL material with the built-in sky disabled; foreground geometry remains visible.

Keyboard testing exposed a missing root tab group. The root group and hidden-section visibility handling are fixed and covered by a deterministic regression test. After unlocking the Mac, the final live check passed: Tab and Shift-Tab navigate visible controls with both side panels hidden, arrow keys adjust the speed slider, the focus ring renders, and the tooltip displays without moving the camera. The final runtime log contains no tab-group warnings or other warnings/errors.

Use `cargo run --features debug_scene_camera` for the shader diagnostic: **1** toggles the built-in sky and **2** toggles the custom WESL sky. The custom sky starts hidden. `cargo run --features debug_basic_scene` runs the minimal sphere/camera diagnostic. `cargo run --release --features dev,dev_camera` enables the FPS overlay and F2 camera.

## Release performance comparison

The pre-upgrade source at commit `e90dc8b` was built separately with Bevy 0.19 and Rust 1.95, outside the working checkout. Each version ran separately with the same cached ISS satellite, default 25,000 km Earth view, default textures, hidden side panels, uncropped viewport, 1x simulation, and AutoVsync. Both had the FPS overlay enabled and a 2560 x 1536 pixel window including the title bar. Compilation had completed before measurement. The upgraded build also included the inactive development camera.

| Build | Visible FPS reading |
| --- | ---: |
| Bevy 0.19 release | 120.83 |
| Bevy 0.20 release | 119.85 |

Both readings are near the display's 120 Hz ceiling. This is a light-scene regression check, not a statistically sampled throughput benchmark or evidence about large satellite groups. Live time and weather changed between runs.
