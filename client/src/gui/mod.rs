#![allow(clippy::unwrap_used, reason = "SDL")]

use crate::console::Console;
use crate::typography::Typography;
use esprit2::prelude::*;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use std::ops::Range;

pub mod widget;

const MINIMUM_NAMEPLATE_WIDTH: u32 = 100;

pub struct Context<'canvas, 'ttf_module, 'rwops> {
	pub canvas: &'canvas mut Canvas<Window>,
	pub typography: &'ttf_module Typography<'ttf_module, 'rwops>,
	/// Used by draw_text to store textures of fonts before drawing them.
	font_texture_creator: TextureCreator<WindowContext>,
	pub rect: Rect,
	/// These values control the position of the cursor.
	pub x: i32,
	pub y: i32,
	/// Determines which direction the cursor moves in.
	orientation: Orientation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Justification {
	Left,
	Right,
}

enum Orientation {
	Vertical,
	Horizontal { height: i32 },
}

impl<'canvas, 'ttf_module, 'rwops> Context<'canvas, 'ttf_module, 'rwops> {
	pub fn new(
		canvas: &'canvas mut Canvas<Window>,
		typography: &'ttf_module Typography<'ttf_module, 'rwops>,
		rect: Rect,
	) -> Self {
		let font_texture_creator = canvas.texture_creator();
		Self {
			canvas,
			typography,
			font_texture_creator,
			rect,
			y: rect.y,
			x: rect.x,
			orientation: Orientation::Vertical,
		}
	}

	pub fn relocate(&mut self, rect: Rect) {
		self.rect = rect;
		self.x = rect.x;
		self.y = rect.y;
	}

	pub fn advance(&mut self, width: u32, height: u32) {
		let (width, height) = (width as i32, height as i32);
		match self.orientation {
			Orientation::Horizontal { height: o_height } => {
				self.x += width;
				let height = o_height.max(height);
				self.orientation = Orientation::Horizontal { height };
			}
			Orientation::Vertical => {
				self.y += height;
			}
		}
	}

	pub fn horizontal(&mut self) {
		self.orientation = Orientation::Horizontal { height: 0 };
	}

	pub fn vertical(&mut self) {
		if let Orientation::Horizontal { height } = self.orientation {
			self.orientation = Orientation::Vertical;
			self.x = self.rect.x;
			self.y += height;
		}
	}

	pub fn hsplit(&mut self, views: &mut [Option<impl FnMut(&mut Context)>]) {
		// We need to keep track of the tallest child so that we can advance our own pointer by the end of this.
		let mut lowest_child = 0;
		let view_count = views.len();
		for (i, view) in views
			.iter_mut()
			.enumerate()
			.filter_map(|(i, view)| view.as_mut().map(|view| (i, view)))
		{
			let mut child = Context::new(
				self.canvas,
				self.typography,
				Rect::new(
					self.x + (self.rect.width() as i32) / (view_count as i32) * i as i32,
					self.y,
					self.rect.width() / (view_count as u32),
					self.rect.height(),
				),
			);
			view(&mut child);
			child.vertical();
			lowest_child = lowest_child.max(child.y);
		}
		self.advance(0, (lowest_child - self.y) as u32);
	}

	pub fn margin_list(&mut self, list: impl IntoIterator<Item = (&str, &str)>) {
		let font = &self.typography.normal;
		let color = self.typography.color;
		let mut largest_margin = 0;
		let mut cursor = 0;
		for (margin, content) in list
			.into_iter()
			.map(|(margin, content)| {
				let margin = font
					.render(margin)
					.blended(color)
					.unwrap()
					.as_texture(&self.font_texture_creator)
					.unwrap();
				let content = font
					.render(content)
					.blended(color)
					.unwrap()
					.as_texture(&self.font_texture_creator)
					.unwrap();
				largest_margin = margin.query().width.max(largest_margin);

				(margin, content)
			})
			// This is silly looking but necessary to calculate the longest margin's width.
			.collect::<Box<[_]>>()
		{
			let margin_q = margin.query();
			self.canvas
				.copy(
					&margin,
					None,
					Rect::new(
						self.x + largest_margin as i32 - margin_q.width as i32,
						self.y + cursor,
						margin_q.width,
						margin_q.height,
					),
				)
				.unwrap();
			let content_q = content.query();
			self.canvas
				.copy(
					&content,
					None,
					Rect::new(
						self.x + largest_margin as i32,
						self.y + cursor,
						content_q.width,
						content_q.height,
					),
				)
				.unwrap();
			cursor += margin_q.height.max(content_q.height) as i32;
		}
	}

	pub fn progress_bar(
		&mut self,
		progress: f32,
		fill: Color,
		empty: Color,
		margin: u32,
		height: u32,
	) {
		self.canvas.set_draw_color(empty);
		self.canvas
			.fill_rect(Rect::new(
				self.x + margin as i32,
				self.y,
				self.rect.width() - margin * 2,
				height,
			))
			.unwrap();
		self.canvas.set_draw_color(fill);
		self.canvas
			.fill_rect(Rect::new(
				self.x + margin as i32,
				self.y,
				(((self.rect.width() - margin * 2) as f32) * progress) as u32,
				height,
			))
			.unwrap();
		self.advance(self.rect.width(), height);
	}

	pub fn label(&mut self, s: &str) {
		self.label_color(s, self.typography.color)
	}

	pub fn label_justified(&mut self, s: &str, justification: Justification) {
		self.label_custom(
			s,
			self.typography.color,
			&self.typography.normal,
			justification,
		);
	}

	pub fn label_color(&mut self, s: &str, color: Color) {
		self.label_custom(s, color, &self.typography.normal, Justification::Left);
	}

	pub fn label_styled(&mut self, s: &str, color: Color, font: &Font) {
		self.label_custom(s, color, font, Justification::Left);
	}

	pub fn label_custom(
		&mut self,
		s: &str,
		color: Color,
		font: &Font,
		justification: Justification,
	) {
		let font_texture = font
			.render(s)
			.blended(color)
			.unwrap()
			.as_texture(&self.font_texture_creator)
			.unwrap();
		let TextureQuery { width, height, .. } = font_texture.query();
		self.canvas
			.copy(
				&font_texture,
				None,
				Rect::new(
					match justification {
						Justification::Left => self.x,
						Justification::Right => self.rect.right() - width as i32,
					},
					self.y,
					width,
					height,
				),
			)
			.unwrap();
		// I feel like this drop is kinda silly?
		drop(font_texture);

		self.advance(width, height);
	}

	pub fn opposing_labels(&mut self, s1: &str, s2: &str, color: Color, font: &Font) {
		let font_texture = font
			.render(s1)
			.blended(color)
			.unwrap()
			.as_texture(&self.font_texture_creator)
			.unwrap();
		let TextureQuery { width, height, .. } = font_texture.query();
		self.canvas
			.copy(
				&font_texture,
				None,
				Rect::new(self.x, self.y, width, height),
			)
			.unwrap();
		drop(font_texture);
		let font_texture = font
			.render(s2)
			.blended(color)
			.unwrap()
			.as_texture(&self.font_texture_creator)
			.unwrap();
		let TextureQuery { width, height, .. } = font_texture.query();
		self.canvas
			.copy(
				&font_texture,
				None,
				Rect::new((self.rect.width() - width) as i32, self.y, width, height),
			)
			.unwrap();
		drop(font_texture);

		self.advance(self.rect.width(), height);
	}

	pub fn expression<Colors: VariableColors>(&mut self, expression: &Expression, font: &Font) {
		fn enter_op(
			op: &expression::Operation,
			expression: &Expression,
			spans: &mut Vec<Range<usize>>,
		) {
			match op {
				expression::Operation::Variable(start, end) => spans.push(*start..*end),

				expression::Operation::Add(a, b)
				| expression::Operation::Sub(a, b)
				| expression::Operation::Mul(a, b)
				| expression::Operation::Div(a, b) => {
					enter_op(&expression.leaves[*a], expression, spans);
					enter_op(&expression.leaves[*b], expression, spans);
				}
				expression::Operation::AddC(x, _)
				| expression::Operation::SubC(x, _)
				| expression::Operation::MulC(x, _)
				| expression::Operation::DivC(x, _) => enter_op(&expression.leaves[*x], expression, spans),

				expression::Operation::Integer(_) | expression::Operation::Roll(_, _) => {}
			}
		}

		let was_horizontal = matches!(self.orientation, Orientation::Horizontal { .. });
		if was_horizontal {
			self.horizontal();
		}

		// This is implicitly sorted
		let mut variable_spans = Vec::new();
		enter_op(&expression.root, expression, &mut variable_spans);

		let mut last_char = 0;

		for span in &variable_spans {
			let uncolored_range = last_char..span.start;
			if !uncolored_range.is_empty() {
				self.label_styled(
					&expression.source[uncolored_range],
					self.typography.color,
					font,
				);
			}
			let colored_range = span.start..span.end;
			if !colored_range.is_empty() {
				let var = &expression.source[colored_range];
				let color = Colors::get(var).unwrap_or((255, 0, 0, 255));
				self.label_styled(var, color, font);
			}
			last_char = span.end;
		}

		if last_char != expression.source.len() {
			self.label_styled(&expression.source[last_char..], self.typography.color, font);
		}

		if was_horizontal {
			self.vertical();
		}
	}

	pub fn htexture(&mut self, texture: &Texture, width: u32) {
		let query = texture.query();
		let height = width / query.width * query.height;
		self.canvas
			.copy(
				texture,
				None,
				Some(Rect::new(self.x, self.y, width, height)),
			)
			.unwrap();
		self.advance(width, height)
	}

	pub fn console(&mut self, console: &Console, colors: &crate::options::ConsoleColors) {
		let canvas = &mut self.canvas;
		let rect = Rect::new(
			self.x,
			self.y,
			(self.rect.right() - self.x) as u32,
			(self.rect.bottom() - self.y) as u32,
		);
		let font_texture_creator = canvas.texture_creator();
		canvas.set_clip_rect(rect);

		let mut cursor = rect.y + (rect.height() as i32);

		let text = |message, color: Color| {
			let texture = self
				.typography
				.normal
				.render(message)
				.blended(color)
				.unwrap()
				.as_texture(&font_texture_creator)
				.unwrap();
			let TextureQuery { width, height, .. } = texture.query();
			(texture, width, height)
		};
		for message in console.history.iter().rev() {
			match &message.printer {
				console::MessagePrinter::Console(color) => {
					let (font_texture, width, height) = text(
						&message.text,
						match color {
							console::Color::Normal => colors.normal,
							console::Color::System => colors.system,
							console::Color::Unimportant => colors.unimportant,
							console::Color::Defeat => colors.defeat,
							console::Color::Danger => colors.danger,
							console::Color::Important => colors.important,
							console::Color::Special => colors.special,
						},
					);
					cursor -= height as i32;
					canvas
						.copy(
							&font_texture,
							None,
							Rect::new(rect.x, cursor, width, height),
						)
						.unwrap();
				}
				console::MessagePrinter::Dialogue { speaker, progress } => {
					let (font_texture, text_width, height) = text(speaker, (0, 0, 0, 255));
					let width = text_width.max(MINIMUM_NAMEPLATE_WIDTH);
					let margin = ((width - text_width) / 2) as i32;
					canvas
						.rounded_box(
							rect.x as i16,
							cursor as i16,
							(rect.x + (width as i32)) as i16,
							(cursor - (height as i32) + 2) as i16,
							5,
							colors.normal,
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
					let (font_texture, width, height) =
						text(&message.text[0..shown_characters], colors.normal);
					canvas
						.copy(
							&font_texture,
							None,
							Rect::new(rect.x + last_width + 10, cursor, width, height),
						)
						.unwrap();
				}
				console::MessagePrinter::Combat(log) => {
					let color = if log.is_weak() {
						colors.unimportant
					} else {
						colors.normal
					};
					let (texture, width, height) = text(&message.text, color);
					cursor -= height as i32;
					canvas
						.copy(&texture, None, Rect::new(rect.x, cursor, width, height))
						.unwrap();
					let last_width = width as i32;
					let info = format!("({log})");
					let texture = self
						.typography
						.annotation
						.render(&info)
						.blended(colors.combat)
						.unwrap()
						.as_texture(&font_texture_creator)
						.unwrap();
					let TextureQuery { width, height, .. } = texture.query();
					canvas
						.copy(
							&texture,
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

pub trait VariableColors {
	fn get(s: &str) -> Option<Color>;
}

impl VariableColors for () {
	fn get(_s: &str) -> Option<Color> {
		None
	}
}

impl VariableColors for esprit2::character::Stats {
	fn get(s: &str) -> Option<Color> {
		const HEART_COLOR: Color = (96, 67, 18, 255);
		const SOUL_COLOR: Color = (128, 128, 128, 255);
		const POWER_COLOR: Color = (255, 11, 64, 255);
		const DEFENSE_COLOR: Color = (222, 120, 64, 255);
		const MAGIC_COLOR: Color = (59, 115, 255, 255);
		const RESISTANCE_COLOR: Color = (222, 64, 255, 255);
		match s {
			"heart" => Some(HEART_COLOR),
			"soul" => Some(SOUL_COLOR),
			"power" => Some(POWER_COLOR),
			"defense" => Some(DEFENSE_COLOR),
			"magic" => Some(MAGIC_COLOR),
			"resistance" => Some(RESISTANCE_COLOR),
			_ => None,
		}
	}
}
