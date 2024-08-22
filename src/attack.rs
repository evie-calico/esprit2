use crate::prelude::*;

/// Unlike spells, `Attack` is only for melee "bump attacks",
/// so their usage can be a lot simpler.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Attack {
	pub name: String,
	pub description: String,
	pub magnitude: Expression,
	pub on_input: resource::Id,
	pub on_use: resource::Id,
	pub on_consider: Option<resource::Id>,
	pub use_time: Aut,
}
