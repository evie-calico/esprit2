use crate::prelude::*;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
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
}

type Resource<T> = HashMap<PathBuf, T>;

pub struct ResourceManager<'texture> {
	attacks: Resource<Attack>,
	spells: Resource<Spell>,
	sheets: Resource<character::Sheet>,
	textures: Resource<Texture<'texture>>,
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
			texture_creator.load_texture(path).map_err(Error::Texture)
		})?;

		let mut vaults = HashMap::new();
		begin_recurse(&mut vaults, &path.join("vaults"), &|path| {
			Ok(Vault::open(path))
		})?;

		// Include a missing texture placeholder, rather than returning an Option.
		let missing_texture = texture_creator
			.load_texture_bytes(include_bytes!("res/missing_texture.png"))
			.unwrap();

		Ok(Self {
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
		self.textures
			.get(path.as_ref())
			.unwrap_or(&self.missing_texture)
	}

	pub fn get_vault(&self, path: impl AsRef<Path>) -> Option<&Vault> {
		self.vaults.get(path.as_ref())
	}
}
