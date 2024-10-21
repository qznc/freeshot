use image::{ImageBuffer, Rgba};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use xcap::Monitor;

#[derive(Default)]
struct App {
    window: Option<Window>,
    image: ImageBuffer<Rgba<u8>, Vec<u8>>,
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
                println!("Draw!");

                let w = self.window.as_ref().unwrap();
                let size = w.inner_size();
                let iw = self.image.width();
                let ih = self.image.height();
                let ww = size.width;
                let wh = size.height;
                let surface_texture = SurfaceTexture::new(ww, wh, &w);
                let mut pixels = Pixels::new(iw, ih, surface_texture).unwrap();
                let frame = pixels.frame_mut();
                for (i, rgba) in self.image.pixels().enumerate() {
                    let x: &Rgba<u8> = rgba;
                    frame[i * 4 + 0] = x[0];
                    frame[i * 4 + 1] = x[1];
                    frame[i * 4 + 2] = x[2];
                    frame[i * 4 + 3] = x[3];
                }
                pixels.render().expect("rendered");

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                //self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn main() {
    let mut app = App::default();

    // Capture screenshot
    let monitors = Monitor::all().unwrap();
    let monitor = &monitors[0]; // Display the first monitor for simplicity
    let image = monitor.capture_image().unwrap();
    app.image = image;
    //let (width, height) = (image.width(), image.height());
    //let raw_data = image.as_rgba8().unwrap().to_vec(); // Convert the image to raw RGBA data

    // Setup window
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).expect("runs");
}
