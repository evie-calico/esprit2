use crate::prelude::*;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Io(#[from] io::Error),
	#[error(transparent)]
	Toml(#[from] toml::de::Error),
	#[error("{0}")]
	Texture(String),
	#[error(transparent)]
	Vault(#[from] vault::Error),
}

type Resource<T> = HashMap<PathBuf, T>;

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

/// Manages all resource loading in a central, abstracted structure.
///
/// The primary benefit of using this structure is that it abstracts
/// the path and extension used to load any given asset.
/// `ResourceManager` can also cache certain resources to avoid repeated disk reads,
/// meaning outside code doesn't need to store permanent references to resources.
pub struct ResourceManager<'texture> {
	texture_creator: &'texture TextureCreator<WindowContext>,

	/// `Attack`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Arc`.
	attacks: Resource<Arc<Attack>>,
	/// `Spells`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Arc`.
	spells: Resource<Arc<Spell>>,
	/// Unlike `Attack`s and `Spell`s, `character::Sheet`s are likely to be modified.
	sheets: Resource<character::Sheet>,
	textures: Resource<TextureInfo<'texture>>,
	vaults: Resource<Vault>,

	missing_texture: Texture<'texture>,
}

fn register<T>(
	directory: &Path,
	loader: &dyn Fn(&Path) -> Result<T, Error>,
) -> Result<Resource<T>, Error> {
	let mut container = Resource::new();
	recurse(&mut container, directory, directory, loader)?;
	Ok(container)
}

fn recurse<T>(
	container: &mut HashMap<PathBuf, T>,
	base_directory: &Path,
	directory: &Path,
	loader: &dyn Fn(&Path) -> Result<T, Error>,
) -> Result<(), Error> {
	match fs::read_dir(directory) {
		Ok(read_dir) => {
			for entry in read_dir {
				let entry = entry?;
				let path = entry.path();
				if entry.metadata()?.is_dir() {
					recurse(container, base_directory, &path, loader)?;
				} else {
					let reference = path
						.strip_prefix(base_directory)
						.map(PathBuf::from)
						.unwrap_or_default()
						.parent()
						.unwrap_or(Path::new(""))
						.join(path.file_prefix().unwrap());

					match loader(&path) {
						Ok(resource) => {
							container.insert(reference, resource);
						}
						Err(msg) => {
							error!(
								"Failed to load {} ({}): {msg}",
								reference.display(),
								path.display()
							);
						}
					}
				}
			}
		}
		Err(msg) => {
			error!("Failed to read {}: {msg}", directory.display());
		}
	}
	Ok(())
}

impl<'texture> ResourceManager<'texture> {
	/// Collect known resources into a new resource manager.
	///
	/// # Errors
	///
	/// Returns an error if ANYTHING fails to be read/parsed.
	/// This is probably undesireably and should be moved to logging/diagnostics.
	pub fn open(
		path: impl AsRef<Path>,
		texture_creator: &'texture TextureCreator<WindowContext>,
	) -> Result<ResourceManager<'texture>, Error> {
		let path = path.as_ref();

		let sheets = register(&path.join("sheets"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let attacks = register(&path.join("attacks"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let spells = register(&path.join("spells"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let textures = register(&path.join("textures"), &|path| {
			Ok(TextureInfo {
				path: path.to_path_buf(),
				..Default::default()
			})
		})?;

		let vaults = register(&path.join("vaults"), &|path| Ok(Vault::open(path)?))?;

		// Include a missing texture placeholder, rather than returning an Option.
		let missing_texture = texture_creator
			.load_texture_bytes(include_bytes!("res/missing_texture.png"))
			.unwrap();

		Ok(Self {
			texture_creator,

			attacks,
			spells,
			sheets,
			textures,
			vaults,

			missing_texture,
		})
	}

	pub fn get_sheet(&self, path: impl AsRef<Path>) -> Option<&character::Sheet> {
		self.sheets.get(path.as_ref())
	}

	pub fn get_attack(&self, path: impl AsRef<Path>) -> Option<&Arc<Attack>> {
		self.attacks.get(path.as_ref())
	}

	pub fn get_spell(&self, path: impl AsRef<Path>) -> Option<&Arc<Spell>> {
		self.spells.get(path.as_ref())
	}

	pub fn get_texture(&self, path: impl AsRef<Path>) -> &Texture {
		let path = path.as_ref();
		let Some(texture_info) = self.textures.get(path) else {
			return &self.missing_texture;
		};
		texture_info
			.texture
			.get_or_try_init(|| self.texture_creator.load_texture(&texture_info.path))
			.unwrap_or_else(|msg| {
				if !texture_info.had_error.get() {
					eprintln!(
						"failed to load {} ({}): {msg}",
						path.display(),
						texture_info.path.display()
					);
					texture_info.had_error.set(true);
				}
				&self.missing_texture
			})
	}

	/// `ResourceManager` *must* be immutable to function,
	/// but sometimes sdl expects you to have ownership over a texture.
	/// In these situations, `get_owned_texture` can be used to create an owned instance of a texture.
	/// This *does* allocate more VRAM every time it's called.
	/// It should *not* be called in a loop or on every frame.
	///
	/// The resource manager is still serving a function here:
	/// paths are abstracted, and the image data is saved in memory instead of vram,
	/// meaning that repeated calls to this function will not incur a disk read.
	pub fn get_owned_texture(&self, path: impl AsRef<Path>) -> Option<Texture> {
		let path = path.as_ref();
		let texture_info = self.textures.get(path)?;
		let handle_error = |msg: &str| {
			if !texture_info.had_error.get() {
				eprintln!(
					"failed to load {} ({}): {msg}",
					path.display(),
					texture_info.path.display()
				);
				texture_info.had_error.set(true);
			}
			None
		};
		let image = texture_info
			.image
			.get_or_try_init(|| fs::read(&texture_info.path))
			.map_or_else(|msg| handle_error(&msg.to_string()), Some)?;
		let result = self.texture_creator.load_texture_bytes(image);
		if let Err(msg) = &result {
			handle_error(msg);
		}
		result.ok()
	}

	pub fn get_vault(&self, path: impl AsRef<Path>) -> Option<&Vault> {
		self.vaults.get(path.as_ref())
	}
}
