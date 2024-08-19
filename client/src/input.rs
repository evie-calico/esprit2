use crate::options::Options;
use esprit2::prelude::*;
use sdl2::{event::Event, keyboard::Keycode};
use tracing::warn;

#[derive(Clone, Copy, Debug, Default)]
pub struct SinWave(u16);

impl SinWave {
	pub fn increment(&mut self, delta: f64) {
		self.0 = self.0.wrapping_add((u16::MAX as f64 * delta) as u16);
	}

	pub fn as_sin_period(self) -> f64 {
		(self.0 as f64) / (u16::MAX as f64) * std::f64::consts::TAU
	}

	pub fn sin(self) -> f64 {
		self.as_sin_period().sin()
	}
}

/// Anything beyond the bare minimum for cursor input.
/// This doesn't have anything to do with input,
/// but it is exclusive to the `Cursor` `input::Mode`.
#[derive(Clone, Copy, Default)]
pub struct CursorState {
	pub float: SinWave,
}

#[derive(Clone)]
pub enum Mode {
	Normal,
	Attack,
	Cast,
	Cursor {
		origin: (i32, i32),
		position: (i32, i32),
		range: u32,
		radius: Option<u32>,
		submitted: bool,
		state: CursorState,
	},
	Prompt {
		response: Option<bool>,
		message: String,
	},
	DirectionPrompt {
		response: Option<CardDir>,
		message: String,
	},
}

pub enum Response<'lua> {
	Exit,
	Fullscreen,
	Debug,
	Act(character::Action<'lua>),
}

#[allow(clippy::too_many_arguments)] // The alternative is to inline this.
pub fn controllable_character<'lua>(
	event_pump: &mut sdl2::EventPump,
	next_character: world::CharacterRef,
	world_manager: &mut world::Manager,
	console: &Console,
	resources: &resource::Manager,
	scripts: &'lua resource::Scripts,
	mode: &mut Mode,
	options: &Options,
) -> Result<Option<Response<'lua>>> {
	match mode {
		Mode::Cursor {
			submitted: true, ..
		}
		| Mode::Prompt {
			response: Some(_), ..
		}
		| Mode::DirectionPrompt {
			response: Some(_), ..
		} => *mode = Mode::Normal,
		_ => (),
	}

	for event in event_pump.poll_iter() {
		match event {
			Event::Quit { .. } => return Ok(Some(Response::Exit)),
			Event::KeyDown {
				keycode: Some(keycode),
				..
			} => {
				match mode {
					Mode::Normal => {
						// Eventually this will be a more involved binding.
						if options.controls.escape.contains(keycode) {
							return Ok(Some(Response::Exit));
						}
						if options.controls.debug.contains(keycode) {
							return Ok(Some(Response::Debug));
						}
						if options.controls.fullscreen.contains(keycode) {
							return Ok(Some(Response::Fullscreen));
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
								if let Some(potential_target) = world_manager.get_character_at(x, y)
								{
									let default_attack = next_character
										.borrow()
										.sheet
										.attacks
										.first()
										.map(|k| resources.get_attack(k));
									if let Some(default_attack) = default_attack {
										return Ok(Some(Response::Act(character::Action::Attack(
											default_attack?.clone(),
											Some(scripts.runtime.create_table_from([(
												"target",
												potential_target.clone(),
											)])?),
										))));
									}
								} else {
									return Ok(Some(Response::Act(character::Action::Move(x, y))));
								}
							}
						}

						if options.controls.cast.contains(keycode) {
							*mode = Mode::Cast;
						}

						if options.controls.attack.contains(keycode) {
							*mode = Mode::Attack;
						}

						let (x, y) = {
							let next_character = next_character.borrow();
							(next_character.x, next_character.y)
						};

						if options.controls.underfoot.contains(keycode) {
							match world_manager.current_floor.get(x as usize, y as usize) {
								Some(floor::Tile::Floor) => {
									console.print_unimportant(
										"There's nothing on the ground here.".into(),
									);
								}
								Some(floor::Tile::Exit) => {
									world_manager.new_floor(resources, console)?;
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
							let considerations = world_manager.consider_turn(resources, scripts)?;
							let action = world_manager.consider_action(
								scripts,
								next_character.clone(),
								considerations,
							)?;
							return Ok(Some(Response::Act(action)));
						}
					}
					Mode::Attack => {
						if options.controls.escape.contains(keycode) {
							*mode = Mode::Normal;
						}

						// TODO: just make an array of keys in the options file or something.
						let next_character = next_character.borrow();
						let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
						if (0..=26).contains(&selected_index)
							&& (selected_index as usize) < next_character.sheet.attacks.len()
						{
							return Ok(Some(Response::Act(character::Action::Attack(
								resources
									.get_attack(
										&next_character.sheet.attacks[selected_index as usize],
									)?
									.clone(),
								None,
							))));
						}
						*mode = Mode::Normal;
					}
					Mode::Cast => {
						if options.controls.escape.contains(keycode) {
							*mode = Mode::Normal;
						}

						// TODO: just make an array of keys in the options file or something.
						let next_character = next_character.borrow();
						let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
						if (0..=26).contains(&selected_index)
							&& (selected_index as usize) < next_character.sheet.spells.len()
						{
							return Ok(Some(Response::Act(character::Action::Cast(
								resources
									.get_spell(
										&next_character.sheet.spells[selected_index as usize],
									)?
									.clone(),
								None,
							))));
						}
						*mode = Mode::Normal;
					}
					Mode::Cursor {
						origin,
						range,
						position: (ref mut x, ref mut y),
						ref mut submitted,
						..
					} => {
						let range = *range as i32 + 1;

						if *submitted {
							warn!("entering cursor mode after submission");
						}

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
								let tx = *x + x_off;
								let ty = *y + y_off;
								if origin.0 - range < tx && origin.0 + range > tx {
									*x = tx;
								}
								if origin.1 - range < ty && origin.1 + range > ty {
									*y = ty;
								}
							}
						}

						if options.controls.escape.contains(keycode) {
							*mode = Mode::Normal;
						} else if options.controls.confirm.contains(keycode) {
							*submitted = true;
						}
					}
					Mode::Prompt { response, .. } => {
						if options.controls.yes.contains(keycode) {
							*response = Some(true);
						}
						if options.controls.no.contains(keycode) {
							*response = Some(false);
						}
						if options.controls.escape.contains(keycode) {
							*mode = Mode::Normal;
						}
					}
					Mode::DirectionPrompt { response, .. } => {
						if options.controls.left.contains(keycode) {
							*response = Some(CardDir::Left);
						}
						if options.controls.right.contains(keycode) {
							*response = Some(CardDir::Right);
						}
						if options.controls.up.contains(keycode) {
							*response = Some(CardDir::Up);
						}
						if options.controls.down.contains(keycode) {
							*response = Some(CardDir::Down);
						}
						if options.controls.escape.contains(keycode) {
							*mode = Mode::Normal;
						}
					}
				}
			}
			_ => {}
		}
	}

	Ok(None)
}
