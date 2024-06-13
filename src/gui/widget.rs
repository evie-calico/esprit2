use crate::prelude::*;
use sdl2::{pixels::Color, rect::Rect, render::Texture};

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
				character_info(menu, &selected_character.read(), font);
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

					character_info(player_window, &piece, font);
					player_window.advance(0, 10);
					player_window.label("Attacks", font);
					for attack in &piece.attacks {
						player_window.horizontal();
						player_window.label(&attack.name, font);
						player_window.advance(20, 0);
						player_window.expression::<character::Stats>(&attack.damage, font);
						player_window.vertical();
					}
					player_window.advance(0, 10);
					player_window.label("Spells", font);
					let mut spells = piece.sheet.spells.iter().peekable();
					while spells.peek().is_some() {
						let textures_per_row = player_window.rect.width() / (32 + 8);
						player_window.horizontal();
						for _ in 0..textures_per_row {
							if let Some(spell) = spells.next() {
								player_window.htexture(resources.get_texture(spell), 32);
								player_window.advance(8, 0);
							}
						}
						player_window.vertical();
						player_window.advance(8, 8);
					}
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

fn character_info(
	player_window: &mut gui::Context<'_>,
	piece: &character::Piece,
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

	player_window.label_color(
		&format!("{name} ({:08x})", piece.id.as_fields().0),
		match piece.sheet.nouns.pronouns {
			nouns::Pronouns::Female => Color::RGB(247, 141, 246),
			nouns::Pronouns::Male => Color::RGB(104, 166, 232),
			_ => Color::WHITE,
		},
		font,
	);
	player_window.label(&format!("Level {level}"), font);
	player_window.label(&format!("HP: {hp}/{heart}"), font);
	player_window.progress_bar(
		(*hp as f32) / (*heart as f32),
		Color::GREEN,
		Color::RED,
		10,
		5,
	);
	player_window.label(&format!("SP: {sp}/{soul}"), font);
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
			stat_half.label(&format!("{stat_name}: {stat}"), font)
		});
	}
	player_window.hsplit(&mut physical_stats);
	let magical_stat_info = [("Mag", magic), ("Res", resistance)];
	let mut magical_stats = [None, None];
	for ((stat_name, stat), stat_half) in
		magical_stat_info.into_iter().zip(magical_stats.iter_mut())
	{
		*stat_half = Some(move |stat_half: &mut gui::Context| {
			stat_half.label(&format!("{stat_name}: {stat}"), font)
		});
	}
	player_window.hsplit(&mut magical_stats);
}
