use crate::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Energy {
	/// Positive energy, like heat.
	Positive,
	/// Negative energy, like cold.
	Negative,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Harmony {
	/// Spells with unconventional, unpredictable effects.
	Chaos,
	/// Simple spells with predictable effects.
	Order,
}

/// A character's magical skills.
///
/// Only skill from each axis may be chosen, and the minor skill is optional.
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Spell {
	pub name: String,
	pub icon: String,

	/// Whether the spell concentrates or disperses energy.
	pub energy: Energy,
	/// Whether the spell is ordered or chaotic.
	pub harmony: Harmony,

	/// This is also the cost of the spell.
	pub level: u8,
}

impl Spell {
	pub fn castable_by(&self, character: &character::Piece) -> bool {
		character.sp >= self.level as i32
	}

	pub fn affinity(&self, character: &character::Piece) -> Affinity {
		match character.sheet.read().skillset {
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
}
