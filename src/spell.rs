use crate::prelude::*;
use std::collections::HashMap;

#[derive(
	Copy,
	Clone,
	Debug,
	Eq,
	PartialEq,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub enum Energy {
	/// Positive energy, like heat.
	Positive,
	/// Negative energy, like cold.
	Negative,
}

#[derive(
	Copy,
	Clone,
	Debug,
	Eq,
	PartialEq,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub enum Harmony {
	/// Spells with unconventional, unpredictable effects.
	Chaos,
	/// Simple spells with predictable effects.
	Order,
}

/// A character's magical skills.
///
/// Only skill from each axis may be chosen, and the minor skill is optional.
#[derive(
	Copy,
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
// This gives the Skillset a cool toml representation.
#[serde(untagged)]
pub enum Skillset {
	EnergyMajor {
		major: Energy,
		minor: Option<Harmony>,
	},
	HarmonyMajor {
		major: Harmony,
		minor: Option<Energy>,
	},
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Affinity {
	/// No skillset matches; the spell is not castable.
	Uncastable,
	/// Only a minor skill matches; spell is difficult to cast.
	Weak,
	/// Only a major skill matches; spell is slightly more difficult to cast.
	Average,
	/// Both skills match; spell is easy to cast.
	Strong,
}

impl Affinity {
	pub fn magnitude(self, magnitude: u32) -> u32 {
		match self {
			Affinity::Uncastable => 0,
			Affinity::Weak => magnitude / 2,
			Affinity::Average => magnitude * 3 / 4,
			Affinity::Strong => magnitude,
		}
	}
}

impl mlua::UserData for Affinity {
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_method("magnitude", |_, this, magnitude: u32| {
			Ok(this.magnitude(magnitude))
		});
		methods.add_method("weak", |_, this, ()| Ok(matches!(this, Affinity::Weak)));
		methods.add_method("average", |_, this, ()| {
			Ok(matches!(this, Affinity::Average))
		});
		methods.add_method("strong", |_, this, ()| Ok(matches!(this, Affinity::Strong)));
	}
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[serde(untagged)]
pub enum Parameter {
	Integer(i32),
	Expression(Expression),
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Spell {
	pub name: String,
	pub description: String,
	pub icon: String,
	/// Configurable spell parameters.
	///
	/// There are passed to spell scripts as global variables,
	/// and may be displayed to the user as information about the spell.
	/// (in addition to its description)
	#[serde(default)]
	pub parameters: HashMap<Box<str>, Parameter>,

	/// Whether the spell concentrates or disperses energy.
	pub energy: Energy,
	/// Whether the spell is ordered or chaotic.
	pub harmony: Harmony,

	/// This is also the cost of the spell.
	pub level: u8,

	/// Script to execute upon casting the spell.
	pub on_cast: resource::Script,
	/// Script to return all possible spell actions.
	///
	/// Returns an array of `consider::Consideration`s for each possible usage of the spell.
	/// For an attack, this means potential targets.
	/// For a self-buff, this should roughly estimate the potential benefit of casting the spell.
	///
	/// When an on_consider script is about to be called, it's fed a list of characters that are potential targets for the spell.
	/// If a spell parameter named "range" exists, the script will only be provided with characters within this range.
	/// Otherwise, consideration scripts are expected to filter targets themselves.
	pub on_consider: Option<resource::Script>,
	pub on_input: resource::Script,
}

#[derive(Clone, Copy, Debug)]
pub enum Castable {
	Yes,
	NotEnoughSP,
	UncastableAffinity,
}

impl Spell {
	pub fn castable_by(&self, character: &character::Piece) -> Castable {
		// Special case for debug spells
		if self.level == 0 {
			Castable::Yes
		} else if character.sp < self.level as i32 {
			Castable::NotEnoughSP
		} else if self.affinity(character) == Affinity::Uncastable {
			Castable::UncastableAffinity
		} else {
			Castable::Yes
		}
	}

	pub fn affinity(&self, character: &character::Piece) -> Affinity {
		match character.sheet.skillset {
			Skillset::EnergyMajor { major, minor } => {
				let minor_affinity = minor.is_some_and(|x| x == self.harmony);
				if major == self.energy {
					if minor_affinity {
						Affinity::Strong
					} else {
						Affinity::Average
					}
				} else if minor_affinity {
					Affinity::Weak
				} else {
					Affinity::Uncastable
				}
			}
			Skillset::HarmonyMajor { major, minor } => {
				let minor_affinity = minor.is_some_and(|x| x == self.energy);
				if major == self.harmony {
					if minor_affinity {
						Affinity::Strong
					} else {
						Affinity::Average
					}
				} else if minor_affinity {
					Affinity::Weak
				} else {
					Affinity::Uncastable
				}
			}
		}
	}

	pub fn parameter_table<'lua>(
		&self,
		scripts: &resource::Scripts<'lua>,
		eval_vars: &impl expression::Variables,
	) -> mlua::Result<mlua::Table<'lua>> {
		scripts
			.runtime
			.create_table_from(self.parameters.iter().filter_map(|(k, v)| {
				let k = k.as_ref();
				match v {
					spell::Parameter::Integer(v) => Some((k, *v)),
					spell::Parameter::Expression(v) => {
						let result = i32::evalv(v, eval_vars);
						if let Err(msg) = &result {
							error!("failed to evaluate {}: {msg}", v.source);
						}
						result.ok().map(|v| (k, v))
					}
				}
			}))
	}
}
