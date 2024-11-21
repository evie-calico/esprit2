use crate::prelude::*;
use mlua::{FromLua, FromLuaMulti};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("resource {0} not found")]
	NotFound(Box<str>),
	#[error("keys (file names) must be representable in UTF8")]
	InvalidKey,
}

pub trait Id<Manager> {
	type Resource;
	fn get<'resources>(&self, resources: &'resources Manager)
		-> Result<&'resources Self::Resource>;
}

macro_rules! impl_resource {
	(impl$(<$($lifetime:lifetime),*>)? $Name:ident as $Resource:ty where ($self:ident, $resources:ident: $Manager:ty) $body:tt $(impl$(<$($next_lifetime:lifetime),*>)? $NextName:ident as $NextResource:ty where ($next_self:ident, $next_resources:ident: $NextManager:ty) $next_body:tt)+) => {
		impl_resource! {
			impl<$($lifetime),*> $Name as $Resource where ($self, $resources: $Manager) $body
		}
		impl_resource! {
			$(impl$(<$($next_lifetime),*>)? $NextName as $NextResource where ($next_self, $next_resources: $NextManager) $next_body)+
		}
	};
	(impl$(<$($lifetime:lifetime),*>)? $Name:ident as $Resource:ty where ($self:ident, $resources:ident: $Manager:ty) $body:tt ) => {
		#[derive(
			Clone, Debug, Eq, PartialEq,
			serde::Serialize, serde::Deserialize,
			rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
		)]
		pub struct $Name(Box<str>);

		impl $Name {
			pub fn new(id: &str) -> Self {
				id.into()
			}
		}

		impl<T: Into<Box<str>>> From<T> for $Name {
			fn from(s: T) -> Self {
				Self(s.into())
			}
		}

		impl AsRef<str> for $Name {
			fn as_ref(&self) -> &str {
				&*self.0
			}
		}

		impl$(<$($lifetime),*>)? Id<$Manager> for $Name {
			type Resource = $Resource;
			fn get<'resources>(
				&$self,
				$resources: &'resources $Manager,
			) -> Result<&'resources Self::Resource> {
				#[allow(unused_braces)]
				$body
			}
		}

		impl ::std::fmt::Display for $Name {
			fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
				f.write_str(&self.0)
			}
		}

		impl mlua::IntoLua for $Name {
			fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
				Ok(mlua::Value::String(lua.create_string(self.as_ref())?))
			}
		}

		impl mlua::FromLua for $Name {
			fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
				Ok(value
					.as_str()
					.ok_or_else(|| mlua::Error::runtime("expected string"))?
					.as_ref()
					.into())
			}
		}
	};
}

impl_resource! {
	impl Sheet as character::Sheet where (self, resources: resource::Manager) {
		resources.sheets.get(&self.0)
	}

	impl Status as status::Status where (self, resources: resource::Manager) {
		resources.statuses.get(&self.0)
	}

	impl Attack as attack::Attack where (self, resources: resource::Manager) {
		resources.attacks.get(&self.0)
	}

	impl Spell as spell::Spell where (self, resources: resource::Manager) {
		resources.spells.get(&self.0)
	}

	impl Vault as vault::Vault where (self, resources: resource::Manager) {
		resources.vaults.get(&self.0)
	}
}

#[derive(Debug)]
pub struct Resource<T: ?Sized>(HashMap<Box<str>, Rc<T>>);

impl<T> Resource<T> {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn get(&self, key: &str) -> Result<&T> {
		Ok(self
			.0
			.get(key)
			.ok_or_else(|| crate::Error::Resource(Error::NotFound(key.into())))?)
	}
}

impl<T> Default for Resource<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// Manages all resource loading in a central, abstracted structure.
///
/// The primary benefit of using this structure is that it abstracts
/// the path and extension used to load any given asset.
/// `resource::Manager` can also cache certain resources to avoid repeated disk reads,
/// meaning outside code doesn't need to store permanent references to resources.
#[derive(Debug)]
pub struct Manager {
	/// `Attack`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Rc`.
	pub attacks: Resource<attack::Attack>,
	/// `Spells`s need to be owned by many pieces, but rarely need to be mutated, so it's more convenient to provide an `Rc`.
	pub spells: Resource<spell::Spell>,
	/// Unlike `Attack`s and `Spell`s, `character::Sheet`s are likely to be modified.
	pub sheets: Resource<character::Sheet>,
	pub statuses: Resource<status::Status>,
	pub vaults: Resource<vault::Vault>,
}

#[derive(Debug, Clone, FromLua)]
pub struct Handle(Rc<Manager>);

impl Handle {
	pub fn new(resources: Rc<Manager>) -> Self {
		Self(resources)
	}
}

impl std::ops::Deref for Handle {
	type Target = Manager;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl mlua::UserData for Handle {
	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("status", |_lua, this, key: Box<str>| {
			this.statuses
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
		methods.add_method("attack", |_lua, this, key: Box<str>| {
			this.attacks
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
		methods.add_method("spell", |_lua, this, key: Box<str>| {
			this.spells
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
	}
}

// TODO: Remove this.
pub struct Scripts<'lua> {
	pub runtime: &'lua mlua::Lua,
	sandbox_metatable: mlua::Table,
}

pub fn register<T>(
	directory: &Path,
	mut loader: impl FnMut(&Path) -> Result<T>,
) -> Result<Resource<T>> {
	let mut container = Resource::new();
	recurse(directory, |path, reference| {
		loader(path).map(|x| {
			container.0.insert(reference.into(), x.into());
		})
	})?;
	Ok(container)
}

fn recurse(directory: &Path, mut loader: impl FnMut(&Path, &str) -> Result<()>) -> Result<()> {
	fn recurse(
		base_directory: &Path,
		directory: &Path,
		loader: &mut impl FnMut(&Path, &str) -> Result<()>,
	) -> Result<()> {
		let read_dir = fs::read_dir(directory)?;
		for entry in read_dir {
			let entry = entry?;
			let path = entry.path();
			if entry.metadata()?.is_dir() {
				recurse(base_directory, &path, loader)?;
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
				loader(&path, reference)?;
			}
		}
		Ok(())
	}

	recurse(directory, directory, &mut loader)
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

		let sheets = register(&path.join("sheets"), |path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let statuses = register(&path.join("statuses"), |path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let attacks = register(&path.join("attacks"), |path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let spells = register(&path.join("spells"), |path| {
			Ok(toml::from_str(&fs::read_to_string(path)?)?)
		})?;

		let vaults = register(&path.join("vaults"), |path| vault::Vault::open(path))?;

		Ok(Self {
			attacks,
			spells,
			sheets,
			statuses,
			vaults,
		})
	}

	pub fn get<T: Id<Self>>(&self, id: &T) -> Result<&T::Resource> {
		id.get(self)
	}
}

pub struct SandboxBuilder<'lua> {
	runtime: &'lua mlua::Lua,
	function: mlua::Function,
	environment: mlua::Table,
}

impl<'lua> SandboxBuilder<'lua> {
	pub fn insert(self, key: impl mlua::IntoLua, value: impl mlua::IntoLua) -> Result<Self> {
		self.environment.set(key, value)?;
		Ok(self)
	}

	pub fn thread(self) -> mlua::Result<mlua::Thread> {
		self.function.set_environment(self.environment)?;
		self.runtime.create_thread(self.function.clone())
	}

	pub fn call<R: FromLuaMulti>(self, args: impl mlua::IntoLuaMulti) -> mlua::Result<R> {
		self.function.set_environment(self.environment)?;
		self.function.call::<R>(args)
	}

	pub fn world<R: FromLua>(
		self,
		world: &world::Manager,
		args: impl mlua::IntoLuaMulti,
	) -> Result<R> {
		self.function.set_environment(self.environment)?;
		let thread = self.runtime.create_thread(self.function.clone())?;
		Ok(world.poll(self.runtime, thread, args)?)
	}
}

impl<'lua> Scripts<'lua> {
	pub fn open(path: impl AsRef<Path>, lua: &'lua mlua::Lua) -> Result<Scripts<'lua>> {
		let scripts = lua.create_table()?;
		recurse(path.as_ref(), |path, reference| {
			scripts.set(
				reference,
				lua.load(&fs::read_to_string(path)?)
					.set_name(reference)
					.into_function()?,
			)?;
			Ok(())
		})?;
		lua.globals().set("Scripts", scripts)?;
		Ok(Self {
			runtime: lua,
			sandbox_metatable: lua.create_table_from([("__index", lua.globals())])?,
		})
	}

	pub fn function(&self, key: &str) -> Result<mlua::Function> {
		Ok(self
			.runtime
			.globals()
			.get::<mlua::Table>("Scripts")?
			.get(key)?)
	}

	pub fn sandbox(&self, key: &str) -> Result<SandboxBuilder<'lua>> {
		let environment = self.runtime.create_table()?;
		// This is cloning a reference, which is a lot cheaper than creating a new table.
		environment.set_metatable(Some(self.sandbox_metatable.clone()));
		Ok(SandboxBuilder {
			runtime: self.runtime,
			function: self.function(key)?,
			environment,
		})
	}
}
