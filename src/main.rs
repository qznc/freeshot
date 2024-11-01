use std::cmp::{max, min};
use std::time::Instant;

use arboard::Clipboard;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba};
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
    pixels: Option<Pixels>,
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
            pixels: None,
        }
    }

    fn selection_image(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let iw = self.image.width();
        let ih = self.image.height();
        let mut min_x: u32 = iw;
        let mut min_y: u32 = ih;
        let mut max_x: u32 = 0;
        let mut max_y: u32 = 0;
        for pos in self.selection.clone() {
            min_x = min(min_x, pos.x as u32);
            min_y = min(min_y, pos.y as u32);
            max_x = max(max_x, pos.x as u32);
            max_y = max(max_y, pos.y as u32);
        }
        println!("Selection size is {} x {}", max_x - min_x, max_y - min_y);
        let mut image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::new(max_x - min_x, max_y - min_y);
        let inside_mask = selection_mask(iw as usize, ih as usize, &self.selection);
        for y in min_y..max_y {
            for x in min_x..max_x {
                unsafe {
                    let pixel = self.image.unsafe_get_pixel(x, y);
                    let i = (y * iw) + x;
                    if inside_mask[i as usize] {
                        image.unsafe_put_pixel(x - min_x, y - min_y, pixel);
                    }
                }
            }
        }
        image
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let height = self.image.height() * 10 / 11;
        let width = self.image.width() * 10 / 11;
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
                let w = self.window.as_ref().unwrap();
                let _window_size = w.inner_size();
                let iw = self.image.width();
                let ih = self.image.height();
                if self.pixels.is_none() {
                    let surface_texture = SurfaceTexture::new(iw, ih, &w);
                    let pixels = Pixels::new(iw, ih, surface_texture).unwrap();
                    self.pixels = Some(pixels);
                }
                let pixels = self.pixels.as_mut().unwrap();
                let frame = pixels.frame_mut();
                // TODO the following can probably be done faster somehow?!
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
                    self.selecting = true;
                    self.selection = vec![];
                } else {
                    println!("Mouse up, finishing");
                    self.selecting = false;
                    provide_image_for_pasting(&self.selection_image());
                    // keep process alive for pasting!
                    //event_loop.exit();
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

fn provide_image_for_pasting(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let mut clipboard = Clipboard::new().unwrap();
    let raw_rgba = image.clone().into_raw();
    let clip_image = arboard::ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: std::borrow::Cow::Owned(raw_rgba),
    };
    clipboard.set_image(clip_image).unwrap();
    println!("Image copied to clipboard!");
    // TODO keep process alive?!
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
