use crate::prelude::*;
use mlua::{chunk, AsChunk};

pub fn init(
	lua: &mlua::Lua,
	resources: resource::Handle,
	console: impl console::Handle + Clone + 'static,
) -> mlua::Result<()> {
	// These two "libraries" are actually just handles to engine resources.
	// Maybe they really should be globals?
	lua.load_from_function::<mlua::Value>(
		"esprit.resources",
		lua.create_function(move |_, ()| Ok(resources.clone()))?,
	)?;
	lua.load_from_function::<mlua::Value>(
		"esprit.console",
		lua.create_function(move |_, ()| Ok(console::LuaHandle(console.clone())))?,
	)?;

	// Libraries
	lua.load_from_function::<mlua::Value>("esprit.combat", lua.create_function(combat)?)?;
	lua.load_from_function::<mlua::Value>("esprit.world", lua.load(world()).into_function()?)?;

	// Constructors
	lua.load_from_function::<mlua::Value>(
		"esprit.types.heuristic",
		lua.create_function(move |_, ()| Ok(consider::HeuristicConstructor))?,
	)?;
	lua.load_from_function::<mlua::Value>(
		"esprit.types.action",
		lua.create_function(move |_, ()| Ok(character::ActionConstructor))?,
	)?;
	let consider_constructor =
		lua.create_function(|_lua, (action, heuristics)| Ok(Consider { action, heuristics }))?;
	lua.load_from_function::<mlua::Value>(
		"esprit.types.consider",
		lua.create_function(move |_, ()| Ok(consider_constructor.clone()))?,
	)?;
	lua.load_from_function::<mlua::Value>(
		"esprit.types.log",
		lua.create_function(move |_, ()| Ok(combat::LogConstructor))?,
	)?;
	Ok(())
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
///
/// These serde tables should really be constructor functions (like the client-only input library)
fn world() -> impl AsChunk<'static> {
	chunk! {
		return {
			characters = function()
				return coroutine.yield({ type = "Characters" })
			end,

			character_at = function(x, y)
				assert(x, "missing x position")
				assert(y, "missing y position")
				return coroutine.yield({
					type = "Characters",
					query = {
						Within = {
							x = x,
							y = y,
							range = 0,
						}
					}
				})[1]
			end,

			characters_within = function(x, y, radius)
				return coroutine.yield({
					type = "Characters",
					query = {
						Within = {
							x = x,
							y = y,
							range = radius,
						}
					}
				})
			end,

			tile = function(x, y)
				return coroutine.yield({ type = "Tile", x = x, y = y })
			end,
		}
	}
}
