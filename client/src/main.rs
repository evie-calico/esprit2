#![feature(
	anonymous_lifetime_in_impl_trait,
	once_cell_try,
	let_chains,
	int_roundings,
	core_io_borrowed_buf,
	read_buf,
	trait_alias
)]

pub mod console_impl;
pub mod draw;
pub mod gui;
pub mod input;
pub mod menu;
pub mod options;
pub mod select;
pub mod texture;
pub mod typography;

use clap::Parser;
pub use console_impl::Console;
use esprit2_server::protocol;
pub use options::Options;
use sdl2::rect::Rect;
pub use server_handle::ServerHandle;
use tracing::error_span;
pub use typography::Typography;

pub mod prelude {
	pub use super::*;
}

mod server_handle;
mod world_state;

use esprit2::prelude::*;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::process::exit;
use std::{fs, io, thread};
use world_state::State as WorldState;

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

#[derive(Debug, Clone)]
pub enum RootMenuResponse {
	OpenSingleplayer,
	OpenMultiplayer { host: String },
}

#[derive(clap::Parser)]
struct Cli {
	#[clap(long)]
	username: Option<Box<str>>,

	host: Option<Box<str>>,
}

#[allow(clippy::unwrap_used, reason = "SDL")]
pub fn main() {
	let cli = Cli::parse();
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

	let mut menu: Option<Box<dyn menu::Menu<RootMenuResponse>>> = Some(Box::new(
		menu::login::State::new(cli.username.as_deref(), cli.host.as_deref()),
	));
	let mut world: Option<(input::Mode, WorldState)> = None;
	let mut internal_server: Option<thread::JoinHandle<()>> = None;

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
					if let Some(menu_state) = &mut menu {
						match menu_state.event(&event, &options) {
							input::Signal::None => {}
							input::Signal::Cancel => break 'game,
							input::Signal::Yield(RootMenuResponse::OpenSingleplayer) => {
								// TODO: handle and display connection errors.
								let (address, server) = spawn_instance();
								internal_server = Some(server);
								world = Some((
									input::Mode::Normal,
									WorldState::new(address, &lua, &textures).unwrap(),
								));
								menu = None;
							}
							input::Signal::Yield(RootMenuResponse::OpenMultiplayer { host }) => {
								world = Some((
									input::Mode::Normal,
									// TODO: handle and display connection errors.
									WorldState::new(
										(host, protocol::DEFAULT_PORT),
										&lua,
										&textures,
									)
									.unwrap(),
								));
								menu = None;
							}
						}
					} else if let Some((mut input_mode, mut world_state)) = world {
						input_mode = world_state.event(input_mode, event, &scripts, &options);
						world = Some((input_mode, world_state));
					}
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

		if let Some((input_mode, world)) = &mut world {
			world.tick(delta, input_mode, &scripts);
		}

		let canvas_size = canvas.window().size();
		let viewport = Rect::new(0, 0, canvas_size.0, canvas_size.1);
		canvas.set_draw_color((20, 20, 20));
		canvas.clear();
		canvas.set_viewport(viewport);

		let mut gui = gui::Context::new(&mut canvas, &typography, viewport);

		if let Some(menu) = &menu {
			menu.draw(&mut gui, &textures);
		}
		if let Some((input_mode, world)) = &world {
			world.draw(input_mode, &mut gui, &textures, &options);
		}

		canvas.present();
	}

	// TODO: join the internal server thread.
}

fn spawn_instance() -> (SocketAddr, thread::JoinHandle<()>) {
	let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).unwrap();
	let address = listener.local_addr().unwrap();
	(
		address,
		thread::spawn(move || {
			singular_host(listener);
		}),
	)
}

fn singular_host(listener: TcpListener) {
	let _span = error_span!(
		"internal server",
		addr = listener.local_addr().unwrap().to_string()
	)
	.entered();
	info!("listening for connections");
	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let _enter =
					tracing::error_span!("client", addr = stream.peer_addr().unwrap().to_string())
						.entered();
				info!("connected");
				esprit2_server::connection(stream, options::resource_directory().clone());
				info!("disconnected");
			}
			// TODO: What errors may occur? How should they be handled?
			Err(msg) => error!("failed to read incoming stream: {msg}"),
		}
	}
}
