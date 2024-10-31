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
                let start = Instant::now();
                if self.selection.len() >= 3 {
                    let inside_mask = selection_mask(iw as usize, ih as usize, &self.selection);
                    for (i, rgba) in self.image.pixels().enumerate() {
                        let color: &Rgba<u8> = rgba;
                        frame[i * 4 + 0] = color[0];
                        frame[i * 4 + 1] = color[1];
                        frame[i * 4 + 2] = color[2];
                        // reduce alpha if outside of selection
                        if inside_mask[i] {
                            frame[i * 4 + 3] = color[3];
                        } else {
                            frame[i * 4 + 3] = color[3] / 2;
                        }
                    }
                } else {
                    for (i, rgba) in self.image.pixels().enumerate() {
                        let color: &Rgba<u8> = rgba;
                        frame[i * 4 + 0] = color[0];
                        frame[i * 4 + 1] = color[1];
                        frame[i * 4 + 2] = color[2];
                        frame[i * 4 + 3] = color[3];
                    }
                }
                println!("Drawing took {} ms", start.elapsed().as_millis());
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

fn selection_mask(width: usize, height: usize, polygon: &Vec<PhysicalPosition<f64>>) -> Vec<bool> {
    let mut mask = vec![false; width * height];
    let len = polygon.len();
    if len < 3 {
        return mask;
    }

    // Find bounding box for the polygon
    let min_y = polygon.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = polygon
        .iter()
        .map(|p| p.y)
        .fold(f64::NEG_INFINITY, f64::max);

    // Limit scan lines to within the image height
    let min_y = min_y.max(0.0).ceil() as usize;
    let max_y = max_y.min((height - 1) as f64).floor() as usize;

    for y in min_y..=max_y {
        let mut intersections = Vec::new();

        for i in 0..len {
            let j = (i + 1) % len;
            let (p1, p2) = (&polygon[i], &polygon[j]);

            if (p1.y <= y as f64 && p2.y > y as f64) || (p2.y <= y as f64 && p1.y > y as f64) {
                let slope = (p2.x - p1.x) / (p2.y - p1.y);
                let intersect_x = p1.x + (y as f64 - p1.y) * slope;
                intersections.push(intersect_x);
            }
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        for span in intersections.chunks(2) {
            if let [start, end] = span {
                let start = start.max(0.0).ceil() as usize;
                let end = end.min((width - 1) as f64).floor() as usize;

                for x in start..=end {
                    if x < width {
                        mask[y * width + x] = true;
                    }
                }
            }
        }
    }
    mask
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
