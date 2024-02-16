use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};

pub struct Context<'canvas> {
    canvas: &'canvas mut Canvas<Window>,
    /// Used by draw_text to store textures of fonts before drawing them.
    font_texture_creator: TextureCreator<WindowContext>,
    rect: Rect,
    /// These values control the position of the cursor.
    x: i32,
    y: i32,
    /// Determines which direction the cursor moves in.
    orientation: Orientation,
}

enum Orientation {
    Vertical,
    Horizontal { height: i32 },
}

impl<'canvas> Context<'canvas> {
    pub fn new(canvas: &'canvas mut Canvas<Window>, rect: Rect) -> Self {
        let font_texture_creator = canvas.texture_creator();
        Self {
            canvas,
            font_texture_creator,
            rect,
            y: rect.y,
            x: rect.x,
            orientation: Orientation::Vertical,
        }
    }

    pub fn advance(&mut self, width: u32, height: u32) {
        let (width, height) = (width as i32, height as i32);
        match self.orientation {
            Orientation::Horizontal { height: o_height } => {
                self.x += width;
                let height = o_height.max(height);
                self.orientation = Orientation::Horizontal { height };
            }
            Orientation::Vertical => {
                self.y += height;
            }
        }
    }

    pub fn horizontal(&mut self) {
        self.orientation = Orientation::Horizontal { height: 0 };
    }

    pub fn vertical(&mut self) {
        if let Orientation::Horizontal { height } = self.orientation {
            self.orientation = Orientation::Vertical;
            self.x = self.rect.x;
            self.y += height;
        }
    }

    pub fn label(&mut self, s: &str, font: &Font) {
        let font_texture = font
            .render(s)
            .shaded(Color::WHITE, Color::BLACK)
            .unwrap()
            .as_texture(&self.font_texture_creator)
            .unwrap();
        let TextureQuery { width, height, .. } = font_texture.query();
        self.canvas
            .copy(
                &font_texture,
                None,
                Rect::new(self.x, self.y, width, height),
            )
            .unwrap();
        // I feel like this drop is kinda silly?
        drop(font_texture);

        self.advance(width, height);
    }
}
