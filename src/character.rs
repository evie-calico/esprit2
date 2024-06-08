use crate::prelude::*;
use parking_lot::RwLock;
use sdl2::pixels::Color;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, alua::UserData)]
pub struct Piece {
	// These are nice and serializable :)
	#[alua(as_lua = "string", get)]
	pub id: Uuid,
	#[alua(get)]
	pub sheet: Arc<RwLock<Sheet>>,

	#[alua(get, set)]
	pub hp: i32,
	#[alua(get, set)]
	pub sp: i32,
	pub attacks: Vec<Attack>,
	pub spells: Vec<Arc<Spell>>,

	#[alua(get, set)]
	pub x: i32,
	#[alua(get, set)]
	pub y: i32,
	pub next_action: Option<Action>,
	#[alua(get, set)]
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
			_ => self.sheet.read().get(s),
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
			.map(|x| Arc::new(resources.get_spell(x).unwrap().clone()))
			.collect();
		let sheet = Arc::new(RwLock::new(sheet));

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
	Cast(Arc<Spell>),
}

#[derive(Copy, PartialEq, Eq, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Alliance {
	Friendly,
	#[default]
	Enemy,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, alua::UserData)]
pub struct Sheet {
	/// Note that this includes the character's name.
	#[alua(get)]
	pub nouns: Arc<parking_lot::RwLock<Nouns>>,

	#[alua(get)]
	pub level: u32,
	#[alua(get)]
	pub stats: Stats,
	pub skillset: spell::Skillset,
	#[alua(get)]
	pub speed: Aut,

	#[alua(get)]
	pub attacks: Vec<String>,
	#[alua(get)]
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

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, alua::UserData)]
pub struct Stats {
	/// Health, or HP; Heart Points
	#[alua(get)]
	pub heart: u32,
	/// Magic, or SP; Soul Points
	#[alua(get)]
	pub soul: u32,
	/// Bonus damage applied to physical attacks.
	#[alua(get)]
	pub power: u32,
	/// Damage reduction when recieving physical attacks.
	#[alua(get)]
	pub defense: u32,
	/// Bonus damage applied to magical attacks.
	#[alua(get)]
	pub magic: u32,
	/// Damage reduction when recieving magical attacks.
	/// Also makes harmful spells more likely to fail.
	#[alua(get)]
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

const HEART_COLOR: Color = Color::RGB(96, 67, 18);
const SOUL_COLOR: Color = Color::RGB(128, 128, 128);
const POWER_COLOR: Color = Color::RGB(255, 11, 64);
const DEFENSE_COLOR: Color = Color::RGB(222, 120, 64);
const MAGIC_COLOR: Color = Color::RGB(59, 115, 255);
const RESISTANCE_COLOR: Color = Color::RGB(222, 64, 255);

impl gui::VariableColors for Stats {
	fn get(s: &str) -> Option<Color> {
		match s {
			"heart" => Some(HEART_COLOR),
			"soul" => Some(SOUL_COLOR),
			"power" => Some(POWER_COLOR),
			"defense" => Some(DEFENSE_COLOR),
			"magic" => Some(MAGIC_COLOR),
			"resistance" => Some(RESISTANCE_COLOR),
			_ => None,
		}
	}
}
