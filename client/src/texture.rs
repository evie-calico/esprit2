use esprit2::prelude::*;
use rkyv::rancor::{self, ResultExt};
use sdl3::image::LoadTexture;
use sdl3::render::{Texture, TextureCreator};
use sdl3::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Handles lazy loading of textures into memory and video memory.
#[derive(Default)]
pub(crate) struct TextureInfo<'texture> {
	pub(crate) path: PathBuf,

	/// This is populated upon calling `get_texture`.
	pub(crate) texture: OnceCell<Texture<'texture>>,
	/// Unlike `texture`, this keeps a copy of the texture in memory,
	/// so that further owned instances can be constructed as needed.
	/// This isn't always necessary since usually owned textures are only
	/// for modulation, but it can persist across scene changes unlike owned textures,
	/// which avoids an uneccessary disk read.
	pub(crate) image: OnceCell<Vec<u8>>,

	/// Silence further error messages if any errors occur.
	pub(crate) had_error: Cell<bool>,
}

pub(crate) struct Manager<'texture> {
	pub(crate) texture_creator: &'texture TextureCreator<WindowContext>,
	pub(crate) textures: HashMap<Box<str>, TextureInfo<'texture>>,
	pub(crate) missing_texture: Texture<'texture>,

	pub(crate) sheets: HashMap<Box<str>, Sheet>,
}

impl<'texture> Manager<'texture> {
	pub(crate) fn new(texture_creator: &'texture TextureCreator<WindowContext>) -> Self {
		Self {
			textures: HashMap::new(),
			texture_creator,
			missing_texture: texture_creator
				.load_texture_bytes(include_bytes!("res/missing_texture.png"))
				.expect("missing texture should never fail to load"),

			sheets: HashMap::new(),
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
	pub(crate) fn open(&self, key: &str) -> Result<Texture<'texture>, rancor::BoxedError> {
		let texture_info = self
			.textures
			.get(key)
			.ok_or_else(|| resource::Error::NotFound(key.into()))
			.into_error()?;

		let image = texture_info
			.image
			.get_or_try_init(|| fs::read(&texture_info.path))
			.into_error()?;
		self.texture_creator.load_texture_bytes(image).into_error()
	}
}

// Texture fields for character sheets
#[derive(Debug)]
pub(crate) struct Sheet {
	pub(crate) icon: Box<str>,
}
