use crate::prelude::*;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("resource {0} not found")]
	NotFound(String),
	#[error("keys (file names) must be representable in UTF8")]
	InvalidKey,
}

type Resource<T> = HashMap<Box<str>, T>;

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
/// `resource::Manager` can also cache certain resources to avoid repeated disk reads,
/// meaning outside code doesn't need to store permanent references to resources.
pub struct Manager<'texture> {
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

fn register<T>(directory: &Path, loader: &dyn Fn(&Path) -> Result<T>) -> Result<Resource<T>> {
	let mut container = Resource::new();
	recurse(&mut container, directory, directory, loader)?;
	Ok(container)
}

fn recurse<T>(
	container: &mut Resource<T>,
	base_directory: &Path,
	directory: &Path,
	loader: &dyn Fn(&Path) -> Result<T>,
) -> Result<()> {
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
						.join(
							#[allow(
								clippy::unwrap_used,
								reason = "file_prefix can only fail when used on directories"
							)]
							path.file_prefix().unwrap(),
						);
					let reference = reference.to_str().ok_or(Error::InvalidKey)?;

					match loader(&path) {
						Ok(resource) => {
							container.insert(reference.into(), resource);
						}
						Err(msg) => {
							error!("Failed to load {reference} ({}): {msg}", path.display());
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

impl<'texture> Manager<'texture> {
	/// Collect known resources into a new resource manager.
	///
	/// # Errors
	///
	/// Returns an error if ANYTHING fails to be read/parsed.
	/// This is probably undesirable and should be moved to logging/diagnostics.
	pub fn open(
		path: impl AsRef<Path>,
		texture_creator: &'texture TextureCreator<WindowContext>,
	) -> Result<Manager<'texture>> {
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

		let vaults = register(&path.join("vaults"), &|path| Vault::open(path))?;

		// Include a missing texture placeholder, rather than returning an Option.
		let missing_texture = texture_creator
			.load_texture_bytes(include_bytes!("res/missing_texture.png"))
			.map_err(crate::Error::Sdl)?;

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

	/// Return the given sheet.
	///
	/// # Errors
	///
	/// Returns an error if the sheet could not be found.
	pub fn get_sheet(&self, key: &str) -> Result<&character::Sheet> {
		Ok(self
			.sheets
			.get(key)
			.ok_or_else(|| Error::NotFound(key.into()))?)
	}

	/// Return the given attack.
	///
	/// # Errors
	///
	/// Returns an error if the attack could not be found.
	pub fn get_attack(&self, key: &str) -> Result<&Arc<Attack>> {
		Ok(self
			.attacks
			.get(key)
			.ok_or_else(|| Error::NotFound(key.into()))?)
	}

	/// Return the given spell.
	///
	/// # Errors
	///
	/// Returns an error if the spell could not be found.
	pub fn get_spell(&self, key: &str) -> Result<&Arc<Spell>> {
		Ok(self
			.spells
			.get(key)
			.ok_or_else(|| Error::NotFound(key.into()))?)
	}

	/// Return the given texture.
	/// If the texture cannot be found, returns the missing texture placeholder.
	pub fn get_texture(&self, key: &str) -> &Texture {
		let Some(texture_info) = self.textures.get(key) else {
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
	pub fn get_owned_texture(&self, key: &str) -> Result<Texture> {
		let texture_info = self
			.textures
			.get(key)
			.ok_or_else(|| Error::NotFound(key.into()))?;

		let image = texture_info
			.image
			.get_or_try_init(|| fs::read(&texture_info.path))?;
		self.texture_creator
			.load_texture_bytes(image)
			.map_err(crate::Error::Sdl)
	}

	/// Return the given vault.
	///
	/// # Errors
	///
	/// Returns an error if the vault could not be found.
	pub fn get_vault(&self, key: &str) -> Result<&Vault> {
		Ok(self
			.vaults
			.get(key)
			.ok_or_else(|| Error::NotFound(key.into()))?)
	}
}
