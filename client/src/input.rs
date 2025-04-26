use crate::prelude::*;
use esprit2::anyhow::Context;
use esprit2::prelude::*;
use mlua::FromLua;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;

pub(crate) enum Signal<T> {
	None,
	Cancel,
	Yield(T),
}

#[derive(Debug, Default)]
pub(crate) struct LineInput {
	pub(crate) line: String,
	pub(crate) submitted: bool,
}

impl std::ops::Deref for LineInput {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		self.line.as_str()
	}
}

impl LineInput {
	/// Returns `true` when closed.
	///
	/// # Panics
	///
	/// Panics when recieving an empty set of leaves.
	pub(crate) fn dispatch<T>(
		&mut self,
		event: &Event,
		options: &Options,
		submit: impl FnOnce(&str) -> Signal<T>,
	) -> Signal<T> {
		if self.submitted {
			let signal = submit(&self.line);
			if let Signal::Cancel = signal {
				self.submitted = false;
			} else {
				return signal;
			}
		} else {
			match event {
				Event::TextInput { text, .. } => self.line.push_str(text),
				Event::KeyDown {
					keycode: Some(Keycode::Backspace),
					..
				} => {
					self.line.pop();
				}
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.confirm.contains(*keycode) => {
					self.submitted = true;
				}
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.escape.contains(*keycode) => return Signal::Cancel,
				_ => {}
			}
		}
		Signal::None
	}
}

pub(crate) trait RadioBacker {
	fn inc(&mut self) -> bool;
	fn dec(&mut self) -> bool;
	fn index(&self) -> usize;
}

/// A dialogue describing a list of things and whether or not they have been selected.
#[derive(Debug, Default)]
pub(crate) struct Radio<Backer: RadioBacker> {
	/// Tracks the currently "hovered" option.
	pub(crate) backer: Backer,
	/// Whether or not the radio is currently "submitted".
	///
	/// An unsubmitted radio consumes events,
	/// using them to translate the cursor.
	pub(crate) submitted: bool,
}

impl<Backer: RadioBacker> Radio<Backer> {
	/// Returns `true` when closed.
	///
	/// # Panics
	///
	/// Panics when recieving an empty set of leaves.
	pub(crate) fn dispatch<T>(
		&mut self,
		event: &Event,
		options: &Options,
		submit: impl FnOnce(&Backer) -> Signal<T>,
	) -> Signal<T> {
		if self.submitted {
			let signal = submit(&self.backer);
			if let Signal::Cancel = signal {
				self.submitted = false;
			} else {
				return signal;
			}
		} else {
			match event {
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.down.contains(*keycode) => {
					self.backer.inc();
				}
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.up.contains(*keycode) => {
					self.backer.dec();
				}
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.confirm.contains(*keycode) => {
					self.submitted = true;
				}
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} if options.controls.escape.contains(*keycode) => return Signal::Cancel,
				_ => {}
			}
		}
		Signal::None
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SinWave(u16);

impl SinWave {
	pub(crate) fn increment(&mut self, delta: f64) {
		self.0 = self.0.wrapping_add((u16::MAX as f64 * delta) as u16);
	}

	pub(crate) fn as_sin_period(self) -> f64 {
		(self.0 as f64) / (u16::MAX as f64) * std::f64::consts::TAU
	}

	pub(crate) fn sin(self) -> f64 {
		self.as_sin_period().sin()
	}
}

/// Anything beyond the bare minimum for cursor input.
/// This doesn't have anything to do with input,
/// but it is exclusive to the `Cursor` `input::Mode`.
#[derive(Clone, Copy, Default)]
pub(crate) struct CursorState {
	pub(crate) float: SinWave,
}

#[derive(Clone, Debug, FromLua)]
pub(crate) enum Request {
	Cursor {
		x: i32,
		y: i32,
		range: u32,
		radius: Option<u32>,
	},
	Prompt {
		message: String,
	},
	Direction {
		message: String,
	},
}

impl mlua::UserData for Request {}

pub(crate) enum PartialAction {
	Ability(Box<str>, character::Ref, mlua::Thread),
}

impl PartialAction {
	fn resolve(self, lua: &mlua::Lua, arg: impl mlua::IntoLuaMulti) -> mlua::Result<Response> {
		match self {
			PartialAction::Ability(ability, next_character, thread) => {
				let value = thread.resume(arg)?;
				if let mlua::ThreadStatus::Resumable = thread.status() {
					Ok(Response::Partial(
						PartialAction::Ability(ability, next_character, thread),
						Request::from_lua(value, lua)?,
					))
				} else {
					Ok(Response::Action(character::Action::Ability(
						ability,
						Value::from_lua(value, lua)?,
					)))
				}
			}
		}
	}
}

pub(crate) struct Cursor {
	pub(crate) origin: (i32, i32),
	pub(crate) position: (i32, i32),
	pub(crate) range: u32,
	pub(crate) radius: Option<u32>,
	pub(crate) state: CursorState,
	pub(crate) callback: PartialAction,
}

pub(crate) struct Prompt {
	pub(crate) message: String,
	pub(crate) callback: PartialAction,
}

pub(crate) struct DirectionPrompt {
	pub(crate) message: String,
	pub(crate) callback: PartialAction,
}

pub(crate) enum Mode {
	Normal,
	// Select modes
	Select,
	Act,
	// Prompt modes
	Cursor(Cursor),
	Prompt(Prompt),
	DirectionPrompt(DirectionPrompt),
}

pub(crate) enum Response {
	Select(select::Point),
	Action(character::Action),
	Partial(PartialAction, Request),
}

pub(crate) fn controllable_character(
	keycode: sdl3::keyboard::Keycode,
	world: &world::Manager,
	console: impl console::Handle,
	resources: &resource::Manager,
	lua: &mlua::Lua,
	mode: Mode,
	options: &Options,
) -> anyhow::Result<(Mode, Option<Response>)> {
	match mode {
		Mode::Normal => {
			let directions = [
				(&options.controls.left, -1, 0),
				(&options.controls.right, 1, 0),
				(&options.controls.up, 0, -1),
				(&options.controls.down, 0, 1),
				(&options.controls.up_left, -1, -1),
				(&options.controls.up_right, 1, -1),
				(&options.controls.down_left, -1, 1),
				(&options.controls.down_right, 1, 1),
			];
			for (triggers, xoff, yoff) in directions {
				if triggers.contains(keycode) {
					let (x, y) = {
						let next_character = world.next_character().borrow();
						(next_character.x + xoff, next_character.y + yoff)
					};
					return Ok((mode, Some(Response::Action(character::Action::Move(x, y)))));
				}
			}

			if options.controls.act.contains(keycode) {
				return Ok((Mode::Act, None));
			}

			if options.controls.select.contains(keycode) {
				return Ok((Mode::Select, None));
			}

			let (x, y) = {
				let next_character = world.next_character().borrow();
				(next_character.x, next_character.y)
			};

			if options.controls.underfoot.contains(keycode) {
				match world.current_floor.get(x, y) {
					Some(floor::Tile::Floor) => {
						console.print_unimportant("There's nothing on the ground here.");
					}
					Some(floor::Tile::Exit) => {
						todo!();
					}
					None => {
						console.print_unimportant("That's the void.");
					}
					Some(floor::Tile::Wall) => (),
				}
			}

			if options.controls.talk.contains(keycode) {
				console.say("Luvui", "Meow!");
				console.say("Aris", "I am a kitty :3");
			}

			if options.controls.autocombat.contains(keycode) {
				if let Some(action) = world.consider_action(lua, world.next_character().clone())? {
					Ok((Mode::Normal, Some(Response::Action(action))))
				} else {
					console.print_system("autocombat failed");
					Ok((Mode::Normal, None))
				}
			} else {
				Ok((Mode::Normal, None))
			}
		}
		Mode::Select => {
			let candidates = select::assign_indicies(world);
			// TODO: just make an array of keys in the options file or something.
			let selected_index = (u32::from(keycode)) - (u32::from(Keycode::A));
			if (0..=26).contains(&selected_index)
				&& let Some(candidate) = candidates.into_iter().nth(selected_index as usize)
			{
				Ok((Mode::Normal, Some(Response::Select(candidate))))
			} else {
				Ok((Mode::Normal, None))
			}
		}
		Mode::Act => {
			if options.controls.escape.contains(keycode) {
				return Ok((Mode::Normal, None));
			}

			// TODO: just make an array of keys in the options file or something.
			let selected_index = (u32::from(keycode)) - (u32::from(Keycode::A));
			let ability_id = world
				.next_character()
				.borrow()
				.sheet
				.abilities
				.get(selected_index as usize)
				.cloned();
			if (0..=26).contains(&selected_index)
				&& let Some(ability_id) = ability_id
			{
				let ability = resources
					.ability
					.get(&ability_id)
					.context("failed to retrieve ability")?;
				let character = world.next_character().clone();
				if ability.usable(character.clone())?.is_none() {
					Ok((
						Mode::Normal,
						Some(gather_ability_inputs(
							lua,
							ability,
							ability_id,
							world.next_character().clone(),
						)?),
					))
				} else {
					Ok((Mode::Normal, None))
				}
			} else {
				Ok((Mode::Normal, None))
			}
		}
		Mode::Cursor(mut cursor) => {
			let range = cursor.range as i32 + 1;

			let directions = [
				(-1, 0, &options.controls.left),
				(1, 0, &options.controls.right),
				(0, -1, &options.controls.up),
				(0, 1, &options.controls.down),
				(-1, -1, &options.controls.up_left),
				(1, -1, &options.controls.up_right),
				(-1, 1, &options.controls.down_left),
				(1, 1, &options.controls.down_right),
			];
			for (x_off, y_off, triggers) in directions {
				if triggers.contains(keycode) {
					let tx = cursor.position.0 + x_off;
					let ty = cursor.position.1 + y_off;
					if cursor.origin.0 - range < tx && cursor.origin.0 + range > tx {
						cursor.position.0 = tx;
					}
					if cursor.origin.1 - range < ty && cursor.origin.1 + range > ty {
						cursor.position.1 = ty;
					}
				}
			}

			if options.controls.escape.contains(keycode) {
				Ok((Mode::Normal, None))
			} else if options.controls.confirm.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(cursor.callback.resolve(lua, cursor.position)?),
				))
			} else {
				Ok((Mode::Cursor(cursor), None))
			}
		}
		Mode::Prompt(prompt) => {
			if options.controls.yes.contains(keycode) {
				Ok((Mode::Normal, Some(prompt.callback.resolve(lua, true)?)))
			} else if options.controls.no.contains(keycode) {
				Ok((Mode::Normal, Some(prompt.callback.resolve(lua, false)?)))
			} else if options.controls.escape.contains(keycode) {
				Ok((Mode::Normal, None))
			} else {
				Ok((Mode::Prompt(prompt), None))
			}
		}
		Mode::DirectionPrompt(prompt) => {
			if options.controls.left.contains(keycode) {
				Ok((Mode::Normal, Some(prompt.callback.resolve(lua, "Left")?)))
			} else if options.controls.right.contains(keycode) {
				Ok((Mode::Normal, Some(prompt.callback.resolve(lua, "Right")?)))
			} else if options.controls.up.contains(keycode) {
				Ok((Mode::Normal, Some(prompt.callback.resolve(lua, "Up")?)))
			} else if options.controls.down.contains(keycode) {
				Ok((Mode::Normal, Some(prompt.callback.resolve(lua, "Down")?)))
			} else if options.controls.escape.contains(keycode) {
				Ok((Mode::Normal, None))
			} else {
				Ok((Mode::DirectionPrompt(prompt), None))
			}
		}
	}
}

fn gather_ability_inputs(
	lua: &mlua::Lua,
	ability: &Ability,
	ability_id: Box<str>,
	next_character: character::Ref,
) -> anyhow::Result<Response> {
	lua.create_thread(ability.on_input.clone())
		.and_then(|thread| {
			PartialAction::Ability(ability_id, next_character.clone(), thread)
				.resolve(lua, (next_character, ability.clone()))
		})
		.context("failed to run ability input thread")
}
