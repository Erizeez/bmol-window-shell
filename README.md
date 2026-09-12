# bmol-window-shell

Modern seamless frameless window shell, macOS SkyLight blur & EDR integration for GUI applications.

Compatible with any windowing or GUI framework via `raw-window-handle` (Iced, Winit, Qt, Slint, Tauri, etc.).

## Crates

The split is deliberate, and each crate owns exactly one concern:

| Crate | Owns |
| --- | --- |
| `bmol-window-shell` | The shell: chrome config and layout plans, the non-client rim, the loyal drag bar, the border resizer, the Iced controller, and the one-shot native setup. |
| `bmol-window-traffic-lights` | Traffic-light **semantics**: the hover/press state machine and its spring, the shared interaction frame, the material tuning, and the `GlassScene` builder. |
| `bmol-window-native` | macOS integration: SkyLight blur, EDR, corner masking, desktop backdrop capture, icon and wallpaper assets. |
| `bmol-window-platform` | Platform abstractions and the window metrics the others share. |

Two things that look like they belong here deliberately do not:

- The traffic-light **widget** — the Apple vector glyphs, the optical hit
  testing, and the overlay routing that keeps the glyphs above the glass — lives
  in the shared widget layer, [`bmol-glass-ui`](https://github.com/Erizeez/bmol-glass-ui),
  because an application needs the same widget a window shell does.
  `bmol-window-traffic-lights` re-exports it.
- The Liquid Glass **effect** — the bead material variant and the WGSL that
  implements it — lives in [`liquid-rs`](https://github.com/Erizeez/liquid-rs).
  Nothing here reimplements it; this repository only selects and tunes it.

## Features

- **SkyLight Window Blur**: Direct macOS SkyLight private compositor blur integration.
- **Stage Manager Guard**: Prevents blur flash during Stage Manager & Mission Control space transitions.
- **Extended Dynamic Range (EDR)**: Configures `CAMetalLayer` extended-linear sRGB for HDR specular highlights.
- **Window Corner Masking**: Native window layer corner radius clipping matching Apple continuous corners.
- **Backdrop Capture**: Captures desktop pixels beneath the transparent window for optical refraction shaders.
- **Asset Authoring Hooks**: Vector SF Symbols extraction and `.car` asset catalog rendition reading.

## Running the example

There is one example, and it is the reference integration:

```bash
cargo run -p bmol-window-shell --example window_demo
```

Its module documentation walks the six steps of the canonical wiring — the
controller, the frameless window settings, the one-shot native setup, the resize
events, `wrap_window_with_resizer`, and the drag bar — immediately above the code
that performs them.

The rest of the file is the feature surface:

1. **Interactive Traffic Lights**:
   - Red (Close), Yellow (Minimize), Green (Zoom), rendered as Liquid Glass
     beads with the Apple vector glyph layer above them.
   - Inactive window auto-dimming to Apple silver-gray.
   - Hover reveals the internal vector glyphs (✕, −, diagonal arrows).
   - Press runs the scale spring; dragging off the control cancels the tint but
     keeps it enlarged until release, as AppKit does.
2. **Three Chrome Layout Strategies**:
   - Standalone Titlebar: a separate titlebar with the title placed after the
     traffic lights.
   - Unified Chrome, single pane: content starting at the window's top edge.
   - Unified Chrome, multi-pane: a sidebar running to the top edge, with
     automatic traffic-light clearance.
3. **Native Titlebar Dragging**:
   - Hold & drag anywhere on the titlebar to move the window.
   - Double-click the titlebar to toggle maximize/restore.
4. **Continuous Squircle Window Frame**:
   - Continuous corner radius mask clipping the window layer.
   - Transparent background revealing true macOS SkyLight desktop wallpaper blur.
5. **Live Chrome Controls**:
   - SkyLight blur radius and opacity, corner radius, EDR, the native system
     shadow, and the Stage Manager guard. Toggling them re-runs the one-shot
     native setup.

## License

MIT OR Apache-2.0
