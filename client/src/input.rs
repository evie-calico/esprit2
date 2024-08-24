use crate::{options::Options, ServerHandle};
use esprit2::prelude::*;
use mlua::LuaSerdeExt;
use sdl2::keyboard::Keycode;
use std::rc::Rc;

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub(crate) enum InputRequest {
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

pub(crate) enum PartialAction<'lua> {
	Attack(Rc<Attack>, character::Ref, mlua::Thread<'lua>),
	Spell(Rc<Spell>, character::Ref, mlua::Thread<'lua>),
}

impl<'lua> PartialAction<'lua> {
	fn resolve(
		self,
		lua: &'lua mlua::Lua,
		arg: impl mlua::IntoLuaMulti<'lua>,
	) -> esprit2::Result<Response> {
		match self {
			PartialAction::Attack(attack, next_character, thread) => {
				let value = thread.resume(arg)?;
				if let mlua::ThreadStatus::Resumable = thread.status() {
					Ok(Response::Partial(
						PartialAction::Attack(attack, next_character.clone(), thread),
						lua.from_value(value)?,
					))
				} else {
					Ok(Response::Act(character::Action::Attack(
						attack,
						lua.from_value(value)?,
					)))
				}
			}
			PartialAction::Spell(spell, next_character, thread) => {
				let value = thread.resume(arg)?;
				if let mlua::ThreadStatus::Resumable = thread.status() {
					Ok(Response::Partial(
						PartialAction::Spell(spell, next_character.clone(), thread),
						lua.from_value(value)?,
					))
				} else {
					Ok(Response::Act(character::Action::Cast(
						spell,
						lua.from_value(value)?,
					)))
				}
			}
		}
	}
}

pub(crate) struct Cursor<'lua> {
	pub(crate) origin: (i32, i32),
	pub(crate) position: (i32, i32),
	pub(crate) range: u32,
	pub(crate) radius: Option<u32>,
	pub(crate) state: CursorState,
	pub(crate) callback: PartialAction<'lua>,
}

pub(crate) struct Prompt<'lua> {
	pub(crate) message: String,
	pub(crate) callback: PartialAction<'lua>,
}

pub(crate) struct DirectionPrompt<'lua> {
	pub(crate) message: String,
	pub(crate) callback: PartialAction<'lua>,
}

pub(crate) enum Mode<'lua> {
	Normal,
	Attack,
	Cast,
	Cursor(Cursor<'lua>),
	Prompt(Prompt<'lua>),
	DirectionPrompt(DirectionPrompt<'lua>),
}

pub(crate) enum Response<'lua> {
	Fullscreen,
	Debug,
	Act(character::Action),
	Partial(PartialAction<'lua>, InputRequest),
}

pub(crate) fn controllable_character<'lua>(
	keycode: sdl2::keyboard::Keycode,
	next_character: world::CharacterRef,
	server: &ServerHandle,
	scripts: &resource::Scripts<'lua>,
	mode: Mode<'lua>,
	options: &Options,
) -> Result<(Mode<'lua>, Option<Response<'lua>>)> {
	match mode {
		Mode::Normal => {
			if options.controls.debug.contains(keycode) {
				return Ok((mode, Some(Response::Debug)));
			}
			if options.controls.fullscreen.contains(keycode) {
				return Ok((mode, Some(Response::Fullscreen)));
			}
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
						let next_character = next_character.borrow();
						(next_character.x + xoff, next_character.y + yoff)
					};
					if server.world().get_character_at(x, y).is_some() {
						let default_attack = next_character
							.borrow()
							.sheet
							.attacks
							.first()
							.map(|k| server.resources().get_attack(k));
						if let Some(default_attack) = default_attack {
							return Ok((
								mode,
								Some(Response::Act(character::Action::Attack(
									default_attack?.clone(),
									character::ActionArgs(std::collections::HashMap::from([(
										"target".into(),
										character::ActionArg::Position { x, y },
									)])),
								))),
							));
						}
					} else {
						return Ok((mode, Some(Response::Act(character::Action::Move(x, y)))));
					}
				}
			}

			if options.controls.cast.contains(keycode) {
				return Ok((Mode::Cast, None));
			}

			if options.controls.attack.contains(keycode) {
				return Ok((Mode::Attack, None));
			}

			let (x, y) = {
				let next_character = next_character.borrow();
				(next_character.x, next_character.y)
			};

			if options.controls.underfoot.contains(keycode) {
				match server.world().current_floor.get(x as usize, y as usize) {
					Some(floor::Tile::Floor) => {
						server
							.console()
							.print_unimportant("There's nothing on the ground here.".into());
					}
					Some(floor::Tile::Exit) => {
						// TODO: move to server.
						// world.new_floor(resources, console)?;
						todo!();
					}
					None => {
						server
							.console()
							.print_unimportant("That's the void.".into());
					}
					Some(floor::Tile::Wall) => (),
				}
			}

			if options.controls.talk.contains(keycode) {
				server.console().say("Luvui".into(), "Meow!".into());
				server
					.console()
					.say("Aris".into(), "I am a kitty :3".into());
			}

			if options.controls.autocombat.contains(keycode) {
				let considerations = server.world().consider_turn(server.resources(), scripts)?;
				let action = server.world().consider_action(
					scripts,
					next_character.clone(),
					considerations,
				)?;
				Ok((Mode::Normal, Some(Response::Act(action))))
			} else {
				Ok((Mode::Normal, None))
			}
		}
		Mode::Attack => {
			if options.controls.escape.contains(keycode) {
				return Ok((Mode::Normal, None));
			}

			// TODO: just make an array of keys in the options file or something.
			let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
			if (0..=26).contains(&selected_index)
				&& (selected_index as usize) < next_character.borrow().sheet.attacks.len()
			{
				let attack = server
					.resources()
					.get_attack(&next_character.borrow().sheet.attacks[selected_index as usize])?;
				let response = gather_attack_inputs(scripts, attack.clone(), &next_character)?;
				Ok((Mode::Normal, Some(response)))
			} else {
				Ok((Mode::Normal, None))
			}
		}
		Mode::Cast => {
			if options.controls.escape.contains(keycode) {
				return Ok((Mode::Normal, None));
			}

			// TODO: just make an array of keys in the options file or something.
			let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
			if (0..=26).contains(&selected_index)
				&& (selected_index as usize) < next_character.borrow().sheet.spells.len()
			{
				let spell = server
					.resources()
					.get_spell(&next_character.borrow().sheet.spells[selected_index as usize])?
					.clone();
				Ok((
					Mode::Normal,
					Some(gather_spell_inputs(
						scripts,
						spell.clone(),
						&next_character,
					)?),
				))
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
					Some(cursor.callback.resolve(scripts.runtime, cursor.position)?),
				))
			} else {
				Ok((Mode::Cursor(cursor), None))
			}
		}
		Mode::Prompt(prompt) => {
			if options.controls.yes.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(prompt.callback.resolve(scripts.runtime, true)?),
				))
			} else if options.controls.no.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(prompt.callback.resolve(scripts.runtime, false)?),
				))
			} else if options.controls.escape.contains(keycode) {
				Ok((Mode::Normal, None))
			} else {
				Ok((Mode::Prompt(prompt), None))
			}
		}
		Mode::DirectionPrompt(prompt) => {
			if options.controls.left.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(prompt.callback.resolve(scripts.runtime, "Left")?),
				))
			} else if options.controls.right.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(prompt.callback.resolve(scripts.runtime, "Right")?),
				))
			} else if options.controls.up.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(prompt.callback.resolve(scripts.runtime, "Up")?),
				))
			} else if options.controls.down.contains(keycode) {
				Ok((
					Mode::Normal,
					Some(prompt.callback.resolve(scripts.runtime, "Down")?),
				))
			} else if options.controls.escape.contains(keycode) {
				Ok((Mode::Normal, None))
			} else {
				Ok((Mode::DirectionPrompt(prompt), None))
			}
		}
	}
}

fn gather_attack_inputs<'lua>(
	scripts: &resource::Scripts<'lua>,
	attack: Rc<Attack>,
	next_character: &character::Ref,
) -> Result<Response<'lua>, Error> {
	let thread = scripts
		.sandbox(&attack.on_input)?
		.insert("UseTime", attack.use_time)?
		.insert(
			"Magnitude",
			u32::evalv(&attack.magnitude, &*next_character.borrow()),
		)?
		.insert("User", next_character.clone())?
		.thread()?;
	let value = thread.resume(())?;
	if let mlua::ThreadStatus::Resumable = thread.status() {
		Ok(Response::Partial(
			PartialAction::Attack(attack, next_character.clone(), thread),
			scripts.runtime.from_value(value)?,
		))
	} else {
		Ok(Response::Act(character::Action::Attack(
			attack,
			scripts.runtime.from_value(value)?,
		)))
	}
}

fn gather_spell_inputs<'lua>(
	scripts: &resource::Scripts<'lua>,
	spell: Rc<Spell>,
	next_character: &character::Ref,
) -> Result<Response<'lua>, Error> {
	let parameters = spell.parameter_table(scripts, &*next_character.borrow())?;
	let thread = scripts
		.sandbox(&spell.on_input)?
		.insert("Parameters", parameters)?
		.insert("User", next_character.clone())?
		.thread()?;
	let value = thread.resume(())?;
	if let mlua::ThreadStatus::Resumable = thread.status() {
		Ok(Response::Partial(
			PartialAction::Spell(spell, next_character.clone(), thread),
			scripts.runtime.from_value(value)?,
		))
	} else {
		Ok(Response::Act(character::Action::Cast(
			spell,
			scripts.runtime.from_value(value)?,
		)))
	}
}
