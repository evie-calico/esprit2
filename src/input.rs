use crate::prelude::*;
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
		submitted: bool,
		state: CursorState,
	},
	Prompt {
		response: Option<bool>,
		message: String,
	},
}

pub enum Response {
	Exit,
	Fullscreen,
	Debug,
}

pub fn world(
	event_pump: &mut sdl2::EventPump,
	world_manager: &mut world::Manager,
	resources: &resource::Manager,
	mode: &mut Mode,
	options: &Options,
) -> Result<Option<Response>> {
	match mode {
		input::Mode::Cursor {
			submitted: true, ..
		}
		| input::Mode::Prompt {
			response: Some(_), ..
		} => *mode = input::Mode::Normal,
		_ => (),
	}

	for event in event_pump.poll_iter() {
		match event {
			Event::Quit { .. } => return Ok(Some(Response::Exit)),
			Event::KeyDown {
				keycode: Some(keycode),
				..
			} => {
				let mut next_character = world_manager.next_character().borrow_mut();
				if true {
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
								(&options.controls.left, character::OrdDir::Left),
								(&options.controls.right, character::OrdDir::Right),
								(&options.controls.up, character::OrdDir::Up),
								(&options.controls.down, character::OrdDir::Down),
								(&options.controls.up_left, character::OrdDir::UpLeft),
								(&options.controls.up_right, character::OrdDir::UpRight),
								(&options.controls.down_left, character::OrdDir::DownLeft),
								(&options.controls.down_right, character::OrdDir::DownRight),
							];
							for (triggers, direction) in directions {
								if triggers.contains(keycode) {
									next_character.next_action =
										Some(character::Action::Move(direction));
								}
							}

							if options.controls.cast.contains(keycode) {
								*mode = Mode::Cast;
							}

							if options.controls.attack.contains(keycode) {
								*mode = Mode::Attack;
							}

							let (x, y) = (next_character.x, next_character.y);
							drop(next_character);

							if options.controls.underfoot.contains(keycode) {
								match world_manager.current_floor.map.get(y, x) {
									Some(floor::Tile::Floor) => {
										world_manager.console.print_unimportant(
											"There's nothing on the ground here.".into(),
										);
									}
									Some(floor::Tile::Exit) => {
										world_manager.new_floor(resources)?;
									}
									None => {
										world_manager
											.console
											.print_unimportant("That's the void.".into());
									}
									Some(floor::Tile::Wall) => (),
								}
							}

							if options.controls.talk.contains(keycode) {
								world_manager.console.say("Luvui".into(), "Meow!".into());
								world_manager
									.console
									.say("Aris".into(), "I am a kitty :3".into());
							}
						}
						Mode::Attack => {
							if options.controls.escape.contains(keycode) {
								*mode = Mode::Normal;
							}

							// TODO: just make an array of keys in the options file or something.
							let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
							if (0..=26).contains(&selected_index)
								&& (selected_index as usize) < next_character.spells.len()
							{
								next_character.next_action = Some(character::Action::Attack(
									next_character.attacks[selected_index as usize].clone(),
								))
							}
							*mode = Mode::Normal;
						}
						Mode::Cast => {
							if options.controls.escape.contains(keycode) {
								*mode = Mode::Normal;
							}

							// TODO: just make an array of keys in the options file or something.
							let selected_index = (keycode.into_i32()) - (Keycode::A.into_i32());
							if (0..=26).contains(&selected_index)
								&& (selected_index as usize) < next_character.spells.len()
							{
								next_character.next_action = Some(character::Action::Cast(
									next_character.spells[selected_index as usize].clone(),
								))
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
						}
					}
				}
			}
			_ => {}
		}
	}

	Ok(None)
}
