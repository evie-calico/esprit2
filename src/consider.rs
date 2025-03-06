//! Enumerate and assign scores to potential actions.
//!
//! This is mainly for enemy logic, but may have some use for player UI,
//! such as showing a sorted list of potential spell targets rather than a cursor.

use crate::prelude::*;
use mlua::IntoLuaMulti;

/// Rough approximations of an action's result.
/// Used to estimate the outcome of a certain action.
///
/// It's worth noting that this is intentionally a VERY rough estimation.
/// Many effects and outcomes will be ignored or oversimplified by this system.
/// If this becomes an issue more heuristics can be added to better express the outcomes of spells,
/// but low performance costs should be a priority over accuracy.
#[derive(Clone, Debug, mlua::FromLua)]
pub enum Heuristic {
	Damage {
		target: character::Ref,
		amount: u32,
	},
	/// Reflects a rough estimation of the lasting effects of a debuff from this attack.
	/// `amount` should be considered a measure of stat loss, even if the debuff is more complicated than a simple stat loss.
	/// For example, the bleed effect builds up a defense loss after repeated attacks,
	/// and is represented by a debuff heuristic of 1 despite being more variable than that.
	Debuff {
		target: character::Ref,
		amount: u32,
	},
	Move {
		x: i32,
		y: i32,
	},
}

fn wrong_variant() -> mlua::Error {
	mlua::Error::runtime("attempted to retrieve missing field from heuristic variant")
}

impl mlua::UserData for Heuristic {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("x", |_, this| match this {
			Heuristic::Move { x, .. } => Ok(*x),
			Heuristic::Damage { .. } | Heuristic::Debuff { .. } => Err(wrong_variant()),
		});
		fields.add_field_method_get("y", |_, this| match this {
			Heuristic::Move { y, .. } => Ok(*y),
			Heuristic::Damage { .. } | Heuristic::Debuff { .. } => Err(wrong_variant()),
		});
		fields.add_field_method_get("target", |_, this| match this {
			Heuristic::Damage { target, .. } | Heuristic::Debuff { target, .. } => {
				Ok(target.clone())
			}
			Heuristic::Move { .. } => Err(wrong_variant()),
		});
		fields.add_field_method_get("amount", |_, this| match this {
			Heuristic::Damage { amount, .. } | Heuristic::Debuff { amount, .. } => Ok(*amount),
			Heuristic::Move { .. } => Err(wrong_variant()),
		});
	}

	fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("damage", |_, this, ()| {
			Ok(matches!(this, Heuristic::Damage { .. }))
		});
		methods.add_method("debuff", |_, this, ()| {
			Ok(matches!(this, Heuristic::Debuff { .. }))
		});
	}
}

#[derive(Clone, Debug, mlua::FromLua)]
pub struct Consider {
	pub action: character::Action,
	pub heuristics: Vec<Heuristic>,
}

impl mlua::UserData for Consider {
	fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
		methods.add_meta_function("__ipairs", |_, this: mlua::AnyUserData| {
			Ok((
				this.metatable()?.get::<mlua::Function>("__next")?,
				this,
				mlua::Nil,
			))
		});
		methods.add_meta_method("__next", |lua, this, index: mlua::Value| {
			let index = index.as_usize().unwrap_or(0);
			if let Some(heuristic) = this.heuristics.get(index) {
				lua.pack_multi((index + 1, heuristic.clone()))
			} else {
				mlua::Nil.into_lua_multi(lua)
			}
		});
		methods.add_method("attack", |_, this, ()| {
			Ok(matches!(this.action, character::Action::Attack(..)))
		});
		methods.add_method("spell", |_, this, ()| {
			Ok(matches!(this.action, character::Action::Cast(..)))
		});
	}
}
