use crate::prelude::*;
use mlua::{FromLua, FromLuaMulti};
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

/// Manages all resource loading in a central, abstracted structure.
///
/// The primary benefit of using this structure is that it abstracts
/// the path and extension used to load any given asset.
/// `resource::Manager` can also cache certain resources to avoid repeated disk reads,
/// meaning outside code doesn't need to store permanent references to resources.
pub struct Manager {
	/// `Attack`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Rc`.
	attacks: Resource<Rc<Attack>>,
	/// `Spells`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Rc`.
	spells: Resource<Rc<Spell>>,
	/// Unlike `Attack`s and `Spell`s, `character::Sheet`s are likely to be modified.
	sheets: Resource<character::Sheet>,
	statuses: Rc<Resource<Status>>,
	vaults: Resource<Vault>,
}

impl mlua::UserData for Manager {}

pub struct Scripts<'lua> {
	pub runtime: &'lua mlua::Lua,
	sandbox_metatable: mlua::Table<'lua>,
	scripts: Resource<mlua::Function<'lua>>,
}

pub fn register<T>(directory: &Path, loader: &dyn Fn(&Path) -> Result<T>) -> Result<Resource<T>> {
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

impl Manager {
	/// Collect known resources into a new resource manager.
	///
	/// # Errors
	///
	/// Returns an error if ANYTHING fails to be read/parsed.
	/// This is probably undesirable and should be moved to logging/diagnostics.
	pub fn open(path: impl AsRef<Path>) -> Result<Manager> {
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

		let vaults = register(&path.join("vaults"), &|path| Vault::open(path))?;

		Ok(Self {
			attacks,
			spells,
			sheets,
			statuses,
			vaults,
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

	/// Return the given vault.
	///
	/// # Errors
	///
	/// Returns an error if the vault could not be found.
	pub fn get_vault(&self, key: &str) -> Result<&Vault> {
		self.vaults.get(key)
	}
}

pub struct SandboxBuilder<'lua, 'scripts> {
	runtime: &'lua mlua::Lua,
	function: &'scripts mlua::Function<'lua>,
	environment: mlua::Table<'lua>,
}

impl<'lua, 'scripts> SandboxBuilder<'lua, 'scripts> {
	pub fn insert(
		self,
		key: impl mlua::IntoLua<'lua>,
		value: impl mlua::IntoLua<'lua>,
	) -> Result<Self> {
		self.environment.set(key, value)?;
		Ok(self)
	}

	pub fn thread(self) -> mlua::Result<mlua::Thread<'lua>> {
		self.function.set_environment(self.environment)?;
		self.runtime.create_thread(self.function.clone())
	}

	pub fn call<R: FromLuaMulti<'lua>>(
		self,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<R> {
		self.function.set_environment(self.environment)?;
		self.function.call::<_, R>(args)
	}

	pub fn world<R: FromLua<'lua>>(
		self,
		world: &world::Manager,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> Result<R> {
		self.function.set_environment(self.environment)?;
		let thread = self.runtime.create_thread(self.function.clone())?;
		Ok(world.poll(self.runtime, thread, args)?)
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

	pub fn function(&self, key: &str) -> Result<&mlua::Function<'lua>> {
		self.scripts.get(key)
	}

	pub fn sandbox<'scripts>(&'scripts self, key: &str) -> Result<SandboxBuilder<'lua, 'scripts>> {
		let function = self.scripts.get(key)?;
		let environment = self.runtime.create_table()?;
		// This is cloning a reference, which is a lot cheaper than creating a new table.
		environment.set_metatable(Some(self.sandbox_metatable.clone()));
		Ok(SandboxBuilder {
			runtime: self.runtime,
			function,
			environment,
		})
	}
}
