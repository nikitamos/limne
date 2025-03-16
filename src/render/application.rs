use std::sync::Arc;

use tokio::{runtime::{Handle, Runtime}, task::spawn_blocking};
use wgpu::{
  Backends, Instance, InstanceDescriptor, RequestAdapterOptions, Surface,
};
use winit::{
  application::ApplicationHandler,
  event::{KeyEvent, WindowEvent::*},
  keyboard::{Key, NamedKey},
  window::{Window, WindowAttributes},
};

struct State<'a> {
  instance: Instance,
  surface: Surface<'a>,
}

pub struct App<'a> {
  runtime: Runtime,
  window: Option<Arc<Window>>,
  state: Option<State<'a>>,
}

impl<'a> State<'a> {
  pub async fn create(window: Arc<Window>) -> Self {
    let instance = wgpu::Instance::new(&InstanceDescriptor {
      backends: Backends::PRIMARY,
      ..Default::default()
    });
    let surface = instance
      .create_surface(window)
      .expect("Unable to create a surface");
    let adapter = instance
      .request_adapter(&RequestAdapterOptions {
        compatible_surface: Some(&surface),
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
      })
      .await
      .expect("Unable to create an adapter!");
    Self { instance, surface }
  }
}

impl<'a> ApplicationHandler for App<'a> {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if self.window.is_none() {
      self.window.replace(Arc::new(
        event_loop
          .create_window(WindowAttributes::default())
          .expect("Error creating a window"),
      ));
      self.state.replace(self.runtime.block_on(State::create(self.window())));
    }
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
  ) {
    match event {
      KeyboardInput {
        event:
          KeyEvent {
            logical_key: Key::Named(NamedKey::Escape),
            state,
            ..
          },
        ..
      } if state.is_pressed() => println!("ESC pressed"),
      _ => (),
    }
  }
}

impl App<'_> {
  pub fn new() -> Self {
    Self {
      runtime: tokio::runtime::Builder::new_multi_thread().build().unwrap(),
      window: None,
      state: None,
    }
  }
  fn window(&self) -> Arc<Window> {
    if let Some(ref w) = self.window {
      Arc::clone(w)
    } else {
      panic!("")
    }
  }
  fn state(&self) -> &State {
    if let Some(ref s) = self.state {
      s
    } else {
      panic!("")
    }
  }
}
