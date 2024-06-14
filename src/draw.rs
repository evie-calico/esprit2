use crate::prelude::*;
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
