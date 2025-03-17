use crate::input::Signal;
use crate::prelude::*;
use sdl3::event::Event;

pub(crate) trait Menu<T> {
	fn event(&mut self, event: &Event, options: &Options) -> Signal<T>;
	fn draw(&self, gui: &mut gui::Context);
}

pub(crate) mod login {
	use sdl3::render::Texture;

	use super::Menu;
	use crate::input::{LineInput, Radio, RadioBacker, Signal};
	use crate::prelude::*;
	use crate::RootMenuResponse;

	#[derive(Default)]
	pub(crate) enum RootMenu {
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

	pub(crate) struct State<'texture> {
		pub(crate) cursor: Texture<'texture>,

		pub(crate) username: LineInput,
		pub(crate) root_menu: Radio<RootMenu>,
		pub(crate) url: LineInput,
	}

	impl<'texture> State<'texture> {
		pub(crate) fn new(
			username: Option<&str>,
			url: Option<&str>,
			cursor: Texture<'texture>,
		) -> Self {
			Self {
				cursor,

				username: LineInput {
					line: username.unwrap_or("").to_string(),
					submitted: username.is_some(),
				},
				root_menu: if url.is_some() {
					Radio {
						backer: RootMenu::Multiplayer,
						submitted: true,
					}
				} else {
					Radio::default()
				},
				url: LineInput {
					line: url.unwrap_or("esprit://").to_string(),
					submitted: url.is_some(),
				},
			}
		}
	}

	impl Menu<RootMenuResponse> for State<'_> {
		fn event(
			&mut self,
			event: &sdl3::event::Event,
			options: &crate::Options,
		) -> Signal<RootMenuResponse> {
			self.username.dispatch(event, options, |username| {
				self.root_menu
					.dispatch(event, options, |backer| match backer {
						RootMenu::Singleplayer => {
							Signal::Yield(RootMenuResponse::OpenSingleplayer {
								username: username.into(),
							})
						}
						RootMenu::Multiplayer => self.url.dispatch(event, options, |url| {
							Signal::Yield(RootMenuResponse::OpenMultiplayer {
								username: username.into(),
								url: url.into(),
							})
						}),
					})
			})
		}

		fn draw(&self, gui: &mut gui::Context) {
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
					Some((self.root_menu.backer.index(), &self.cursor)),
					["Singleplayer", "Multiplayer"],
				);

				gui.horizontal();
				gui.advance(10, 0);
				if let menu::login::RootMenu::Multiplayer = self.root_menu.backer
					&& self.root_menu.submitted
				{
					gui.label("Connect to server: ");
					gui.label(&self.url);
				}
				gui.vertical();
			}
		}
	}
}
