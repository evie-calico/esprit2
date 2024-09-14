#![feature(
	anonymous_lifetime_in_impl_trait,
	once_cell_try,
	let_chains,
	int_roundings,
	core_io_borrowed_buf,
	new_uninit,
	read_buf
)]

mod console;
pub(crate) mod draw;
pub(crate) mod gui;
pub(crate) mod input;
pub(crate) mod options;
pub(crate) mod select;
pub(crate) mod texture;
pub(crate) mod typography;

use console::Console;
use esprit2::prelude::*;
use esprit2_server::{protocol, Server};
use options::Options;
use rkyv::Deserialize;
use sdl2::rect::Rect;
use std::io::{prelude::*, BorrowedBuf};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::exit;
use std::{fs, io};
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
	Internal {
		server: Server,
	},
	External {
		stream: TcpStream,
		packet_reciever: protocol::PacketReciever,
		world_cache: world::Manager,
		resources: resource::Manager,
	},
}

struct DummyConsole;

impl esprit2::console::Handle for DummyConsole {
	fn send_message(&self, _message: esprit2::console::Message) {}
}

impl ServerHandle {
	/// Create an internal server.
	fn internal(resource_directory: PathBuf) -> Self {
		let mut server = Server::new(resource_directory);
		server.send_ping();
		Self::Internal { server }
	}

	/// Connect to an external server.
	fn external(address: impl ToSocketAddrs, resource_directory: PathBuf) -> Self {
		let mut stream = TcpStream::connect(address).unwrap();
		let mut packet_len = [0; 4];
		// Read a ping back, reusing the buffers we used to send it.
		stream.read_exact(&mut packet_len).unwrap();
		let mut packet = Box::new_uninit_slice(u32::from_le_bytes(packet_len) as usize);
		let mut packet_buf = BorrowedBuf::from(&mut *packet);
		stream
			.set_nonblocking(true)
			.expect("failed to set nonblocking");
		stream.read_buf_exact(packet_buf.unfilled()).unwrap();
		// SAFETY: read_buf_exact always fills the entire buffer.
		let packet = unsafe { packet.assume_init() };
		// Parse the ping and print its message
		let packet = rkyv::check_archived_root::<protocol::ServerPacket>(&packet).unwrap();
		let world_cache = match packet {
			protocol::ArchivedServerPacket::World { world } => {
				let mut deserializer = rkyv::de::deserializers::SharedDeserializeMap::new();
				world.deserialize(&mut deserializer).unwrap()
			}
			_ => {
				todo!();
			}
		};
		Self::External {
			stream,
			packet_reciever: protocol::PacketReciever::default(),
			world_cache,
			resources: resource::Manager::open(resource_directory).unwrap(),
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
			Self::External { world_cache, .. } => world_cache,
		}
	}

	/// Access resources.
	///
	/// Both servers and clients keep track of game resources, and they may not be the same.
	/// However, the client can share its resources with the server when it is running internally.
	fn resources(&self) -> &resource::Manager {
		match self {
			Self::Internal { server } => &server.resources,
			Self::External { resources, .. } => resources,
		}
	}

	fn tick(
		&mut self,
		scripts: &resource::Scripts,
		console: &mut console::Console,
	) -> esprit2::Result<()> {
		match self {
			Self::Internal { server } => {
				server.recv_ping();
				server.tick(scripts, &console.handle)?;
			}
			Self::External {
				stream,
				packet_reciever,
				world_cache,
				..
			} => {
				let packet =
					rkyv::to_bytes::<_, 16>(&protocol::ClientPacket::Ping("meow".into())).unwrap();
				stream
					.write_all(&(packet.len() as u32).to_le_bytes())
					.unwrap();
				stream.write_all(&packet).unwrap();
				match packet_reciever.recv(stream, |packet| {
					let packet =
						rkyv::check_archived_root::<protocol::ServerPacket>(&packet).unwrap();
					match packet {
						protocol::ArchivedServerPacket::Ping(_) => todo!(),
						protocol::ArchivedServerPacket::World { world } => {
							let mut deserializer =
								rkyv::de::deserializers::SharedDeserializeMap::new();
							*world_cache = world.deserialize(&mut deserializer).unwrap();
						}
						protocol::ArchivedServerPacket::Message(message) => {
							let mut deserializer =
								rkyv::de::deserializers::SharedDeserializeMap::new();
							console
								.history
								.push(message.deserialize(&mut deserializer).unwrap());
						}
					}
				}) {
					Ok(()) => {}
					Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
					Err(e) => Err(e)?,
				}
			}
		}
		Ok(())
	}
}

/// Send operations.
impl ServerHandle {
	fn send_action(
		&mut self,
		console: impl esprit2::console::Handle,
		scripts: &resource::Scripts,
		action: character::Action,
	) -> esprit2::Result<()> {
		match self {
			Self::Internal { server } => server.recv_action(console, scripts, action),
			Self::External {
				stream,
				world_cache,
				resources,
				..
			} => {
				let packet =
					rkyv::to_bytes::<_, 16>(&protocol::ClientPacket::Action(action.clone()))
						.unwrap();
				stream
					.write_all(&(packet.len() as u32).to_le_bytes())
					.unwrap();
				stream.write_all(&packet).unwrap();

				world_cache.perform_action(DummyConsole, resources, scripts, action)
			}
		}
	}
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

	// Create a console.
	// An internal server will send messages to it using a console::Handle.
	// An external server will send messages to it over TCP. (local messages generated by the world cache are discarded)
	let mut console = Console::default();

	// Create an internal server instance
	let mut server = if let Some(address) = &cli.address {
		ServerHandle::external(
			(&**address, protocol::DEFAULT_PORT),
			options::resource_directory().clone(),
		)
	} else {
		ServerHandle::internal(options::resource_directory().clone())
	};

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
	if let ServerHandle::Internal { .. } = server {
		lua.globals()
			.set(
				"Console",
				esprit2::console::LuaHandle(console.handle.clone()),
			)
			.unwrap();
	} else {
		lua.globals()
			.set("Console", esprit2::console::LuaHandle(DummyConsole))
			.unwrap();
	}
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
	// TODO: Make this part of input::Mode::Select;
	let mut chase_point = None;
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
								&console.handle,
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
										Some(input::Response::Select(point)) => {
											chase_point = Some(point);
										}
										Some(input::Response::Act(action)) => server
											.send_action(&console.handle, &scripts, action)
											.unwrap(),

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
			if next_character.borrow().player_controlled {
				if let Some(point) = &chase_point {
					match point {
						select::Point::Character(character) => {
							let (x, y) = {
								let c = character.borrow();
								(c.x, c.y)
							};
							// Give a safe range of 2 tiles if the target is an enemy.
							let distance = if next_character.borrow().alliance
								!= character.borrow().alliance
							{
								2
							} else {
								1
							};
							if (next_character.borrow().x - x).abs() <= distance
								&& (next_character.borrow().y - y).abs() <= distance
							{
								chase_point = None;
							} else {
								server
									.send_action(
										&console.handle,
										&scripts,
										character::Action::Move(x, y),
									)
									.unwrap();
							}
						}
						select::Point::Exit(x, y) => {
							if next_character.borrow().x == *x && next_character.borrow().y == *y {
								chase_point = None;
							} else {
								server
									.send_action(
										&console.handle,
										&scripts,
										character::Action::Move(*x, *y),
									)
									.unwrap();
							}
						}
					}
				}
			}

			if let Err(msg) = server.tick(&scripts, &mut console) {
				error!("server tick failed: {msg}");
				exit(1);
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
