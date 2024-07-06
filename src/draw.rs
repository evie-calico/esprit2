#![allow(clippy::unwrap_used, reason = "SDL")]

use crate::prelude::*;
use rand::Rng;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::f64::consts::{PI, TAU};

const TILE_SIZE: u32 = 64;
const ITILE_SIZE: i32 = TILE_SIZE as i32;

#[derive(Clone, Debug, Default)]
pub struct Camera {
	x: i32,
	y: i32,
	width: u32,
	height: u32,
}

impl Camera {
	pub fn update_size(&mut self, width: u32, height: u32) {
		self.width = width;
		self.height = height;
	}

	pub fn focus_character(&mut self, character: &character::Piece) {
		self.x = character.x * ITILE_SIZE - (self.width as i32 - ITILE_SIZE) / 2;
		self.y = character.y * ITILE_SIZE - (self.height as i32 - ITILE_SIZE) / 2;
	}

	pub fn focus_character_with_cursor(
		&mut self,
		character: &character::Piece,
		cursor: (i32, i32),
	) {
		self.x = (character.x * ITILE_SIZE + cursor.0 * ITILE_SIZE) / 2
			- (self.width as i32 - ITILE_SIZE) / 2;
		self.y = (character.y * ITILE_SIZE + cursor.1 * ITILE_SIZE) / 2
			- (self.height as i32 - ITILE_SIZE) / 2;
	}
}

pub fn tilemap(canvas: &mut Canvas<Window>, world_manager: &world::Manager, camera: &Camera) {
	canvas.set_draw_color(Color::WHITE);
	for (x, col) in world_manager.current_floor.map.iter_cols().enumerate() {
		for (y, tile) in col.enumerate() {
			match tile {
				floor::Tile::Floor => (),
				floor::Tile::Wall => canvas
					.fill_rect(Rect::new(
						(x as i32) * ITILE_SIZE - camera.x,
						(y as i32) * ITILE_SIZE - camera.y,
						TILE_SIZE,
						TILE_SIZE,
					))
					.unwrap(),
				floor::Tile::Exit => canvas
					.draw_rect(Rect::new(
						(x as i32) * ITILE_SIZE + 4 - camera.x,
						(y as i32) * ITILE_SIZE + 4 - camera.y,
						TILE_SIZE - 8,
						TILE_SIZE - 8,
					))
					.unwrap(),
			}
		}
	}
}

pub fn cursor(
	input_mode: &input::Mode,
	resources: &resource::Manager<'_>,
	canvas: &mut Canvas<Window>,
	camera: &Camera,
) {
	if let input::Mode::Cursor {
		position: (x, y),
		state: input::CursorState { float, .. },
		..
	} = *input_mode
	{
		enum Side {
			TopLeft,
			TopRight,
			BottomLeft,
			BottomRight,
		}

		let cursor = resources.get_texture("cursor");
		let cursor_info = cursor.query();
		let cursor_scale = TILE_SIZE / 16;
		let cursor_width = cursor_info.width * cursor_scale;
		let cursor_height = cursor_info.height * cursor_scale;
		let right_offset = ITILE_SIZE - cursor_width as i32;
		let bottom_offset = ITILE_SIZE - cursor_height as i32;
		let float = ((float.sin() + 1.0) * ((TILE_SIZE / 16) as f64)) as i32;

		for side in [
			Side::TopLeft,
			Side::TopRight,
			Side::BottomLeft,
			Side::BottomRight,
		] {
			let (x_off, y_off) = match side {
				Side::TopLeft => (-float, -float),
				Side::TopRight => (right_offset + float, -float),
				Side::BottomLeft => (-float, bottom_offset + float),
				Side::BottomRight => (right_offset + float, bottom_offset + float),
			};
			let hflip = match side {
				Side::TopLeft | Side::BottomLeft => false,
				Side::TopRight | Side::BottomRight => true,
			};
			let vflip = match side {
				Side::TopLeft | Side::TopRight => false,
				Side::BottomLeft | Side::BottomRight => true,
			};

			let rect = Rect::new(
				x * ITILE_SIZE + x_off - camera.x,
				y * ITILE_SIZE + y_off - camera.y,
				cursor_width,
				cursor_height,
			);
			canvas
				.copy_ex(cursor, None, Some(rect), 0.0, None, hflip, vflip)
				.unwrap();
		}
	}
}

pub fn characters(
	world_manager: &world::Manager,
	canvas: &mut Canvas<Window>,
	resources: &resource::Manager<'_>,
	camera: &Camera,
) {
	for character in world_manager.characters.iter().map(|x| x.borrow()) {
		canvas
			.copy(
				resources.get_texture(&character.sheet.icon),
				Some(Rect::new(0, 0, 16, 16)),
				Some(Rect::new(
					character.x * ITILE_SIZE - camera.x,
					character.y * ITILE_SIZE - camera.y,
					TILE_SIZE,
					TILE_SIZE,
				)),
			)
			.unwrap();
	}
}

#[derive(Clone, Copy, Debug)]
pub struct CloudState {
	timer: f64,
	current_seed: u32,
	next_seed: u32,
}

impl Default for CloudState {
	fn default() -> Self {
		let mut rng = rand::thread_rng();
		Self {
			timer: 0.0,
			current_seed: rng.gen(),
			next_seed: rng.gen(),
		}
	}
}

fn xorshift(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^ x << 5
}

impl CloudState {
	pub fn tick(&mut self, delta: f64) {
		self.timer += delta;
		if self.timer > 1.0 {
			self.current_seed = self.next_seed;
			self.next_seed = rand::thread_rng().gen();
			self.timer %= 1.0;
		}
	}

	pub fn draw(&self, canvas: &mut Canvas<Window>, rect: Rect, radius: i16, color: Color) {
		let bx = rect.x as i16;
		let by = rect.y as i16;
		let width = rect.width() as i16;
		let height = rect.height() as i16;
		let spacing = radius;

		let cloud_width = width / radius;
		let cloud_height = height / radius;

		let mut last_random = xorshift(self.current_seed);
		let mut next_random = xorshift(self.next_seed);
		for (x, y) in (1..cloud_width)
			.map(|x| (x, 0))
			.chain((1..cloud_height).map(|y| (0, y)))
			.chain((1..cloud_width).map(|x| (x, cloud_height)))
			.chain((1..cloud_height).map(|y| (cloud_width, y)))
		{
			let bias = 0.2;
			let x_middle_weight = (x as f64 / cloud_width as f64 * PI).sin() * (1.0 - bias) + bias;
			let y_middle_weight = (y as f64 / cloud_height as f64 * PI).sin() * (1.0 - bias) + bias;
			let weight = x_middle_weight.max(y_middle_weight);

			let is_active = |rand| rand & 1 == 0;
			let last_radius = if is_active(last_random) {
				radius / 4 * 3
			} else {
				radius
			};
			let next_radius = if is_active(next_random) {
				radius / 4 * 3
			} else {
				radius
			};
			let percent = self.timer % 1.0;
			let radius = last_radius as f64 * (1.0 - percent) + next_radius as f64 * (percent);
			canvas
				.filled_circle(
					bx + x * spacing,
					by + y * spacing,
					(radius * weight) as i16,
					color,
				)
				.unwrap();
			last_random = xorshift(last_random);
			next_random = xorshift(next_random);
		}

		// fill in the corners to hide sharp edges
		for (x, y) in [
			(
				rect.left() as i16 + radius / 3,
				rect.top() as i16 + radius / 3,
			),
			(
				rect.right() as i16 - radius / 3,
				rect.top() as i16 + radius / 3,
			),
			(
				rect.left() as i16 + radius / 3,
				rect.bottom() as i16 - radius / 3,
			),
			(
				rect.right() as i16 - radius / 3,
				rect.bottom() as i16 - radius / 3,
			),
		] {
			canvas.filled_circle(x, y, radius / 2, color).unwrap();
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct CloudTrail {
	timer: f64,
}

impl Default for CloudTrail {
	fn default() -> Self {
		Self { timer: 0.0 }
	}
}

impl CloudTrail {
	pub fn tick(&mut self, delta: f64) {
		self.timer += delta;
		self.timer %= 1.0;
	}

	pub fn draw(
		&self,
		canvas: &mut Canvas<Window>,
		density: u32,
		from: Point,
		to: Point,
		radius: f64,
		color: Color,
	) {
		for i in 0..density {
			let weight = (self.timer + (i as f64) / density as f64) % 1.0;
			let scale = (weight * PI).sin();
			let x = (from.x as f64 * (1.0 - weight) + to.x as f64 * weight) as i16;
			let y = (from.y as f64 * (1.0 - weight) + to.y as f64 * weight) as i16;
			canvas
				.filled_circle(x, y, (radius * scale) as i16, color)
				.unwrap();
		}
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CloudyWave {
	timer: f64,
}

impl CloudyWave {
	pub fn tick(&mut self, delta: f64) {
		self.timer += delta;
		self.timer %= TAU;
	}

	pub fn draw(&self, canvas: &mut Canvas<Window>, rect: Rect, radius: i16, color: Color) {
		let x = rect.x as f64;

		for (i, y) in (rect.top()..=rect.bottom()).step_by(30).enumerate() {
			let wave = (i as f64 - self.timer / 1.0).sin();
			let superwave = (i as f64 / 8.0 + self.timer).sin();
			let x = x + wave * 20.0 - superwave * radius as f64;

			canvas
				.filled_circle(x as i16, y as i16, radius, color)
				.unwrap();
			canvas.set_draw_color(color);
			let rect = Rect::new(
				x as i32,
				y - radius as i32,
				(rect.right() - x as i32) as u32,
				radius as u32 * 2,
			);
			canvas.fill_rect(rect).unwrap();
		}

		let mut seed = 0x12345678;
		for _ in 0..(rect.width() * rect.height() / 9000) {
			seed = xorshift(seed);
			let twinkle = seed % 3 == 0
				&& (self.timer + TAU * (xorshift(seed) as f64 / u32::MAX as f64)) % TAU > 5.0;
			if twinkle {
				continue;
			}
			let x = rect.x
				+ radius as i32
				+ (rect.width() as f64 * (seed & 0xFFFF) as f64 / u16::MAX as f64) as i32;
			let y = rect.y + (rect.height() as f64 * (seed >> 16) as f64 / u16::MAX as f64) as i32;
			let color = match y % 3 {
				0 => Color::RED,
				1 => Color::GREEN,
				_ => Color::BLUE,
			};
			canvas.set_draw_color(color);
			canvas.draw_point((x, y)).unwrap();
		}
	}
}
