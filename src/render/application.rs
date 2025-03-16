use std::sync::Arc;
use crate::render::state::*;
use tokio::runtime::Runtime;

use winit::{
  application::ApplicationHandler,
  event::{KeyEvent, WindowEvent::*},
  keyboard::{Key, NamedKey},
  window::{Window, WindowAttributes},
};

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
      CloseRequested => {
        self.state = None;
        self.window = None;
        event_loop.exit();
      }
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
  fn state(&self) -> &State {
    if let Some(ref s) = self.state {
      s
    } else {
      panic!("")
    }
  }
}
