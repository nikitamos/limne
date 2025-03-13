#![feature(tuple_trait)]

use raylib::{self, color::Color, ffi::KeyboardKey, prelude::RaylibDraw};

mod math;

fn main() {
    let (mut h, t) = raylib::init()
        .title("Some fluid sim")
        .size(1200, 800)
        .build();
    h.set_target_fps(60);
    h.set_exit_key(Some(KeyboardKey::KEY_ESCAPE));

    let mut i = 1;
    while !h.window_should_close() {
        let mut draw = h.begin_drawing(&t);
        draw.clear_background(Color::WHITE);
        draw.draw_text(&format!("Hello world {i}!"), 600, 380, 20, Color::AQUA);
        i += 1;
    }
}
