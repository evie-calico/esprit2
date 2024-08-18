#![allow(clippy::unwrap_used, reason = "SDL")]

use crate::gui;
use esprit2::prelude::*;
use rand::Rng;
use sdl2::rect::{Point, Rect};
use sdl2::render::Texture;

#[derive(Clone, Default, Debug)]
pub struct PartyReferenceDrawState {
	pub cloud: draw::CloudState,
	pub cloud_trail: draw::CloudTrail,
}

pub struct SoulJar<'texture> {
	souls: Vec<Soul>,
	light_texture: Texture<'texture>,
}

impl<'texture> SoulJar<'texture> {
	pub fn new(resources: &'texture resource::Manager<'_>) -> Result<Self> {
		let mut rng = rand::thread_rng();
		let souls = (0..=9)
			.map(|_| Soul::new((rng.gen(), rng.gen(), rng.gen(), 255)))
			.collect();
		Ok(Self {
			souls,
			light_texture: resources.get_owned_texture("light")?,
		})
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
	input_mode: &input::Mode,
	world_manager: &world::Manager,
	console: &Console,
	resources: &resource::Manager<'_>,
) {
	for (i, color) in [(0x14, 0x17, 0x14), (0xE3, 0xBD, 0xEF), (0x14, 0x17, 0x14)]
		.into_iter()
		.enumerate()
	{
		menu.canvas.set_draw_color(color);
		menu.canvas
			.fill_rect(menu.rect.top_shifted(i as i32 * -10))
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
			menu.label_styled(
				"Normal",
				options.ui.colors.normal_mode,
				&menu.typography.annotation,
			);
			menu.console(console);
		}
		input::Mode::Attack => {
			menu.label_styled(
				"Attack",
				options.ui.colors.cast_mode,
				&menu.typography.annotation,
			);
			attack_menu(menu, &world_manager.next_character().borrow());
		}
		input::Mode::Cast => {
			menu.label_styled(
				"Cast",
				options.ui.colors.cast_mode,
				&menu.typography.annotation,
			);
			spell_menu(menu, &world_manager.next_character().borrow());
		}
		input::Mode::Cursor {
			position: (x, y), ..
		} => {
			menu.label_styled(
				"Cursor",
				options.ui.colors.cursor_mode,
				&menu.typography.annotation,
			);
			if let Some(selected_character) = world_manager.get_character_at(*x, *y) {
				let mut character_fn = |menu: &mut gui::Context| {
					character_info(menu, &selected_character.borrow());
				};
				let mut buff_fn = |menu: &mut gui::Context| {
					character_buffs(menu, &selected_character.borrow(), resources);
				};
				menu.hsplit(&mut [
					Some((&mut character_fn) as &mut dyn FnMut(&mut gui::Context)),
					Some(&mut buff_fn),
				]);
			} else {
				menu.console(console);
			}
		}
		input::Mode::Prompt { message, .. } => {
			menu.label_styled(
				"Prompt",
				options.ui.colors.cursor_mode,
				&menu.typography.annotation,
			);
			menu.label(message);
			menu.margin_list([
				("Yes: ", options.controls.yes.to_string().as_str()),
				("No: ", options.controls.no.to_string().as_str()),
				("Cancel: ", options.controls.escape.to_string().as_str()),
			]);
		}
		input::Mode::DirectionPrompt { message, .. } => {
			menu.label_styled(
				"Direction Prompt",
				options.ui.colors.cursor_mode,
				&menu.typography.annotation,
			);
			menu.label(message);
			menu.margin_list([
				("Left: ", options.controls.left.to_string().as_str()),
				("Up: ", options.controls.up.to_string().as_str()),
				("Down: ", options.controls.down.to_string().as_str()),
				("Right: ", options.controls.right.to_string().as_str()),
			]);
		}
	}
}

pub fn spell_menu(gui: &mut gui::Context, character: &character::Piece) {
	for (spell, letter) in character.spells.iter().zip('a'..='z') {
		let color = match spell.castable_by(character) {
			spell::Castable::Yes => gui.typography.color,
			spell::Castable::NotEnoughSP => (255, 0, 0, 255),
			spell::Castable::UncastableAffinity => (128, 128, 128, 255),
		};
		gui.label_color(
			&format!("({letter}) {} - {} SP", spell.name, spell.level),
			color,
		);
	}
}

pub fn attack_menu(gui: &mut gui::Context, character: &character::Piece) {
	for (attack, letter) in character.attacks.iter().zip('a'..='z') {
		gui.label(&format!("({letter}) {}", attack.name));
	}
}

pub struct Pamphlet {
	pub party_member_clouds: Vec<PartyReferenceDrawState>,
}

impl Pamphlet {
	pub fn new() -> Self {
		Self {
			party_member_clouds: vec![
				PartyReferenceDrawState::default(),
				PartyReferenceDrawState::default(),
				PartyReferenceDrawState::default(),
				PartyReferenceDrawState::default(),
			],
		}
	}

	pub fn draw(
		&self,
		pamphlet: &mut gui::Context,
		world_manager: &world::Manager,
		resources: &resource::Manager<'_>,
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
		for ((character_chunk, layout_chunk), cloud_chunk) in world_manager
			.party
			.chunks(2)
			.zip(member_layout.chunks(2))
			.zip(self.party_member_clouds.chunks(2))
		{
			let mut character_windows = [None, None];
			for (((character_id, window), layout), cloud) in character_chunk
				.iter()
				.zip(character_windows.iter_mut())
				.zip(layout_chunk)
				.zip(cloud_chunk)
			{
				*window = Some(|player_window: &mut gui::Context| {
					let rect = player_window.rect;
					player_window.relocate(Rect::new(
						rect.x + layout.x,
						rect.y + layout.y,
						rect.width(),
						rect.height(),
					));
					let piece = character_id.piece.borrow();
					let texture = resources.get_texture("luvui_sleep");
					character_thinking(
						cloud,
						character_id.accent_color,
						player_window,
						texture,
						layout.flipped,
						|player_window| {
							character_info(player_window, &piece);
							character_buffs(player_window, &piece, resources);
						},
					);
				});
			}
			pamphlet.hsplit(&mut character_windows);
		}
		pamphlet.advance(0, 10);

		let mut inventory_fn = |pamphlet: &mut gui::Context| {
			pamphlet.label("Inventory");
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
			pamphlet.label("Souls");

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
}

fn character_thinking(
	draw_state: &PartyReferenceDrawState,
	accent_color: Color,
	player_window: &mut gui::Context<'_, '_, '_>,
	texture: &Texture,
	flipped: bool,
	f: impl FnOnce(&mut gui::Context),
) {
	on_cloud(&draw_state.cloud, 20, accent_color, player_window, f);
	let center =
		player_window.x + player_window.rect.width() as i32 * if flipped { 1 } else { 2 } / 3;
	let corner = player_window.x + player_window.rect.width() as i32 * 9 / 10;
	draw_state.cloud_trail.draw(
		player_window.canvas,
		if flipped { 8 } else { 4 },
		Point::new(center, player_window.y + 10),
		Point::new(corner, player_window.y - 25),
		15.0,
		accent_color.into(),
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
	gui: &mut gui::Context<'_, '_, '_>,
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
				gui.typography,
				Rect::new(0, 0, width - radius * 2, height - radius * 2),
			);
			f(&mut gui);
			gui.advance(0, 4);
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

fn character_info(player_window: &mut gui::Context<'_, '_, '_>, piece: &character::Piece) {
	let character::Piece {
		sheet: character::Sheet { nouns, level, .. },
		hp,
		sp,
		..
	} = piece;
	let name = &nouns.name;
	let character::StatOutcomes {
		stats:
			character::Stats {
				heart,
				soul,
				power,
				defense,
				magic,
				resistance,
			},
		buffs,
		debuffs,
	} = piece.stat_outcomes();

	let font = &player_window.typography.normal;
	let get_color = |buff, debuff| {
		if buff > debuff {
			(0, 0, 255, 255)
		} else if debuff > 0 {
			(255, 0, 0, 255)
		} else {
			(255, 255, 255, 255)
		}
	};

	player_window.opposing_labels(
		name,
		&format!("Level {level}"),
		player_window.typography.color,
		font,
	);
	player_window.label(&format!("HP: {hp}/{heart}"));
	player_window.progress_bar(
		(*hp as f32) / (heart as f32),
		(0, 255, 0, 255),
		(255, 0, 0, 255),
		10,
		5,
	);
	player_window.label(&format!("SP: {sp}/{soul}"));
	player_window.progress_bar(
		(*sp as f32) / (soul as f32),
		(0, 0, 255, 255),
		(255, 0, 0, 255),
		10,
		5,
	);
	let physical_stat_info = [
		("Pwr", power, buffs.power, debuffs.power),
		("Def", defense, buffs.defense, debuffs.defense),
	];
	let mut physical_stats = [None, None];
	for ((stat_name, stat, buff, debuff), stat_half) in physical_stat_info
		.into_iter()
		.zip(physical_stats.iter_mut())
	{
		*stat_half = Some(move |stat_half: &mut gui::Context| {
			let color = get_color(buff, debuff);
			stat_half.horizontal();
			stat_half.label_color(&stat.to_string(), color);
			stat_half.advance(4, 0);
			stat_half.label_color(stat_name, color);
		});
	}
	player_window.hsplit(&mut physical_stats);
	let magical_stat_info = [
		("Mag", magic, buffs.magic, debuffs.magic),
		("Res", resistance, buffs.resistance, debuffs.resistance),
	];
	let mut magical_stats = [None, None];
	for ((stat_name, stat, buff, debuff), stat_half) in
		magical_stat_info.into_iter().zip(magical_stats.iter_mut())
	{
		*stat_half = Some(move |stat_half: &mut gui::Context| {
			let color = get_color(buff, debuff);
			stat_half.horizontal();
			stat_half.label_color(&stat.to_string(), color);
			stat_half.advance(4, 0);
			stat_half.label_color(stat_name, color);
		});
	}
	player_window.hsplit(&mut magical_stats);
}

fn character_buffs(
	gui: &mut gui::Context<'_, '_, '_>,
	piece: &character::Piece,
	resources: &resource::Manager<'_>,
) {
	let mut statuses = piece.statuses.values().peekable();
	while statuses.peek().is_some() {
		let textures_per_row = gui.rect.width() / (32 + 8);
		gui.horizontal();
		for _ in 0..textures_per_row {
			if let Some(status) = statuses.next() {
				gui.htexture(resources.get_texture(&status.icon), 32);
				gui.advance(8, 0);
			}
		}
		gui.vertical();
		gui.advance(8, 8);
	}
	for status in piece.statuses.values() {
		character_status(gui, status);
	}
}

fn character_status(gui: &mut gui::Context, status: &Status) {
	gui.label(&status.name);

	let mut print_stats = |stats: character::Stats| {
		for (name, value) in [
			("Heart", stats.heart),
			("Soul", stats.soul),
			("Power", stats.power),
			("Defense", stats.defense),
			("Magic", stats.magic),
			("Resistance", stats.resistance),
		] {
			if value > 0 {
				gui.horizontal();
				gui.advance(30, 0);
				gui.label(&format!("-{value} {name}"));
				gui.vertical();
			}
		}
	};

	match &status.effect {
		status::Effect::Debuff(debuff) => {
			if let Some(stats) = debuff.get() {
				print_stats(stats);
			}
		}
		status::Effect::StaticDebuff(stats) => print_stats(*stats),
	}
}
