use std::fs;

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

	/// Script to execute upon casting the spell.
	pub on_cast: ScriptOrInline,

	/// This is also the cost of the spell.
	pub level: u8,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "source")]
pub enum ScriptOrInline {
	Inline(String),
	Path(Script),
}

impl ScriptOrInline {
	pub fn contents(&self) -> &str {
		match self {
			ScriptOrInline::Inline(s) => &s,
			ScriptOrInline::Path(expression) => &expression.contents,
		}
	}
}

#[derive(Clone, Debug)]
pub struct Script {
	pub path: String,
	pub contents: String,
}

impl serde::Serialize for Script {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.path)
	}
}

struct ScriptVisitor;

impl<'de> serde::de::Visitor<'de> for ScriptVisitor {
	type Value = String;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("the path to a Lua script")
	}

	fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(value)
	}

	fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(value.to_string())
	}
}

impl<'de> serde::Deserialize<'de> for Script {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de::Error;
		let path = deserializer.deserialize_string(ScriptVisitor)?;
		let contents = fs::read_to_string(options::RESOURCE_DIRECTORY.join(&path))
			.map_err(D::Error::custom)?;
		Ok(Script { path, contents })
	}
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
