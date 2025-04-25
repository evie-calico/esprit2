//! Long-term resource loading.
//!
//! Esprit modules contain scripts which construct the game's
//! resources at initialization and define runtime behavior via lua callbacks.
//! Resources are tied to a lua runtime, so any lua objects created at this
//! stage will be available for the rest of the game.
//!
//! A module is laid out as follows:
//!
//! ```
//! <module name>/
//! ├── init/
//! │   └── **.lua
//! ├── lib/
//! │   └── **.lua
//! └── textures/
//!     ├── **.png
//!     └── **.png
//! ```
//!
//! All lua files underneath the `init/` directory (as represented by `**.lua`)
//! are executed at the initialization phase, and can access the `init.*` lua modules
//! for constructing resources.
//!
//! The `lib` directory is registered with `require` during the initialization phase,
//! with `<module name>/lib/` replaced by `<module name>:`.
//!
//! For example, `foo/lib/bar/baz.lua` becomes `require "foo:bar/baz"`.
//!
//! There is no way to access other modules' `lib/` directories during initialization.
//! After the intialization phase, all modules' `lib/` directories will permanently be accessible via `require`.
//! This means that—like the `runtime.*` modules—they may only be `require`d from callback functions.
//!
//! ## Auxilary init modules
//!
//! Users of this crate may want to define additional information via `init/`
//! scripts which is not recognized by [`Manager`]—such as textures or sound effects.
//! This can be accomplished via [`open`]'s `load` parameter;
//! this function will be called in place of initializing the module,
//! and recieves the module name, directory, and an "init" function
//! which executes all lua scripts in `init/`.
//! A reference to the lua state may be captured by this closure,
//! allowing custom modules to be loaded and unloaded around the "init" call.

use crate::prelude::*;
use anyhow::Context;
use mlua::FromLua;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("resource {0} not found")]
	NotFound(Box<str>),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Resource<T>(HashMap<Box<str>, T>);

impl<T> Resource<T> {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn get(&self, key: &str) -> Result<&T, Error> {
		self.0.get(key).ok_or_else(|| Error::NotFound(key.into()))
	}

	pub fn get_key_value<'a>(&'a self, key: &str) -> Result<(&'a str, &'a T), Error> {
		self.0
			.get_key_value(key)
			.map(|(key, value)| (&**key, value))
			.ok_or_else(|| Error::NotFound(key.into()))
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
#[derive(Debug, Default)]
pub struct Manager {
	pub ability: Resource<Rc<ability::Ability>>,
	pub attack: Resource<Rc<attack::Attack>>,
	pub component: Resource<Rc<component::Component>>,
	pub sheet: Resource<Rc<character::Sheet>>,
	pub vault: Resource<Rc<vault::Vault>>,
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
		methods.add_method("attack", |_lua, this, key: Box<str>| {
			this.attack
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
		methods.add_method("ability", |_lua, this, key: Box<str>| {
			this.ability
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
		methods.add_method("component", |_lua, this, key: Box<str>| {
			this.component
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
	}
}

macro_rules! get {
	($table:ident.$field:ident) => {
		$table
			.get(stringify!($field))
			.context(concat!("failed to retrieve ", stringify!($field)))
	};
}

fn attack(_id: &str, table: mlua::Table) -> anyhow::Result<attack::Attack> {
	Ok(attack::Attack {
		name: get!(table.name)?,
		description: get!(table.description)?,
		on_input: get!(table.on_input)?,
		on_use: get!(table.on_use)?,
		on_consider: get!(table.on_consider)?,
	})
}

fn sheet(id: &str, table: mlua::Table) -> anyhow::Result<character::Sheet> {
	let stats = |table: mlua::Table| -> anyhow::Result<_> {
		Ok(character::Stats {
			heart: get!(table.heart)?,
			soul: get!(table.soul)?,
			power: get!(table.power)?,
			defense: get!(table.defense)?,
			magic: get!(table.magic)?,
			resistance: get!(table.resistance)?,
		})
	};

	Ok(character::Sheet {
		id: id.into(),
		nouns: get!(table.nouns)?,
		stats: stats(get!(table.stats)?)?,
		attacks: table.get::<Option<_>>("attacks")?.unwrap_or_default(),
		abilities: table.get::<Option<_>>("abilities")?.unwrap_or_default(),
		on_consider: get!(table.on_consider)?,
	})
}

fn ability(_id: &str, table: mlua::Table) -> anyhow::Result<ability::Ability> {
	Ok(ability::Ability {
		name: get!(table.name)?,
		usage: get!(table.usage)?,
		description: get!(table.description)?,
		usable: get!(table.castable)?,
		on_input: get!(table.on_input)?,
		on_use: get!(table.on_cast)?,
		on_consider: get!(table.on_consider)?,
	})
}

fn component(_id: &str, table: mlua::Table) -> anyhow::Result<component::Component> {
	Ok(component::Component {
		name: get!(table.name)?,
		icon: get!(table.icon)?,
		visible: table.get::<Option<bool>>("visible")?.unwrap_or_default(),
		on_attach: get!(table.on_attach)?,
		on_detach: get!(table.on_detach)?,
		on_turn: get!(table.on_turn)?,
		on_debuff: get!(table.on_debuff)?,
	})
}

fn vault(_id: &str, table: mlua::Table) -> anyhow::Result<vault::Vault> {
	let source = table.get::<mlua::String>(1)?;
	let source = source.to_str()?;
	let source = source.as_ref();
	table.set(1, mlua::Nil)?;
	let symbols: Box<[(char, vault::SymbolMeaning)]> = table
		.pairs::<mlua::String, mlua::Either<vault::SymbolMeaning, floor::Tile>>()
		.map(|x| {
			x.and_then(|(k, v)| {
				let k = k.to_str()?;
				let mut chars = k.as_ref().chars();
				if let Some(c) = chars.next()
					&& chars.next().is_none()
				{
					Ok((c, v.left_or_else(vault::SymbolMeaning::Tile)))
				} else {
					Err(mlua::Error::runtime(
						"expected a string containing a single character",
					))
				}
			})
		})
		.collect::<mlua::Result<Box<[(char, vault::SymbolMeaning)]>>>()?;
	Ok(vault::Vault::parse(source, symbols.iter())?)
}

fn lib_searcher(
	lua: &mlua::Lua,
	module: String,
	directory: PathBuf,
) -> mlua::Result<mlua::Function> {
	lua.create_function(move |lua, path: mlua::String| {
		let path = path.to_str()?;
		if let Some((path_module, path)) = path.as_ref().split_once(':')
			&& module == path_module
		{
			let mut directory = directory.join("lib");
			directory.push(path);
			directory.set_extension("lua");
			let chunk = mlua::ErrorContext::with_context(
				fs::read_to_string(&directory).map_err(mlua::Error::external),
				|_| format!("while loading {}", directory.display()),
			)?;
			Ok(mlua::Value::Function(
				lua.load(chunk)
					.set_name(format!("@{}", directory.display()))
					.into_function()?,
			))
		} else {
			Ok(mlua::Value::String(
				lua.create_string(format!("not a member of {module}"))?,
			))
		}
	})
}

/// Organizes initialization scripts' resources.
fn init<Load: FnMut(&str, &Path, &mut dyn FnMut() -> anyhow::Result<()>) -> anyhow::Result<()>>(
	lua: &mlua::Lua,
	name: &str,
	directory: impl AsRef<Path>,
	mut load: Load,
) -> anyhow::Result<mlua::Table> {
	fn recurse(directory: &Path, lua: &mlua::Lua) -> anyhow::Result<()> {
		for entry in fs::read_dir(directory)? {
			let entry = entry?;
			let path = entry.path();
			if entry.metadata()?.is_dir() {
				recurse(&path, lua)?;
			} else {
				lua.load(&fs::read_to_string(&path)?)
					.set_name(format!("@{}", path.display()))
					.exec()?;
			}
		}

		Ok(())
	}

	let directory = directory.as_ref();

	let lua_name = mlua::Value::String(lua.create_string(name)?);
	let lua_directory = directory
		.to_str()
		.map(|x| lua.create_string(x))
		.transpose()?
		.map_or(mlua::Value::Nil, mlua::Value::String);
	let resources = lua.load_from_function::<mlua::Table>(
		"init.resources",
		lua.create_function(move |lua, ()| {
			lua.create_table_from([
				("ability", lua.create_table()?),
				("attack", lua.create_table()?),
				("component", lua.create_table()?),
				("sheet", lua.create_table()?),
				("vault", lua.create_table()?),
				(
					"module",
					lua.create_table_from([
						(lua.create_string("name")?, lua_name.clone()),
						(lua.create_string("path")?, lua_directory.clone()),
					])?,
				),
			])
		})?,
	)?;
	let init = directory.join("init/");
	let mut init = || recurse(&init, lua);
	let result = load(name, directory, &mut init);
	lua.unload("init.resources")?;
	result?; // defer errors to hopefully unload init.resources?

	Ok(resources)
}

struct PreliminaryModule<'a> {
	name: &'a str,
	path: &'a Path,
	prototypes: Result<(mlua::Table, Manager), Vec<anyhow::Error>>,
}

fn produce(name: &str, prototypes: &mlua::Table) -> Result<Manager, Vec<anyhow::Error>> {
	let mut products = Ok(Manager::default());
	let append_err = |errors: Result<Manager, Vec<anyhow::Error>>, e: anyhow::Error| {
		Err(match errors {
			Ok(_) => vec![e],
			Err(mut errors) => {
				errors.push(e);
				errors
			}
		})
	};
	macro_rules! produce {
			($type:ident) => {
				match prototypes.get::<mlua::Table>(stringify!($type)).context(concat!("missing ", stringify!($type), " prototypes")) {
					Ok(table) => for i in table.pairs::<mlua::String, mlua::Table>() {
						match i.context("failed to read resource prototype").and_then(|(id, table)| {
							let id = format!("{name}:{}", id.to_str()?.as_ref()).into_boxed_str();
							let resource = $type(&id, table).with_context(|| format!("failed to produce {} \"{id}\"", stringify!($type)))?;
							Ok((id, resource))
						}) {
							Ok((id, product)) => {
								if let Ok(products) = &mut products {
									products.$type.0.insert(id, product.into());
								}
							}
							Err(e) => products = append_err(products, e)
						}
					}
					Err(e) => products = append_err(products, e)
				}
			};
			($($type:ident),+) => {
				$( produce!($type); )+
			}
		}
	produce!(ability, attack, sheet, component, vault);

	products
}

pub struct FailedModule<'a> {
	pub name: &'a str,
	pub errors: Box<[anyhow::Error]>,
}

pub fn open<
	'a,
	Load: FnMut(&str, &Path, &mut dyn FnMut() -> anyhow::Result<()>) -> anyhow::Result<()>,
>(
	lua: &mlua::Lua,
	modules: impl IntoIterator<Item = &'a Path>,
	mut load: Load,
) -> (Manager, Vec<FailedModule<'a>>) {
	let mut manager = Manager::default();
	// Inject hard-coded engine resources here.
	manager.component.0.insert(
		":conscious".into(),
		Component {
			name: "Conscious".into(),
			icon: None,
			visible: false,
			on_attach: None,
			on_detach: None,
			on_turn: None,
			on_debuff: None,
		}
		.into(),
	);

	let mut preliminary_modules = modules
		.into_iter()
		.filter_map(|path| {
			path.file_name().and_then(OsStr::to_str).map(|name| {
				PreliminaryModule {
					name,
					path,
					// This value should go unused until being replaced after libraries are loaded.
					prototypes: Err(Vec::new()),
				}
			})
		})
		.collect::<Vec<PreliminaryModule>>();

	// Register modules with lua's `require` function.
	anyhow::Result::<()>::expect(
		try {
			let package = lua.globals().get::<mlua::Table>("package")?;
			let loaders = lua.create_sequence_from(
				preliminary_modules
					.iter()
					.filter_map(|x| lib_searcher(lua, x.name.into(), x.path.into()).ok()),
			)?;
			package.set("loaders", loaders)?;
		},
		"package loaders must not fail to load",
	);

	// Fill out dummy prototype fields.
	for module in &mut preliminary_modules {
		module.prototypes = init(lua, module.name, module.path, &mut load)
			.map(|table| produce(module.name, &table).map(|x| (table, x)))
			.unwrap_or_else(|e| Err(vec![e]));
	}

	// TODO: dependency directory.

	let errors = preliminary_modules
		.into_iter()
		.filter_map(|preliminary_module| match preliminary_module {
			PreliminaryModule {
				name: _,
				path: _,
				prototypes: Ok((_, prototypes)),
			} => {
				macro_rules! combine{
						($type:ident) => {
							for (id, value) in prototypes.$type.0.into_iter() {
								manager.$type.0.insert(id, value);
							}
						};
						($($type:ident),+) => {
							$( combine!($type); )+
						}
					}
				combine!(ability, attack, sheet, component, vault);
				None
			}
			PreliminaryModule {
				name,
				path: _,
				prototypes: Err(errors),
			} => Some(FailedModule {
				name,
				errors: errors.into(),
			}),
		})
		.collect();
	(manager, errors)
}
