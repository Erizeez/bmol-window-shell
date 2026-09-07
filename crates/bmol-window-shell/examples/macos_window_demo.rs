//! Complete macOS-style Window Replication Demo.
//!
//! Replicates an authentic macOS application window using `bmol-window-shell`:
//! - Real-time SkyLight desktop background blur through a transparent window frame
//! - Interactive traffic lights (Close / Minimize / Zoom) with active/inactive states and hover glyphs
//! - Native titlebar dragging and double-click maximize/restore
//! - Continuous squircle corner clipping
//! - Structured macOS split-view layout: frosted sidebar, search pill, and settings cards.

use std::{num::NonZeroU32, sync::Arc, time::Instant};

use bmol_window_shell::{
    configure_extended_dynamic_range, configure_window_corner_radius, desktop_blur_target,
    install_stage_manager_guard, refresh_desktop_blur,
};
use raw_window_handle::HasWindowHandle;
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Fullscreen, Window, WindowId},
};

const WINDOW_CORNER_RADIUS: f64 = 16.0;
const SIDEBAR_WIDTH: u32 = 210;
const TITLEBAR_HEIGHT: u32 = 52;

// Traffic light geometry
const TL_RADIUS: f32 = 6.0;
const TL_Y: f32 = 26.0;
const TL_CLOSE_X: f32 = 20.0;
const TL_MIN_X: f32 = 40.0;
const TL_ZOOM_X: f32 = 60.0;

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    mouse_pos: (f32, f32),
    is_focused: bool,
    last_click_time: Option<Instant>,
    toggle_state: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("BMOL macOS Window Shell Demo")
            .with_inner_size(LogicalSize::new(880.0, 560.0))
            .with_min_inner_size(LogicalSize::new(640.0, 420.0))
            .with_transparent(true)
            .with_decorations(false); // Seamless frameless

        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));

        // Connect softbuffer surface
        let context = Context::new(window.clone()).expect("create softbuffer context");
        let surface = Surface::new(&context, window.clone()).expect("create softbuffer surface");

        // Apply native macOS window modifications via bmol-window-shell
        if let Ok(handle) = window.window_handle() {
            if let Some(target) = desktop_blur_target(handle.as_raw()) {
                refresh_desktop_blur(target);
                install_stage_manager_guard(target);
                configure_extended_dynamic_range(target, true);
                configure_window_corner_radius(target, WINDOW_CORNER_RADIUS);
            }
        }

        self.window = Some(window);
        self.surface = Some(surface);
        self.is_focused = true;
        self.toggle_state = true;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else { return };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Focused(focused) => {
                self.is_focused = focused;
                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = window.scale_factor();
                self.mouse_pos = (
                    (position.x / scale) as f32,
                    (position.y / scale) as f32,
                );
                window.request_redraw();
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let (mx, my) = self.mouse_pos;

                // 1. Check traffic lights click
                if (mx - TL_CLOSE_X).hypot(my - TL_Y) <= TL_RADIUS + 2.0 {
                    event_loop.exit();
                    return;
                }
                if (mx - TL_MIN_X).hypot(my - TL_Y) <= TL_RADIUS + 2.0 {
                    window.set_minimized(true);
                    return;
                }
                if (mx - TL_ZOOM_X).hypot(my - TL_Y) <= TL_RADIUS + 2.0 {
                    if window.fullscreen().is_some() {
                        window.set_fullscreen(None);
                    } else {
                        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                    return;
                }

                // 2. Check toggle switch click in main card
                let size = window.inner_size();
                let scale = window.scale_factor() as f32;
                let logical_w = size.width as f32 / scale;
                let toggle_x = logical_w - 74.0;
                let toggle_y = 338.0;
                if mx >= toggle_x - 6.0 && mx <= toggle_x + 36.0 && my >= toggle_y - 6.0 && my <= toggle_y + 22.0 {
                    self.toggle_state = !self.toggle_state;
                    window.request_redraw();
                    return;
                }

                // 3. Titlebar dragging or double-click maximize
                if my <= TITLEBAR_HEIGHT as f32 && mx > 80.0 {
                    let now = Instant::now();
                    if let Some(prev) = self.last_click_time {
                        if now.duration_since(prev).as_millis() < 300 {
                            // Double click: toggle maximize
                            let is_max = window.is_maximized();
                            window.set_maximized(!is_max);
                            self.last_click_time = None;
                            return;
                        }
                    }
                    self.last_click_time = Some(now);

                    // Native window drag
                    let _ = window.drag_window();
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if let Key::Named(NamedKey::Escape) = key_event.logical_key {
                    event_loop.exit();
                } else if let Key::Character(ref ch) = key_event.logical_key {
                    if ch == "q" || ch == "Q" {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::Resized(_) => {
                window.request_redraw();
            }
            _ => {}
        }
    }
}

impl App {
    fn render(&mut self) {
        let Some(window) = self.window.as_ref() else { return };
        let Some(surface) = self.surface.as_mut() else { return };

        let size = window.inner_size();
        let Some(w) = NonZeroU32::new(size.width) else { return };
        let Some(h) = NonZeroU32::new(size.height) else { return };

        surface.resize(w, h).expect("resize surface");
        let mut buffer = surface.buffer_mut().expect("get surface buffer");

        let width = size.width;
        let height = size.height;
        let scale = window.scale_factor() as f32;

        let logical_w = width as f32 / scale;
        let logical_h = height as f32 / scale;

        // Clear buffer with 0
        buffer.fill(0);

        // Render macOS split-view layout
        let (mx, my) = self.mouse_pos;
        let is_tl_hovered = self.is_focused && mx <= 80.0 && my <= TITLEBAR_HEIGHT as f32;

        let mut painter = Painter {
            buffer: &mut buffer,
            width: width as usize,
            height: height as usize,
            scale,
        };

        // 1. Draw Sidebar Background (Frosted translucent wash)
        // macOS settings sidebar: neutral translucent wash over SkyLight blur
        painter.fill_rect(
            0.0,
            0.0,
            SIDEBAR_WIDTH as f32,
            logical_h,
            if self.is_focused { 0x30FFFFFF } else { 0x20FFFFFF },
        );

        // 2. Draw Content Pane Background (slightly warmer/whiter translucent layer)
        painter.fill_rect(
            SIDEBAR_WIDTH as f32,
            0.0,
            logical_w - SIDEBAR_WIDTH as f32,
            logical_h,
            if self.is_focused { 0xDDF6F6F8 } else { 0xCEF0F0F2 },
        );

        // 3. Draw Vertical Divider between sidebar and content
        painter.fill_rect(
            SIDEBAR_WIDTH as f32 - 0.5,
            0.0,
            1.0,
            logical_h,
            0x18000000,
        );

        // 4. Draw Traffic Lights
        let (close_c, min_c, zoom_c) = if !self.is_focused {
            (0xFF4E4E52, 0xFF4E4E52, 0xFF4E4E52) // macOS inactive gray
        } else {
            (0xFFFF5F56, 0xFFFFBD2E, 0xFF27C93F) // macOS active vibrant red, yellow, green
        };

        painter.draw_circle(TL_CLOSE_X, TL_Y, TL_RADIUS, close_c);
        painter.draw_circle(TL_MIN_X, TL_Y, TL_RADIUS, min_c);
        painter.draw_circle(TL_ZOOM_X, TL_Y, TL_RADIUS, zoom_c);

        // If hovered, draw internal glyphs
        if is_tl_hovered {
            painter.draw_close_glyph(TL_CLOSE_X, TL_Y, 0x884D0000);
            painter.draw_minimize_glyph(TL_MIN_X, TL_Y, 0x88995700);
            painter.draw_zoom_glyph(TL_ZOOM_X, TL_Y, 0x88006500);
        }

        // 5. Draw Sidebar Content
        // Search Pill
        painter.draw_rounded_rect(16.0, 48.0, SIDEBAR_WIDTH as f32 - 32.0, 26.0, 6.0, 0x14000000);
        painter.draw_text("Search...", 32.0, 55.0, 0x55000000);

        // Sidebar Items
        let sidebar_items = [
            ("General", true),
            ("Appearance", false),
            ("Displays", false),
            ("Sound", false),
            ("Focus", false),
            ("Battery", false),
        ];

        let mut item_y = 88.0;
        for (label, selected) in sidebar_items {
            if selected {
                // Selected blue pill
                painter.draw_rounded_rect(
                    12.0,
                    item_y,
                    SIDEBAR_WIDTH as f32 - 24.0,
                    28.0,
                    6.0,
                    0xFF0A7AFF, // macOS accent blue
                );
                painter.draw_text(label, 26.0, item_y + 8.0, 0xFFFFFFFF);
            } else {
                painter.draw_text(label, 26.0, item_y + 8.0, 0xCC1A1A1A);
            }
            item_y += 34.0;
        }

        // 6. Draw Content Area (General Settings View)
        // Window Title & Toolbar
        painter.draw_text("General", SIDEBAR_WIDTH as f32 + 32.0, 22.0, 0xEE111111);

        // Section Heading
        painter.draw_text_large("General", SIDEBAR_WIDTH as f32 + 32.0, 64.0, 0xFF111111);

        // Settings Card 1: System Info
        let card_x = SIDEBAR_WIDTH as f32 + 32.0;
        let card_w = (logical_w - card_x - 36.0).max(320.0);

        painter.draw_rounded_rect(card_x, 100.0, card_w, 114.0, 10.0, 0xFAFFFFFF);
        painter.draw_rounded_rect_border(card_x, 100.0, card_w, 114.0, 10.0, 0x10000000);

        painter.draw_text("About", card_x + 16.0, 112.0, 0xEE1A1A1A);
        painter.draw_text("macOS Sequoia — 15.0", card_x + card_w - 150.0, 112.0, 0x66000000);

        painter.fill_rect(card_x + 16.0, 137.0, card_w - 32.0, 1.0, 0x0C000000);

        painter.draw_text("Software Update", card_x + 16.0, 149.0, 0xEE1A1A1A);
        painter.draw_text("Up to date", card_x + card_w - 88.0, 149.0, 0x66000000);

        painter.fill_rect(card_x + 16.0, 175.0, card_w - 32.0, 1.0, 0x0C000000);

        painter.draw_text("Storage", card_x + 16.0, 187.0, 0xEE1A1A1A);
        painter.draw_text("Macintosh HD", card_x + card_w - 96.0, 187.0, 0x66000000);

        // Settings Card 2: Shell & Vibrancy
        painter.draw_rounded_rect(card_x, 230.0, card_w, 150.0, 10.0, 0xFAFFFFFF);
        painter.draw_rounded_rect_border(card_x, 230.0, card_w, 150.0, 10.0, 0x10000000);

        painter.draw_text("SkyLight Background Blur", card_x + 16.0, 244.0, 0xEE1A1A1A);
        painter.draw_text("Active (CGS)", card_x + card_w - 94.0, 244.0, 0xFF34C759);

        painter.fill_rect(card_x + 16.0, 269.0, card_w - 32.0, 1.0, 0x0C000000);

        painter.draw_text("Stage Manager Guard", card_x + 16.0, 281.0, 0xEE1A1A1A);
        painter.draw_text("Protected", card_x + card_w - 78.0, 281.0, 0xFF34C759);

        painter.fill_rect(card_x + 16.0, 306.0, card_w - 32.0, 1.0, 0x0C000000);

        painter.draw_text("EDR & Extended Linear sRGB", card_x + 16.0, 318.0, 0xEE1A1A1A);
        painter.draw_text("Enabled", card_x + card_w - 68.0, 318.0, 0xFF0A7AFF);

        painter.fill_rect(card_x + 16.0, 343.0, card_w - 32.0, 1.0, 0x0C000000);

        painter.draw_text("Continuous Corner Radius", card_x + 16.0, 355.0, 0xEE1A1A1A);
        // Toggle Pill
        let toggle_x = card_x + card_w - 48.0;
        let toggle_y = 351.0;
        let toggle_color = if self.toggle_state { 0xFF34C759 } else { 0xFFCCCCCC };
        painter.draw_rounded_rect(toggle_x, toggle_y, 34.0, 18.0, 9.0, toggle_color);
        let knob_x = if self.toggle_state { toggle_x + 18.0 } else { toggle_x + 2.0 };
        painter.draw_circle(knob_x + 7.0, toggle_y + 9.0, 7.0, 0xFFFFFFFF);

        // 7. Footer Tips
        painter.draw_text(
            "Drag top area to move. Double-click to toggle maximize. Press ESC or Q to quit.",
            card_x,
            logical_h - 24.0,
            0x66000000,
        );

        buffer.present().expect("present softbuffer frame");
    }
}

// Minimal fast pixel painter
struct Painter<'a> {
    buffer: &'a mut [u32],
    width: usize,
    height: usize,
    scale: f32,
}

impl Painter<'_> {
    fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            let src_a = (color >> 24) & 0xFF;
            if src_a == 0xFF {
                self.buffer[idx] = color;
            } else if src_a > 0 {
                // Alpha blend
                let dst = self.buffer[idx];
                let dst_a = (dst >> 24) & 0xFF;
                let dst_r = (dst >> 16) & 0xFF;
                let dst_g = (dst >> 8) & 0xFF;
                let dst_b = dst & 0xFF;

                let src_r = (color >> 16) & 0xFF;
                let src_g = (color >> 8) & 0xFF;
                let src_b = color & 0xFF;

                let a = src_a + dst_a * (255 - src_a) / 255;
                let r = (src_r * src_a + dst_r * (255 - src_a)) / 255;
                let g = (src_g * src_a + dst_g * (255 - src_a)) / 255;
                let b = (src_b * src_a + dst_b * (255 - src_a)) / 255;

                self.buffer[idx] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32) {
        let x0 = (x * self.scale).max(0.0) as usize;
        let y0 = (y * self.scale).max(0.0) as usize;
        let x1 = ((x + w) * self.scale).min(self.width as f32) as usize;
        let y1 = ((y + h) * self.scale).min(self.height as f32) as usize;

        for py in y0..y1 {
            for px in x0..x1 {
                self.set_pixel(px, py, color);
            }
        }
    }

    fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, color: u32) {
        let px_cx = cx * self.scale;
        let px_cy = cy * self.scale;
        let px_r = r * self.scale;

        let x0 = (px_cx - px_r - 1.0).max(0.0) as usize;
        let y0 = (px_cy - px_r - 1.0).max(0.0) as usize;
        let x1 = (px_cx + px_r + 1.0).min(self.width as f32) as usize;
        let y1 = (px_cy + px_r + 1.0).min(self.height as f32) as usize;

        for py in y0..y1 {
            let dy = py as f32 - px_cy;
            for px in x0..x1 {
                let dx = px as f32 - px_cx;
                let dist = dx.hypot(dy);
                if dist <= px_r {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }

    fn draw_close_glyph(&mut self, cx: f32, cy: f32, color: u32) {
        let px_cx = cx * self.scale;
        let px_cy = cy * self.scale;
        let span = 2.5 * self.scale;

        for d in -2..=2 {
            let step = d as f32 * span / 2.0;
            self.set_pixel((px_cx + step) as usize, (px_cy + step) as usize, color);
            self.set_pixel((px_cx + step) as usize, (px_cy - step) as usize, color);
        }
    }

    fn draw_minimize_glyph(&mut self, cx: f32, cy: f32, color: u32) {
        let px_cx = cx * self.scale;
        let px_cy = cy * self.scale;
        let span = 3.0 * self.scale;

        let x0 = (px_cx - span) as usize;
        let x1 = (px_cx + span) as usize;
        let y = px_cy as usize;
        for px in x0..=x1 {
            self.set_pixel(px, y, color);
        }
    }

    fn draw_zoom_glyph(&mut self, cx: f32, cy: f32, color: u32) {
        let px_cx = cx * self.scale;
        let px_cy = cy * self.scale;
        let span = 2.0 * self.scale;

        // Top right arrow & bottom left arrow
        self.set_pixel((px_cx + span) as usize, (px_cy - span) as usize, color);
        self.set_pixel((px_cx + span - 1.0) as usize, (px_cy - span) as usize, color);
        self.set_pixel((px_cx + span) as usize, (px_cy - span + 1.0) as usize, color);

        self.set_pixel((px_cx - span) as usize, (px_cy + span) as usize, color);
        self.set_pixel((px_cx - span + 1.0) as usize, (px_cy + span) as usize, color);
        self.set_pixel((px_cx - span) as usize, (px_cy + span - 1.0) as usize, color);
    }

    fn draw_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
        let x0 = (x * self.scale).max(0.0) as usize;
        let y0 = (y * self.scale).max(0.0) as usize;
        let x1 = ((x + w) * self.scale).min(self.width as f32) as usize;
        let y1 = ((y + h) * self.scale).min(self.height as f32) as usize;
        let r_px = r * self.scale;

        let left_c = x * self.scale + r_px;
        let right_c = (x + w) * self.scale - r_px;
        let top_c = y * self.scale + r_px;
        let bot_c = (y + h) * self.scale - r_px;

        for py in y0..y1 {
            let py_f = py as f32;
            for px in x0..x1 {
                let px_f = px as f32;
                let inside_corner = if px_f < left_c && py_f < top_c {
                    (px_f - left_c).hypot(py_f - top_c) <= r_px
                } else if px_f > right_c && py_f < top_c {
                    (px_f - right_c).hypot(py_f - top_c) <= r_px
                } else if px_f < left_c && py_f > bot_c {
                    (px_f - left_c).hypot(py_f - bot_c) <= r_px
                } else if px_f > right_c && py_f > bot_c {
                    (px_f - right_c).hypot(py_f - bot_c) <= r_px
                } else {
                    true
                };

                if inside_corner {
                    self.set_pixel(px, py, color);
                }
            }
        }
    }

    fn draw_rounded_rect_border(&mut self, x: f32, y: f32, w: f32, h: f32, _r: f32, color: u32) {
        // Simple outline pass
        let x0 = x * self.scale;
        let y0 = y * self.scale;
        let x1 = (x + w) * self.scale;
        let y1 = (y + h) * self.scale;

        let left = x0 as usize;
        let right = x1 as usize;
        let top = y0 as usize;
        let bottom = y1 as usize;

        for px in left..right {
            self.set_pixel(px, top, color);
            self.set_pixel(px, bottom, color);
        }
        for py in top..bottom {
            self.set_pixel(left, py, color);
            self.set_pixel(right, py, color);
        }
    }

    fn draw_text(&mut self, text: &str, x: f32, y: f32, color: u32) {
        let mut cur_x = (x * self.scale) as usize;
        let cur_y = (y * self.scale) as usize;

        for ch in text.chars() {
            if let Some(bitmap) = get_char_bitmap(ch) {
                for (row, byte) in bitmap.iter().enumerate() {
                    for col in 0..8 {
                        if (byte & (1 << (7 - col))) != 0 {
                            self.set_pixel(cur_x + col, cur_y + row, color);
                        }
                    }
                }
            }
            cur_x += 8;
        }
    }

    fn draw_text_large(&mut self, text: &str, x: f32, y: f32, color: u32) {
        let mut cur_x = (x * self.scale) as usize;
        let cur_y = (y * self.scale) as usize;

        for ch in text.chars() {
            if let Some(bitmap) = get_char_bitmap(ch) {
                for (row, byte) in bitmap.iter().enumerate() {
                    for col in 0..8 {
                        if (byte & (1 << (7 - col))) != 0 {
                            // 2x2 block
                            self.set_pixel(cur_x + col * 2, cur_y + row * 2, color);
                            self.set_pixel(cur_x + col * 2 + 1, cur_y + row * 2, color);
                            self.set_pixel(cur_x + col * 2, cur_y + row * 2 + 1, color);
                            self.set_pixel(cur_x + col * 2 + 1, cur_y + row * 2 + 1, color);
                        }
                    }
                }
            }
            cur_x += 16;
        }
    }
}

// Built-in crisp 8x8 font glyph table
fn get_char_bitmap(ch: char) -> Option<[u8; 8]> {
    match ch {
        'A' => Some([0x18, 0x24, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x00]),
        'B' => Some([0x7C, 0x22, 0x22, 0x3C, 0x22, 0x22, 0x7C, 0x00]),
        'C' => Some([0x3C, 0x42, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00]),
        'D' => Some([0x78, 0x24, 0x22, 0x22, 0x22, 0x24, 0x78, 0x00]),
        'E' => Some([0x7E, 0x40, 0x40, 0x78, 0x40, 0x40, 0x7E, 0x00]),
        'F' => Some([0x7E, 0x40, 0x40, 0x78, 0x40, 0x40, 0x40, 0x00]),
        'G' => Some([0x3C, 0x42, 0x40, 0x4E, 0x42, 0x42, 0x3C, 0x00]),
        'H' => Some([0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x00]),
        'I' => Some([0x38, 0x10, 0x10, 0x10, 0x10, 0x10, 0x38, 0x00]),
        'L' => Some([0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00]),
        'M' => Some([0x42, 0x66, 0x5A, 0x42, 0x42, 0x42, 0x42, 0x00]),
        'N' => Some([0x42, 0x62, 0x52, 0x4A, 0x46, 0x42, 0x42, 0x00]),
        'O' => Some([0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00]),
        'P' => Some([0x7C, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x00]),
        'R' => Some([0x7C, 0x42, 0x42, 0x7C, 0x48, 0x44, 0x42, 0x00]),
        'S' => Some([0x3C, 0x42, 0x40, 0x3C, 0x02, 0x42, 0x3C, 0x00]),
        'T' => Some([0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00]),
        'U' => Some([0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00]),
        'V' => Some([0x42, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x00]),
        'W' => Some([0x42, 0x42, 0x42, 0x5A, 0x5A, 0x66, 0x42, 0x00]),
        'Y' => Some([0x42, 0x42, 0x24, 0x18, 0x18, 0x18, 0x18, 0x00]),
        'Z' => Some([0x7E, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7E, 0x00]),
        'a' => Some([0x00, 0x00, 0x38, 0x04, 0x3C, 0x44, 0x3A, 0x00]),
        'b' => Some([0x40, 0x40, 0x5C, 0x62, 0x42, 0x62, 0x5C, 0x00]),
        'c' => Some([0x00, 0x00, 0x3C, 0x42, 0x40, 0x42, 0x3C, 0x00]),
        'd' => Some([0x02, 0x02, 0x3A, 0x46, 0x42, 0x46, 0x3A, 0x00]),
        'e' => Some([0x00, 0x00, 0x3C, 0x42, 0x7E, 0x40, 0x3C, 0x00]),
        'f' => Some([0x0C, 0x12, 0x10, 0x38, 0x10, 0x10, 0x10, 0x00]),
        'g' => Some([0x00, 0x00, 0x3A, 0x46, 0x46, 0x3E, 0x06, 0x3C]),
        'h' => Some([0x40, 0x40, 0x5C, 0x62, 0x42, 0x42, 0x42, 0x00]),
        'i' => Some([0x10, 0x00, 0x30, 0x10, 0x10, 0x10, 0x38, 0x00]),
        'k' => Some([0x40, 0x40, 0x44, 0x48, 0x70, 0x48, 0x44, 0x00]),
        'l' => Some([0x30, 0x10, 0x10, 0x10, 0x10, 0x10, 0x38, 0x00]),
        'm' => Some([0x00, 0x00, 0x6C, 0x92, 0x92, 0x92, 0x92, 0x00]),
        'n' => Some([0x00, 0x00, 0x5C, 0x62, 0x42, 0x42, 0x42, 0x00]),
        'o' => Some([0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x3C, 0x00]),
        'p' => Some([0x00, 0x00, 0x5C, 0x62, 0x62, 0x5C, 0x40, 0x40]),
        'q' => Some([0x00, 0x00, 0x3A, 0x46, 0x46, 0x3A, 0x02, 0x02]),
        'r' => Some([0x00, 0x00, 0x2E, 0x32, 0x20, 0x20, 0x20, 0x00]),
        's' => Some([0x00, 0x00, 0x3C, 0x40, 0x3C, 0x02, 0x7C, 0x00]),
        't' => Some([0x10, 0x10, 0x3C, 0x10, 0x10, 0x12, 0x0C, 0x00]),
        'u' => Some([0x00, 0x00, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x00]),
        'v' => Some([0x00, 0x00, 0x42, 0x42, 0x24, 0x24, 0x18, 0x00]),
        'w' => Some([0x00, 0x00, 0x42, 0x5A, 0x5A, 0x24, 0x24, 0x00]),
        'x' => Some([0x00, 0x00, 0x42, 0x24, 0x18, 0x24, 0x42, 0x00]),
        'y' => Some([0x00, 0x00, 0x42, 0x42, 0x46, 0x3A, 0x02, 0x3C]),
        'z' => Some([0x00, 0x00, 0x7E, 0x08, 0x10, 0x20, 0x7E, 0x00]),
        '0' => Some([0x3C, 0x46, 0x4A, 0x52, 0x62, 0x42, 0x3C, 0x00]),
        '1' => Some([0x18, 0x28, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00]),
        '2' => Some([0x3C, 0x42, 0x02, 0x0C, 0x30, 0x40, 0x7E, 0x00]),
        '3' => Some([0x3C, 0x42, 0x02, 0x1C, 0x02, 0x42, 0x3C, 0x00]),
        '4' => Some([0x08, 0x18, 0x28, 0x48, 0x7E, 0x08, 0x08, 0x00]),
        '5' => Some([0x7E, 0x40, 0x7C, 0x02, 0x02, 0x42, 0x3C, 0x00]),
        '6' => Some([0x3C, 0x42, 0x40, 0x7C, 0x42, 0x42, 0x3C, 0x00]),
        '7' => Some([0x7E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x20, 0x00]),
        '8' => Some([0x3C, 0x42, 0x42, 0x3C, 0x42, 0x42, 0x3C, 0x00]),
        '9' => Some([0x3C, 0x42, 0x42, 0x3E, 0x02, 0x42, 0x3C, 0x00]),
        '.' => Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00]),
        ',' => Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30]),
        '-' => Some([0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00]),
        ':' => Some([0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00]),
        '(' => Some([0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00]),
        ')' => Some([0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00]),
        '/' => Some([0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x00]),
        ' ' => Some([0x00; 8]),
        _ => Some([0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00]),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
