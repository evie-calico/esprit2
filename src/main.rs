use esprit2::options::{RESOURCE_DIRECTORY, USER_DIRECTORY};
use esprit2::prelude::*;
use esprit2::world::CharacterRef;
use sdl2::render::Texture;
use sdl2::{pixels::Color, rect::Rect, rwops::RWops};
use std::process::exit;
use std::str::FromStr;
use tracing::*;
use uuid::Uuid;

fn update_delta(
	last_time: &mut f64,
	current_time: &mut f64,
	timer_subsystem: &sdl2::TimerSubsystem,
) -> f64 {
	*last_time = *current_time;
	*current_time = timer_subsystem.performance_counter() as f64;
	((*current_time - *last_time) * 1000.0
				    / (timer_subsystem.performance_frequency() as f64))
				    // Convert milliseconds to seconds.
				    / 1000.0
}

struct SoulJar<'texture> {
	souls: Vec<Soul>,
	light_texture: Texture<'texture>,
}

impl SoulJar<'_> {
	fn tick(&mut self, delta: f32) {
		for i in &mut self.souls {
			i.tick(delta);
		}
	}
}

pub fn main() {
	// SDL initialization.
	let sdl_context = sdl2::init().unwrap();
	let ttf_context = sdl2::ttf::init().unwrap();
	let video_subsystem = sdl_context.video().unwrap();
	let timer_subsystem = sdl_context.timer().unwrap();
	let window = video_subsystem
		.window("Esprit 2", 1280, 720)
		.resizable()
		.position_centered()
		.build()
		.unwrap();

	let mut canvas = window
		.into_canvas()
		.accelerated()
		.present_vsync()
		.build()
		.unwrap();
	let texture_creator = canvas.texture_creator();
	let mut event_pump = sdl_context.event_pump().unwrap();

	let mut current_time = timer_subsystem.performance_counter() as f64;
	let mut last_time = current_time;

	// Logging initialization.
	tracing_subscriber::fmt::init();

	struct FakeStats;

	impl expression::Variables for FakeStats {
		fn get<'expression>(
			&self,
			_: &'expression str,
		) -> Result<u32, expression::Error<'expression>> {
			Ok(1)
		}
	}

	info!(
		"{:?}",
		expression::Equation::from_str("2 + magic * 3")
			.unwrap()
			.eval(&FakeStats)
	);

	// Game initialization.
	let resources = match ResourceManager::open(&*RESOURCE_DIRECTORY, &texture_creator) {
		Ok(resources) => resources,
		Err(msg) => {
			error!("Failed to open resource directory: {msg}");
			exit(1);
		}
	};
	let options = Options::open(USER_DIRECTORY.join("options.toml")).unwrap_or_else(|msg| {
		error!("failed to open options.toml: {msg}");
		Options::default()
	});
	// Create a piece for the player, and register it with the world manager.
	let party = [
		(
			Uuid::new_v4(),
			resources.get_sheet("luvui").unwrap().clone(),
		),
		(Uuid::new_v4(), resources.get_sheet("aris").unwrap().clone()),
	];
	let player = character::Piece {
		player_controlled: true,
		alliance: character::Alliance::Friendly,
		..character::Piece::new(party[0].1.clone(), &resources)
	};
	let ally = character::Piece {
		player_controlled: false,
		alliance: character::Alliance::Friendly,
		..character::Piece::new(party[1].1.clone(), &resources)
	};
	let mut world_manager = world::Manager {
		location: world::Location {
			level: String::from("New Level"),
			floor: 0,
		},
		console: Console::default(),

		current_level: world::Level::default(),
		current_floor: Floor::default(),
		characters: Vec::new(),
		items: Vec::new(),

		party: vec![
			world::PartyReference::new(player.id, party[0].0),
			world::PartyReference::new(ally.id, party[1].0),
		],
		inventory: vec![
			"items/aloe".into(),
			"items/apple".into(),
			"items/blinkfruit".into(),
			"items/fabric_shred".into(),
			"items/grapes".into(),
			"items/ice_cream".into(),
			"items/lily".into(),
			"items/pear_on_a_stick".into(),
			"items/pear".into(),
			"items/pepper".into(),
			"items/purefruit".into(),
			"items/raspberry".into(),
			"items/reviver_seed".into(),
			"items/ring_alt".into(),
			"items/ring".into(),
			"items/scarf".into(),
			"items/slimy_apple".into(),
			"items/super_pepper".into(),
			"items/twig".into(),
			"items/water_chestnut".into(),
			"items/watermelon".into(),
		],
	};
	world_manager.characters.push(CharacterRef::new(player));
	world_manager.characters.push(CharacterRef::new(ally));
	world_manager.apply_vault(1, 1, resources.get_vault("example").unwrap(), &resources);
	let sleep_texture = resources.get_texture("luvui_sleep");
	let font = ttf_context
		.load_font_from_rwops(
			RWops::from_bytes(include_bytes!(
				"res/FantasqueSansMNerdFontPropo-Regular.ttf"
			))
			.unwrap(),
			options.ui.font_size,
		)
		.unwrap();

	let mut soul_jar = SoulJar {
		souls: vec![
			Soul::new(Color::RED),
			Soul::new(Color::YELLOW),
			Soul::new(Color::BLUE),
			Soul::new(Color::GREEN),
			Soul::new(Color::CYAN),
			Soul::new(Color::RGB(255, 128, 0)),
			Soul::new(Color::WHITE),
			Soul::new(Color::RGB(255, 0, 255)),
			Soul::new(Color::RGB(255, 128, 128)),
		],
		light_texture: resources.get_owned_texture("light").unwrap(),
	};

	// Print some debug messages to test the console.
	world_manager.console.print("Hello, world!");
	world_manager.console.print("Luvui scratches the cat.");
	world_manager.console.print_defeat("The cat ran away.");
	world_manager.console.print("Luvui casts Magic Missile.");
	world_manager
		.console
		.print("Her magic missile strikes the cat!");
	world_manager.console.print("The cat scratches Aris");
	world_manager.console.print("Aris bites the cat");
	world_manager.console.print_defeat("The cat scampered off.");
	world_manager
		.console
		.print_special("Luvui's level increased to 2!");

	// TODO: Display this on-screen.
	let mut input_mode = input::Mode::Normal;
	let mut global_time = 0;
	loop {
		// Input processing
		if input::world(
			&mut event_pump,
			&mut world_manager,
			&mut input_mode,
			&options,
		)
		.exit
		{
			break;
		};

		// Logic
		// This is the only place where delta time should be used.
		{
			let delta = update_delta(&mut last_time, &mut current_time, &timer_subsystem);

			world_manager.pop_action();
			world_manager.console.update(delta);
			soul_jar.tick(delta as f32);
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
		global_time = (global_time + 1) % 255;
		canvas.set_draw_color(Color::RGB(global_time, 64, 255 - global_time));
		canvas
			.fill_rect(Rect::new(0, 0, window_size.0, window_size.1))
			.unwrap();

		// Draw tilemap
		canvas.set_draw_color(Color::WHITE);
		for (x, col) in world_manager.current_floor.map.iter_cols().enumerate() {
			for (y, tile) in col.enumerate() {
				if *tile == floor::Tile::Wall {
					canvas
						.fill_rect(Rect::new((x as i32) * 64, (y as i32) * 64, 64, 64))
						.unwrap();
				}
			}
		}

		// Draw characters
		for character in world_manager.characters.iter().map(|x| x.borrow()) {
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

		let mut menu = gui::Context::new(
			&mut canvas,
			Rect::new(
				0,
				(window_size.1 - options.ui.console_height) as i32,
				window_size.0 - options.ui.pamphlet_width,
				options.ui.console_height,
			),
		);

		match input_mode {
			input::Mode::Normal => {
				// Draw Console
				world_manager.console.draw(
					&mut canvas,
					Rect::new(
						0,
						(window_size.1 - options.ui.console_height) as i32,
						window_size.0 - options.ui.pamphlet_width,
						options.ui.console_height,
					),
					&font,
				);
			}
			input::Mode::Cast => {
				spell_menu::draw(&mut menu, &font);
			}
		}

		// Draw pamphlet
		pamphlet(
			&mut canvas,
			window_size,
			&options,
			&font,
			&world_manager,
			&resources,
			&mut soul_jar,
		);

		canvas.present();
	}
}

fn pamphlet(
	canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
	window_size: (u32, u32),
	options: &Options,
	font: &sdl2::ttf::Font<'_, '_>,
	world_manager: &world::Manager,
	resources: &ResourceManager<'_>,
	soul_jar: &mut SoulJar<'_>,
) {
	let mut pamphlet = gui::Context::new(
		canvas,
		Rect::new(
			(window_size.0 - options.ui.pamphlet_width) as i32,
			0,
			options.ui.pamphlet_width,
			window_size.1,
		),
	);
	pamphlet.label("Forest: Floor 1/8", font);
	pamphlet.advance(0, 10);
	// Draw party stats
	for character_chunk in world_manager.party.chunks(2) {
		let mut character_windows = [None, None];
		for (character_id, window) in character_chunk.iter().zip(character_windows.iter_mut()) {
			*window = Some(|player_window: &mut gui::Context| {
				if let Some(piece) = world_manager.get_character(character_id.piece) {
					let piece = piece.borrow();
					player_window.label_color(
						&format!(
							"{} ({:08x})",
							piece.sheet.nouns.name,
							piece.id.as_fields().0
						),
						match piece.sheet.nouns.pronouns {
							nouns::Pronouns::Female => Color::RGB(247, 141, 246),
							nouns::Pronouns::Male => Color::RGB(104, 166, 232),
							_ => Color::WHITE,
						},
						font,
					);
					player_window.label(&format!("Level {}", piece.sheet.level), font);
					player_window.label(
						&format!("HP: {}/{}", piece.hp, piece.sheet.stats.heart),
						font,
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
						font,
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
						*stat_half = Some(move |stat_half: &mut gui::Context| {
							stat_half.label(&format!("{stat_name}: {stat}"), font)
						});
					}
					player_window.hsplit(&mut magical_stats);
					player_window.label("Spells", font);
					let mut spells = piece.sheet.spells.iter().peekable();
					while spells.peek().is_some() {
						let textures_per_row = player_window.rect.width() / (32 + 8);
						player_window.horizontal();
						for _ in 0..textures_per_row {
							if let Some(spell) = spells.next() {
								player_window.htexture(resources.get_texture(spell), 32);
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
					player_window.label("???", font);
				}
			});
		}
		pamphlet.hsplit(&mut character_windows);
	}
	pamphlet.advance(0, 10);

	let mut inventory_fn = |pamphlet: &mut gui::Context| {
		pamphlet.label("Inventory", font);
		let mut items = world_manager.inventory.iter().peekable();
		while items.peek().is_some() {
			let textures_per_row = pamphlet.rect.width() / (32 + 8);
			pamphlet.horizontal();
			for _ in 0..textures_per_row {
				if let Some(item_name) = items.next() {
					pamphlet.htexture(resources.get_texture(item_name), 32);
					pamphlet.advance(8, 0);
				}
			}
			pamphlet.vertical();
			pamphlet.advance(8, 8);
		}
	};
	let mut souls_fn = |pamphlet: &mut gui::Context| {
		const SOUL_SIZE: u32 = 50;
		pamphlet.label("Souls", font);

		let bx = pamphlet.x as f32;
		let by = pamphlet.y as f32;
		let display_size = pamphlet.rect.width();

		for soul in &soul_jar.souls {
			let display_size = (display_size - SOUL_SIZE) as f32;
			let ox = soul.x * display_size;
			let oy = soul.y * display_size;
			soul_jar
				.light_texture
				.set_color_mod(soul.color.r, soul.color.g, soul.color.b);
			pamphlet
				.canvas
				.copy(
					&soul_jar.light_texture,
					None,
					Rect::new((bx + ox) as i32, (by + oy) as i32, SOUL_SIZE, SOUL_SIZE),
				)
				.unwrap();
		}
		pamphlet.advance(0, display_size);
	};
	pamphlet.hsplit(&mut [
		Some((&mut inventory_fn) as &mut dyn FnMut(&mut gui::Context)),
		Some(&mut souls_fn),
	]);
}
