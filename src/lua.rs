use crate::prelude::*;
use consider::Heuristic;
use mlua::Function as F;
use mlua::{chunk, AsChunk};

pub fn init() -> mlua::Result<mlua::Lua> {
	let lua = mlua::Lua::new();
	// Libraries
	lua.load_from_function::<mlua::Value>("esprit.combat", lua.create_function(combat)?)?;
	lua.load_from_function::<mlua::Value>("esprit.world", lua.load(world()).into_function()?)?;

	// Constructors
	lua.load_from_function::<mlua::Value>("esprit.types.action", lua.create_function(action)?)?;
	lua.load_from_function::<mlua::Value>(
		"esprit.types.consider",
		lua.create_function(move |lua, ()| {
			lua.create_function(|_, (action, heuristics)| Ok(Consider { action, heuristics }))
		})?,
	)?;
	lua.load_from_function::<mlua::Value>("esprit.types.duration", lua.create_function(duration)?)?;
	lua.load_from_function::<mlua::Value>(
		"esprit.types.expression",
		lua.create_function(|lua, ()| {
			lua.create_function(|_, s: String| {
				Expression::try_from(s).map_err(mlua::Error::external)
			})
		})?,
	)?;
	lua.load_from_function::<mlua::Value>(
		"esprit.types.heuristic",
		lua.create_function(heuristic)?,
	)?;
	lua.load_from_function::<mlua::Value>("esprit.types.log", lua.create_function(log)?)?;
	lua.load_from_function::<mlua::Value>("esprit.types.stats", lua.create_function(stats)?)?;
	Ok(lua)
}

fn combat(lua: &mlua::Lua, _: ()) -> mlua::Result<mlua::Table> {
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

fn action(lua: &mlua::Lua, _: ()) -> mlua::Result<mlua::Table> {
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

fn duration(lua: &mlua::Lua, _: ()) -> mlua::Result<mlua::Table> {
	let duration = lua.create_table()?;
	duration.set("turn", status::Duration::Turn)?;
	duration.set("rest", status::Duration::Rest)?;
	Ok(duration)
}

fn heuristic(lua: &mlua::Lua, _: ()) -> mlua::Result<mlua::Table> {
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

fn log(lua: &mlua::Lua, _: ()) -> mlua::Result<mlua::Table> {
	let log = lua.create_table()?;
	log.set("Success", combat::Log::Success)?;
	log.set("Miss", combat::Log::Miss)?;
	log.set("Glance", combat::Log::Glance)?;
	log.set("Hit", F::wrap(|damage| Ok(combat::Log::Hit { damage })))?;
	Ok(log)
}

fn stats(lua: &mlua::Lua, _: ()) -> mlua::Result<mlua::Table> {
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
				lua.create_function(|_, table: mlua::Table| {
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
