#![feature(anonymous_lifetime_in_impl_trait, once_cell_try)]
#![warn(
	clippy::module_name_repetitions,
	clippy::items_after_statements,
	clippy::inconsistent_struct_constructor,
	clippy::unwrap_used
)]

pub mod draw;
pub mod gui;
pub mod input;
pub mod options;
pub mod texture;
pub mod typography;

use esprit2::prelude::*;
use esprit2_server::Server;
use options::Options;
use sdl2::rect::Rect;
use std::path::PathBuf;
use std::process::exit;
use std::{fs, io};
use tracing::{error, info, warn};
use typography::Typography;

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

/// Encapsulate resources can be (but aren't necessarily!) shared by both the client and the server.
///
/// This special handling is necessary because the server may be an internal process,
/// or an external program connected over TCP.
///
/// When connected to an external server, it's necessary for us to have a local copy of most information,
/// such as the lua runtime, console, resource directory, and even the world manager.
/// The server is expected to communicate with the client so that everything is kept in sync,
/// but in the cast of desyncs the world manager should be easy to replace.
///
/// Because both the server and client have a copy of the game's resources,
/// and these copies may not be exactly the same,
/// client and server state may become desyncronized; this is expected.
/// The client should simulate changes to its cache of the world manager and send a checksum
/// along with whatever action it performed. If this checksum differs from the server's copy,
/// it will send an complete copy of the world state back to the client, which it *must* accept.
enum ServerHandle {
	Internal { server: Server },
	// External { },
}

impl ServerHandle {
	/// Create an internal server.
	fn internal(console: console::Handle, resource_directory: PathBuf) -> Self {
		Self::Internal {
			server: Server::new(console, resource_directory),
		}
	}

	/// Access the currently cached world manager.
	///
	/// This is NOT necessarily the same as the server's copy!
	/// The client must be careful to communicate its expected state with the server,
	/// and replace its state in the event of a desync.
	fn world(&self) -> &world::Manager {
		match self {
			Self::Internal { server } => &server.world,
		}
	}

	/// Access the console handle.
	///
	/// This is a handle to whatever console was originally passed to the server.
	/// Sending messages over this handle does *not* implicitly send them to a remote server,
	/// that must be done in a separate step.
	fn console(&self) -> &console::Handle {
		match self {
			Self::Internal { server } => &server.console,
		}
	}

	/// Access resources.
	///
	/// Both servers and clients keep track of game resources, and they may not be the same.
	/// However, the client can share its resources with the server when it is running internally.
	fn resources(&self) -> &resource::Manager {
		match self {
			Self::Internal { server } => &server.resources,
		}
	}

	fn act(
		&mut self,
		scripts: &resource::Scripts,
		action: character::Action,
	) -> esprit2::Result<()> {
		match self {
			Self::Internal { server } => server.act(scripts, action),
		}
	}
}

#[allow(clippy::unwrap_used, reason = "SDL")]
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

	let options_path = options::user_directory().join("options.toml");
	let options = Options::open(&options_path).unwrap_or_else(|msg| {
		// This is `info` because it's actually very expected for first-time players.
		info!("failed to open options.toml: {msg}");
		info!("initializing options.toml instead");
		// Attempt to save the old file, in case it exists.

		if let Err(msg) = fs::rename(&options_path, options_path.with_extension("toml.old")) {
			if msg.kind() != io::ErrorKind::NotFound {
				warn!("failed to backup existing options.toml: {msg}");
			}
		} else {
			info!("existing options.toml was backed up to options.toml.old");
		}
		let options = Options::default();
		if let Err(msg) = fs::write(&options_path, toml::to_string(&options).unwrap()) {
			error!("failed to initialize options.toml: {msg}");
		}
		options
	});

	// Create a console.
	// An internal server will send messages to it using a console::Handle.
	// An external server will send messages to it over TCP.
	let mut console = Console::new(options.ui.colors.console.clone());

	// Create an internal server instance
	// TODO: allow for external servers over a TCP protocol.
	let mut server = ServerHandle::internal(
		console.handle.clone(),
		options::resource_directory().clone(),
	);

	// Create a Lua runtime.
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
	lua.globals().set("Console", console.clone()).unwrap();
	lua.globals()
		.set("Status", server.resources().statuses_handle())
		.unwrap();
	lua.globals()
		.set("Heuristic", consider::HeuristicConstructor)
		.unwrap();
	lua.globals().set("Log", combat::LogConstructor).unwrap();
	lua.globals()
		.set("Input", input::RequestConstructor)
		.unwrap();

	let scripts =
		match resource::Scripts::open(options::resource_directory().join("scripts/"), &lua) {
			Ok(scripts) => scripts,
			Err(msg) => {
				error!("failed to open scripts directory: {msg}");
				exit(1);
			}
		};

	let textures = match texture::Manager::new(
		options::resource_directory().join("textures/"),
		&texture_creator,
	) {
		Ok(resources) => resources,
		Err(msg) => {
			error!("failed to open resource directory: {msg}");
			exit(1);
		}
	};

	let typography = Typography::new(&options.ui.typography, &ttf_context);

	let mut soul_jar = gui::widget::SoulJar::new(&textures).unwrap_or_else(|msg| {
		error!("failed to initialize soul jar: {msg}");
		exit(1);
	});
	// This disperses the souls enough to cause them to fly in from the sides
	// the same effect can be seen if a computer is put to sleep and then woken up.
	soul_jar.tick(5.0);
	let mut cloudy_wave = draw::CloudyWave::default();
	let mut pamphlet = gui::widget::Pamphlet::new();

	let mut input_mode = input::Mode::Normal;
	let mut fps = 60.0;
	let mut fps_timer = 0.0;
	let mut debug = false;
	loop {
		// Input processing
		{
			for event in event_pump.poll_iter() {
				match event {
					sdl2::event::Event::Quit { .. } => return,
					sdl2::event::Event::KeyDown {
						keycode: Some(keycode),
						..
					} => {
						let next_character = server.world().next_character().clone();
						if next_character.borrow().player_controlled {
							let controllable_character = input::controllable_character(
								keycode,
								next_character,
								&server,
								&scripts,
								input_mode,
								&options,
							);
							match controllable_character {
								Ok((mode, response)) => {
									input_mode = mode;
									match response {
										Some(input::Response::Fullscreen) => {
											use sdl2::video::FullscreenType;
											match canvas.window().fullscreen_state() {
												FullscreenType::Off => {
													let _ = canvas
														.window_mut()
														.set_fullscreen(FullscreenType::Desktop);
												}
												FullscreenType::True | FullscreenType::Desktop => {
													let _ = canvas
														.window_mut()
														.set_fullscreen(FullscreenType::Off);
												}
											}
										}
										Some(input::Response::Debug) => debug ^= true,
										Some(input::Response::Act(action)) => {
											server.act(&scripts, action).unwrap();
										}
										Some(input::Response::Partial(partial, request)) => {
											match request {
												input::Request::Cursor {
													x,
													y,
													range,
													radius,
												} => {
													input_mode =
														input::Mode::Cursor(input::Cursor {
															origin: (x, y),
															position: (x, y),
															range,
															radius,
															state: input::CursorState::default(),
															callback: partial,
														});
												}
												input::Request::Prompt { message } => {
													input_mode =
														input::Mode::Prompt(input::Prompt {
															message,
															callback: partial,
														})
												}
												input::Request::Direction { message } => {
													input_mode = input::Mode::DirectionPrompt(
														input::DirectionPrompt {
															message,
															callback: partial,
														},
													)
												}
											}
										}
										None => (),
									}
								}
								Err(msg) => {
									input_mode = input::Mode::Normal;
									error!("world input processing returned an error: {msg}");
								}
							}
						}
					}
					_ => {}
				}
			}
		}
		// Logic
		{
			let next_character = server.world().next_character().clone();
			if !next_character.borrow().player_controlled {
				let considerations = server
					.world()
					.consider_turn(server.resources(), &scripts)
					.unwrap();
				let action = server
					.world()
					.consider_action(&scripts, next_character, considerations)
					.unwrap();
				server.act(&scripts, action).unwrap();
			}
			// This is the only place where delta time should be used.
			let delta = update_delta(&mut last_time, &mut current_time, &timer_subsystem);

			fps_timer += delta;
			if fps_timer > 0.3 {
				fps_timer = 0.0;
				fps = (fps + 1.0 / delta) / 2.0;
			}

			for i in &mut pamphlet.party_member_clouds {
				i.cloud.tick(delta);
				i.cloud_trail.tick(delta / 4.0);
			}
			console.update(delta);
			soul_jar.tick(delta as f32);
			cloudy_wave.tick(delta);
			if let input::Mode::Cursor(input::Cursor { state, .. }) = &mut input_mode {
				state.float.increment(delta * 0.75);
			}
		}

		// Rendering
		{
			let window_size = canvas.window().size();

			// Clear the screen.
			canvas.set_draw_color((20, 20, 20));
			canvas.clear();
			canvas.set_viewport(Rect::new(0, 0, window_size.0, window_size.1));

			// Render World
			let width = 480;
			let height = 320;
			let mut camera = draw::Camera::default();
			camera.update_size(width, height);
			let focused_character = &server
				.world()
				.characters
				.iter()
				.find(|x| x.borrow().player_controlled)
				.unwrap();
			if let input::Mode::Cursor(input::Cursor { position, .. }) = &input_mode {
				camera.focus_character_with_cursor(&focused_character.borrow(), *position);
			} else {
				camera.focus_character(&focused_character.borrow());
			}

			let texture_creator = canvas.texture_creator();
			let mut world_texture = texture_creator
				.create_texture_target(texture_creator.default_pixel_format(), width, height)
				.unwrap();

			canvas
				.with_texture_canvas(&mut world_texture, |canvas| {
					canvas.set_draw_color((20, 20, 20));
					canvas.clear();
					draw::tilemap(canvas, server.world(), &camera);
					draw::characters(canvas, server.world(), &textures, &camera);
					draw::cursor(canvas, &input_mode, &textures, &camera);
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
				for i in &server.world().characters {
					debug.label(&format!(
						"{} delay: {}",
						i.borrow().sheet.nouns.name,
						i.borrow().action_delay
					));
				}
				for member in &server.world().party {
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
			gui::widget::menu(
				&mut menu,
				&options,
				&input_mode,
				server.world(),
				&console,
				server.resources(),
				&textures,
			);

			// Draw pamphlet
			let mut pamphlet_ctx = gui::Context::new(
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
				pamphlet_ctx.canvas,
				pamphlet_ctx.rect,
				20,
				(0x08, 0x0f, 0x25).into(),
			);

			pamphlet.draw(&mut pamphlet_ctx, server.world(), &textures, &mut soul_jar);

			canvas.present();
		}
	}
}
