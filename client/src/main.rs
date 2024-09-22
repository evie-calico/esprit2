#![feature(
	anonymous_lifetime_in_impl_trait,
	once_cell_try,
	let_chains,
	int_roundings,
	core_io_borrowed_buf,
	read_buf
)]

pub mod console_impl;
pub mod draw;
pub mod gui;
pub mod input;
pub mod options;
pub mod select;
pub mod texture;
pub mod typography;

mod server_handle;
mod state;

pub use console_impl::Console;
pub use options::Options;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
pub use server_handle::ServerHandle;
pub use typography::Typography;

pub mod prelude {
	pub use super::*;
}

use esprit2::prelude::*;
use state::State;
use std::process::exit;
use std::{fs, io};

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

#[derive(clap::Parser)]
struct Cli {
	address: Option<Box<str>>,
}

#[allow(clippy::unwrap_used, reason = "SDL")]
pub fn main() {
	let cli = <Cli as clap::Parser>::parse();
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
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::TRACE)
		.init();
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

	// let mut state = State::world(cli.address.as_deref(), &lua, &textures);
	let mut state = State::Login(String::new());

	let text_input = video_subsystem.text_input();
	text_input.start();

	let mut fps = 60.0;
	let mut fps_timer = 0.0;
	'game: loop {
		for event in event_pump.poll_iter() {
			use sdl2::event::Event;
			match event {
				Event::Quit { .. } => break 'game,
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.fullscreen.contains(keycode) => {
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
				_ => {
					state = match state {
						State::Login(mut host) => match event {
							Event::TextInput { text, .. } => {
								host.push_str(&text);
								State::Login(host)
							}
							Event::KeyDown {
								keycode: Some(Keycode::BACKSPACE),
								..
							} => {
								host.pop();
								State::Login(host)
							}
							Event::KeyDown {
								keycode: Some(Keycode::RETURN),
								..
							} => State::world(
								Some(host.as_str()).filter(|x| !x.is_empty()),
								&lua,
								&textures,
							),
							_ => State::Login(host),
						},
						State::World(mut input_mode, mut world_state) => {
							input_mode = world_state.input(input_mode, event, &scripts, &options);
							State::World(input_mode, world_state)
						}
					};
				}
			}
		}

		// This is the only place where delta time should be used.
		let delta = update_delta(&mut last_time, &mut current_time, &timer_subsystem);

		fps_timer += delta;
		if fps_timer > 0.3 {
			fps_timer = 0.0;
			fps = (fps + 1.0 / delta) / 2.0;
		}

		state = match state {
			s @ State::Login(_) => s,
			State::World(mut input_mode, mut world_state) => {
				world_state.tick(delta, &mut input_mode, &scripts);
				State::World(input_mode, world_state)
			}
		};

		let canvas_size = canvas.window().size();
		let viewport = Rect::new(0, 0, canvas_size.0, canvas_size.1);
		canvas.set_draw_color((20, 20, 20));
		canvas.clear();
		canvas.set_viewport(viewport);

		let mut gui = gui::Context::new(&mut canvas, &typography, viewport);

		match &mut state {
			State::Login(user) => {
				gui.horizontal();
				gui.label("Connect to server: ");
				gui.label(user);
				gui.vertical();
			}
			State::World(input_mode, world_state) => {
				world_state.draw(input_mode, &mut gui, &textures, &options);
			}
		};

		canvas.present();
	}
}
