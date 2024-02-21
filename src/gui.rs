use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};

pub struct Context<'canvas> {
    canvas: &'canvas mut Canvas<Window>,
    /// Used by draw_text to store textures of fonts before drawing them.
    font_texture_creator: TextureCreator<WindowContext>,
    pub rect: Rect,
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

    pub fn hsplit(&mut self, views: &mut [Option<impl FnMut(&mut Context)>]) {
        // We need to keep track of the tallest child so that we can advance our own pointer by the end of this.
        let mut lowest_child = 0;
        let view_count = views.len();
        for (i, view) in views
            .iter_mut()
            .enumerate()
            .filter_map(|(i, view)| view.as_mut().map(|view| (i, view)))
        {
            let mut child = Context::new(
                self.canvas,
                Rect::new(
                    self.x + (self.rect.width() as i32) / (view_count as i32) * i as i32,
                    self.y,
                    self.rect.width() / (view_count as u32),
                    self.rect.height(),
                ),
            );
            view(&mut child);
            child.vertical();
            lowest_child = lowest_child.max(child.y);
        }
        self.advance(0, (lowest_child - self.y) as u32);
    }

    pub fn progress_bar(
        &mut self,
        progress: f32,
        fill: Color,
        empty: Color,
        margin: u32,
        height: u32,
    ) {
        self.canvas.set_draw_color(empty);
        self.canvas
            .fill_rect(Rect::new(
                self.x + margin as i32,
                self.y,
                self.rect.width() - margin * 2,
                height,
            ))
            .unwrap();
        self.canvas.set_draw_color(fill);
        self.canvas
            .fill_rect(Rect::new(
                self.x + margin as i32,
                self.y,
                (((self.rect.width() - margin * 2) as f32) * progress) as u32,
                height,
            ))
            .unwrap();
        self.advance(self.rect.width(), height);
    }

    pub fn label(&mut self, s: &str, font: &Font) {
        self.label_color(s, Color::WHITE, font)
    }
    pub fn label_color(&mut self, s: &str, color: Color, font: &Font) {
        let font_texture = font
            .render(s)
            .shaded(color, Color::BLACK)
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

    pub fn htexture(&mut self, texture: &Texture, width: u32) {
        let query = texture.query();
        let height = width / query.width * query.height;
        self.canvas
            .copy(
                texture,
                None,
                Some(Rect::new(self.x, self.y, width, height)),
            )
            .unwrap();
        self.advance(width, height)
    }
}
