#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Spell {
	name: String,
	icon: String,
	/// This is also the cost of the spell.
	level: u8,
}
