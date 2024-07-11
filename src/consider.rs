//! Enumerate and assign scores to potential actions.
//!
//! This is mainly for enemy logic, but may have some use for player UI,
//! such as showing a sorted list of potential spell targets rather than a cursor.

use crate::prelude::*;
use std::rc::Rc;

/// Rough approximations of an action's result.
/// Used to estimate the outcome of a certain action.
///
/// It's worth noting that this is intentionally a VERY rough estimation.
/// Many effects and outcomes will be ignored or oversimplified by this system.
/// If this becomes an issue more heuristics can be added to better express the outcomes of spells,
/// but low performance costs should be a priority over accuracy.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, mlua::FromLua)]
pub enum Heuristic {
	Damage {
		target: world::CharacterRef,
		amount: u32,
	},
	/// Reflects a rough estimation of the lasting effects of a debuff from this attack.
	/// `amount` should be considered a measure of stat loss, even if the debuff is more complicated than a simple stat loss.
	/// For example, the bleed effect builds up a defense loss after repeated attacks,
	/// and is represented by a debuff heuristic of 1 despite being more variable than that.
	Debuff {
		target: world::CharacterRef,
		amount: u32,
	},
}

impl mlua::UserData for Heuristic {}

pub struct HeuristicConstructor;

impl mlua::UserData for HeuristicConstructor {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method(
			"damage",
			|_, _, (target, amount): (world::CharacterRef, u32)| {
				Ok(Heuristic::Damage { target, amount })
			},
		);
		methods.add_method(
			"debuff",
			|_, _, (target, amount): (world::CharacterRef, u32)| {
				Ok(Heuristic::Debuff { target, amount })
			},
		);
	}
}

#[derive(Clone, Debug)]
/// These owned tables are relatively dangerous; don't put them in UserData!
pub enum Consider {
	Attack(Rc<Attack>, Vec<Heuristic>, mlua::OwnedTable),
	Spell(Rc<Spell>, Vec<Heuristic>, mlua::OwnedTable),
}

impl mlua::UserData for Consider {}

#[derive(Clone, Debug, mlua::FromLua)]
pub struct AttackList {
	base: Rc<Attack>,
	pub results: Vec<Consider>,
}

impl AttackList {
	pub fn new(base: Rc<Attack>) -> Self {
		Self {
			base,
			results: Vec::new(),
		}
	}
}

impl mlua::UserData for AttackList {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method_mut(
			"push",
			|_, this, (table, heuristics): (mlua::OwnedTable, mlua::Variadic<Heuristic>)| {
				this.results.push(Consider::Attack(
					this.base.clone(),
					heuristics.into_iter().collect(),
					table,
				));
				Ok(())
			},
		);
	}
}

#[derive(Clone, Debug, mlua::FromLua)]
pub struct SpellList {
	base: Rc<Spell>,
	pub results: Vec<Consider>,
}

impl SpellList {
	pub fn new(base: Rc<Spell>) -> Self {
		Self {
			base,
			results: Vec::new(),
		}
	}
}

impl mlua::UserData for SpellList {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method_mut(
			"push",
			|_, this, (table, heuristics): (mlua::OwnedTable, mlua::Variadic<Heuristic>)| {
				this.results.push(Consider::Spell(
					this.base.clone(),
					heuristics.into_iter().collect(),
					table,
				));
				Ok(())
			},
		);
	}
}
