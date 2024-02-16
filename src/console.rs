use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::Window;
use std::fmt::Display;

const MAX_DISPLAYED_MESSAGES: usize = 10;

#[derive(Clone, Debug, Default)]
pub struct Console {
    history: Vec<String>,
}

impl Console {
    pub fn println(&mut self, message: impl Display) {
        self.history.push(message.to_string());
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>, rect: Rect, font: &Font) {
        let font_texture_creator = canvas.texture_creator();
        canvas.set_clip_rect(rect);

        let mut cursor = rect.y + (rect.height() as i32);

        for message in self.history.iter().rev().take(MAX_DISPLAYED_MESSAGES) {
            let font_texture = font
                .render(message)
                .shaded(Color::WHITE, Color::BLACK)
                .unwrap()
                .as_texture(&font_texture_creator)
                .unwrap();
            let TextureQuery { width, height, .. } = font_texture.query();
            cursor -= height as i32;
            canvas
                .copy(
                    &font_texture,
                    None,
                    Rect::new(rect.x, cursor, width, height),
                )
                .unwrap();
        }

        canvas.set_clip_rect(None);
    }
}
