#![allow(clippy::unwrap_used, reason = "SDL")]

use crate::prelude::*;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, SwashCache};
use esprit2::prelude::*;
use parking_lot::RwLock;
use sdl3::rect::Rect;
use sdl3::render::FPoint;
use sdl3::render::{Canvas, FRect, Texture};
use sdl3::video::Window;
use std::sync::OnceLock;

pub(crate) mod widget;

const MINIMUM_NAMEPLATE_WIDTH: u32 = 100;

fn font_system() -> &'static RwLock<FontSystem> {
	static CACHE: OnceLock<RwLock<FontSystem>> = OnceLock::new();
	CACHE.get_or_init(|| RwLock::new(FontSystem::new()))
}

fn swash_cache() -> &'static RwLock<SwashCache> {
	static CACHE: OnceLock<RwLock<SwashCache>> = OnceLock::new();
	CACHE.get_or_init(|| RwLock::new(SwashCache::new()))
}

pub(crate) struct Context<'canvas> {
	pub(crate) canvas: &'canvas mut Canvas<Window>,
	pub(crate) rect: Rect,
	/// These values control the position of the cursor.
	pub(crate) x: i32,
	pub(crate) y: i32,
	/// Determines which direction the cursor moves in.
	orientation: Orientation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum Justification {
	Left,
	Right,
}

enum Orientation {
	Vertical,
	Horizontal { height: i32 },
}

impl<'canvas> Context<'canvas> {
	pub(crate) fn new(canvas: &'canvas mut Canvas<Window>, rect: Rect) -> Self {
		Self {
			canvas,
			rect,
			y: rect.y,
			x: rect.x,
			orientation: Orientation::Vertical,
		}
	}

	pub(crate) fn view(&mut self, x: i32, y: i32, width: u32, height: u32) -> Context {
		Context::new(
			self.canvas,
			Rect::new(self.x + x, self.y + y, width, height),
		)
	}

	pub(crate) fn relocate(&mut self, rect: Rect) {
		self.rect = rect;
		self.x = rect.x;
		self.y = rect.y;
	}

	pub(crate) fn advance(&mut self, width: u32, height: u32) {
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

	pub(crate) fn horizontal(&mut self) {
		self.orientation = Orientation::Horizontal { height: 0 };
	}

	pub(crate) fn vertical(&mut self) {
		if let Orientation::Horizontal { height } = self.orientation {
			self.orientation = Orientation::Vertical;
			self.x = self.rect.x;
			self.y += height;
		}
	}

	pub(crate) fn hsplit(&mut self, views: &mut [Option<impl FnMut(&mut Context)>]) {
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

	pub(crate) fn menu<'texture>(
		&mut self,
		index: Option<(usize, &Texture<'texture>)>,
		entries: impl IntoIterator<Item = &str>,
	) {
		for (i, label) in entries.into_iter().enumerate() {
			self.horizontal();
			if let Some((index, texture)) = index
				&& index == i
			{
				self.htexture(texture, 16);
			} else {
				self.advance(16, 0);
			}
			self.label(label);
			self.vertical();
		}
	}

	pub(crate) fn margin_list(&mut self, list: impl IntoIterator<Item = (&str, &str)>) {}

	pub(crate) fn progress_bar(
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

	pub(crate) fn label(&mut self, s: &str) {
		self.label_color(s, (255, 255, 255, 255));
	}

	pub(crate) fn label_color(&mut self, s: &str, color: Color) {
		let mut font_system = font_system().write();
		let mut buffer = Buffer::new(&mut font_system, Metrics::new(18.0, 20.0));
		let mut buffer = buffer.borrow_with(&mut font_system);
		buffer.set_text(s, Attrs::new(), cosmic_text::Shaping::Advanced);
		buffer.shape_until_scroll(true);
		let mut swash_cache = swash_cache().write();
		let mut advancement = (0, 0);
		buffer.draw(
			&mut swash_cache,
			cosmic_text::Color::rgba(color.0, color.1, color.2, color.3),
			|x, y, w, h, c| {
				if c.a() == 0 {
					return;
				}
				advancement = (
					advancement.0.max(x.try_into().unwrap_or(0) + w),
					advancement.1.max(y.try_into().unwrap_or(0) + h),
				);
				let x = self.x + x;
				let y = self.y + y;

				self.canvas.set_draw_color(c.as_rgba_tuple());
				let _ = self.canvas.draw_point(FPoint::new(x as f32, y as f32));
			},
		);
		self.advance(advancement.0, advancement.1);
	}

	pub(crate) fn htexture(&mut self, texture: &Texture, width: u32) {
		let query = texture.query();
		let height = width / query.width * query.height;
		self.canvas
			.copy(
				texture,
				None,
				Some(FRect::new(
					self.x as f32,
					self.y as f32,
					width as f32,
					height as f32,
				)),
			)
			.unwrap();
		self.advance(width, height)
	}

	pub(crate) fn console(&mut self, console: &Console, colors: &crate::options::ConsoleColors) {
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

		canvas.set_clip_rect(None);
	}
}

pub(crate) trait VariableColors {
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
