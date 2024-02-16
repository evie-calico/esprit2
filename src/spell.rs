#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Spell {
    name: String,
    /// This is also the cost of the spell.
    level: u8,
}
