use esprit2::prelude::*;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Handles lazy loading of textures into memory and video memory.
#[derive(Default)]
pub struct TextureInfo<'texture> {
	path: PathBuf,

	/// This is populated upon calling `get_texture`.
	texture: OnceCell<Texture<'texture>>,
	/// Unlike `texture`, this keeps a copy of the texture in memory,
	/// so that further owned instances can be constructed as needed.
	/// This isn't always necessary since usually owned textures are only
	/// for modulation, but it can persist across scene changes unlike owned textures,
	/// which avoids an uneccessary disk read.
	image: OnceCell<Vec<u8>>,

	/// Silence further error messages if any errors occur.
	had_error: Cell<bool>,
}

pub(crate) struct Manager<'texture> {
	texture_creator: &'texture TextureCreator<WindowContext>,
	textures: HashMap<Box<str>, TextureInfo<'texture>>,
	missing_texture: Texture<'texture>,
}

impl<'texture> Manager<'texture> {
	pub(crate) fn new(texture_creator: &'texture TextureCreator<WindowContext>) -> Self {
		Self {
			textures: HashMap::new(),
			texture_creator,
			missing_texture: texture_creator
				.load_texture_bytes(include_bytes!("res/missing_texture.png"))
				.expect("missing texture should never fail to load"),
		}
	}

	/// Return the given texture.
	/// If the texture cannot be found, returns the missing texture placeholder.
	pub(crate) fn get(&self, key: &str) -> &Texture {
		let Some(texture_info) = self.textures.get(key) else {
			return &self.missing_texture;
		};
		texture_info
			.texture
			.get_or_try_init(|| self.texture_creator.load_texture(&texture_info.path))
			.unwrap_or_else(|msg| {
				if !texture_info.had_error.get() {
					error!(
						"failed to load {key} ({}): {msg}",
						texture_info.path.display()
					);
					texture_info.had_error.set(true);
				}
				&self.missing_texture
			})
	}

	/// `Manager` *must* be immutable to function,
	/// but sometimes sdl expects you to have ownership over a texture.
	/// In these situations, `get_owned_texture` can be used.
	/// This *does* allocate more VRAM every time it's called.
	/// It should *not* be called in a loop or on every frame.
	///
	/// The resource manager is still serving a function here:
	/// keys are abstracted, and the image data is saved in memory instead of vram,
	/// meaning that repeated calls to this function will not incur a disk read.
	///
	/// # Errors
	///
	/// Returns an error if the texture could not be found, loaded, or parsed.
	pub(crate) fn open(&self, key: &str) -> Result<Texture<'texture>> {
		let texture_info = self
			.textures
			.get(key)
			.ok_or_else(|| resource::Error::NotFound(key.into()))?;

		let image = texture_info
			.image
			.get_or_try_init(|| fs::read(&texture_info.path))?;
		self.texture_creator
			.load_texture_bytes(image)
			.map_err(crate::Error::Sdl)
	}
}
