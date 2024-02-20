use esprit2::options::{Options, RESOURCE_DIRECTORY, USER_DIRECTORY};
use esprit2::{character, console::Console, gui, world};
use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::fs;
use std::time::Duration;

pub fn main() {
    // SDL initialization.
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

    // Game initialization.
    let mut options = Options::default();
    let mut console = Console::default();
    // Create a piece for the player, and register it with the world manager.
    let player = character::Piece {
        player_controlled: true,
        alliance: character::Alliance::Friendly,
        sheet: toml::from_str(
            &fs::read_to_string(RESOURCE_DIRECTORY.join("party/luvui.toml")).unwrap(),
        )
        .unwrap(),
        ..Default::default()
    };
    let ally = character::Piece {
        player_controlled: false,
        alliance: character::Alliance::Friendly,
        sheet: toml::from_str(
            &fs::read_to_string(RESOURCE_DIRECTORY.join("party/aris.toml")).unwrap(),
        )
        .unwrap(),
        ..Default::default()
    };
    let mut world_manager = world::Manager {
        location: world::Location {
            level: String::from("New Level"),
            floor: 0,
        },

        current_level: world::Level::default(),
        party: vec![player.id, ally.id],
    };
    world_manager.get_floor_mut().characters.push(player);
    world_manager.get_floor_mut().characters.push(ally);
    let sleep_texture = texture_creator
        .load_texture(RESOURCE_DIRECTORY.join("luvui_sleep.png"))
        .unwrap();
    let font = ttf_context
        .load_font(
            RESOURCE_DIRECTORY.join("FantasqueSansMNerdFontPropo-Regular.ttf"),
            24,
        )
        .unwrap();

    // Print some debug messages to test the console.
    console.print("Hello, world!");
    console.print("Luvui scratches the cat.");
    console.print_defeat("The cat ran away.");
    console.print("Luvui casts Magic Missile.");
    console.print("Her magic missile strikes the cat!");
    console.print("The cat scratches Aris");
    console.print("Aris bites the cat");
    console.print_defeat("The cat scampered off.");
    console.print_special("Luvui's level increased to 2!");

    // This is mostly just to see what the toml looks like: very pretty of course.
    fs::write(
        USER_DIRECTORY.join("options.toml"),
        toml::to_string(&options).unwrap(),
    )
    .unwrap();

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
                    // This will need to be refactored.
                    let next_character = world_manager.next_character();
                    if next_character.player_controlled {
                        if options.controls.left.contains(&(keycode as i32)) {
                            options.ui.pamphlet_width += 50;
                            next_character.x -= 1;
                        }
                        if options.controls.right.contains(&(keycode as i32)) {
                            options.ui.pamphlet_width -= 50;
                            next_character.x += 1;
                        }
                        if options.controls.up.contains(&(keycode as i32)) {
                            options.ui.console_height += 50;
                            next_character.y -= 1;
                        }
                        if options.controls.down.contains(&(keycode as i32)) {
                            options.ui.console_height -= 50;
                            next_character.y += 1;
                        }
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
        let window_size = canvas.window().size();
        canvas.set_viewport(Rect::new(
            0,
            0,
            window_size.0 - options.ui.pamphlet_width,
            window_size.1 - options.ui.console_height,
        ));
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas
            .fill_rect(Rect::new(0, 0, window_size.0, window_size.1))
            .unwrap();

        // Draw tilemap
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        for (x, col) in world_manager.get_floor().map.iter_cols().enumerate() {
            for (y, tile) in col.enumerate() {
                if *tile == world::Tile::Wall {
                    canvas
                        .fill_rect(Rect::new((x as i32) * 64, (y as i32) * 64, 64, 64))
                        .unwrap();
                }
            }
        }

        // Draw characters
        for character in &world_manager.get_floor().characters {
            canvas
                .copy(
                    &sleep_texture,
                    None,
                    Some(Rect::new(character.x * 64, character.y * 64, 64, 64)),
                )
                .unwrap();
        }

        // Render User Interface
        canvas.set_viewport(None);

        // Draw Console
        console.draw(
            &mut canvas,
            Rect::new(
                0,
                (window_size.1 - options.ui.console_height) as i32,
                window_size.0 - options.ui.pamphlet_width,
                options.ui.console_height,
            ),
            &font,
        );

        // Draw pamphlet
        let mut pamphlet = gui::Context::new(
            &mut canvas,
            Rect::new(
                (window_size.0 - options.ui.pamphlet_width) as i32,
                0,
                options.ui.pamphlet_width,
                window_size.1,
            ),
        );
        // Draw party stats
        for character_chunk in world_manager.party.chunks(2) {
            let mut character_windows = [None, None];
            for (character_id, window) in character_chunk.iter().zip(character_windows.iter_mut()) {
                *window = Some(|player_window: &mut gui::Context| {
                    if let Some(piece) = world_manager.get_character(*character_id) {
                        player_window.label_color(
                            &format!(
                                "{} ({:08x})",
                                piece.sheet.nouns.name,
                                piece.id.as_fields().0
                            ),
                            match piece.sheet.nouns.pronouns {
                                character::Pronouns::Female => Color::RGB(247, 141, 246),
                                character::Pronouns::Male => Color::RGB(104, 166, 232),
                                _ => Color::WHITE,
                            },
                            &font,
                        );
                        player_window.label(&format!("Level {}", piece.sheet.level), &font);
                        player_window.label(
                            &format!(
                                "HP: {}/{}",
                                piece.sheet.stats.heart, piece.sheet.stats.heart
                            ),
                            &font,
                        );
                        player_window.progress_bar(0.6, Color::GREEN, Color::RED, 10, 10);
                        player_window.label(
                            &format!("SP: {}/{}", piece.sheet.stats.soul, piece.sheet.stats.soul),
                            &font,
                        );
                        player_window.progress_bar(1.0, Color::BLUE, Color::RED, 10, 10);
                        let stats = &piece.sheet.stats;
                        player_window.label(&format!("Pwr: {}", stats.power), &font);
                        player_window.label(&format!("Def: {}", stats.defense), &font);
                        player_window.label(&format!("Mag: {}", stats.magic), &font);
                        player_window.label(&format!("Res: {}", stats.resistance), &font);
                    } else {
                        // If the party array also had a reference to the character's last known character sheet,
                        // a name could be displayed here.
                        // I don't actually know if this is desirable;
                        // this should probably never happen anyways.
                        player_window.label("???", &font);
                    }
                });
            }
            pamphlet.hsplit(&mut character_windows);
        }

        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
