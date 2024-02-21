use esprit2::options::{Options, RESOURCE_DIRECTORY, USER_DIRECTORY};
use esprit2::resource_manager::ResourceManager;
use esprit2::{character, console::Console, gui, world};
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::rwops::RWops;
use std::process::exit;
use std::time::Duration;
use tracing::error;

pub fn main() {
    // SDL initialization.
    let sdl_context = sdl2::init().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Esprit 2", 1280, 720)
        .resizable()
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Logging initialization.
    tracing_subscriber::fmt::init();

    // Game initialization.
    let resources = match ResourceManager::open(&*RESOURCE_DIRECTORY, &texture_creator) {
        Ok(resources) => resources,
        Err(msg) => {
            error!("Failed to open resource directory: {msg}");
            exit(1);
        }
    };
    let mut options = Options::open(USER_DIRECTORY.join("options.toml")).unwrap_or_default();
    let mut console = Console::default();
    // Create a piece for the player, and register it with the world manager.
    let player = character::Piece {
        player_controlled: true,
        alliance: character::Alliance::Friendly,
        ..character::Piece::new(resources.get_sheet("luvui").unwrap().clone())
    };
    let ally = character::Piece {
        player_controlled: false,
        alliance: character::Alliance::Enemy,
        ..character::Piece::new(resources.get_sheet("aris").unwrap().clone())
    };
    let mut world_manager = world::Manager {
        location: world::Location {
            level: String::from("New Level"),
            floor: 0,
        },

        current_level: world::Level::default(),
        party: vec![player.id, ally.id],
        inventory: vec![
            "aloe".into(),
            "apple".into(),
            "blinkfruit".into(),
            "fabric_shred".into(),
            "grapes".into(),
            "ice_cream".into(),
            "lily".into(),
            "pear_on_a_stick".into(),
            "pear".into(),
            "pepper".into(),
            "purefruit".into(),
            "raspberry".into(),
            "reviver_seed".into(),
            "ring_alt".into(),
            "ring".into(),
            "scarf".into(),
            "slimy_apple".into(),
            "super_pepper".into(),
            "twig".into(),
            "water_chestnut".into(),
            "watermelon".into(),
        ],
    };
    world_manager.get_floor_mut().characters.push(player);
    world_manager.get_floor_mut().characters.push(ally);
    let sleep_texture = resources.get_texture("luvui_sleep");
    let font = ttf_context
        .load_font_from_rwops(
            RWops::from_bytes(include_bytes!(
                "res/FantasqueSansMNerdFontPropo-Regular.ttf"
            ))
            .unwrap(),
            20,
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
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::Left));
                        }
                        if options.controls.right.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::Right));
                        }
                        if options.controls.up.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::Up));
                        }
                        if options.controls.down.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::Down));
                        }
                        if options.controls.up_left.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::UpLeft));
                        }
                        if options.controls.up_right.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::UpRight));
                        }
                        if options.controls.down_left.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::DownLeft));
                        }
                        if options.controls.down_right.contains(&(keycode as i32)) {
                            next_character.next_action =
                                Some(character::Action::Move(character::OrdDir::DownRight));
                        }
                    }
                }
                _ => {}
            }
        }

        world_manager.pop_action(&mut console);

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
        canvas.set_draw_color(Color::WHITE);
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
                    sleep_texture,
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
        pamphlet.label("Forest: Floor 1/8", &font);
        pamphlet.advance(0, 10);
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
                            &format!("HP: {}/{}", piece.hp, piece.sheet.stats.heart),
                            &font,
                        );
                        player_window.progress_bar(
                            (piece.hp as f32) / (piece.sheet.stats.heart as f32),
                            Color::GREEN,
                            Color::RED,
                            10,
                            5,
                        );
                        player_window.label(
                            &format!("SP: {}/{}", piece.sp, piece.sheet.stats.soul),
                            &font,
                        );
                        player_window.progress_bar(
                            (piece.sp as f32) / (piece.sheet.stats.soul as f32),
                            Color::BLUE,
                            Color::RED,
                            10,
                            5,
                        );
                        let stats = &piece.sheet.stats;
                        let physical_stat_info = [("Pwr", stats.power), ("Def", stats.defense)];
                        let mut physical_stats = [None, None];
                        for ((stat_name, stat), stat_half) in physical_stat_info
                            .into_iter()
                            .zip(physical_stats.iter_mut())
                        {
                            let font = &font;
                            *stat_half = Some(move |stat_half: &mut gui::Context| {
                                stat_half.label(&format!("{stat_name}: {stat}"), font)
                            });
                        }
                        player_window.hsplit(&mut physical_stats);
                        let magical_stat_info = [("Mag", stats.magic), ("Res", stats.resistance)];
                        let mut magical_stats = [None, None];
                        for ((stat_name, stat), stat_half) in
                            magical_stat_info.into_iter().zip(magical_stats.iter_mut())
                        {
                            let font = &font;
                            *stat_half = Some(move |stat_half: &mut gui::Context| {
                                stat_half.label(&format!("{stat_name}: {stat}"), font)
                            });
                        }
                        player_window.hsplit(&mut magical_stats);
                        player_window.label("Spells", &font);
                        let mut spells = (0..6).peekable();
                        while spells.peek().is_some() {
                            let textures_per_row = player_window.rect.width() / (32 + 8);
                            player_window.horizontal();
                            for i in 0..textures_per_row {
                                if let Some(_) = spells.next() {
                                    player_window
                                        .htexture(resources.get_texture("magic_missile"), 32);
                                    player_window.advance(8, 0);
                                }
                            }
                            player_window.vertical();
                            player_window.advance(8, 8);
                        }
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
        pamphlet.advance(0, 10);
        pamphlet.label("Inventory", &font);
        let mut items = world_manager.inventory.iter().peekable();
        while items.peek().is_some() {
            let textures_per_row = pamphlet.rect.width() / (32 + 8);
            pamphlet.horizontal();
            for i in 0..textures_per_row {
                if let Some(item_name) = items.next() {
                    pamphlet.htexture(resources.get_texture(item_name), 32);
                    pamphlet.advance(8, 0);
                }
            }
            pamphlet.vertical();
            pamphlet.advance(8, 8);
        }
        pamphlet.advance(0, 10);
        pamphlet.label("Options", &font);
        pamphlet.label("- Settings", &font);
        pamphlet.label("- Escape", &font);
        pamphlet.label("- Quit", &font);

        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
