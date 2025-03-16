#![feature(
	anonymous_lifetime_in_impl_trait,
	once_cell_try,
	let_chains,
	int_roundings,
	core_io_borrowed_buf,
	read_buf,
	trait_alias
)]

mod server_handle;

use clap::Parser;
use esprit2::prelude::*;
use esprit2_server::protocol::{self, ClientAuthentication, ClientRouting};
use esprit2_server::Client;
use rkyv::rancor::{self, ResultExt};
use sdl3::image::LoadTexture;
use sdl3::rect::Rect;
use sdl3::timer;
use std::net::{Ipv4Addr, SocketAddr};
use std::process::exit;
use std::{fs, io, thread};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

pub(crate) mod console_impl;
pub(crate) mod draw;
pub(crate) mod gui;
pub(crate) mod input;
pub(crate) mod menu;
pub(crate) mod options;
pub(crate) mod select;
pub(crate) mod texture;
pub(crate) mod typography;

pub(crate) mod prelude {
	pub(crate) use super::*;
}

pub(crate) use console_impl::Console;
pub(crate) use options::Options;
pub(crate) use server_handle::ServerHandle;
pub(crate) use typography::Typography;

fn update_delta(last_time: &mut f64, current_time: &mut f64) -> f64 {
	*last_time = *current_time;
	*current_time = timer::performance_counter() as f64;
	((*current_time - *last_time) * 1000.0
		/ (timer::performance_frequency() as f64))
		// Convert milliseconds to seconds.
		/ 1000.0
}

#[derive(Debug, Clone)]
pub(crate) enum RootMenuResponse {
	OpenSingleplayer { username: String },
	OpenMultiplayer { username: String, url: String },
}

#[derive(clap::Parser)]
struct Cli {
	#[clap(long)]
	username: Option<Box<str>>,

	host: Option<Box<str>>,
}

#[allow(clippy::unwrap_used, reason = "SDL")]
#[tokio::main]
pub(crate) async fn main() {
	let cli = Cli::parse();
	// SDL initialization.
	let sdl_context = sdl3::init().unwrap();
	let ttf_context = sdl3::ttf::init().unwrap();
	let video_subsystem = sdl_context.video().unwrap();
	let window = video_subsystem
		.window("Esprit 2", 1280, 720)
		.resizable()
		.position_centered()
		.build()
		.unwrap();
	let mut canvas = window.clone().into_canvas();
	let texture_creator = canvas.texture_creator();
	let mut event_pump = sdl_context.event_pump().unwrap();

	let mut current_time = timer::performance_counter() as f64;
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

	let lua = esprit2::lua::init().unwrap_or_else(|e| {
		error!("failed to initialize lua runtime: {e}");
		exit(1);
	});

	let typography = Typography::new(&options.ui.typography, &ttf_context);

	let mut menu: Option<Box<dyn menu::Menu<RootMenuResponse>>> =
		Some(Box::new(menu::login::State::new(
			cli.username.as_deref(),
			cli.host.as_deref(),
			texture_creator
				.load_texture_bytes(include_bytes!("res/missing_texture.png"))
				.expect("missing texture should not fail to load"),
		)));
	let mut server: Option<(input::Mode, ServerHandle)> = None;
	let mut internal_server = None;

	let text_input = video_subsystem.text_input();
	text_input.start(&window);

	let mut fps = 60.0;
	let mut fps_timer = 0.0;
	'game: loop {
		for event in event_pump.poll_iter() {
			use sdl3::event::Event;
			match event {
				Event::Quit { .. } => break 'game,
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.fullscreen.contains(keycode) => {
					use sdl3::video::FullscreenType;
					match canvas.window().fullscreen_state() {
						FullscreenType::Off => {
							let _ = canvas.window_mut().set_fullscreen(true);
						}
						FullscreenType::True | FullscreenType::Desktop => {
							let _ = canvas.window_mut().set_fullscreen(false);
						}
					}
				}
				_ => {
					if let Some(menu_state) = &mut menu {
						match menu_state.event(&event, &options) {
							input::Signal::None => {}
							input::Signal::Cancel => break 'game,
							input::Signal::Yield(RootMenuResponse::OpenSingleplayer {
								username,
							}) => {
								// TODO: handle and display connection errors.
								let new_server = InternalServer::new().await.unwrap();
								let stream = TcpStream::connect(new_server.address).await.unwrap();
								internal_server = Some(new_server);
								server = Some((
									input::Mode::Normal,
									ServerHandle::new(
										stream,
										ClientAuthentication { username },
										None,
										&lua,
										texture::Manager::new(&texture_creator),
									)
									.await
									.unwrap(),
								));
								menu = None;
							}
							input::Signal::Yield(RootMenuResponse::OpenMultiplayer {
								username,
								url,
							}) => {
								let (client_routing, address) = ClientRouting::new(&url).unwrap();
								let stream = TcpStream::connect(address).await.unwrap();
								server = Some((
									input::Mode::Normal,
									// TODO: handle and display connection errors.
									ServerHandle::new(
										stream,
										ClientAuthentication { username },
										client_routing,
										&lua,
										texture::Manager::new(&texture_creator),
									)
									.await
									.unwrap(),
								));
								menu = None;
							}
						}
					} else if let Some((mut input_mode, mut world_state)) = server {
						input_mode = world_state
							.event(input_mode, event, &lua, &options)
							.await
							.unwrap();
						server = Some((input_mode, world_state));
					}
				}
			}
		}

		// tick
		{
			let delta = update_delta(&mut last_time, &mut current_time);

			fps_timer += delta;
			if fps_timer > 0.3 {
				fps_timer = 0.0;
				fps = (fps + 1.0 / delta) / 2.0;
			}

			if let Some((input_mode, server)) = &mut server {
				server.tick(delta, input_mode).await.unwrap();
				if let Some(world) = &mut server.world {
					// TODO: Avoid ticking more than once when too late in the frame.
					world
						.tick(&server.resources, &lua, &server.console)
						.unwrap();
				}
			}
		}

		// draw
		{
			let canvas_size = canvas.window().size();
			let viewport = Rect::new(0, 0, canvas_size.0, canvas_size.1);
			canvas.set_draw_color((20, 20, 20));
			canvas.clear();
			canvas.set_viewport(viewport);

			let mut gui = gui::Context::new(&mut canvas, &typography, viewport);

			if let Some(menu) = &menu {
				menu.draw(&mut gui);
			}
			if let Some((input_mode, world)) = &server {
				world.draw(input_mode, &mut gui, &lua, &options);
			}

			canvas.present();
		}
	}

	if let Some(internal_server) = internal_server {
		// TODO: join the internal server thread.
		drop(internal_server.instance);
		internal_server.router.abort();
	}

	exit(0);
}

struct InternalServer {
	address: SocketAddr,
	router: task::JoinHandle<()>,
	instance: thread::JoinHandle<esprit2::Result<()>>,
}

impl InternalServer {
	async fn new() -> Result<InternalServer, rancor::BoxedError> {
		let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), protocol::DEFAULT_PORT))
			.await
			.into_trace("while binding TCP listener")?;
		let address = listener.local_addr().expect("missing local addr");
		let (router, reciever) = mpsc::channel(4);
		let instance = thread::Builder::new()
			.name(String::from("instance"))
			.spawn(move || {
				let result = esprit2_server::instance(reciever, options::resource_directory());
				if let Err(e) = &result {
					error!("server instance returned an error: {e}");
				}
				result
			})
			.into_trace("while spawning instance thread")?;
		let router = task::spawn(
			async move {
				loop {
					match listener.accept().await {
						// No routing necessary, just forward all streams to the instance.
						Ok((stream, peer_addr)) => {
							info!(peer = peer_addr.to_string(), "connected");
							let (client, receiver) = Client::new(stream);
							if router
								.send((client, ReceiverStream::new(receiver)))
								.await
								.is_err()
							{
								warn!("recieved connection after instance reciever channel closed");
								break;
							}
						}
						// TODO: What errors may occur? How should they be handled?
						Err(msg) => error!("failed to read incoming stream: {msg}"),
					}
				}
			}
			.instrument(tracing::error_span!("router", addr = address.to_string())),
		);
		Ok(InternalServer {
			address,
			router,
			instance,
		})
	}
}
