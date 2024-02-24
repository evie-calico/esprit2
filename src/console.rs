use paste::paste;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::Window;
use std::fmt::Display;

#[derive(Clone, Debug, Default)]
pub struct Console {
	history: Vec<Message>,
	pub colors: Colors,
}

#[derive(Clone, Debug)]
pub struct Colors {
	pub normal: Color,
	pub system: Color,
	pub unimportant: Color,
	pub defeat: Color,
	pub danger: Color,
	pub important: Color,
	pub special: Color,
}

impl Default for Colors {
	fn default() -> Self {
		Self {
			normal: Color::WHITE,
			system: Color::GREY,
			unimportant: Color::GREY,
			defeat: Color::RGB(255, 128, 128),
			danger: Color::RED,
			important: Color::YELLOW,
			special: Color::GREEN,
		}
	}
}

#[derive(Clone, Debug)]
pub struct Message {
	text: String,
	color: Color,
}

macro_rules! colored_print {
	($which:ident) => {
		paste! {
			pub fn [<print_ $which>](&mut self, message: impl Display) {
				self.history.push(Message {
					text: message.to_string(),
					color: self.colors.$which,
				});
			}
		}
	};
}

impl Console {
	pub fn print(&mut self, message: impl Display) {
		self.history.push(Message {
			text: message.to_string(),
			color: self.colors.normal,
		});
	}

	pub fn print_colored(&mut self, message: impl Display, color: Color) {
		self.history.push(Message {
			text: message.to_string(),
			color,
		});
	}

	colored_print!(system);
	colored_print!(danger);
	colored_print!(defeat);
	colored_print!(important);
	colored_print!(special);

	pub fn draw(&self, canvas: &mut Canvas<Window>, rect: Rect, font: &Font) {
		let font_texture_creator = canvas.texture_creator();
		canvas.set_clip_rect(rect);

		let mut cursor = rect.y + (rect.height() as i32);

		for message in self.history.iter().rev() {
			let font_texture = font
				.render(&message.text)
				.shaded(message.color, Color::BLACK)
				.unwrap()
				.as_texture(&font_texture_creator)
				.unwrap();
			let TextureQuery { width, height, .. } = font_texture.query();
			cursor -= height as i32;
			canvas
				.copy(
					&font_texture,
					None,
					Rect::new(rect.x, cursor, width, height),
				)
				.unwrap();
			if cursor < rect.y {
				break;
			}
		}

		canvas.set_clip_rect(None);
	}
}
