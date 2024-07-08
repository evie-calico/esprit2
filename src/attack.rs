use crate::prelude::*;

/// Unlike spells, `Attack` is only for melee "bump attacks",
/// so their usage can be a lot simpler.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Attack {
	pub name: String,
	pub description: String,
	pub magnitude: Expression,
	pub on_use: script::MaybeInline,
	pub use_time: Aut,
}
