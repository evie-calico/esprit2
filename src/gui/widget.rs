use crate::prelude::*;
use sdl2::{
	pixels::Color,
	rect::{Point, Rect},
	render::Texture,
};

pub struct SoulJar<'texture> {
	souls: Vec<Soul>,
	light_texture: Texture<'texture>,
}

impl<'texture> SoulJar<'texture> {
	pub fn new(resources: &'texture ResourceManager<'_>) -> Self {
		Self {
			souls: vec![
				Soul::new(Color::RED),
				Soul::new(Color::YELLOW),
				Soul::new(Color::BLUE),
				Soul::new(Color::GREEN),
				Soul::new(Color::CYAN),
				Soul::new(Color::RGB(255, 128, 0)),
				Soul::new(Color::WHITE),
				Soul::new(Color::RGB(255, 0, 255)),
				Soul::new(Color::RGB(255, 128, 128)),
			],
			light_texture: resources.get_owned_texture("light").unwrap(),
		}
	}

	pub fn tick(&mut self, delta: f32) {
		for i in &mut self.souls {
			i.tick(delta);
		}
	}
}

pub fn menu(
	menu: &mut gui::Context,
	options: &Options,
	font: &sdl2::ttf::Font<'_, '_>,
	input_mode: &input::Mode,
	world_manager: &world::Manager,
) {
	for (i, color) in (0..3).zip(
		[
			Color::RGB(0x14, 0x17, 0x14),
			Color::RGB(0xE3, 0xBD, 0xEF),
			Color::RGB(0x14, 0x17, 0x14),
		]
		.into_iter(),
	) {
		menu.canvas.set_draw_color(color);
		menu.canvas
			.fill_rect(menu.rect.top_shifted(i * -10))
			.unwrap();
	}
	menu.advance(0, 30);
	// Paint scanlines on the rest of the terminal
	for (i, y) in ((menu.y + 5)..menu.rect.bottom()).step_by(25).enumerate() {
		menu.canvas.set_draw_color(if i % 3 == 2 {
			Color::RGB(0x20, 0x37, 0x21)
		} else {
			Color::RGB(0x18, 0x23, 0x18)
		});
		menu.canvas
			.fill_rect(Rect::new(menu.x, y, menu.rect.width(), 2))
			.unwrap();
	}
	match input_mode {
		input::Mode::Normal => {
			menu.label_color("Normal", options.ui.normal_mode_color.into(), font);
			world_manager.console.draw(menu, font);
		}
		input::Mode::Cast => {
			menu.label_color("Cast", options.ui.cast_mode_color.into(), font);
			spell_menu::draw(menu, &world_manager.next_character().read(), font);
		}
		input::Mode::Cursor { x, y, .. } => {
			menu.label_color("Cursor", options.ui.cursor_mode_color.into(), font);
			if let Some(selected_character) = world_manager.get_character_at(*x, *y) {
				character_info(menu, &selected_character.read(), Color::WHITE, font);
			} else {
				world_manager.console.draw(menu, font);
			}
		}
	}
}

pub fn pamphlet(
	pamphlet: &mut gui::Context,
	font: &sdl2::ttf::Font<'_, '_>,
	world_manager: &world::Manager,
	resources: &ResourceManager<'_>,
	soul_jar: &mut SoulJar<'_>,
) {
	pamphlet.label("Forest: Floor 1/8", font);
	pamphlet.advance(0, 10);
	// Draw party stats
	for character_chunk in world_manager.party.chunks(2) {
		let mut character_windows = [None, None];
		for (character_id, window) in character_chunk.iter().zip(character_windows.iter_mut()) {
			*window = Some(|player_window: &mut gui::Context| {
				if let Some(piece) = world_manager.get_character(character_id.piece) {
					let piece = piece.read();
					let texture = resources.get_texture("luvui_sleep");
					character_thinking(character_id, player_window, texture, |player_window| {
						character_info(player_window, &piece, Color::WHITE, font);
					});
				} else {
					// If the party array also had a reference to the character's last known character sheet,
					// a name could be displayed here.
					// I don't actually know if this is desirable;
					// this should probably never happen anyways.
					player_window.label("???", font);
				}
			});
		}
		pamphlet.hsplit(&mut character_windows);
	}
	pamphlet.advance(0, 10);

	let mut inventory_fn = |pamphlet: &mut gui::Context| {
		pamphlet.label("Inventory", font);
		let mut items = world_manager.inventory.iter().peekable();
		while items.peek().is_some() {
			let textures_per_row = pamphlet.rect.width() / (32 + 8);
			pamphlet.horizontal();
			for _ in 0..textures_per_row {
				if let Some(item_name) = items.next() {
					pamphlet.htexture(resources.get_texture(item_name), 32);
					pamphlet.advance(8, 0);
				}
			}
			pamphlet.vertical();
			pamphlet.advance(8, 8);
		}
	};
	let mut souls_fn = |pamphlet: &mut gui::Context| {
		const SOUL_SIZE: u32 = 50;
		pamphlet.label("Souls", font);

		let bx = pamphlet.x as f32;
		let by = pamphlet.y as f32;
		let display_size = pamphlet.rect.width();

		for soul in &soul_jar.souls {
			let display_size = (display_size - SOUL_SIZE) as f32;
			let ox = soul.x * display_size;
			let oy = soul.y * display_size;
			soul_jar
				.light_texture
				.set_color_mod(soul.color.r, soul.color.g, soul.color.b);
			pamphlet
				.canvas
				.copy(
					&soul_jar.light_texture,
					None,
					Rect::new((bx + ox) as i32, (by + oy) as i32, SOUL_SIZE, SOUL_SIZE),
				)
				.unwrap();
		}
		pamphlet.advance(0, display_size);
	};
	pamphlet.hsplit(&mut [
		Some((&mut inventory_fn) as &mut dyn FnMut(&mut gui::Context)),
		Some(&mut souls_fn),
	]);
}

fn character_thinking(
	character_id: &world::PartyReference,
	player_window: &mut gui::Context<'_>,
	texture: &Texture,
	f: impl FnOnce(&mut gui::Context),
) {
	on_cloud(
		&character_id.draw_state.cloud,
		20,
		character_id.accent_color.into(),
		player_window,
		f,
	);
	let center = player_window.x + player_window.rect.width() as i32 * 2 / 3;
	let corner = player_window.x + player_window.rect.width() as i32 * 9 / 10;
	character_id.draw_state.cloud_trail.draw(
		player_window.canvas,
		4,
		Point::new(center, player_window.y + 10),
		Point::new(corner, player_window.y - 25),
		15.0,
		character_id.accent_color.into(),
	);
	let query = texture.query();
	let width = query.width * 4;
	let height = query.height * 4;
	player_window
		.canvas
		.copy(
			texture,
			None,
			Rect::new(center - (width / 2) as i32, player_window.y, width, height),
		)
		.unwrap();
	player_window.advance(0, 10 + height);
}

pub fn on_cloud(
	cloud: &draw::CloudState,
	radius: u32,
	color: Color,
	gui: &mut gui::Context<'_>,
	f: impl FnOnce(&mut gui::Context),
) {
	let width = gui.rect.width();
	let height = gui.rect.height();

	let texture_creator = gui.canvas.texture_creator();
	let mut player_texture = texture_creator
		.create_texture_target(texture_creator.default_pixel_format(), width, height)
		.unwrap();
	let mut height_used = 0;

	gui.canvas
		.with_texture_canvas(&mut player_texture, |canvas| {
			canvas.set_draw_color(color);
			canvas.clear();
			let mut gui = gui::Context::new(
				canvas,
				Rect::new(0, 0, width - radius * 2, height - radius * 2),
			);
			f(&mut gui);
			height_used = gui.y as u32;
		})
		.unwrap();
	let target = Rect::new(
		gui.x + radius as i32,
		gui.y + radius as i32,
		width - radius * 2,
		height_used,
	);
	cloud.draw(gui.canvas, target, radius as i16, color);
	gui.canvas
		.copy(
			&player_texture,
			Rect::new(0, 0, width - radius * 2, height_used),
			target,
		)
		.unwrap();
	gui.advance(width, height_used + radius * 2);
}

fn character_info(
	player_window: &mut gui::Context<'_>,
	piece: &character::Piece,
	color: Color,
	font: &sdl2::ttf::Font<'_, '_>,
) {
	let character::Piece {
		sheet:
			character::Sheet {
				nouns,
				level,
				stats:
					character::Stats {
						heart,
						soul,
						power,
						defense,
						magic,
						resistance,
					},
				..
			},
		hp,
		sp,
		..
	} = piece;
	let name = &nouns.name;

	player_window.opposing_labels(name, &format!("Level {level}"), color, font);
	player_window.label_color(&format!("HP: {hp}/{heart}"), color, font);
	player_window.progress_bar(
		(*hp as f32) / (*heart as f32),
		Color::GREEN,
		Color::RED,
		10,
		5,
	);
	player_window.label_color(&format!("SP: {sp}/{soul}"), color, font);
	player_window.progress_bar(
		(*sp as f32) / (*soul as f32),
		Color::BLUE,
		Color::RED,
		10,
		5,
	);
	let physical_stat_info = [("Pwr", power), ("Def", defense)];
	let mut physical_stats = [None, None];
	for ((stat_name, stat), stat_half) in physical_stat_info
		.into_iter()
		.zip(physical_stats.iter_mut())
	{
		*stat_half = Some(move |stat_half: &mut gui::Context| {
			stat_half.label_color(&format!("{stat_name}: {stat}"), color, font)
		});
	}
	player_window.hsplit(&mut physical_stats);
	let magical_stat_info = [("Mag", magic), ("Res", resistance)];
	let mut magical_stats = [None, None];
	for ((stat_name, stat), stat_half) in
		magical_stat_info.into_iter().zip(magical_stats.iter_mut())
	{
		*stat_half = Some(move |stat_half: &mut gui::Context| {
			stat_half.label_color(&format!("{stat_name}: {stat}"), color, font)
		});
	}
	player_window.hsplit(&mut magical_stats);
}
