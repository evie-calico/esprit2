use crate::{nouns::Nouns, spell::Spell, Aut};
use uuid::Uuid;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Piece {
    // These are nice and serializable :)
    pub id: Uuid,
    pub sheet: Sheet,

    pub hp: i32,
    pub sp: i32,
    pub x: i32,
    pub y: i32,
    pub next_action: Option<Action>,

    pub player_controlled: bool,
    pub alliance: Alliance,
}

impl Piece {
    pub fn new(sheet: Sheet) -> Self {
        let hp = sheet.stats.heart as i32;
        let sp = sheet.stats.soul as i32;
        Self {
            id: Uuid::new_v4(),
            sheet,
            hp,
            sp,
            x: 0,
            y: 0,
            next_action: None,
            player_controlled: false,
            alliance: Alliance::default(),
        }
    }
}

impl Default for Piece {
    fn default() -> Self {
        Self::new(Sheet::default())
    }
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum OrdDir {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

/// Anything a character piece can "do".
///
/// This is the only way that character logic or player input should communicate with pieces.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Action {
    Move(OrdDir),
}

#[derive(Copy, PartialEq, Eq, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Alliance {
    Friendly,
    #[default]
    Enemy,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Sheet {
    /// Note that this includes the character's name.
    pub nouns: Nouns,
    pub level: u32,
    pub stats: Stats,
    pub spells: Vec<Spell>,
    pub speed: Aut,
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
