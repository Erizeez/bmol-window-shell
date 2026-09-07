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

## License

MIT OR Apache-2.0
