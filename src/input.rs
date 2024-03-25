use crate::prelude::*;
use sdl2::{
	event::Event,
	keyboard::{Keycode, Scancode},
};

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
			Event::Quit { .. }
			| Event::KeyDown {
				scancode: Some(Scancode::Escape),
				..
			} => return Result { exit: true },
			Event::KeyDown {
				keycode: Some(keycode),
				..
			} => {
				let mut next_character = world_manager.next_character().borrow_mut();
				if next_character.player_controlled {
					match *mode {
						Mode::Normal => {
							// This will need to be refactored.
							if options.controls.left.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::Left));
							}
							if options.controls.right.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::Right));
							}
							if options.controls.up.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::Up));
							}
							if options.controls.down.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::Down));
							}
							if options.controls.up_left.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::UpLeft));
							}
							if options.controls.up_right.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::UpRight));
							}
							if options.controls.down_left.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::DownLeft));
							}
							if options.controls.down_right.contains(&(keycode as i32)) {
								next_character.next_action =
									Some(character::Action::Move(character::OrdDir::DownRight));
							}
							if options.controls.cast.contains(&(keycode as i32)) {
								*mode = Mode::Cast;
							}
							drop(next_character);

							if options.controls.talk.contains(&(keycode as i32)) {
								world_manager.console.say("Luvui".into(), "Meow!");
								world_manager.console.say("Aris".into(), "I am a kitty :3");
							}
						}
						Mode::Cast => {
							let selected_index = (keycode as i32) - (Keycode::A as i32);
							if (0..=26).contains(&selected_index) {
								if let Some(spell) =
									next_character.sheet.spells.get(selected_index as usize)
								{
									let _message = format!("{spell:?}");
								}
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
