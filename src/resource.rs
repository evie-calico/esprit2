use crate::prelude::*;
use mlua::FromLua;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

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
#[derive(Debug)]
pub struct Manager {
	pub attacks: Resource<Rc<attack::Attack>>,
	pub functions: Resource<mlua::Function>,
	/// Unlike `Attack`s and `Spell`s, `character::Sheet`s are likely to be modified.
	pub sheets: Resource<Rc<character::Sheet>>,
	pub spells: Resource<Rc<spell::Spell>>,
	pub components: Resource<Rc<component::Component>>,
	pub vaults: Resource<Rc<vault::Vault>>,
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
			this.attacks
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
		methods.add_method("function", |_lua, this, key: Box<str>| {
			this.functions
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
		methods.add_method("component", |_lua, this, key: Box<str>| {
			this.components
				.get(&key)
				.cloned()
				.map_err(mlua::Error::external)
		});
	}
}

pub fn register<T>(
	directory: &Path,
	mut loader: impl FnMut(&Path) -> Result<T>,
) -> Result<Resource<T>> {
	let mut container = Resource::new();
	recurse(directory, |path, reference| {
		loader(path).map(|x| {
			container.0.insert(reference.into(), x);
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

fn make_registrar<T: 'static, IntoT: Into<T>>(
	handle: Weak<RefCell<Resource<T>>>,
	constructor: impl Fn(mlua::Table) -> mlua::Result<IntoT> + Clone + 'static,
) -> impl Fn(&mlua::Lua, Box<str>) -> mlua::Result<mlua::Function> {
	move |lua, identifier| {
		let handle = handle.clone();
		let identifier = Cell::new(Some(identifier));
		let constructor = constructor.clone();
		lua.create_function(move |_, table: mlua::Table| {
			let Some(handle) = handle.upgrade() else {
				return Err(mlua::Error::runtime("resource registration has closed"));
			};
			let Some(identifier) = identifier.take() else {
				return Err(mlua::Error::runtime(
					"resources registration functions may only be used once",
				));
			};

			handle
				.borrow_mut()
				.0
				.insert(identifier, constructor(table)?.into());
			Ok(())
		})
	}
}

impl Manager {
	/// Collect known resources into a new resource manager.
	///
	/// # Errors
	///
	/// Returns an error if ANYTHING fails to be read/parsed.
	/// This is probably undesirable and should be moved to logging/diagnostics.
	pub fn open(path: impl AsRef<Path>, lua: &mlua::Lua) -> Result<Manager> {
		let path = path.as_ref();

		let attacks = Rc::new(RefCell::new(Resource::<Rc<attack::Attack>>::default()));
		let attack_registrar =
			lua.create_function(make_registrar(Rc::downgrade(&attacks), |table| {
				Ok(attack::Attack {
					name: table.get("name")?,
					description: table.get("description")?,
					magnitude: table.get("magnitude")?,
					on_input: table.get("on_input")?,
					on_use: table.get("on_use")?,
					on_consider: table.get("on_consider")?,
					use_time: table.get("use_time")?,
				})
			}))?;
		lua.load_from_function::<mlua::Value>(
			"esprit.resources.attack",
			lua.create_function(move |_, ()| Ok(attack_registrar.clone()))?,
		)?;

		let functions = Rc::new(RefCell::new(Resource::<mlua::Function>::default()));
		let functions_handle = Rc::downgrade(&functions);
		let function_registrar =
			lua.create_function(move |_, (id, func): (Box<str>, mlua::Function)| {
				let Some(handle) = functions_handle.upgrade() else {
					return Err(mlua::Error::runtime("resource registration has closed"));
				};
				handle.borrow_mut().0.insert(id, func);
				Ok(())
			})?;
		lua.load_from_function::<mlua::Value>(
			"esprit.resources.function",
			lua.create_function(move |_, ()| Ok(function_registrar.clone()))?,
		)?;

		let sheets = Rc::new(RefCell::new(Resource::<Rc<character::Sheet>>::default()));
		let sheet_registrar =
			lua.create_function(make_registrar(Rc::downgrade(&sheets), |table| {
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
			}))?;
		lua.load_from_function::<mlua::Value>(
			"esprit.resources.sheet",
			lua.create_function(move |_, ()| Ok(sheet_registrar.clone()))?,
		)?;

		let spells = Rc::new(RefCell::new(Resource::<Rc<spell::Spell>>::default()));
		let spell_registrar =
			lua.create_function(make_registrar(Rc::downgrade(&spells), |table| {
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
			}))?;
		lua.load_from_function::<mlua::Value>(
			"esprit.resources.spell",
			lua.create_function(move |_, ()| Ok(spell_registrar.clone()))?,
		)?;

		let components = Rc::new(RefCell::new(Resource::<Rc<component::Component>>::default()));
		let component_registrar =
			lua.create_function(make_registrar(Rc::downgrade(&components), |table| {
				Ok(component::Component {
					name: table.get("name")?,
					icon: table.get("icon")?,
					duration: table
						.get::<Option<component::Duration>>("duration")?
						.unwrap_or_default(),
					on_debuff: table.get("on_debuff")?,
				})
			}))?;
		lua.load_from_function::<mlua::Value>(
			"esprit.resources.component",
			lua.create_function(move |_, ()| Ok(component_registrar.clone()))?,
		)?;

		let vaults = Rc::new(RefCell::new(Resource::<Rc<vault::Vault>>::default()));
		let vault_registrar = make_registrar(Rc::downgrade(&vaults), |table| {
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
		});
		let vault = lua.create_table()?;
		let vault_meta = lua.create_table()?;
		vault_meta.set(
			"__call",
			lua.create_function(move |lua, (_, id): (mlua::Table, _)| vault_registrar(lua, id))?,
		)?;
		vault.set_metatable(Some(vault_meta));
		vault.set(
			"character",
			lua.create_function(|_, (sheet, tile): (Box<str>, Option<floor::Tile>)| {
				Ok(vault::SymbolMeaning::Character {
					sheet,
					tile: tile.unwrap_or(floor::Tile::Floor),
				})
			})?,
		)?;
		lua.load_from_function::<mlua::Value>(
			"esprit.resources.vault",
			lua.create_function(move |_, ()| Ok(vault.clone()))?,
		)?;

		lua.globals()
			.get::<mlua::Table>("package")?
			.set("path", path.join("lib/?.lua"))?;

		recurse(&path.join("init"), |path, _| {
			lua.load(&fs::read_to_string(path)?)
				.set_name(path.to_string_lossy())
				.exec()?;
			Ok(())
		})?;

		lua.globals()
			.get::<mlua::Table>("package")?
			.set("path", "")?;

		lua.unload("esprit.resources.attack")?;
		lua.unload("esprit.resources.function")?;
		lua.unload("esprit.resources.sheet")?;
		lua.unload("esprit.resources.spells")?;
		lua.unload("esprit.resources.component")?;
		lua.unload("esprit.resources.vault")?;

		let attacks = Rc::into_inner(attacks)
			.expect("attacks must have only one strong reference")
			.into_inner();
		let functions = Rc::into_inner(functions)
			.expect("functions must have only one strong reference")
			.into_inner();
		let sheets = Rc::into_inner(sheets)
			.expect("sheets must have only one strong reference")
			.into_inner();
		let spells = Rc::into_inner(spells)
			.expect("spells must have only one strong reference")
			.into_inner();
		let components = Rc::into_inner(components)
			.expect("components must have only one strong reference")
			.into_inner();
		let vaults = Rc::into_inner(vaults)
			.expect("vaults must have only one strong reference")
			.into_inner();

		Ok(Self {
			attacks,
			functions,
			sheets,
			spells,
			components,
			vaults,
		})
	}
}
