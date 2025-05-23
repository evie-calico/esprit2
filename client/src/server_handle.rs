use std::path::{Path, PathBuf};

use crate::prelude::*;
use esprit2::prelude::*;
use protocol::{
	ClientAuthentication, ClientIdentifier, ClientPacket, PacketReceiver, PacketSender,
};
use rkyv::util::AlignedVec;
use sdl3::rect::Rect;
use tokio::net::TcpStream;

pub(crate) struct ServerHandle<'texture> {
	sender: PacketSender,
	_internal_receiver: PacketReceiver,
	receiver: mpsc::Receiver<AlignedVec>,
	identifier: Option<ClientIdentifier>,

	pub(crate) world: Option<world::Manager>,
	pub(crate) resources: resource::Handle,
	pub(crate) textures: texture::Manager<'texture>,
	pub(crate) console: Console,
	pub(crate) soul_jar: gui::widget::SoulJar<'texture>,
	pub(crate) cloudy_wave: draw::CloudyWave,
	pub(crate) pamphlet: gui::widget::Pamphlet,
	pub(crate) chase_point: Option<select::Point>,
}

impl<'texture> ServerHandle<'texture> {
	pub(crate) async fn new<'lua>(
		stream: TcpStream,
		authentication: ClientAuthentication,
		routing: Option<ClientRouting>,
		lua: &'lua mlua::Lua,
		mut texture_manager: texture::Manager<'texture>,
	) -> anyhow::Result<Self> {
		// Create a console.
		// (local messages generated by the world cache are discarded)
		let console = Console::default();

		let modules = anyhow::Context::context(
			options::resource_directory().read_dir(),
			"failed to read contents of resource directory",
		)?
		.filter_map(|x| {
			let x = x.ok()?;
			if x.metadata().ok()?.is_dir() {
				Some(x.path().into_boxed_path())
			} else {
				None
			}
		})
		.collect::<Box<[Box<Path>]>>();
		let (resources, errors) =
			resource::open(lua, modules.iter().map(|x| x.as_ref()), |name, _, init| {
				use mlua::ErrorContext;

				let textures = lua.load_from_function::<mlua::Table>(
					"init.client.textures",
					lua.create_function(|lua, ()| {
						lua.create_table_from([
							("sheet", lua.create_table()?),
							("texture", lua.create_table()?),
						])
					})?,
				)?;
				let result = init();
				lua.unload("init.client.textures")?;
				for i in textures
					.get::<mlua::Table>("texture")
					.context("failed to read init.client.textures[\"texture\"]")?
					.pairs::<mlua::String, PathBuf>()
				{
					let (k, v) = i?;
					texture_manager.textures.insert(
						format!("{name}:{}", k.to_str()?).into_boxed_str(),
						texture::TextureInfo {
							path: v,
							..Default::default()
						},
					);
				}
				for i in textures
					.get::<mlua::Table>("sheet")
					.context("failed to read init.client.textures[\"sheet\"]")?
					.pairs::<mlua::String, mlua::Table>()
				{
					let (k, v) = i?;
					texture_manager.sheets.insert(
						format!("{name}:{}", k.to_str()?).into_boxed_str(),
						texture::Sheet {
							icon: v.get("icon")?,
						},
					);
				}
				result
			});
		let resources = resource::Handle::new(resources.into());
		for (module, error) in errors
			.into_iter()
			.flat_map(|x| <Box<[_]> as IntoIterator>::into_iter(x.errors).map(move |e| (x.name, e)))
		{
			error!(module, "{error:?}");
		}

		let mut soul_jar = gui::widget::SoulJar::new(texture_manager.texture_creator);
		// This disperses the souls enough to cause them to fly in from the sides
		// the same effect can be seen if a computer is put to sleep and then woken up.
		soul_jar.tick(5.0);
		let cloudy_wave = draw::CloudyWave::default();
		let pamphlet = gui::widget::Pamphlet::new();

		// TODO: Make this part of input::Mode::Select;
		let chase_point = None;

		let handle = resources.clone();
		lua.load_from_function::<mlua::Value>(
			"runtime.resources",
			lua.create_function(move |_, ()| Ok(handle.clone()))?,
		)?;
		lua.load_from_function::<mlua::Value>(
			"runtime.console",
			lua.create_function(move |_, ()| Ok(console::LuaHandle(console_impl::Dummy)))?,
		)?;
		// input requests need to yield so this library is written in lua.
		let make_cursor = mlua::Function::wrap(|x, y, range, radius| {
			Ok(input::Request::Cursor {
				x,
				y,
				range,
				radius,
			})
		});
		let make_prompt = mlua::Function::wrap(|message| Ok(input::Request::Prompt { message }));
		let make_direction =
			mlua::Function::wrap(|message| Ok(input::Request::Direction { message }));
		lua.load_from_function::<mlua::Value>(
			"runtime.input",
			lua.load(mlua::chunk! {
				return {
					cursor = function(x, y, range, radius)
						x, y = coroutine.yield($make_cursor(x, y, range, radius))
						return { x = x, y = y }
					end,

					prompt = function(message)
						return coroutine.yield($make_prompt(message))
					end,

					direction = function(message)
						return coroutine.yield($make_direction(message))
					end,
				}
			})
			.into_function()?,
		)?;

		let (receiver, sender) = stream.into_split();
		let sender = PacketSender::new(sender);
		sender
			.send(&ClientPacket::Authenticate(authentication))
			.await?;
		if let Some(routing) = routing {
			sender.send(&ClientPacket::Route(routing)).await?;
		} else {
			sender.send(&ClientPacket::Instantiate).await?;
		}
		let (_internal_receiver, receiver) = PacketReceiver::new(receiver);

		Ok(Self {
			sender,
			_internal_receiver,
			receiver,
			identifier: None,

			world: None,
			resources,
			textures: texture_manager,
			console,
			soul_jar,
			cloudy_wave,
			pamphlet,
			chase_point,
		})
	}

	pub(crate) async fn perform_action(
		&mut self,
		lua: &mlua::Lua,
		action: character::Action,
	) -> anyhow::Result<()> {
		use anyhow::Context;

		let world = self.world.as_mut().expect("world must be present");
		world
			.perform_action(console_impl::Dummy, &self.resources, lua, action.clone())
			.context("failed to perform action")?;
		self.sender
			.send(&protocol::ClientPacket::Action { action })
			.await
			.context("failed to serialize action packet")
	}

	pub(crate) async fn event(
		&mut self,
		input_mode: input::Mode,
		event: sdl3::event::Event,
		lua: &mlua::Lua,
		options: &Options,
	) -> anyhow::Result<input::Mode> {
		let sdl3::event::Event::KeyDown {
			keycode: Some(keycode),
			..
		} = event
		else {
			return Ok(input_mode);
		};
		let Some(world) = &self.world else {
			return Ok(input_mode);
		};

		if !world
			.next_character()
			.borrow()
			.components
			.contains_key(":conscious")
		{
			return Ok(input_mode);
		}
		let result = match input::controllable_character(
			keycode,
			world,
			&self.console,
			&self.resources,
			lua,
			input_mode,
			options,
		) {
			Ok((mode, response)) => match response {
				Some(input::Response::Select(point)) => {
					self.chase_point = Some(point);
					mode
				}
				Some(input::Response::Action(action)) => {
					self.perform_action(lua, action).await?;
					mode
				}

				Some(input::Response::Partial(partial, request)) => match request {
					input::Request::Cursor {
						x,
						y,
						range,
						radius,
					} => input::Mode::Cursor(input::Cursor {
						origin: (x, y),
						position: (x, y),
						range,
						radius,
						state: input::CursorState::default(),
						callback: partial,
					}),
					input::Request::Prompt { message } => input::Mode::Prompt(input::Prompt {
						message,
						callback: partial,
					}),
					input::Request::Direction { message } => {
						input::Mode::DirectionPrompt(input::DirectionPrompt {
							message,
							callback: partial,
						})
					}
				},
				None => mode,
			},
			Err(msg) => {
				error!("world input processing returned an error: {msg}");
				input::Mode::Normal
			}
		};
		Ok(result)
	}

	pub(crate) async fn tick(
		&mut self,
		delta: f64,
		input_mode: &mut input::Mode,
	) -> Result<(), rancor::BoxedError> {
		while let Ok(packet) = self.receiver.try_recv() {
			let packet = rkyv::access(&packet)?;
			match packet {
				protocol::ArchivedServerPacket::Ping => {
					// TODO: Respond to pings
				}
				protocol::ArchivedServerPacket::Register(identifier) => {
					self.identifier = Some(identifier.to_native());
				}
				protocol::ArchivedServerPacket::World { world } => {
					self.world =
						Some(rkyv::deserialize(world).trace("while deserializing world packet")?);
				}
				protocol::ArchivedServerPacket::Message(message) => {
					self.console.history.push(
						rkyv::deserialize(message).trace("while deserializing message packet")?,
					);
				}
			}
		}

		for i in &mut self.pamphlet.party_member_clouds {
			i.cloud.tick(delta);
			i.cloud_trail.tick(delta / 4.0);
		}
		self.console.update(delta);
		self.soul_jar.tick(delta as f32);
		self.cloudy_wave.tick(delta);
		if let input::Mode::Cursor(input::Cursor { state, .. }) = input_mode {
			state.float.increment(delta * 0.75);
		}
		Ok(())
	}

	#[allow(clippy::unwrap_used, reason = "SDL")]
	pub(crate) fn draw(
		&self,
		input_mode: &input::Mode,
		ctx: &mut gui::Context,
		lua: &mlua::Lua,
		options: &Options,
	) {
		if let Some(world) = &self.world {
			// Render World
			let width = 480;
			let height = 320;
			let mut camera = draw::Camera::default();
			camera.update_size(width, height);
			if let Some(focused_character) = &world
				.characters
				.iter()
				.find(|x| x.borrow().components.contains_key(":conscious"))
			{
				if let input::Mode::Cursor(input::Cursor { position, .. }) = &input_mode {
					camera.focus_character_with_cursor(&focused_character.borrow(), *position);
				} else {
					camera.focus_character(&focused_character.borrow());
				}
			}

			let texture_creator = ctx.canvas.texture_creator();
			let mut world_texture = texture_creator
				.create_texture_target(texture_creator.default_pixel_format(), width, height)
				.unwrap();

			ctx.canvas
				.with_texture_canvas(&mut world_texture, |canvas| {
					canvas.set_draw_color((20, 20, 20));
					canvas.clear();
					draw::tilemap(canvas, world, &camera);
					draw::characters(canvas, world, &self.textures, &camera);
					draw::cursor(canvas, input_mode, &self.textures, &camera);
				})
				.unwrap();

			ctx.canvas
				.copy(
					&world_texture,
					None,
					Rect::new(
						(ctx.rect.width() as i32
							- options.ui.pamphlet_width as i32
							- width as i32 * options.board.scale as i32)
							/ 2,
						(ctx.rect.height() as i32
							- options.ui.console_height as i32
							- height as i32 * options.board.scale as i32)
							/ 2,
						width * options.board.scale,
						height * options.board.scale,
					),
				)
				.unwrap();

			// Render User Interface
			ctx.canvas.set_viewport(None);

			let mut menu = ctx.view(
				0,
				(ctx.rect.height() - options.ui.console_height) as i32,
				ctx.rect.width() - options.ui.pamphlet_width,
				options.ui.console_height,
			);
			gui::widget::menu(
				&mut menu,
				options,
				input_mode,
				world,
				lua,
				&self.console,
				&self.resources,
				&self.textures,
			);

			// Draw pamphlet
			let mut pamphlet_ctx = ctx.view(
				(ctx.rect.width() - options.ui.pamphlet_width) as i32,
				0,
				options.ui.pamphlet_width,
				ctx.rect.height(),
			);

			self.cloudy_wave.draw(
				pamphlet_ctx.canvas,
				pamphlet_ctx.rect,
				20,
				(0x08, 0x0f, 0x25).into(),
			);

			self.pamphlet.draw(
				&mut pamphlet_ctx,
				world,
				lua,
				&self.resources,
				&self.textures,
				&self.soul_jar,
			);
		}
	}
}
