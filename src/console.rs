use paste::paste;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::Window;
use std::fmt::Display;

#[derive(Clone, Debug, Default)]
pub struct Console {
    history: Vec<ConsoleMessage>,
    settings: ConsoleSettings,
}

#[derive(Clone, Debug)]
pub struct ConsoleSettings {
    normal: Color,
    system: Color,
    defeat: Color,
    danger: Color,
    important: Color,
    special: Color,
}

impl Default for ConsoleSettings {
    fn default() -> Self {
        Self {
            normal: Color::WHITE,
            system: Color::GREY,
            defeat: Color::RGB(255, 128, 128),
            danger: Color::RED,
            important: Color::YELLOW,
            special: Color::GREEN,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConsoleMessage {
    text: String,
    color: Color,
}

macro_rules! colored_print {
    ($which:ident) => {
        paste! {
            pub fn [<print_ $which>](&mut self, message: impl Display) {
                self.history.push(ConsoleMessage {
                    text: message.to_string(),
                    color: self.settings.$which,
                });
            }
        }
    };
}

impl Console {
    pub fn print(&mut self, message: impl Display) {
        self.history.push(ConsoleMessage {
            text: message.to_string(),
            color: self.settings.normal,
        });
    }

    colored_print!(system);
    colored_print!(danger);
    colored_print!(defeat);
    colored_print!(important);
    colored_print!(special);

    pub fn draw(&self, canvas: &mut Canvas<Window>, rect: Rect, font: &Font) {
        let font_texture_creator = canvas.texture_creator();
        canvas.set_clip_rect(rect);

        let mut cursor = rect.y + (rect.height() as i32);

        for message in self.history.iter().rev() {
            let font_texture = font
                .render(&message.text)
                .shaded(message.color, Color::BLACK)
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
            if cursor < rect.y {
                break;
            }
        }

        canvas.set_clip_rect(None);
    }
}
