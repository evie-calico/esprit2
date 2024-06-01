use crate::prelude::*;
use sdl2::{event::Event, keyboard::Keycode};

pub enum Mode {
	Normal,
	Cast,
}

pub struct Result {
	pub exit: bool,
}

pub fn world(
	event_pump: &mut sdl2::EventPump,
	world_manager: &mut world::Manager,
	mode: &mut Mode,
	options: &Options,
) -> Result {
	for event in event_pump.poll_iter() {
		match event {
			Event::Quit { .. } => return Result { exit: true },
			Event::KeyDown {
				keycode: Some(keycode),
				..
			} => {
				let mut next_character = world_manager.next_character().write();
				if next_character.player_controlled {
					match *mode {
						Mode::Normal => {
							// Eventually this will be a more involved binding.
							if options.controls.escape.contains(&(keycode as i32)) {
								return Result { exit: true };
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
								if triggers.contains(&(keycode as i32)) {
									next_character.next_action =
										Some(character::Action::Move(direction));
								}
							}
							drop(next_character);

							if options.controls.cast.contains(&(keycode as i32)) {
								*mode = Mode::Cast;
							}

							if options.controls.talk.contains(&(keycode as i32)) {
								let mut console = world_manager.console.write();
								console.say("Luvui".into(), "Meow!".into());
								console.say("Aris".into(), "I am a kitty :3".into());
							}
						}
						Mode::Cast => {
							if options.controls.escape.contains(&(keycode as i32)) {
								*mode = Mode::Normal;
							}

							let selected_index = (keycode as i32) - (Keycode::A as i32);
							if (0..=26).contains(&selected_index)
								&& (selected_index as usize) < next_character.spells.len()
							{
								next_character.next_action = Some(character::Action::Cast(
									next_character.spells[selected_index as usize].clone(),
								))
							}
							*mode = Mode::Normal;
						}
					}
				}
			}
			_ => {}
		}
	}

	Result { exit: false }
}
