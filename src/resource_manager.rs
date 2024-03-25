use crate::prelude::*;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{0}")]
	Io(#[from] io::Error),
	#[error("{0}")]
	Toml(#[from] toml::de::Error),
	#[error("{0}")]
	Texture(String),
	#[error(transparent)]
	Vault(#[from] vault::Error),
}

type Resource<T> = HashMap<PathBuf, T>;

#[derive(Default)]
struct TextureInfo<'texture> {
	path: PathBuf,

	texture: OnceCell<Texture<'texture>>,
	image: OnceCell<Vec<u8>>,

	/// If any errors occur, silence further error messages.
	had_error: Cell<bool>,
}

pub struct ResourceManager<'texture> {
	texture_creator: &'texture TextureCreator<WindowContext>,

	attacks: Resource<Attack>,
	spells: Resource<Spell>,
	sheets: Resource<character::Sheet>,
	textures: Resource<TextureInfo<'texture>>,
	vaults: Resource<Vault>,

	missing_texture: Texture<'texture>,
}

fn begin_recurse<T>(
	container: &mut HashMap<PathBuf, T>,
	directory: &Path,
	loader: &dyn Fn(&Path) -> Result<T, Error>,
) -> Result<(), Error> {
	recurse(container, directory, directory, loader)
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

		let mut sheets = HashMap::new();
		begin_recurse(&mut sheets, &path.join("sheets"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let mut attacks = HashMap::new();
		begin_recurse(&mut attacks, &path.join("attacks"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let mut spells = HashMap::new();
		begin_recurse(&mut spells, &path.join("spells"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let mut textures = HashMap::new();
		begin_recurse(&mut textures, &path.join("textures"), &|path| {
			Ok(TextureInfo {
				path: path.to_path_buf(),
				..Default::default()
			})
		})?;

		let mut vaults = HashMap::new();
		begin_recurse(&mut vaults, &path.join("vaults"), &|path| {
			Ok(Vault::open(path)?)
		})?;

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

	pub fn get_attack(&self, path: impl AsRef<Path>) -> Option<&Attack> {
		self.attacks.get(path.as_ref())
	}

	pub fn get_spell(&self, path: impl AsRef<Path>) -> Option<&Spell> {
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
