use crate::{spell::Spell, Aut};
use uuid::Uuid;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Piece {
    // These are nice and serializable :)
    pub id: Uuid,
    pub sheet: Sheet,

    pub x: i32,
    pub y: i32,
    pub next_action: Option<Action>,

    pub player_controlled: bool,
    pub alliance: Alliance,
}

impl Piece {
    pub fn new(sheet: Sheet) -> Self {
        Self {
            id: Uuid::new_v4(),
            sheet,
            x: 0,
            y: 0,
            next_action: None,
            player_controlled: false,
            alliance: Alliance::default(),
        }
    }

    pub fn act(&mut self) {
        let Some(action) = self.next_action.take() else {
            return;
        };
        match action {
            Action::Move(dir) => {
                self.move_by(dir);
            }
        }
    }

    pub fn move_by(&mut self, dir: OrdDir) {
        let (x, y) = match dir {
            OrdDir::Up => (0, -1),
            OrdDir::UpRight => (1, -1),
            OrdDir::Right => (1, 0),
            OrdDir::DownRight => (1, 1),
            OrdDir::Down => (0, 1),
            OrdDir::DownLeft => (-1, 1),
            OrdDir::Left => (-1, 0),
            OrdDir::UpLeft => (-1, -1),
        };
        self.x += x;
        self.y += y;
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

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
