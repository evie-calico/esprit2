use esprit2::options::{RESOURCE_DIRECTORY, USER_DIRECTORY};
use esprit2::prelude::*;
use sdl2::{pixels::Color, rect::Rect, rwops::RWops};
use std::fs;
use std::process::exit;
use tracing::*;

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
	let resources = match ResourceManager::open(&*RESOURCE_DIRECTORY, &texture_creator) {
		Ok(resources) => resources,
		Err(msg) => {
			error!("Failed to open resource directory: {msg}");
			exit(1);
		}
	};
	let options = Options::open(USER_DIRECTORY.join("options.toml")).unwrap_or_else(|msg| {
		info!("failed to open options.toml ({msg}), initializing instead");
		let options = Options::default();
		if let Err(msg) = fs::write(
			USER_DIRECTORY.join("options.toml"),
			toml::to_string(&options).unwrap(),
		) {
			error!("failed to initialize options.toml: {msg}");
		}
		options
	});
	// Create a piece for the player, and register it with the world manager.
	let party_blueprint = [
		world::PartyReferenceBase {
			sheet: "luvui",
			accent_color: (0xDA, 0x2D, 0x5C),
		},
		world::PartyReferenceBase {
			sheet: "aris",
			accent_color: (0x0C, 0x94, 0xFF),
		},
	];
	let mut world_manager = world::Manager::new(party_blueprint.into_iter(), &resources);
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

	let mut soul_jar = gui::widget::SoulJar::new(&resources);

	let mut input_mode = input::Mode::Normal;
	let mut action_request = None;
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

			for i in &mut world_manager.party {
				i.draw_state.cloud.tick(delta);
				i.draw_state.cloud_trail.tick(delta / 4.0);
			}
			action_request = world_manager.update(action_request, &mut input_mode);
			world_manager.console.update(delta);
			soul_jar.tick(delta as f32);
			if let input::Mode::Cursor { state, .. } = &mut input_mode {
				state.float.increment(delta);
			}
		}

		// Rendering
		// Clear the screen.
		canvas.set_draw_color(Color::RGB(20, 20, 20));
		canvas.clear();

		// Configure world viewport.
		let window_size = canvas.window().size();
		canvas.set_viewport(Rect::new(
			0,
			0,
			window_size.0 - options.ui.pamphlet_width,
			window_size.1 - options.ui.console_height,
		));
		canvas.set_draw_color(Color::BLACK);
		canvas
			.fill_rect(Rect::new(0, 0, window_size.0, window_size.1))
			.unwrap();

		draw::tilemap(&mut canvas, &world_manager);
		draw::characters(&world_manager, &mut canvas, sleep_texture);
		draw::cursor(&input_mode, &resources, &mut canvas);

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
		gui::widget::menu(&mut menu, &options, &font, &input_mode, &world_manager);

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
		gui::widget::pamphlet(
			&mut pamphlet,
			&font,
			&world_manager,
			&resources,
			&mut soul_jar,
		);

		canvas.present();
	}
}
