use esprit2::character;
use esprit2::options::{Options, Shortcut, RESOURCE_DIRECTORY, USER_DIRECTORY};
use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::fs;
use std::time::Duration;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Esprit 2", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let options = Options::default();
    let mut player = character::Piece::default();
    let sleep_texture = texture_creator
        .load_texture(RESOURCE_DIRECTORY.join("luvui_sleep.png"))
        .unwrap();

    // This is mostly just to see what the toml looks like: very pretty of course.
    fs::write(
        USER_DIRECTORY.join("options.toml"),
        toml::to_string(&options).unwrap(),
    )
    .unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if options.controls.left.contains(&(keycode as i32)) {
                        player.x -= 1;
                    }
                    if options.controls.right.contains(&(keycode as i32)) {
                        player.x += 1;
                    }
                    if options.controls.up.contains(&(keycode as i32)) {
                        player.y -= 1;
                    }
                    if options.controls.down.contains(&(keycode as i32)) {
                        player.y += 1;
                    }
                }
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas
            .copy(
                &sleep_texture,
                None,
                Some(Rect::new(player.x * 64, player.y * 64, 64, 64)),
            )
            .unwrap();

        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
