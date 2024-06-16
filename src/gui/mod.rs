use crate::prelude::*;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use std::ops::Range;

pub mod widget;

pub struct Context<'canvas> {
	pub canvas: &'canvas mut Canvas<Window>,
	/// Used by draw_text to store textures of fonts before drawing them.
	font_texture_creator: TextureCreator<WindowContext>,
	pub rect: Rect,
	/// These values control the position of the cursor.
	pub x: i32,
	pub y: i32,
	/// Determines which direction the cursor moves in.
	orientation: Orientation,
}

enum Orientation {
	Vertical,
	Horizontal { height: i32 },
}

impl<'canvas> Context<'canvas> {
	pub fn new(canvas: &'canvas mut Canvas<Window>, rect: Rect) -> Self {
		let font_texture_creator = canvas.texture_creator();
		Self {
			canvas,
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

	pub fn label(&mut self, s: &str, font: &Font) {
		self.label_color(s, Color::WHITE, font)
	}

	pub fn label_color(&mut self, s: &str, color: Color, font: &Font) {
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
				Rect::new(self.x, self.y, width, height),
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

		self.advance(width, height);
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
				self.label(&expression.source[uncolored_range], font);
			}
			let colored_range = span.start..span.end;
			if !colored_range.is_empty() {
				let var = &expression.source[colored_range];
				let color = Colors::get(var).unwrap_or(Color::RED);
				self.label_color(var, color, font);
			}
			last_char = span.end;
		}

		if last_char != expression.source.len() {
			self.label(&expression.source[last_char..], font);
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
}

pub trait VariableColors {
	fn get(s: &str) -> Option<Color>;
}

impl VariableColors for () {
	fn get(_s: &str) -> Option<Color> {
		None
	}
}
