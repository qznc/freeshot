use std::time::Instant;

use image::{ImageBuffer, Rgba};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use xcap::Monitor;

struct App {
    window: Option<Window>,
    image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    cursor_pos: PhysicalPosition<f64>,
    selecting: bool,
    last_selection_event: Instant,
    selection: Vec<PhysicalPosition<f64>>,
}

impl App {
    fn new(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> App {
        App {
            window: None,
            image,
            cursor_pos: PhysicalPosition { x: 0.0, y: 0.0 },
            selecting: false,
            last_selection_event: Instant::now(),
            selection: vec![],
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let height = self.image.height() * 10 / 15;
        let width = self.image.width() * 10 / 15;
        let attr = Window::default_attributes()
            .with_title("Freeshot!")
            .with_inner_size(LogicalSize::new(width, height));
        self.window = Some(event_loop.create_window(attr).unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                println!("Draw! Selection length is {}", self.selection.len());

                let w = self.window.as_ref().unwrap();
                let _window_size = w.inner_size();
                let iw = self.image.width();
                let ih = self.image.height();
                let surface_texture = SurfaceTexture::new(iw, ih, &w);
                let mut pixels = Pixels::new(iw, ih, surface_texture).unwrap();
                let frame = pixels.frame_mut();
                for (i, rgba) in self.image.pixels().enumerate() {
                    let x: f64 = i as f64 % iw as f64;
                    let y: f64 = i as f64 / iw as f64;
                    let color: &Rgba<u8> = rgba;
                    frame[i * 4 + 0] = color[0];
                    frame[i * 4 + 1] = color[1];
                    frame[i * 4 + 2] = color[2];
                    // reduce alpha if outside of selection
                    if is_inside(x, y, &self.selection) {
                        frame[i * 4 + 3] = color[3];
                    } else {
                        frame[i * 4 + 3] = color[3] / 2;
                    }
                }
                pixels.render().expect("rendered");
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.cursor_pos = position;
                let now = Instant::now();
                let elapsed = now - self.last_selection_event;
                if self.selecting && elapsed.as_millis() >= 100 {
                    println!("Push position {:?}", position);
                    self.selection.push(position);
                    // redraw to make the selection visible correctly
                    self.window.as_ref().unwrap().request_redraw();
                    self.last_selection_event = now;
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button: _,
            } => {
                if state.is_pressed() {
                    println!("Mouse down");
                    self.selecting = true;
                    self.selection = vec![];
                } else {
                    // released
                    println!("Mouse up, finishing");
                    self.selecting = false;
                    event_loop.exit();
                }
            }
            _ => (),
        }
    }
}

// Function to check if point (x, y) is inside the polygon
fn is_inside(x: f64, y: f64, polygon: &Vec<PhysicalPosition<f64>>) -> bool {
    let len = polygon.len();
    if len == 0 {
        // no selection -> always inside
        return true;
    }
    if len < 3 {
        return false; // A polygon must have at least 3 vertices
    }
    let mut intersections = 0;

    // Iterate through each edge of the polygon
    for i in 0..len {
        let j = (i + 1) % len;
        let (p1, p2) = (&polygon[i], &polygon[j]);

        // Check if the ray intersects the edge
        if (p1.y > y) != (p2.y > y) {
            let slope = (p2.x - p1.x) / (p2.y - p1.y);
            let intersect_x = p1.x + (y - p1.y) * slope;

            if x < intersect_x {
                intersections += 1;
            }
        }
    }

    intersections % 2 != 0
}

fn main() {
    // Capture screenshot
    let monitors = Monitor::all().unwrap();
    let monitor = &monitors[0]; // Display the first monitor for simplicity
    let image = monitor.capture_image().unwrap();
    let mut app = App::new(image);

    // Setup window
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).expect("runs");
}
