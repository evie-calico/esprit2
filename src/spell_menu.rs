use crate::prelude::*;
use sdl2::ttf::Font;

pub fn draw(gui: &mut gui::Context, character: &character::Piece, font: &Font) {
	for (spell, letter) in character.spells.iter().zip('a'..='z') {
		let color = if spell.castable_by(character) {
			(255, 255, 255, 255)
		} else {
			(255, 0, 0, 255)
		};
		gui.label_color(
			&format!("({letter}) {} - {} SP", spell.name, spell.level),
			color,
			font,
		);
	}
}
