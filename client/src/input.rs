use crate::prelude::*;
use esprit2::prelude::*;
use mlua::FromLua;
use mlua::LuaSerdeExt;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

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
					keycode: Some(Keycode::BACKSPACE),
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, FromLua)]
#[serde(tag = "type")]
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

pub(crate) struct RequestConstructor;

impl mlua::UserData for RequestConstructor {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_function("Cursor", |_, (x, y, range, radius)| {
			Ok(Request::Cursor {
				x,
				y,
				range,
				radius,
			})
		});
		methods.add_function("Prompt", |_, message| Ok(Request::Prompt { message }));
		methods.add_function("Direction", |_, message| Ok(Request::Direction { message }));
	}
}

pub(crate) enum PartialAction<'lua> {
	Attack(resource::Attack, character::Ref, mlua::Thread<'lua>),
	Spell(resource::Spell, character::Ref, mlua::Thread<'lua>),
}

impl<'lua> PartialAction<'lua> {
	fn resolve(
		self,
		lua: &'lua mlua::Lua,
		arg: impl mlua::IntoLuaMulti<'lua>,
	) -> esprit2::Result<Response<'lua>> {
		match self {
			PartialAction::Attack(attack, next_character, thread) => {
				let value = thread.resume(arg)?;
				if let mlua::ThreadStatus::Resumable = thread.status() {
					Ok(Response::Partial(
						PartialAction::Attack(attack, next_character, thread),
						Request::from_lua(value, lua)?,
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
						PartialAction::Spell(spell, next_character, thread),
						Request::from_lua(value, lua)?,
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
	// Select modes
	Select,
	Attack,
	Cast,
	// Prompt modes
	Cursor(Cursor<'lua>),
	Prompt(Prompt<'lua>),
	DirectionPrompt(DirectionPrompt<'lua>),
}

pub(crate) enum Response<'lua> {
	Select(select::Point),
	Act(character::Action),
	Partial(PartialAction<'lua>, Request),
}

pub(crate) fn controllable_character<'lua>(
	keycode: sdl2::keyboard::Keycode,
	world: &world::Manager,
	console: impl console::Handle,
	resources: &resource::Manager,
	scripts: &resource::Scripts<'lua>,
	mode: Mode<'lua>,
	options: &Options,
) -> Result<(Mode<'lua>, Option<Response<'lua>>)> {
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
					if world.get_character_at(x, y).is_some() {
						if let Some(default_attack) = world
							.next_character()
							.borrow()
							.sheet
							.attacks
							.first()
							.cloned()
						{
							return Ok((
								mode,
								Some(Response::Act(character::Action::Attack(
									default_attack,
									Value::Table(Box::new([(
										Value::String("target".into()),
										Value::Table(Box::new([
											(Value::String("x".into()), Value::Integer(x.into())),
											(Value::String("y".into()), Value::Integer(y.into())),
										])),
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
						console.print_unimportant("There's nothing on the ground here.".into());
					}
					Some(floor::Tile::Exit) => {
						// TODO: move to server.
						// world.new_floor(resources, console)?;
						todo!();
					}
					None => {
						console.print_unimportant("That's the void.".into());
					}
					Some(floor::Tile::Wall) => (),
				}
			}

			if options.controls.talk.contains(keycode) {
				console.say("Luvui".into(), "Meow!".into());
				console.say("Aris".into(), "I am a kitty :3".into());
			}

			if options.controls.autocombat.contains(keycode) {
				let considerations = world.consider_turn(resources, scripts)?;
				let action = world.consider_action(
					scripts,
					world.next_character().clone(),
					considerations,
				)?;
				Ok((Mode::Normal, Some(Response::Act(action))))
			} else {
				Ok((Mode::Normal, None))
			}
		}
		Mode::Select => {
			let candidates = select::assign_indicies(world);
			// TODO: just make an array of keys in the options file or something.
			let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
			if (0..=26).contains(&selected_index)
				&& let Some(candidate) = candidates.into_iter().nth(selected_index as usize)
			{
				Ok((Mode::Normal, Some(Response::Select(candidate))))
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
			let attack_id = world
				.next_character()
				.borrow()
				.sheet
				.attacks
				.get(selected_index as usize)
				.cloned();
			if (0..=26).contains(&selected_index)
				&& let Some(attack_id) = attack_id
			{
				Ok((
					Mode::Normal,
					Some(gather_attack_inputs(
						resources,
						scripts,
						attack_id,
						world.next_character().clone(),
					)?),
				))
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
			let spell_id = world
				.next_character()
				.borrow()
				.sheet
				.spells
				.get(selected_index as usize)
				.cloned();
			if (0..=26).contains(&selected_index)
				&& let Some(spell_id) = spell_id
			{
				Ok((
					Mode::Normal,
					Some(gather_spell_inputs(
						resources,
						scripts,
						spell_id,
						world.next_character().clone(),
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
	resources: &resource::Manager,
	scripts: &resource::Scripts<'lua>,
	attack_id: resource::Attack,
	next_character: character::Ref,
) -> Result<Response<'lua>, Error> {
	let attack = resources.get(&attack_id)?;
	let thread = scripts
		.sandbox(&attack.on_input)?
		.insert("UseTime", attack.use_time)?
		.insert(
			"Magnitude",
			u32::evalv(&attack.magnitude, &*next_character.borrow())?,
		)?
		.insert("User", next_character.clone())?
		.thread()?;

	PartialAction::Attack(attack_id, next_character, thread).resolve(scripts.runtime, ())
}

fn gather_spell_inputs<'lua>(
	resources: &resource::Manager,
	scripts: &resource::Scripts<'lua>,
	spell_id: resource::Spell,
	next_character: character::Ref,
) -> Result<Response<'lua>, Error> {
	let spell = resources.get(&spell_id)?;
	let parameters = spell.parameter_table(scripts, &*next_character.borrow())?;
	let thread = scripts
		.sandbox(&spell.on_input)?
		.insert("Parameters", parameters)?
		.insert("User", next_character.clone())?
		.thread()?;
	PartialAction::Spell(spell_id, next_character, thread).resolve(scripts.runtime, ())
}
