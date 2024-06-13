use esprit2::options::{RESOURCE_DIRECTORY, USER_DIRECTORY};
use esprit2::prelude::*;
use sdl2::{pixels::Color, rect::Rect, rwops::RWops};
use std::fs;
use std::process::exit;
use tracing::*;
use uuid::Uuid;

const TILE_SIZE: u32 = 64;
const ITILE_SIZE: i32 = TILE_SIZE as i32;

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
		(
			Uuid::new_v4(),
			resources.get_sheet("luvui").unwrap().clone(),
		),
		(Uuid::new_v4(), resources.get_sheet("aris").unwrap().clone()),
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

			match action_request {
				Some(world::ActionRequest::BeginCursor { x, y, callback }) => {
					match input_mode {
						input::Mode::Cursor {
							x,
							y,
							submitted: true,
							..
						} => {
							input_mode = input::Mode::Normal;
							action_request = callback(&mut world_manager, x, y);
						}
						input::Mode::Cursor {
							submitted: false, ..
						} => {
							// This match statement currently has ownership of `action_request`
							// since the callback is `FnOnce`.
							// Because of this, `action_request` needs to be reconstructed in all match arms,
							// even if this is a no-op.
							action_request =
								Some(world::ActionRequest::BeginCursor { x, y, callback })
						}
						_ => {
							// If cursor mode is cancelled in any way, the callback will be destroyed.
							action_request = None;
						}
					}
				}
				None => {
					action_request = world_manager.pop_action();

					// Set up any new action requests.
					if let Some(world::ActionRequest::BeginCursor { x, y, callback: _ }) =
						action_request
					{
						input_mode = input::Mode::Cursor {
							x,
							y,
							submitted: false,
							state: input::CursorState::default(),
						};
					}
				}
			}
			world_manager.console.update(delta);
			soul_jar.tick(delta as f32);
			if let input::Mode::Cursor { state, .. } = &mut input_mode {
				state.float.increment(delta);
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
		canvas.set_draw_color(Color::BLACK);
		canvas
			.fill_rect(Rect::new(0, 0, window_size.0, window_size.1))
			.unwrap();

		// Draw tilemap
		canvas.set_draw_color(Color::WHITE);
		for (x, col) in world_manager.current_floor.map.iter_cols().enumerate() {
			for (y, tile) in col.enumerate() {
				if *tile == floor::Tile::Wall {
					canvas
						.fill_rect(Rect::new(
							(x as i32) * ITILE_SIZE,
							(y as i32) * ITILE_SIZE,
							TILE_SIZE,
							TILE_SIZE,
						))
						.unwrap();
				}
			}
		}

		// Draw characters
		for character in world_manager.characters.iter().map(|x| x.read()) {
			canvas
				.copy(
					sleep_texture,
					None,
					Some(Rect::new(
						character.x * ITILE_SIZE,
						character.y * ITILE_SIZE,
						TILE_SIZE,
						TILE_SIZE,
					)),
				)
				.unwrap();
		}

		// Draw cursor
		if let input::Mode::Cursor {
			x,
			y,
			state: input::CursorState { float, .. },
			..
		} = input_mode
		{
			enum Side {
				TopLeft,
				TopRight,
				BottomLeft,
				BottomRight,
			}

			let cursor = resources.get_texture("cursor");
			let cursor_info = cursor.query();
			let cursor_scale = TILE_SIZE / 16;
			let cursor_width = cursor_info.width * cursor_scale;
			let cursor_height = cursor_info.height * cursor_scale;
			let right_offset = ITILE_SIZE - cursor_width as i32;
			let bottom_offset = ITILE_SIZE - cursor_height as i32;
			let float = ((float.sin() + 1.0) * ((TILE_SIZE / 16) as f64)) as i32;

			for side in [
				Side::TopLeft,
				Side::TopRight,
				Side::BottomLeft,
				Side::BottomRight,
			] {
				let (x_off, y_off) = match side {
					Side::TopLeft => (-float, -float),
					Side::TopRight => (right_offset + float, -float),
					Side::BottomLeft => (-float, bottom_offset + float),
					Side::BottomRight => (right_offset + float, bottom_offset + float),
				};
				let hflip = match side {
					Side::TopLeft | Side::BottomLeft => false,
					Side::TopRight | Side::BottomRight => true,
				};
				let vflip = match side {
					Side::TopLeft | Side::TopRight => false,
					Side::BottomLeft | Side::BottomRight => true,
				};

				let rect = Rect::new(
					x * ITILE_SIZE + x_off,
					y * ITILE_SIZE + y_off,
					cursor_width,
					cursor_height,
				);
				canvas
					.copy_ex(cursor, None, Some(rect), 0.0, None, hflip, vflip)
					.unwrap();
			}
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
