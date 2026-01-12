use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use std::sync::Arc;
use log::info;

use crate::render::{RenderContext, PollsterBlockOn};

pub struct App {
    script_path: Option<String>,
    use_cpu: bool,
}

impl App {
    pub fn new() -> Self {
        env_logger::init();
        Self {
            script_path: None,
            use_cpu: false,
        }
    }

    pub fn with_use_cpu(mut self, use_cpu: bool) -> Self {
        self.use_cpu = use_cpu;
        self
    }

    pub fn with_script(mut self, path: &str) -> Self {
        self.script_path = Some(path.to_string());
        self
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app_state = AppState::new(self.script_path, self.use_cpu);
        let _ = event_loop.run_app(&mut app_state);
    }
}

struct AppState {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    script_path: Option<String>,
    use_cpu: bool,
    resize_request: Option<winit::dpi::PhysicalSize<u32>>,
}

impl AppState {
    fn new(script_path: Option<String>, use_cpu: bool) -> Self {
        Self {
            window: None,
            render_context: None,
            script_path,
            use_cpu,
            resize_request: None,
        }
    }
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("RasmalaiUI");
            
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            
            // Initialize renderer
            // functionality to be added in RenderContext
            self.render_context = Some(RenderContext::new(window.clone(), self.use_cpu).pollster_block_on());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = &self.window {
            if window.id() == window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Close requested");
                        // Explicitly drop resources to ensure clean shutdown
                        self.render_context = None;
                        self.window = None;
                        event_loop.exit();
                    },
                    WindowEvent::RedrawRequested => {
                        if let Some(render_context) = &mut self.render_context {
                            if let Some(size) = self.resize_request.take() {
                                render_context.resize(size);
                            }
                            render_context.render();
                        }
                    }
                    WindowEvent::Resized(size) => {
                         // Defer resize to RedrawRequested to avoid blocking event loop
                         self.resize_request = Some(size);
                         window.request_redraw();
                    }
                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// Placeholder for State to match README usage, though logic is likely elsewhere
pub struct State {
    pub count: i32,
}

impl State {
    pub fn new(count: i32) -> Self {
        Self { count }
    }
}
