//! Standalone demonstration of the BMOL seamless frameless window shell.
//!
//! This example creates a native macOS window with:
//! - Full desktop background blur via SkyLight
//! - Extended Dynamic Range (EDR) layer configuration
//! - Continuous corner radius clipping
//! - Stage Manager transition flicker protection

use std::sync::Arc;

use bmol_window_shell::{
    configure_extended_dynamic_range, configure_window_corner_radius, desktop_blur_target,
    install_stage_manager_guard, refresh_desktop_blur,
};
use raw_window_handle::HasWindowHandle;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("BMOL Window Shell — Seamless Frameless Window")
            .with_inner_size(LogicalSize::new(840.0, 520.0))
            .with_transparent(true)
            .with_decorations(false);

        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));

        println!("============================================================");
        println!("BMOL Window Shell Native Demo Initialized");
        println!("============================================================");

        if let Ok(handle) = window.window_handle() {
            if let Some(target) = desktop_blur_target(handle.as_raw()) {
                println!("[✓] DesktopBlurTarget resolved: {:?}", target);

                // 1. Reapply the native SkyLight desktop blur radius
                refresh_desktop_blur(target);
                println!("[✓] Applied macOS SkyLight background blur");

                // 2. Install persistent guard for Stage Manager and Space switches
                install_stage_manager_guard(target);
                println!("[✓] Installed Stage Manager & Space transition flicker guard");

                // 3. Configure CAMetalLayer for EDR (Extended Dynamic Range)
                configure_extended_dynamic_range(target, true);
                println!("[✓] Configured CAMetalLayer for Extended-Linear sRGB (EDR)");

                // 4. Clip layer to Apple continuous corner radius
                configure_window_corner_radius(target, 16.0);
                println!("[✓] Applied 16.0pt continuous corner mask to window layer");
            } else {
                println!("[!] Warning: Could not resolve DesktopBlurTarget on this platform");
            }
        }

        println!("------------------------------------------------------------");
        println!("Instructions:");
        println!("  - Press [ESC] or [Q] to close the window");
        println!("  - Drag or observe real-time frosted glass over your wallpaper");
        println!("============================================================");

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested, exiting...");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Key::Named(NamedKey::Escape) = event.logical_key {
                    event_loop.exit();
                } else if let Key::Character(ref ch) = event.logical_key {
                    if ch == "q" || ch == "Q" {
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
