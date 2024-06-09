use paste::paste;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::ttf::Font;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::gui;

const MINIMUM_NAMEPLATE_WIDTH: u32 = 100;

#[derive(Clone, Debug, Default)]
pub struct Console {
	history: Vec<Message>,
	in_progress: VecDeque<usize>,
	pub colors: Colors,
}

impl mlua::UserData for Console {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method_mut("print", |_, this, value: String| {
			this.print(value);
			Ok(())
		});
		methods.add_method_mut("print_unimportant", |_, this, value: String| {
			this.print_unimportant(value);
			Ok(())
		});
	}
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
pub enum MessagePrinter {
	Console,
	Dialogue { speaker: Arc<str>, progress: f64 },
}

#[derive(Clone, Debug)]
pub struct Message {
	text: String,
	color: Color,
	printer: MessagePrinter,
}

macro_rules! colored_print {
	($which:ident) => {
		paste! {
			pub fn [<print_ $which>](&mut self, text: String) {
				self.history.push(Message {
					text,
					color: self.colors.$which,
					printer: MessagePrinter::Console,
				});
			}
		}
	};
}

impl Console {
	pub fn say(&mut self, speaker: Arc<str>, text: String) {
		self.history.push(Message {
			text,
			color: self.colors.normal,
			printer: MessagePrinter::Dialogue {
				speaker,
				progress: 0.0,
			},
		});

		self.in_progress.push_back(self.history.len() - 1);
	}

	pub fn print(&mut self, text: String) {
		self.history.push(Message {
			text,
			color: self.colors.normal,
			printer: MessagePrinter::Console,
		});
	}

	pub fn print_colored(&mut self, text: String, color: Color) {
		self.history.push(Message {
			text,
			color,
			printer: MessagePrinter::Console,
		});
	}

	colored_print!(system);
	colored_print!(unimportant);
	colored_print!(danger);
	colored_print!(defeat);
	colored_print!(important);
	colored_print!(special);
}

impl Console {
	pub fn update(&mut self, delta: f64) {
		let delta_progress = delta / 0.1;

		for i in &self.in_progress {
			let i = *i;
			let max_length = self.history[i].text.len() as f64;
			if let MessagePrinter::Dialogue {
				speaker: _,
				progress,
			} = &mut self.history[i].printer
			{
				let new_progress = *progress + delta_progress;
				if new_progress < max_length {
					*progress = new_progress;
				}
			}
		}

		while self.in_progress.front().is_some_and(|x| {
			if let MessagePrinter::Dialogue {
				speaker: _,
				progress,
			} = &self.history[*x].printer
			{
				self.history[*x].text.len() == (*progress as usize)
			} else {
				true
			}
		}) {
			self.in_progress.pop_front();
		}
	}

	pub fn draw(&self, gui: &mut gui::Context, font: &Font) {
		let canvas = &mut gui.canvas;
		let rect = Rect::new(
			gui.x,
			gui.y,
			(gui.rect.right() - gui.x) as u32,
			(gui.rect.bottom() - gui.y) as u32,
		);
		let font_texture_creator = canvas.texture_creator();
		canvas.set_clip_rect(rect);

		let mut cursor = rect.y + (rect.height() as i32);

		for message in self.history.iter().rev() {
			match &message.printer {
				MessagePrinter::Console => {
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
				}
				MessagePrinter::Dialogue { speaker, progress } => {
					let font_texture = font
						.render(speaker)
						.blended(Color::BLACK)
						.unwrap()
						.as_texture(&font_texture_creator)
						.unwrap();

					let TextureQuery {
						width: text_width,
						height,
						..
					} = font_texture.query();
					let width = text_width.max(MINIMUM_NAMEPLATE_WIDTH);
					let margin = ((width - text_width) / 2) as i32;
					canvas
						.rounded_box(
							rect.x as i16,
							cursor as i16,
							(rect.x + (width as i32)) as i16,
							(cursor - (height as i32) + 2) as i16,
							5,
							message.color,
						)
						.unwrap();
					cursor -= height as i32;
					canvas
						.copy(
							&font_texture,
							None,
							Rect::new(rect.x + margin, cursor, text_width, height),
						)
						.unwrap();

					// Save width of nameplate.
					let last_width = width as i32;

					let shown_characters = message.text.len().min((*progress as usize) + 1);
					let font_texture = font
						.render(&message.text[0..shown_characters])
						.blended(message.color)
						.unwrap()
						.as_texture(&font_texture_creator)
						.unwrap();
					let TextureQuery { width, height, .. } = font_texture.query();
					canvas
						.copy(
							&font_texture,
							None,
							Rect::new(rect.x + last_width + 10, cursor, width, height),
						)
						.unwrap();
				}
			}

			if cursor < rect.y {
				break;
			}
		}

		canvas.set_clip_rect(None);
	}
}
