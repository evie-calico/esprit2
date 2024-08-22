//! Enumerate and assign scores to potential actions.
//!
//! This is mainly for enemy logic, but may have some use for player UI,
//! such as showing a sorted list of potential spell targets rather than a cursor.

use crate::prelude::*;

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
	Move {
		x: i32,
		y: i32,
	},
}

fn wrong_variant() -> mlua::Error {
	mlua::Error::runtime("attempted to retrieve missing field from heuristic variant")
}

impl mlua::UserData for Heuristic {
	fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
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

	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method("damage", |_, this, ()| {
			Ok(matches!(this, Heuristic::Damage { .. }))
		});
		methods.add_method("debuff", |_, this, ()| {
			Ok(matches!(this, Heuristic::Debuff { .. }))
		});
	}
}

pub struct HeuristicConstructor;

impl mlua::UserData for HeuristicConstructor {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_function(
			"damage",
			|_, (target, amount): (world::CharacterRef, mlua::Integer)| {
				Ok(Heuristic::Damage {
					target,
					amount: amount.try_into().unwrap_or_default(),
				})
			},
		);
		methods.add_function(
			"debuff",
			|_, (target, amount): (world::CharacterRef, u32)| {
				Ok(Heuristic::Debuff { target, amount })
			},
		);
	}
}

#[derive(Clone, Debug)]
pub struct Consider {
	pub action: character::Action,
	pub heuristics: Vec<Heuristic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActionType {
	Wait,
	Move,
	Attack,
	Cast,
}

#[derive(Clone, Debug, mlua::FromLua)]
pub struct TaggedHeuristics {
	action_type: ActionType,
	heuristics: Vec<Heuristic>,
}

impl TaggedHeuristics {
	pub fn new(consider: &Consider) -> Self {
		Self {
			action_type: match consider.action {
				character::Action::Wait(_) => ActionType::Wait,
				character::Action::Move(_, _) => ActionType::Move,
				character::Action::Attack(_, _) => ActionType::Attack,
				character::Action::Cast(_, _) => ActionType::Cast,
			},
			heuristics: consider.heuristics.clone(),
		}
	}
}

impl mlua::UserData for TaggedHeuristics {
	fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
		fields.add_field_method_get("heuristics", |_, this| Ok(this.heuristics.clone()));
	}

	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method("attack", |_, this, ()| {
			Ok(this.action_type == ActionType::Attack)
		});
		methods.add_method("spell", |_, this, ()| {
			Ok(this.action_type == ActionType::Cast)
		});
	}
}
