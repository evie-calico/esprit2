use esprit2::prelude::*;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::fs;
use std::process::exit;
use tracing::{error, info};

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

	video_subsystem.vulkan_load_library_default().unwrap();

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

	// Game initialization.
	let resources = match resource::Manager::open(options::resource_directory(), &texture_creator) {
		Ok(resources) => resources,
		Err(msg) => {
			error!("failed to open resource directory: {msg}");
			exit(1);
		}
	};
	let options_path = options::user_directory().join("options.toml");
	let options = Options::open(&options_path).unwrap_or_else(|msg| {
		// This is `info` because it's actually very expected for first-time players.
		info!("failed to open options.toml: {msg}");
		info!("initializing options.toml instead");
		// Attempt to save the old file, in case it exists.
		if let Err(msg) = fs::rename(&options_path, options_path.with_extension("toml.old")) {
			info!("failed to backup existing options.toml: {msg}");
		} else {
			info!("exiting options.toml was backed up to options.toml.old");
		}
		let options = Options::default();
		if let Err(msg) = fs::write(&options_path, toml::to_string(&options).unwrap()) {
			error!("failed to initialize options.toml: {msg}");
		}
		options
	});
	// Create a piece for the player, and register it with the world manager.
	let party_blueprint = [
		world::PartyReferenceBase {
			sheet: "luvui",
			accent_color: (0xDA, 0x2D, 0x5C, 0xFF),
		},
		world::PartyReferenceBase {
			sheet: "aris",
			accent_color: (0x0C, 0x94, 0xFF, 0xFF),
		},
	];
	let lua = mlua::Lua::new();
	lua.globals()
		.get::<&str, mlua::Table>("package")
		.unwrap()
		.set(
			"path",
			options::resource_directory()
				.join("scripts/?.lua")
				.to_str()
				.unwrap(),
		)
		.unwrap();
	let mut world_manager =
		world::Manager::new(party_blueprint.into_iter(), &resources, &lua, &options)
			.unwrap_or_else(|msg| {
				error!("failed to initialize world manager: {msg}");
				exit(1);
			});
	if let Err(msg) =
		world_manager.apply_vault(1, 1, resources.get_vault("example").unwrap(), &resources)
	{
		error!("failed to apply vault \"example\": {msg}");
	}

	let typography = Typography::new(&options.ui.typography, &ttf_context);

	let mut soul_jar = gui::widget::SoulJar::new(&resources).unwrap_or_else(|msg| {
		error!("failed to initialize soul jar: {msg}");
		exit(1);
	});
	// This disperses the souls enough to cause them to fly in from the sides
	// the same effect can be seen if a computer is put to sleep and then woken up.
	soul_jar.tick(5.0);
	let mut cloudy_wave = draw::CloudyWave::default();

	let mut input_mode = input::Mode::Normal;
	let mut action_request = None;
	let mut fps = 60.0;
	let mut fps_timer = 0.0;
	let mut debug = false;
	loop {
		// Input processing
		{
			let next_character = world_manager.next_character().clone();
			if next_character.borrow().player_controlled {
				match input::controllable_character(
					&mut event_pump,
					next_character,
					&mut world_manager,
					&resources,
					&lua,
					&mut input_mode,
					&options,
				) {
					Ok(Some(input::Response::Exit)) => break,
					Ok(Some(input::Response::Fullscreen)) => {
						use sdl2::video::FullscreenType;
						match canvas.window().fullscreen_state() {
							FullscreenType::Off => {
								let _ = canvas.window_mut().set_fullscreen(FullscreenType::Desktop);
							}
							FullscreenType::True | FullscreenType::Desktop => {
								let _ = canvas.window_mut().set_fullscreen(FullscreenType::Off);
							}
						}
					}
					Ok(Some(input::Response::Debug)) => debug ^= true,
					Ok(None) => (),
					Err(msg) => {
						error!("world input processing returned an error: {msg}");
					}
				}
			} else {
				let considerations = world_manager.consider_turn(&lua).unwrap();
				let action = world_manager
					.consider_action(&lua, next_character, considerations)
					.unwrap();
				world_manager.next_character().borrow_mut().next_action = Some(action);
			}
		}
		// Logic
		{
			// This is the only place where delta time should be used.
			let delta = update_delta(&mut last_time, &mut current_time, &timer_subsystem);

			fps_timer += delta;
			if fps_timer > 0.3 {
				fps_timer = 0.0;
				fps = (fps + 1.0 / delta) / 2.0;
			}

			for i in &mut world_manager.party {
				i.draw_state.cloud.tick(delta);
				i.draw_state.cloud_trail.tick(delta / 4.0);
			}
			match world_manager.update(action_request, &lua, &mut input_mode) {
				Ok(result) => action_request = result,
				Err(msg) => {
					error!("world manager update returned an error: {msg}");
					action_request = None;
				}
			}
			world_manager
				.characters
				.retain(|character| character.borrow().hp > 0);
			world_manager.console.update(delta);
			soul_jar.tick(delta as f32);
			cloudy_wave.tick(delta);
			if let input::Mode::Cursor { state, .. } = &mut input_mode {
				state.float.increment(delta * 0.75);
			}
		}

		// Rendering
		{
			let window_size = canvas.window().size();

			// Clear the screen.
			canvas.set_draw_color(Color::RGB(20, 20, 20));
			canvas.clear();
			canvas.set_viewport(Rect::new(0, 0, window_size.0, window_size.1));

			// Render World
			let width = 480;
			let height = 320;
			let mut camera = draw::Camera::default();
			camera.update_size(width, height);
			let focused_character = &world_manager
				.characters
				.iter()
				.find(|x| x.borrow().player_controlled)
				.unwrap();
			if let input::Mode::Cursor { position, .. } = input_mode {
				camera.focus_character_with_cursor(&focused_character.borrow(), position);
			} else {
				camera.focus_character(&focused_character.borrow());
			}

			let texture_creator = canvas.texture_creator();
			let mut world_texture = texture_creator
				.create_texture_target(texture_creator.default_pixel_format(), width, height)
				.unwrap();

			canvas
				.with_texture_canvas(&mut world_texture, |canvas| {
					canvas.set_draw_color(Color::RGB(20, 20, 20));
					canvas.clear();
					draw::tilemap(canvas, &world_manager, &camera);
					draw::characters(canvas, &world_manager, &resources, &camera);
					draw::cursor(canvas, &input_mode, &resources, &camera);
				})
				.unwrap();

			canvas
				.copy(
					&world_texture,
					None,
					Rect::new(
						(window_size.0 as i32
							- options.ui.pamphlet_width as i32
							- width as i32 * options.board.scale as i32)
							/ 2,
						(window_size.1 as i32
							- options.ui.console_height as i32
							- height as i32 * options.board.scale as i32)
							/ 2,
						width * options.board.scale,
						height * options.board.scale,
					),
				)
				.unwrap();

			// Render User Interface
			canvas.set_viewport(None);

			if debug {
				let mut debug =
					gui::Context::new(&mut canvas, &typography, Rect::new(0, 0, 100, 400));
				debug.label(&format!("FPS: {fps:.0}"));
				for i in &world_manager.characters {
					debug.label(&format!(
						"{} delay: {}",
						i.borrow().sheet.nouns.name,
						i.borrow().action_delay
					));
				}
				for member in &world_manager.party {
					let bonuses = member.piece.borrow().sheet.growth_bonuses;
					debug.label(&format!(
						"{}'s Potential",
						&member.piece.borrow().sheet.nouns.name
					));
					debug.label(&format!("Heart: {0:*<1$}", "", bonuses.heart as usize));
					debug.label(&format!("Soul: {0:*<1$}", "", bonuses.soul as usize));
					debug.label(&format!("Power: {0:*<1$}", "", bonuses.power as usize));
					debug.label(&format!("Defense: {0:*<1$}", "", bonuses.defense as usize));
					debug.label(&format!("Magic: {0:*<1$}", "", bonuses.magic as usize));
					debug.label(&format!(
						"Resistance: {0:*<1$}",
						"", bonuses.resistance as usize
					));
				}
			}

			let mut menu = gui::Context::new(
				&mut canvas,
				&typography,
				Rect::new(
					0,
					(window_size.1 - options.ui.console_height) as i32,
					window_size.0 - options.ui.pamphlet_width,
					options.ui.console_height,
				),
			);
			gui::widget::menu(&mut menu, &options, &input_mode, &world_manager, &resources);

			// Draw pamphlet
			let mut pamphlet = gui::Context::new(
				&mut canvas,
				&typography,
				Rect::new(
					(window_size.0 - options.ui.pamphlet_width) as i32,
					0,
					options.ui.pamphlet_width,
					window_size.1,
				),
			);

			cloudy_wave.draw(
				pamphlet.canvas,
				pamphlet.rect,
				20,
				Color::RGB(0x08, 0x0f, 0x25),
			);

			gui::widget::pamphlet(&mut pamphlet, &world_manager, &resources, &mut soul_jar);

			canvas.present();
		}
	}
}
