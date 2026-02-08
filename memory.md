# memory.md

- Implemented cloud layer as a transparent mesh-based volume shell (Material) instead of fullscreen post-process; raymarch happens in the cloud material shader on a scaled Earth mesh.
- Cloud shader uses a procedural 3D noise texture plus a placeholder 2D coverage texture; `use_real_data` currently toggles the placeholder coverage sampling (no external fetch yet).
- Cloud UI controls: enable toggle, density, animation speed, altitude min/max (kept at least 0.5 km apart), and quality radio (Low/Medium/High).
- Moved the Clouds UI section from the right panel to the left panel (placed after Space Weather overview).
- Cloud quality now also modulates noise detail weighting and jitter in the shader to make Low/Medium/High more visually distinct.
- Added NASA GIBS WMS cloud coverage fetcher (WMS GetMap PNG) with 30-minute refresh and fallback to previous day; decoded via `Image::from_buffer` and converted to R8 coverage texture.
- Corrected NASA GIBS layer identifier to `MODIS_Terra_Cloud_Top_Temp_Day` (short "Temp"), based on GIBS layer index listings.
- Smoothed cloud appearance by blending a low-frequency "shape" noise, reducing jitter, and adding vertical fade across the cloud layer to reduce particle-like artifacts.
- Added CPU-side smoothing + remap of cloud coverage map (2-pass 3x3 blur + gamma/threshold) and removed minimum coverage bias in shader to reduce “clouds everywhere” and banding.
- Added `show_raw_coverage` toggle to CloudConfig and UI; spawns a coverage overlay mesh using the fetched coverage texture for direct visualization.
- Left panel is now scrollable (scroll container + scrollbar + mouse wheel support) via `LeftPanelScroll`.
- Switched real cloud coverage fetch to NASA GIBS WMTS tile mosaic: parses GetCapabilities (quick-xml), selects EPSG:4326 tile matrix set + zoom based on 2048x1024 target, downloads tiles in parallel, assembles an RGBA mosaic, and falls back to previous day on failure.
- WMTS mosaic now probes the first tile to surface errors early and retries without TIME when the time-qualified request fails.
- Fixed WMTS capabilities parsing to only treat `<Identifier>` under `<Layer>` as the layer id (avoids picking `Time` dimension identifiers).
- NONE of this worked well or was visually appealing
