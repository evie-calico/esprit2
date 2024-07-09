//! Enumerate and assign scores to potential actions.
//!
//! This is mainly for enemy logic, but may have some use for player UI,
//! such as showing a sorted list of potential spell targets rather than a cursor.

use std::rc::Rc;

use crate::prelude::*;

/// Possible results of an attack
///
/// A list of these is returned by attack consideration scripts,
/// Which are then promoted to `Consider` structures with attacks attatched.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Attack {
	Damage {
		target: world::CharacterRef,
		amount: u32,
	},
}

/// Possible results of casting a spell
///
/// A list of these is returned by spell consideration scripts,
/// Which are then promoted to `Consider` structures with spells attatched.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Spell {
	Damage {
		target: world::CharacterRef,
		amount: u32,
	},
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Consider {
	Attack(Rc<attack::Attack>, Attack),
	Spell(Rc<spell::Spell>, Spell),
}

impl mlua::UserData for Consider {}

#[derive(Clone, Default, Debug, mlua::FromLua)]
pub struct AttackList(pub Vec<Attack>);

impl mlua::UserData for AttackList {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method_mut(
			"damage",
			|_, this, (target, amount): (world::CharacterRef, u32)| {
				this.0.push(Attack::Damage {
					target: target.clone(),
					amount,
				});
				Ok(())
			},
		);
	}
}

#[derive(Clone, Default, Debug, mlua::FromLua)]
pub struct SpellList(pub Vec<Spell>);

impl mlua::UserData for SpellList {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method_mut(
			"damage",
			|_, this, (target, amount): (world::CharacterRef, u32)| {
				this.0.push(Spell::Damage {
					target: target.clone(),
					amount,
				});
				Ok(())
			},
		);
	}
}
