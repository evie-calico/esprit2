#![feature(anonymous_lifetime_in_impl_trait, once_cell_try)]

pub mod draw;
pub mod gui;
pub mod input;
pub mod options;
pub mod texture;
pub mod typography;

use esprit2::prelude::*;
use esprit2::world::{ActionRequest, TurnOutcome};
use mlua::LuaSerdeExt;
use options::Options;
use sdl2::rect::Rect;
use std::process::exit;
use std::{fs, io};
use tracing::{error, info, warn};
use typography::Typography;
use world::PartialAction;

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
	let resources = match resource::Manager::open(options::resource_directory()) {
		Ok(resources) => resources,
		Err(msg) => {
			error!("failed to open resource directory: {msg}");
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
	let mut console = Console::new(options.ui.colors.console.clone());
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
	lua.globals()
		.set("Console", console.handle.clone())
		.unwrap();
	lua.globals()
		.set("Status", resources.statuses_handle())
		.unwrap();
	let scripts =
		match resource::Scripts::open(options::resource_directory().join("scripts/"), &lua) {
			Ok(scripts) => scripts,
			Err(msg) => {
				error!("failed to open scripts directory: {msg}");
				exit(1);
			}
		};
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
	let mut world_manager = world::Manager::new(party_blueprint.into_iter(), &resources)
		.unwrap_or_else(|msg| {
			error!("failed to initialize world manager: {msg}");
			exit(1);
		});
	world_manager.generate_floor(
		"default seed",
		&vault::Set {
			vaults: vec!["example".into()],
			density: 4,
			hall_ratio: 1,
		},
		&resources,
	);

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
	let mut partial_action = None;
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
					&console,
					&resources,
					&scripts,
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
					Ok(Some(input::Response::Act(action))) => {
						partial_action = Some(PartialAction::Action(action))
					}
					Ok(None) => (),
					Err(msg) => {
						error!("world input processing returned an error: {msg}");
					}
				}
			} else {
				let considerations = world_manager.consider_turn(&scripts).unwrap();
				let action = world_manager
					.consider_action(&scripts, next_character, considerations)
					.unwrap();
				partial_action = Some(PartialAction::Action(action));
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

			for i in &mut pamphlet.party_member_clouds {
				i.cloud.tick(delta);
				i.cloud_trail.tick(delta / 4.0);
			}
			if let Some(inner_action) = partial_action {
				match update_world(
					&mut world_manager,
					&scripts,
					&console,
					&mut input_mode,
					inner_action,
				) {
					Ok(result) => partial_action = result.map(PartialAction::Request),
					Err(msg) => {
						error!("world manager update returned an error: {msg}");
						partial_action = None;
					}
				}
			}
			world_manager
				.characters
				.retain(|character| character.borrow().hp > 0);
			console.update(delta);
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
			canvas.set_draw_color((20, 20, 20));
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
					canvas.set_draw_color((20, 20, 20));
					canvas.clear();
					draw::tilemap(canvas, &world_manager, &camera);
					draw::characters(canvas, &world_manager, &textures, &camera);
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
			gui::widget::menu(
				&mut menu,
				&options,
				&input_mode,
				&world_manager,
				&console,
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

			pamphlet.draw(&mut pamphlet_ctx, &world_manager, &textures, &mut soul_jar);

			canvas.present();
		}
	}
}

fn update_world<'lua>(
	this: &mut world::Manager,
	scripts: &'lua resource::Scripts,
	console: &Console,
	input_mode: &mut input::Mode,
	inner_action: PartialAction<'lua>,
) -> Result<Option<world::ActionRequest<'lua>>, Error> {
	let outcome = match (inner_action, input_mode.clone()) {
		// Handle targetted cursor submission
		(
			PartialAction::Request(ActionRequest::BeginTargetCursor { callback, .. }),
			input::Mode::Cursor {
				position: (x, y),
				submitted: true,
				..
			},
		) => {
			if let Some(character) = this.get_character_at(x, y) {
				TurnOutcome::poll(this, scripts.runtime, callback, character.clone())?
			} else {
				// If the cursor hasn't selected a character,
				// cancel the request altogther.
				// This destroys the lua callback.
				TurnOutcome::Yield
			}
		}
		// Handle positional cursor submission
		(
			PartialAction::Request(ActionRequest::BeginCursor { callback, .. }),
			input::Mode::Cursor {
				position: (x, y),
				submitted: true,
				..
			},
		) => TurnOutcome::poll(this, scripts.runtime, callback, (x, y))?,
		// An unsubmitted cursor yields the same action request.
		(
			PartialAction::Request(
				request @ (ActionRequest::BeginCursor { .. }
				| ActionRequest::BeginTargetCursor { .. }),
			),
			input::Mode::Cursor {
				submitted: false, ..
			},
		) => {
			return Ok(Some(request));
		}
		// Prompt with submitted response
		(
			PartialAction::Request(ActionRequest::ShowPrompt { callback, .. }),
			input::Mode::Prompt {
				response: Some(response),
				..
			},
		) => TurnOutcome::poll(this, scripts.runtime, callback, response)?,
		// Prompt with unsubmitted response
		(
			PartialAction::Request(request @ ActionRequest::ShowPrompt { .. }),
			input::Mode::Prompt { response: None, .. },
		) => return Ok(Some(request)),
		// Direction prompt with submitted response
		(
			PartialAction::Request(ActionRequest::ShowDirectionPrompt { callback, .. }),
			input::Mode::DirectionPrompt {
				response: Some(response),
				..
			},
		) => TurnOutcome::poll(
			this,
			scripts.runtime,
			callback,
			scripts.runtime.to_value(&response),
		)?,
		// Direction prompt with unsubmitted response
		(
			PartialAction::Request(request @ ActionRequest::ShowDirectionPrompt { .. }),
			input::Mode::DirectionPrompt { response: None, .. },
		) => return Ok(Some(request)),
		// If the input mode is invalid in any way, the callback will be destroyed.
		(
			PartialAction::Request(
				ActionRequest::BeginCursor { .. }
				| ActionRequest::BeginTargetCursor { .. }
				| ActionRequest::ShowPrompt { .. }
				| ActionRequest::ShowDirectionPrompt { .. },
			),
			_,
		) => TurnOutcome::Yield,
		// If there is no pending request, pop a turn off the character queue.
		(PartialAction::Action(action), _) => this.next_turn(console, scripts, action)?,
	};

	let player_controlled = this.next_character().borrow().player_controlled;
	let mut apply_delay = |delay| {
		#[allow(
			clippy::unwrap_used,
			reason = "next_turn already indexes the first element"
		)]
		let character = this.characters.pop_front().unwrap();
		character.borrow_mut().action_delay = delay;
		// Insert the character into the queue,
		// immediately before the first character to have a higher action delay.
		// This assumes that the queue is sorted.
		this.characters.insert(
			this.characters
				.iter()
				.enumerate()
				.find(|x| x.1.borrow().action_delay > delay)
				.map(|x| x.0)
				.unwrap_or(this.characters.len()),
			character,
		);
	};

	match outcome {
		TurnOutcome::Yield => {
			if !player_controlled {
				apply_delay(TURN);
			}
			Ok(None)
		}
		TurnOutcome::Action { delay } => {
			apply_delay(delay);
			Ok(None)
		}
		TurnOutcome::Request(request) => {
			// Set up any new action requests.
			match &request {
				world::ActionRequest::BeginCursor {
					x,
					y,
					range,
					radius,
					callback: _,
				} => {
					*input_mode = input::Mode::Cursor {
						origin: (*x, *y),
						position: (*x, *y),
						range: *range,
						radius: *radius,
						submitted: false,
						state: input::CursorState::default(),
					};
				}
				world::ActionRequest::BeginTargetCursor {
					x,
					y,
					range,
					callback: _,
				} => {
					*input_mode = input::Mode::Cursor {
						origin: (*x, *y),
						position: (*x, *y),
						range: *range,
						radius: None,
						submitted: false,
						state: input::CursorState::default(),
					};
				}
				world::ActionRequest::ShowPrompt { message, .. } => {
					*input_mode = input::Mode::Prompt {
						response: None,
						message: message.clone(),
					}
				}
				world::ActionRequest::ShowDirectionPrompt { message, .. } => {
					*input_mode = input::Mode::DirectionPrompt {
						response: None,
						message: message.clone(),
					}
				}
			}
			Ok(Some(request))
		}
	}
}
