use crate::input::Signal;
use crate::prelude::*;
use sdl2::event::Event;

pub trait Menu<T> {
	fn event(&mut self, event: &Event, options: &Options) -> Signal<T>;
	fn draw(&self, gui: &mut gui::Context, textures: &texture::Manager);
}

pub mod login {
	use super::Menu;
	use crate::input::{LineInput, Radio, RadioBacker, Signal};
	use crate::prelude::*;
	use crate::RootMenuResponse;

	#[derive(Default)]
	pub enum RootMenu {
		#[default]
		Singleplayer,
		Multiplayer,
	}

	impl RadioBacker for RootMenu {
		fn inc(&mut self) -> bool {
			*self = match self {
				RootMenu::Singleplayer => RootMenu::Multiplayer,
				RootMenu::Multiplayer => RootMenu::Singleplayer,
			};
			true
		}

		fn dec(&mut self) -> bool {
			*self = match self {
				RootMenu::Singleplayer => RootMenu::Multiplayer,
				RootMenu::Multiplayer => RootMenu::Singleplayer,
			};
			true
		}

		fn index(&self) -> usize {
			match self {
				RootMenu::Singleplayer => 0,
				RootMenu::Multiplayer => 1,
			}
		}
	}

	#[derive(Default)]
	pub struct State {
		pub username: LineInput,
		pub root_menu: Radio<RootMenu>,
		pub host: LineInput,
	}

	impl State {
		pub fn new(username: Option<&str>, host: Option<&str>) -> Self {
			Self {
				username: LineInput {
					line: username.unwrap_or("").to_string(),
					submitted: username.is_some(),
				},
				root_menu: Radio::default(),
				host: LineInput {
					line: host.unwrap_or("").to_string(),
					submitted: host.is_some(),
				},
			}
		}
	}

	impl Menu<RootMenuResponse> for State {
		fn event(
			&mut self,
			event: &sdl2::event::Event,
			options: &crate::Options,
		) -> Signal<RootMenuResponse> {
			self.username.dispatch(event, options, |_| {
				self.root_menu
					.dispatch(event, options, |backer| match backer {
						RootMenu::Singleplayer => Signal::Yield(RootMenuResponse::OpenSingleplayer),
						RootMenu::Multiplayer => self.host.dispatch(event, options, |host| {
							Signal::Yield(RootMenuResponse::OpenMultiplayer {
								host: host.to_owned(),
							})
						}),
					})
			})
		}

		fn draw(&self, gui: &mut gui::Context, textures: &texture::Manager) {
			if !self.username.submitted {
				gui.horizontal();
				gui.label("Enter your name: ");
				gui.label(&self.username);
				gui.vertical();
			} else {
				gui.horizontal();
				gui.label("Welcome, ");
				gui.label(&self.username);
				gui.vertical();

				gui.menu(
					Some((self.root_menu.backer.index(), textures.get("null"))),
					["Singleplayer", "Multiplayer"],
				);

				gui.horizontal();
				gui.advance(10, 0);
				if let menu::login::RootMenu::Multiplayer = self.root_menu.backer
					&& self.root_menu.submitted
				{
					gui.label("Connect to server: ");
					gui.label(&self.host);
				}
				gui.vertical();
			}
		}
	}
}
