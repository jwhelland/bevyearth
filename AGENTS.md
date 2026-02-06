# Repository Guidelines

## Project Structure & Module Organization
- Rust crate `bevyearth` using Bevy ECS.
- Entry point: `src/main.rs` (wires plugins and sets up cameras/skybox).
- Modules:
  - `src/visualization/` (earth, axes, arrows, cities, heatmap, ground_track + gizmos, sky material, lighting, config)
  - `src/ui/` (Bevy UI widgets/feathers systems, state, skybox, groups)
  - `src/tle/` (fetcher, parser, systems, types, mock_data)
  - `src/orbital/` (time, propagation, coordinates)
  - `src/satellite/` (components, resources, systems)
  - `src/core/` (shared types/utilities)
- Assets live in `assets/` (Bevy asset root). Integration tests in `tests/`.
- Coordinate frame guide lives in `docs/coordinates.md`.
- Celestrak group URL reference lives in `tle_data_urls.md`.

## Build, Test, and Development Commands
- `cargo run`:
  Run the app locally (requires `assets/`; internet enables TLE fetch).
- `cargo build --release`:
  Optimized build for better runtime performance.
- `cargo test`:
  Run unit and integration tests.
- `cargo fmt --all`:
  Format the codebase with rustfmt.
- `cargo clippy --all-targets --all-features -D warnings`:
  Lint and fail on warnings.

## Coding Style & Naming Conventions
- Formatting: rustfmt defaults (4‑space indent, stable style).
- Naming: files/modules `snake_case`; types/traits `PascalCase`; fns/vars `snake_case`; consts `SCREAMING_SNAKE_CASE`.
- Organization: prefer `mod.rs` per folder; split by concern (`components.rs`, `resources.rs`, `systems.rs`, `types.rs`).
- Bevy: group systems in `systems.rs` and expose via the module’s `Plugin` in `mod.rs`.
- Coordinates: canonical world frame is ECEF in km using `DVec3` (`WorldEcefKm`). Convert to/from render space via `core::space` helpers; avoid using `Transform.translation` as source of truth.

## Testing Guidelines
- Framework: Rust test harness. Place integration tests in `tests/` (e.g., `tests/integration/*.rs`).
- Unit tests live beside code in `#[cfg(test)] mod tests` blocks.
- Keep tests deterministic; avoid network. For TLE parsing, use fixtures or `tle::mock_data`.

## Commit & Pull Request Guidelines
- Commits: imperative mood, concise subject (≤72 chars). Optional scope like `feat(ui): …`, `fix(tle): …`. Reference issues (e.g., `Closes #12`).
- PRs: clear description, motivation, and screenshots/GIFs for UI changes. Link issues, list testing steps, and note any asset or API impacts.

## Security & Configuration Tips
- Networking: TLE fetch uses `reqwest` with `rustls-tls` (no system OpenSSL needed). App runs offline but TLE downloads will fail without internet.
- Assets: load via `asset_server.load("<file>")` relative to `assets/` (e.g., `assets/skybox.png`). Keep large textures in `assets/` and out of `src/`.
- Performance: use `cargo run --release` for smoother rendering when profiling visuals.
