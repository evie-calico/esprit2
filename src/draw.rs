use crate::prelude::*;
use rand::Rng;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::{pixels::Color, rect::Rect};

const TILE_SIZE: u32 = 64;
const ITILE_SIZE: i32 = TILE_SIZE as i32;

pub fn tilemap(
	canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
	world_manager: &world::Manager,
) {
	canvas.set_draw_color(Color::WHITE);
	for (x, col) in world_manager.current_floor.map.iter_cols().enumerate() {
		for (y, tile) in col.enumerate() {
			if *tile == floor::Tile::Wall {
				canvas
					.fill_rect(Rect::new(
						(x as i32) * ITILE_SIZE,
						(y as i32) * ITILE_SIZE,
						TILE_SIZE,
						TILE_SIZE,
					))
					.unwrap();
			}
		}
	}
}

pub fn cursor(
	input_mode: &input::Mode,
	resources: &ResourceManager<'_>,
	canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
) {
	if let input::Mode::Cursor {
		x,
		y,
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
				x * ITILE_SIZE + x_off,
				y * ITILE_SIZE + y_off,
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
	canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
	sleep_texture: &sdl2::render::Texture<'_>,
) {
	for character in world_manager.characters.iter().map(|x| x.read()) {
		canvas
			.copy(
				sleep_texture,
				None,
				Some(Rect::new(
					character.x * ITILE_SIZE,
					character.y * ITILE_SIZE,
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

	pub fn draw(
		&self,
		canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
		rect: Rect,
		radius: i16,
		color: Color,
	) {
		let bx = rect.x as i16;
		let by = rect.y as i16;
		let width = rect.width() as i16;
		let height = rect.height() as i16;
		let spacing = radius;
		canvas
			.rounded_box(bx, by, bx + width, by + height, radius, color)
			.unwrap();

		let cloud_width = width / radius;
		let cloud_height = height / radius;

		let mut last_random = xorshift(self.current_seed);
		let mut next_random = xorshift(self.next_seed);
		for (x, y) in (0..=cloud_width)
			.map(|x| (x, 0))
			.chain((0..=cloud_height).map(|y| (0, y)))
			.chain((0..=cloud_width).map(|x| (x, cloud_height)))
			.chain((0..=cloud_height).map(|y| (cloud_width, y)))
		{
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
				.filled_circle(bx + x * spacing, by + y * spacing, radius as i16, color)
				.unwrap();
			last_random = xorshift(last_random);
			next_random = xorshift(next_random);
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct CloudTrail {
	timer: f64,
	seed: u32,
}

impl Default for CloudTrail {
	fn default() -> Self {
		let mut rng = rand::thread_rng();
		Self {
			timer: 0.0,
			seed: rng.gen(),
		}
	}
}

impl CloudTrail {
	pub fn tick(&mut self, delta: f64) {
		self.timer += delta;
	}

	pub fn draw(
		&self,
		canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
		x1: i32,
		y1: i32,
		x2: i32,
		y2: i32,
		radius: f64,
		density: u32,
		color: Color,
	) {
		for i in 0..density {
			let weight = (self.timer + (i as f64) / density as f64) % 1.0;
			let scale = (weight * std::f64::consts::PI).sin();
			let x = (x1 as f64 * (1.0 - weight) + x2 as f64 * weight) as i16;
			let y = (y1 as f64 * (1.0 - weight) + y2 as f64 * weight) as i16;
			canvas
				.filled_circle(x, y, (radius * scale) as i16, color)
				.unwrap();
		}
	}
}
