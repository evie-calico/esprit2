use crate::prelude::*;
use std::rc::Rc;
use uuid::Uuid;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Piece {
	// These are nice and serializable :)
	pub id: Uuid,
	pub sheet: Sheet,

	pub hp: i32,
	pub sp: i32,
	pub attacks: Vec<Attack>,
	pub spells: Vec<Rc<Spell>>,

	pub x: i32,
	pub y: i32,
	pub next_action: Option<Action>,
	pub player_controlled: bool,
	pub alliance: Alliance,
}

impl expression::Variables for Piece {
	fn get<'expression>(
		&self,
		s: &'expression str,
	) -> Result<expression::Integer, expression::Error<'expression>> {
		match s {
			"hp" => Ok(self.hp as expression::Integer),
			"sp" => Ok(self.sp as expression::Integer),
			_ => self.sheet.get(s),
		}
	}
}

impl Piece {
	pub fn new(sheet: Sheet, resources: &ResourceManager) -> Self {
		let hp = sheet.stats.heart as i32;
		let sp = sheet.stats.soul as i32;
		let attacks = sheet
			.attacks
			.iter()
			.map(|x| resources.get_attack(x).unwrap().clone())
			.collect();
		let spells = sheet
			.spells
			.iter()
			.map(|x| Rc::new(resources.get_spell(x).unwrap().clone()))
			.collect();

		Self {
			id: Uuid::new_v4(),
			sheet,
			hp,
			sp,
			attacks,
			spells,
			x: 0,
			y: 0,
			next_action: None,
			player_controlled: false,
			alliance: Alliance::default(),
		}
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

impl OrdDir {
	pub fn as_offset(self) -> (i32, i32) {
		let (x, y) = match self {
			OrdDir::Up => (0, -1),
			OrdDir::UpRight => (1, -1),
			OrdDir::Right => (1, 0),
			OrdDir::DownRight => (1, 1),
			OrdDir::Down => (0, 1),
			OrdDir::DownLeft => (-1, 1),
			OrdDir::Left => (-1, 0),
			OrdDir::UpLeft => (-1, -1),
		};
		(x, y)
	}
}

/// Anything a character piece can "do".
///
/// This is the only way that character logic or player input should communicate with pieces.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Action {
	Move(OrdDir),
	Cast(Rc<Spell>),
}

#[derive(Copy, PartialEq, Eq, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Alliance {
	Friendly,
	#[default]
	Enemy,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sheet {
	/// Note that this includes the character's name.
	pub nouns: Nouns,

	pub level: u32,
	pub stats: Stats,
	pub skillset: spell::Skillset,
	pub speed: Aut,

	pub attacks: Vec<String>,
	pub spells: Vec<String>,
}

impl expression::Variables for Sheet {
	fn get<'expression>(
		&self,
		s: &'expression str,
	) -> Result<expression::Integer, expression::Error<'expression>> {
		match s {
			"level" => Ok(self.level as expression::Integer),
			"speed" => Ok(self.speed as expression::Integer),
			_ => self.stats.get(s),
		}
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

impl expression::Variables for Stats {
	fn get<'expression>(
		&self,
		s: &'expression str,
	) -> Result<expression::Integer, expression::Error<'expression>> {
		match s {
			"heart" => Ok(self.heart as expression::Integer),
			"soul" => Ok(self.soul as expression::Integer),
			"power" => Ok(self.power as expression::Integer),
			"defense" => Ok(self.defense as expression::Integer),
			"magic" => Ok(self.magic as expression::Integer),
			"resistance" => Ok(self.resistance as expression::Integer),
			_ => Err(expression::Error::MissingVariable(s)),
		}
	}
}
