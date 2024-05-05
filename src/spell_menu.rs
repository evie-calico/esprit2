use crate::prelude::*;
use sdl2::ttf::Font;

pub fn draw(gui: &mut gui::Context, font: &Font) {
	gui.label("(a) Magic Missile", font);
	gui.label("(b) Magic Missile", font);
	gui.label("(c) Magic Missile", font);
	gui.label("(d) Magic Missile", font);
	gui.label("(e) Magic Missile", font);
	gui.label("(f) Magic Missile", font);
}
