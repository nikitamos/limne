use crate::render::state::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

use winit::{
  application::ApplicationHandler,
  event::{KeyEvent, WindowEvent::*},
  keyboard::{Key, NamedKey},
  window::{Window, WindowAttributes},
};

use super::simulation::two_d;

pub struct App<'a> {
  runtime: Runtime,
  window: Option<Arc<Window>>,
  state: Option<State<'a>>,
}

impl<'a> ApplicationHandler for App<'a> {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if self.window.is_none() {
      self.window.replace(Arc::new(
        event_loop
          .create_window(WindowAttributes::default())
          .expect("Error creating a window"),
      ));
      self
        .state
        .replace(self.runtime.block_on(State::create(self.window())));
    }
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    _win_id: winit::window::WindowId,
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
      RedrawRequested => {
        self.window().request_redraw();

        match self.map_state(|s| s.render()) {
          Ok(()) => (),
          Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
            let size = self.window().inner_size();
            let s = self.state.as_mut().unwrap();
            s.resize(size);
          }
          Err(wgpu::SurfaceError::Timeout) => (),
          _ => event_loop.exit(),
        }
      }
      CloseRequested => {
        self.state = None;
        self.window = None;
        event_loop.exit();
      }
      Resized(size) => self.map_state(|s| s.resize(size)),
      _ => {}
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
  fn map_state<T>(&mut self, f: impl FnOnce(&mut State) -> T) -> T {
    self.state.as_mut().map(f).unwrap()
  }
}
