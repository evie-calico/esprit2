use crate::Aut;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Piece {
    pub sheet: Sheet,

    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Sheet {
    /// Note that this includes the character's name.
    pub nouns: Nouns,
    pub stats: Stats,
    pub spells: Vec<Spell>,
    pub speed: Aut,
}

/// For dynamically addressing a character.
/// This should encompass almost every (dynamic) way of addressing someone or something.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Nouns {
    pub name: String,
    /// If true, will be addressed as "Name", rather than "The name" or "A name".
    pub proper_name: bool,
    pub pronouns: Pronouns,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Pronouns {
    Female,
    Male,
    /// Neutral (they) is special because it necessitates "plural verbs".
    /// Even when used as a singular pronoun, verbs still treat "they" as plural.
    Neutral,
    #[default]
    Object,
}

impl Pronouns {
    pub fn plural(&self) -> bool {
        matches!(self, Pronouns::Neutral)
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    /// Health, or HP; Heart Points
    pub heart: u32,
    /// Magic, or SP; Soul Points
    pub soul: u32,
    /// Bonus damage applied to physical attacks.
    pub power: u32,
    /// Damage reduction when recieving physical attacks.
    pub defense: u32,
    /// Bonus damage applied to magical attacks.
    pub magic: u32,
    /// Damage reduction when recieving magical attacks.
    /// Also makes harmful spells more likely to fail.
    pub resistance: u32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Spell {
    name: String,
    /// This is also the cost of the spell.
    level: u8,
}
