use crate::prelude::*;
use rand::Rng;
use sdl2::rect::{Point, Rect};
use sdl2::render::Texture;
use sdl2::ttf::Font;

pub struct SoulJar<'texture> {
	souls: Vec<Soul>,
	light_texture: Texture<'texture>,
}

impl<'texture> SoulJar<'texture> {
	pub fn new(resources: &'texture ResourceManager<'_>) -> Self {
		let mut rng = rand::thread_rng();
		let souls = (0..=9)
			.map(|_| Soul::new((rng.gen(), rng.gen(), rng.gen(), 255)))
			.collect();
		Self {
			souls,
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
	font: &Font<'_, '_>,
	input_mode: &input::Mode,
	world_manager: &world::Manager,
) {
	for (i, color) in
		(0..3).zip([(0x14, 0x17, 0x14), (0xE3, 0xBD, 0xEF), (0x14, 0x17, 0x14)].into_iter())
	{
		menu.canvas.set_draw_color(color);
		menu.canvas
			.fill_rect(menu.rect.top_shifted(i * -10))
			.unwrap();
	}
	menu.advance(0, 30);
	// Paint scanlines on the rest of the terminal
	for (i, y) in ((menu.y + 5)..menu.rect.bottom()).step_by(25).enumerate() {
		menu.canvas.set_draw_color(if i % 3 == 2 {
			(0x20, 0x37, 0x21)
		} else {
			(0x18, 0x23, 0x18)
		});
		menu.canvas
			.fill_rect(Rect::new(menu.x, y, menu.rect.width(), 2))
			.unwrap();
	}
	match input_mode {
		input::Mode::Normal => {
			menu.label_color("Normal", options.ui.colors.normal_mode, font);
			world_manager.console.draw(menu, font);
		}
		input::Mode::Cast => {
			menu.label_color("Cast", options.ui.colors.cast_mode, font);
			spell_menu::draw(menu, &world_manager.next_character().read(), font);
		}
		input::Mode::Cursor { x, y, .. } => {
			menu.label_color("Cursor", options.ui.colors.cursor_mode, font);
			if let Some(selected_character) = world_manager.get_character_at(*x, *y) {
				character_info(menu, &selected_character.read(), (255, 255, 255, 255), font);
			} else {
				world_manager.console.draw(menu, font);
			}
		}
	}
}

pub fn pamphlet(
	pamphlet: &mut gui::Context,
	font: &Font<'_, '_>,
	world_manager: &world::Manager,
	resources: &ResourceManager<'_>,
	soul_jar: &mut SoulJar<'_>,
) {
	struct MemberPosition {
		x: i32,
		y: i32,
		flipped: bool,
	}
	let member_layout = [
		MemberPosition {
			x: -30,
			y: -30,
			flipped: false,
		},
		MemberPosition {
			x: -40,
			y: 0,
			flipped: true,
		},
	];

	pamphlet.advance(0, 32);

	// Draw party stats
	for (character_chunk, layout_chunk) in
		world_manager.party.chunks(2).zip(member_layout.chunks(2))
	{
		let mut character_windows = [None, None];
		for ((character_id, window), layout) in character_chunk
			.iter()
			.zip(character_windows.iter_mut())
			.zip(layout_chunk)
		{
			*window = Some(|player_window: &mut gui::Context| {
				let rect = player_window.rect;
				player_window.relocate(Rect::new(
					rect.x + layout.x,
					rect.y + layout.y,
					rect.width(),
					rect.height(),
				));
				if let Some(piece) = world_manager.get_character(character_id.piece) {
					let piece = piece.read();
					let texture = resources.get_texture("luvui_sleep");
					character_thinking(
						character_id,
						player_window,
						texture,
						layout.flipped,
						|player_window| {
							character_info(player_window, &piece, (255, 255, 255, 255), font);
						},
					);
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
				.set_color_mod(soul.color.0, soul.color.1, soul.color.2);
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
	flipped: bool,
	f: impl FnOnce(&mut gui::Context),
) {
	on_cloud(
		&character_id.draw_state.cloud,
		20,
		character_id.accent_color,
		player_window,
		f,
	);
	let center =
		player_window.x + player_window.rect.width() as i32 * if flipped { 1 } else { 2 } / 3;
	let corner = player_window.x + player_window.rect.width() as i32 * 9 / 10;
	character_id.draw_state.cloud_trail.draw(
		player_window.canvas,
		if flipped { 8 } else { 4 },
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
		.copy_ex(
			texture,
			None,
			Rect::new(center - (width / 2) as i32, player_window.y, width, height),
			0.0,
			None,
			flipped,
			false,
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
	cloud.draw(gui.canvas, target, radius as i16, color.into());
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
	font: &Font<'_, '_>,
) {
	let character::Piece {
		sheet: character::Sheet { nouns, level, .. },
		hp,
		sp,
		..
	} = piece;
	let name = &nouns.name;
	let character::Stats {
		heart,
		soul,
		power,
		defense,
		magic,
		resistance,
	} = piece.sheet.stats();

	player_window.opposing_labels(name, &format!("Level {level}"), color, font);
	player_window.label_color(&format!("HP: {hp}/{heart}"), color, font);
	player_window.progress_bar(
		(*hp as f32) / (heart as f32),
		(0, 255, 0, 255),
		(255, 0, 0, 255),
		10,
		5,
	);
	player_window.label_color(&format!("SP: {sp}/{soul}"), color, font);
	player_window.progress_bar(
		(*sp as f32) / (soul as f32),
		(0, 0, 255, 255),
		(255, 0, 0, 255),
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
