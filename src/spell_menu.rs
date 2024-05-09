use crate::prelude::*;
use sdl2::{pixels::Color, ttf::Font};

pub fn draw(gui: &mut gui::Context, character: &character::Piece, font: &Font) {
	for (spell, letter) in character.spells.iter().zip('a'..='z') {
		let color = if spell.castable_by(character) {
			Color::WHITE
		} else {
			Color::RED
		};
		gui.label_color(
			&format!("({letter}) {} - {} SP", spell.name, spell.level),
			color,
			font,
		);
	}
}
