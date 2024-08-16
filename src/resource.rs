use crate::prelude::*;
use mlua::FromLuaMulti;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("resource {0} not found")]
	NotFound(Id),
	#[error("keys (file names) must be representable in UTF8")]
	InvalidKey,
}

pub type Id = Box<str>;

pub struct Resource<T>(HashMap<Id, T>);

impl<T> Resource<T> {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn get(&self, key: &str) -> Result<&T> {
		self.0
			.get(key)
			.ok_or_else(|| crate::Error::Resource(Error::NotFound(key.into())))
	}
}

impl<T> Default for Resource<T> {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Clone, mlua::FromLua)]
pub struct Handle<T>(pub Rc<Resource<T>>);

impl<T: Clone + mlua::UserData + 'static> mlua::UserData for Handle<T> {
	fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method("get", |_lua, this, key: String| {
			this.0
				.get(key.as_str())
				.cloned()
				.map_err(mlua::Error::external)
		});
	}
}

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

	/// `Attack`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Rc`.
	attacks: Resource<Rc<Attack>>,
	/// `Spells`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Rc`.
	spells: Resource<Rc<Spell>>,
	/// Unlike `Attack`s and `Spell`s, `character::Sheet`s are likely to be modified.
	sheets: Resource<character::Sheet>,
	statuses: Rc<Resource<Status>>,
	textures: Resource<TextureInfo<'texture>>,
	vaults: Resource<Vault>,

	missing_texture: Texture<'texture>,
}

impl mlua::UserData for Manager<'_> {}

pub struct Scripts<'lua> {
	pub runtime: &'lua mlua::Lua,
	sandbox_metatable: mlua::Table<'lua>,
	scripts: Resource<mlua::Function<'lua>>,
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
							path.file_prefix()
								.expect("path should never be a directory"),
						);
					let reference = reference.to_str().ok_or(Error::InvalidKey)?;

					match loader(&path) {
						Ok(resource) => {
							container.0.insert(reference.into(), resource);
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

		let statuses = register(&path.join("statuses"), &|path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?
		.into();

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
			statuses,
			textures,
			vaults,

			missing_texture,
		})
	}

	pub fn statuses_handle(&self) -> Handle<Status> {
		Handle(self.statuses.clone())
	}

	/// Return the given sheet.
	///
	/// # Errors
	///
	/// Returns an error if the sheet could not be found.
	pub fn get_sheet(&self, key: &str) -> Result<&character::Sheet> {
		self.sheets.get(key)
	}

	/// Return the given status.
	///
	/// # Errors
	///
	/// Returns an error if the status could not be found.
	pub fn get_status(&self, key: &str) -> Result<&Status> {
		self.statuses.get(key)
	}

	/// Return the given attack.
	///
	/// # Errors
	///
	/// Returns an error if the attack could not be found.
	pub fn get_attack(&self, key: &str) -> Result<&Rc<Attack>> {
		self.attacks.get(key)
	}

	/// Return the given spell.
	///
	/// # Errors
	///
	/// Returns an error if the spell could not be found.
	pub fn get_spell(&self, key: &str) -> Result<&Rc<Spell>> {
		self.spells.get(key)
	}

	/// Return the given texture.
	/// If the texture cannot be found, returns the missing texture placeholder.
	pub fn get_texture(&self, key: &str) -> &Texture {
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
	pub fn get_owned_texture(&self, key: &str) -> Result<Texture> {
		let texture_info = self.textures.get(key)?;

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
		self.vaults.get(key)
	}
}

pub struct SandboxBuilder<'lua> {
	function: &'lua mlua::Function<'lua>,
	environment: mlua::Table<'lua>,
}

impl<'lua> SandboxBuilder<'lua> {
	pub fn insert(
		self,
		key: impl mlua::IntoLua<'lua>,
		value: impl mlua::IntoLua<'lua>,
	) -> Result<Self> {
		self.environment.set(key, value)?;
		Ok(self)
	}

	pub fn call<R: FromLuaMulti<'lua>>(
		self,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<R> {
		self.function.set_environment(self.environment)?;
		self.function.call::<_, R>(args)
	}
}

impl<'lua> Scripts<'lua> {
	pub fn open(path: impl AsRef<Path>, lua: &'lua mlua::Lua) -> Result<Scripts<'lua>> {
		Ok(Self {
			runtime: lua,
			sandbox_metatable: lua.create_table_from([("__index", lua.globals())])?,
			scripts: register(path.as_ref(), &|path| {
				Ok(lua
					.load(&fs::read_to_string(path)?)
					.set_name(path.to_string_lossy())
					.into_function()?)
			})?,
		})
	}

	pub fn function(&'lua self, key: &str) -> Result<&'lua mlua::Function<'lua>> {
		self.scripts.get(key)
	}

	pub fn sandbox(&'lua self, key: &str) -> Result<SandboxBuilder<'lua>> {
		let function = self.scripts.get(key)?;
		let environment = self.runtime.create_table()?;
		// This is cloning a reference, which is a lot cheaper than creating a new table.
		environment.set_metatable(Some(self.sandbox_metatable.clone()));
		Ok(SandboxBuilder {
			function,
			environment,
		})
	}
}
