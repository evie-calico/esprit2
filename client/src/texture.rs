use esprit2::prelude::*;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::fs;
use std::path::{Path, PathBuf};

/// Handles lazy loading of textures into memory and video memory.
#[derive(Default)]
struct TextureInfo<'texture> {
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
	textures: Resource<TextureInfo<'texture>>,
	missing_texture: Texture<'texture>,
}

impl<'texture> Manager<'texture> {
	pub(crate) fn new(
		path: impl AsRef<Path>,
		texture_creator: &'texture TextureCreator<WindowContext>,
	) -> Result<Self> {
		let textures = resource::register(path.as_ref(), |path| {
			Ok(TextureInfo {
				path: path.to_path_buf(),
				..Default::default()
			})
		})?;

		Ok(Self {
			textures,
			texture_creator,
			missing_texture: texture_creator
				.load_texture_bytes(include_bytes!("res/missing_texture.png"))
				.map_err(crate::Error::Sdl)?,
		})
	}

	/// Return the given texture.
	/// If the texture cannot be found, returns the missing texture placeholder.
	pub(crate) fn get(&self, key: &str) -> &Texture {
		let Ok(texture_info) = self.textures.get(key) else {
			return &self.missing_texture;
		};
		texture_info
			.texture
			.get_or_try_init(|| self.texture_creator.load_texture(&texture_info.path))
			.unwrap_or_else(|msg| {
				if !texture_info.had_error.get() {
					eprintln!(
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
	/// In these situations, `get_owned_texture` can be used to create an owned instance of a texture.
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
	pub(crate) fn open(&self, key: &str) -> Result<Texture> {
		let texture_info = self.textures.get(key)?;

		let image = texture_info
			.image
			.get_or_try_init(|| fs::read(&texture_info.path))?;
		self.texture_creator
			.load_texture_bytes(image)
			.map_err(crate::Error::Sdl)
	}
}
