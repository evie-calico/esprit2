use esprit2::options::{Options, RESOURCE_DIRECTORY, USER_DIRECTORY};
use esprit2::{character, world};
use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::fs;
use std::time::Duration;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Esprit 2", 800, 600)
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let options = Options::default();
    let floor = world::Floor::default();
    let mut player = character::Piece::default();
    let sleep_texture = texture_creator
        .load_texture(RESOURCE_DIRECTORY.join("luvui_sleep.png"))
        .unwrap();
    let font = ttf_context
        .load_font(
            RESOURCE_DIRECTORY.join("FantasqueSansMNerdFontPropo-Regular.ttf"),
            100,
        )
        .unwrap();

    // This is mostly just to see what the toml looks like: very pretty of course.
    fs::write(
        USER_DIRECTORY.join("options.toml"),
        toml::to_string(&options).unwrap(),
    )
    .unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        // Input processing
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

        // Rendering
        // Clear the screen.
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // Configure world viewport.
        const PAMPHLET_WIDTH: u32 = 400;
        const CONSOLE_HEIGHT: u32 = 200;
        let window_size = canvas.window().size();
        canvas.set_viewport(Rect::new(
            0,
            0,
            window_size.0 - PAMPHLET_WIDTH,
            window_size.1 - CONSOLE_HEIGHT,
        ));
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas
            .fill_rect(Rect::new(0, 0, window_size.0, window_size.1))
            .unwrap();

        // Draw tilemap
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        for (x, col) in floor.map.iter_cols().enumerate() {
            for (y, tile) in col.enumerate() {
                if *tile == world::Tile::Wall {
                    canvas
                        .fill_rect(Rect::new((x as i32) * 64, (y as i32) * 64, 64, 64))
                        .unwrap();
                }
            }
        }

        // Draw player
        canvas
            .copy(
                &sleep_texture,
                None,
                Some(Rect::new(player.x * 64, player.y * 64, 64, 64)),
            )
            .unwrap();

        // Configure pamphlet viewport
        canvas.set_viewport(None);

        canvas.set_draw_color(Color::WHITE);

        canvas.copy(
            &font
                .render("Hello, world!")
                .shaded(Color::WHITE, Color::BLACK)
                .unwrap()
                .as_texture(&texture_creator)
                .unwrap(),
            None,
            Rect::new(
                (window_size.0 - PAMPHLET_WIDTH + 50) as i32,
                50,
                PAMPHLET_WIDTH - 100,
                50,
            ),
        );
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
