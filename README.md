# bmol-window-shell

Modern seamless frameless window shell, macOS SkyLight blur & EDR integration for GUI applications.

Compatible with any windowing or GUI framework via `raw-window-handle` (Iced, Winit, Qt, Slint, Tauri, etc.).

## Features

- **SkyLight Window Blur**: Direct macOS SkyLight private compositor blur integration.
- **Stage Manager Guard**: Prevents blur flash during Stage Manager & Mission Control space transitions.
- **Extended Dynamic Range (EDR)**: Configures `CAMetalLayer` extended-linear sRGB for HDR specular highlights.
- **Window Corner Masking**: Native window layer corner radius clipping matching Apple continuous corners.
- **Backdrop Capture**: Captures desktop pixels beneath the transparent window for optical refraction shaders.
- **Asset Authoring Hooks**: Vector SF Symbols extraction and `.car` asset catalog rendition reading.

## Running the Complete Window Demo

Run the complete frameless window replication demo:

```bash
cargo run -p bmol-window-shell --example window_demo
```

### What this demo replicates:
1. **Interactive Traffic Lights**:
   - Red (Close), Yellow (Minimize), Green (Toggle Fullscreen).
   - Inactive window auto-dimming to Apple silver-gray (`#4E4E52`).
   - Hover reveals internal vector glyphs ($\times$, $-$, dual arrows).
2. **Native Titlebar Dragging**:
   - Hold & drag anywhere on the titlebar to move the window.
   - Double-click the titlebar to toggle maximize/restore.
3. **Continuous Squircle Window Frame**:
   - 16.0pt continuous corner radius mask clipping the window layer.
   - Transparent background revealing true macOS SkyLight desktop wallpaper blur.
4. **Structured Split-View Layout & Content**:
   - Frosted sidebar with search pill and active item indicator.
   - Two settings cards with system metrics, toggle switch, and divider lines.

## License

MIT OR Apache-2.0
