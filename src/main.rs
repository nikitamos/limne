use std::error::Error;
use winit::event_loop::EventLoop;

mod math;
mod render;

fn main() -> Result<(), Box<dyn Error>> {
  let event_loop = EventLoop::builder().build()?;
  event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
  let mut app = render::application::App::new();
  event_loop
    .run_app(&mut app)
    .expect("Error running event loop");
  Ok(())
}
