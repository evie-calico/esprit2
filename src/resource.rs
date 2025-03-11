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
use mlua::{ErrorContext, FromLua};
use std::collections::HashMap;
use std::ffi::OsStr;
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

#[derive(Debug)]
pub struct Resource<T>(HashMap<Box<str>, T>);

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

/// Manages all resource loading in a central, abstracted structure.
///
/// The primary benefit of using this structure is that it abstracts
/// the path and extension used to load any given asset.
/// `resource::Manager` can also cache certain resources to avoid repeated disk reads,
/// meaning outside code doesn't need to store permanent references to resources.
#[derive(Debug, Default)]
pub struct Manager {
	pub attack: Resource<Rc<attack::Attack>>,
	pub component: Resource<Rc<component::Component>>,
	pub sheet: Resource<Rc<character::Sheet>>,
	pub spell: Resource<Rc<spell::Spell>>,
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
		methods.add_method("spell", |_lua, this, key: Box<str>| {
			this.spell.get(&key).cloned().map_err(mlua::Error::external)
		});
		methods.add_method("component", |_lua, this, key: Box<str>| {
			this.component
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
	}
}

fn attack(table: mlua::Table) -> mlua::Result<attack::Attack> {
	Ok(attack::Attack {
		name: table.get("name")?,
		description: table.get("description")?,
		magnitude: table.get("magnitude")?,
		on_input: table.get("on_input")?,
		on_use: table.get("on_use")?,
		on_consider: table.get("on_consider")?,
		use_time: table.get("use_time")?,
	})
}

fn sheet(table: mlua::Table) -> mlua::Result<character::Sheet> {
	let stats = |table: mlua::Table| -> mlua::Result<_> {
		Ok(character::Stats {
			heart: table.get("heart")?,
			soul: table.get("soul")?,
			power: table.get("power")?,
			defense: table.get("defense")?,
			magic: table.get("magic")?,
			resistance: table.get("resistance")?,
		})
	};

	Ok(character::Sheet {
		icon: table.get("icon")?,
		nouns: table.get("nouns")?,
		level: table.get("level")?,
		experience: table.get::<Option<_>>("experience")?.unwrap_or_default(),
		bases: stats(table.get("bases")?)?,
		growths: stats(table.get("growths")?)?,
		growth_bonuses: table
			.get::<Option<_>>("growth_bonuses")?
			.unwrap_or_default(),
		skillset: table.get("skillset")?,
		speed: table.get::<Option<_>>("speed")?.unwrap_or(TURN),
		attacks: table.get::<Option<_>>("attacks")?.unwrap_or_default(),
		spells: table.get::<Option<_>>("spells")?.unwrap_or_default(),
		on_consider: table.get("on_consider")?,
	})
}

fn spell(table: mlua::Table) -> mlua::Result<spell::Spell> {
	Ok(spell::Spell {
		name: table.get("name")?,
		icon: table.get("icon")?,
		description: table.get("description")?,
		on_input: table.get("on_input")?,
		on_cast: table.get("on_cast")?,
		on_consider: table.get("on_consider")?,
		parameters: table.get("parameters")?,
		energy: table.get("energy")?,
		harmony: table.get("harmony")?,
		level: table.get("level")?,
	})
}

fn component(table: mlua::Table) -> mlua::Result<component::Component> {
	Ok(component::Component {
		name: table.get("name")?,
		icon: table.get("icon")?,
		visible: table.get::<Option<bool>>("visible")?.unwrap_or_default(),
		on_attach: table.get("on_attach")?,
		on_detach: table.get("on_detach")?,
		on_turn: table.get("on_turn")?,
		on_rest: table.get("on_rest")?,
		on_debuff: table.get("on_debuff")?,
	})
}

fn vault(table: mlua::Table) -> mlua::Result<vault::Vault> {
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
	vault::Vault::parse(source, symbols.iter()).map_err(mlua::Error::external)
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
			let chunk = fs::read_to_string(&directory)
				.map_err(mlua::Error::external)
				.with_context(|_| format!("while loading {}", directory.display()))?;
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
fn init<Load: Fn(&str, &Path, &dyn Fn() -> Result<()>) -> Result<()>>(
	lua: &mlua::Lua,
	name: &str,
	directory: impl AsRef<Path>,
	load: Load,
) -> Result<mlua::Table> {
	fn recurse(directory: &Path, lua: &mlua::Lua) -> Result<()> {
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

	let resources = lua.load_from_function::<mlua::Table>(
		"init.resources",
		lua.create_function(|lua, ()| {
			lua.create_table_from([
				("attack", lua.create_table()?),
				("component", lua.create_table()?),
				("sheet", lua.create_table()?),
				("spell", lua.create_table()?),
				("vault", lua.create_table()?),
			])
		})?,
	)?;
	let init = || recurse(&directory.join("init/"), lua);
	let result = load(name, directory, &init);
	lua.unload("init.resources")?;
	result?; // defer errors to hopefully unload init.resources?

	Ok(resources)
}

struct PreliminaryModule<'a> {
	name: &'a str,
	path: &'a Path,
	prototypes: Result<(mlua::Table, Manager), Vec<crate::Error>>,
}

fn produce(prototypes: &mlua::Table) -> Result<Manager, Vec<crate::Error>> {
	let mut products = Ok(Manager::default());
	let append_err = |errors: Result<Manager, Vec<crate::Error>>, e| {
		Err(match errors {
			Ok(_) => vec![crate::Error::Lua(e)],
			Err(mut errors) => {
				errors.push(crate::Error::Lua(e));
				errors
			}
		})
	};
	macro_rules! produce {
			($type:ident) => {
				match prototypes.get::<mlua::Table>(stringify!($type)) {
					Ok(table) => for i in table.pairs::<mlua::String, mlua::Table>() {
						match i.and_then(|(id, table)| Ok((Box::from(id.to_str()?.as_ref()), $type(table)?))) {
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
	produce!(attack, sheet, spell, component, vault);

	products
}

pub struct FailedModule<'a> {
	pub name: &'a str,
	pub errors: Box<[crate::Error]>,
}

pub fn open<'a, Load: Fn(&str, &Path, &dyn Fn() -> Result<()>) -> Result<()>>(
	lua: &mlua::Lua,
	modules: impl IntoIterator<Item = &'a Path>,
	load: Load,
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
			on_rest: None,
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
	mlua::Result::<()>::expect(
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
		module.prototypes = init(lua, module.name, module.path, &load)
			.map(|table| produce(&table).map(|x| (table, x)))
			.unwrap_or_else(|e| Err(vec![e]));
	}

	// TODO: dependency directory.

	let errors = preliminary_modules
		.into_iter()
		.filter_map(|preliminary_module| match preliminary_module {
			PreliminaryModule {
				name,
				path: _,
				prototypes: Ok((_, prototypes)),
			} => {
				macro_rules! combine {
						($type:ident) => {
							for (id, value) in prototypes.$type.0.into_iter() {
								manager.$type.0.insert(format!("{name}:{id}").into_boxed_str(), value);
							}
						};
						($($type:ident),+) => {
							$( combine!($type); )+
						}
					}
				combine!(attack, sheet, spell, component, vault);
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
