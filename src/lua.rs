use crate::prelude::*;
use consider::Heuristic;
use mlua::Function as F;
use mlua::{chunk, AsChunk, Either, Error, Lua, Result};
use paste::paste;

macro_rules! make_lua_enum{
    { $Type:path: $($variant:ident,)+ | $last:ident} => {
        impl mlua::FromLua for $Type {
			fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
				match value {
					mlua::Value::String(s) => match s.to_str()?.as_ref() {
						$( stringify!($variant) => Ok(paste!(Self::[<$variant:camel>])), )+
						stringify!($last) => Ok(paste!(Self::[<$last:camel>])),
						s => Err(mlua::Error::runtime(format!(
							concat!(
								"unexpected string: {}, expected ",
								$( "\"", stringify!($variant), "\"|", )+
								"\"", stringify!($last), "\"",
							),
							s,
						))),
					},
					mlua::Value::UserData(any) => Ok(*any.borrow()?),
					_ => Err(mlua::Error::runtime(
						format!(
							concat!(
								"unexpected type: {}, expected strings",
								$( "\"", stringify!($variant), "\"|", )+
								"\"", stringify!($last), "\", or the `Energy` userdata type",
							),
							value.type_name(),
						)
					)),
				}
			}
        }

		impl mlua::UserData for $Type {
			fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
				$( methods.add_method(stringify!($variant), |_, this, ()| Ok(*this == paste!(Self::[<$variant:camel>]))); )+
				methods.add_method(stringify!($last), |_, this, ()| Ok(*this == paste!(Self::[<$last:camel>])));
			}
		}
    };
}

make_lua_enum! { spell::Energy: positive, | negative }
make_lua_enum! { spell::Harmony: chaos, | order }
make_lua_enum! { nouns::Pronouns: female, male, neutral, | object }

impl mlua::FromLua for Nouns {
	fn from_lua(value: mlua::Value, _: &Lua) -> Result<Self> {
		let Some(table) = value.as_table() else {
			return Err(Error::runtime(format!(
				"expected table, got {}",
				value.type_name()
			)));
		};
		Ok(Nouns {
			name: table.get::<mlua::String>("name")?.to_str()?.as_ref().into(),
			proper_name: table.get("proper_name")?,
			pronouns: table.get("pronouns")?,
		})
	}
}

impl mlua::UserData for Nouns {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
	}
}

pub fn init() -> Result<Lua> {
	let lua = Lua::new();
	// Libraries
	lua.load_from_function::<mlua::Value>("engine.combat", lua.create_function(combat)?)?;
	lua.load_from_function::<mlua::Value>("engine.world", lua.load(world()).into_function()?)?;

	// Constructors
	lua.load_from_function::<mlua::Value>("engine.types.action", lua.create_function(action)?)?;
	lua.load_from_function::<mlua::Value>(
		"engine.types.consider",
		lua.create_function(move |lua, ()| {
			lua.create_function(|_, (action, heuristics)| Ok(Consider { action, heuristics }))
		})?,
	)?;
	lua.load_from_function::<mlua::Value>(
		"engine.types.heuristic",
		lua.create_function(heuristic)?,
	)?;
	lua.load_from_function::<mlua::Value>("engine.types.log", lua.create_function(log)?)?;
	lua.load_from_function::<mlua::Value>("engine.types.skillset", lua.create_function(skillset)?)?;
	lua.load_from_function::<mlua::Value>("engine.types.stats", lua.create_function(stats)?)?;
	Ok(lua)
}

fn combat(lua: &Lua, _: ()) -> Result<mlua::Table> {
	let combat = lua.create_table()?;
	combat.set(
		"format",
		lua.create_function(
			|_, (user, target, s): (character::Ref, character::Ref, Box<str>)| {
				Ok(
					s.replace_prefixed_nouns(&target.borrow().sheet.nouns, "target_")
						.replace_prefixed_nouns(&user.borrow().sheet.nouns, "self_"),
				)
			},
		)?,
	)?;
	combat.set(
		"apply_pierce",
		lua.create_function(|_, (pierce, magnitude)| {
			if magnitude > 0 && magnitude <= pierce {
				Ok((0, true))
			} else {
				Ok((magnitude, false))
			}
		})?,
	)?;
	Ok(combat)
}

/// Implemented via lua to allow for yields.
fn world() -> impl AsChunk<'static> {
	let make_characters = F::wrap(|| Ok(world::LuaRequest::Characters { query: None }));
	let make_characters_within = F::wrap(|x, y, range| {
		Ok(world::LuaRequest::Characters {
			query: Some(world::LuaCharacterQuery::Within { x, y, range }),
		})
	});
	let make_tile = F::wrap(|x, y| Ok(world::LuaRequest::Tile { x, y }));
	chunk! {
		local world = {}

		function world.characters()
			return coroutine.yield($make_characters())
		end

		function world.character_at(x, y)
			return world.characters_within(x, y, 0)[1]
		end

		function world.characters_within(x, y, range)
			return coroutine.yield($make_characters_within(x, y, range))
		end

		function world.tile(x, y)
			return coroutine.yield($make_tile(x, y))
		end

		return world
	}
}

fn action(lua: &Lua, _: ()) -> Result<mlua::Table> {
	let action = lua.create_table()?;
	action.set("wait", F::wrap(|time| Ok(character::Action::Wait(time))))?;
	action.set("move", F::wrap(|x, y| Ok(character::Action::Move(x, y))))?;
	action.set(
		"attack",
		F::wrap(|attack, args| Ok(character::Action::Attack(attack, args))),
	)?;
	action.set(
		"cast",
		F::wrap(|spell, args| Ok(character::Action::Cast(spell, args))),
	)?;
	Ok(action)
}

fn heuristic(lua: &Lua, _: ()) -> Result<mlua::Table> {
	fn saturating_cast(x: mlua::Integer) -> u32 {
		x.max(u32::MIN as mlua::Integer)
			.min(u32::MAX as mlua::Integer) as u32
	}

	let heuristic = lua.create_table()?;
	heuristic.set(
		"damage",
		F::wrap(|target, amount| {
			Ok(Heuristic::Damage {
				target,
				amount: saturating_cast(amount),
			})
		}),
	)?;
	heuristic.set(
		"debuff",
		F::wrap(|target, amount| {
			Ok(Heuristic::Debuff {
				target,
				amount: saturating_cast(amount),
			})
		}),
	)?;
	heuristic.set("move", F::wrap(|x, y| Ok(Heuristic::Move { x, y })))?;
	Ok(heuristic)
}

fn log(lua: &Lua, _: ()) -> Result<mlua::Table> {
	let log = lua.create_table()?;
	log.set("Success", combat::Log::Success)?;
	log.set("Miss", combat::Log::Miss)?;
	log.set("Glance", combat::Log::Glance)?;
	log.set("Hit", F::wrap(|damage| Ok(combat::Log::Hit { damage })))?;
	Ok(log)
}

type SkillsetArguments = (
	mlua::Table,
	Either<spell::Energy, spell::Harmony>,
	Either<Option<spell::Energy>, Option<spell::Harmony>>,
);

fn skillset(lua: &Lua, _: ()) -> Result<mlua::Table> {
	let skillset = lua.create_table()?;
	skillset.set("chaos", spell::Harmony::Chaos)?;
	skillset.set("order", spell::Harmony::Order)?;
	skillset.set("positive", spell::Energy::Positive)?;
	skillset.set("negative", spell::Energy::Negative)?;
	let skillset_meta = lua.create_table()?;
	skillset_meta.set(
		"__call",
		lua.create_function(|_, (_this, major, minor): SkillsetArguments| {
			Ok(match (major, minor) {
				(Either::Left(energy), Either::Right(harmony)) => spell::Skillset::EnergyMajor {
					major: energy,
					minor: harmony,
				},
				(Either::Right(harmony), Either::Left(energy)) => spell::Skillset::HarmonyMajor {
					major: harmony,
					minor: energy,
				},
				_ => {
					return Err(Error::runtime(
						"skillset arguments must not be of the same axis",
					))
				}
			})
		})?,
	)?;
	skillset.set_metatable(Some(skillset_meta));
	Ok(skillset)
}

fn stats(lua: &Lua, _: ()) -> Result<mlua::Table> {
	use character::Stats;

	let stats_meta = lua.create_table()?;
	let stats = lua.create_table()?;

	macro_rules! single {
		($stat:ident) => {
			stats.set(
				stringify!($stat),
				F::wrap(|$stat|
					Ok(Stats {
						$stat,
						..Default::default()
					})
				),
			)?;
		};
		($stat:ident, $($next:ident),*) => {
			single!($stat);
			single!($($next),*);
		}
	}

	macro_rules! constructor {
		($($stats:ident),*) => {
			stats_meta.set(
				"__call",
				lua.create_function(|_, (_this, table): (mlua::Table, mlua::Table)| {
					$(let mut $stats = 0;)*

					for i in table.pairs::<mlua::String, u16>() {
						let (k, v) = i?;
						match k.to_str()?.as_ref() {
							$( stringify!($stats) => $stats = v,)*
							k => {
								return Err(mlua::Error::runtime(format!(
									"unexpected key name: {k}"
								)))
							}
						}
					}
					Ok(Stats { $($stats),* })
				})?,
			)?;
			stats.set_metatable(Some(stats_meta));
			single!($($stats),*);
		};
	}

	constructor!(heart, soul, power, defense, magic, resistance);

	Ok(stats)
}
