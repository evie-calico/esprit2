use crate::prelude::*;
use nouns::StrExt;
use std::{collections::HashMap, rc::Rc};

mod piece {
	use super::*;

	fn stats(_lua: &mlua::Lua, this: &mut Piece, _: ()) -> mlua::Result<Stats> {
		Ok(this.stats())
	}

	fn replace_nouns(_lua: &mlua::Lua, this: &mut Piece, string: String) -> mlua::Result<String> {
		Ok(string.replace_nouns(&this.sheet.nouns))
	}

	fn replace_prefixed_nouns(
		_lua: &mlua::Lua,
		this: &mut Piece,
		(prefix, string): (String, String),
	) -> mlua::Result<String> {
		Ok(string.replace_prefixed_nouns(&this.sheet.nouns, &prefix))
	}

	/// Used for debugging.
	///
	/// While fields of `Piece` are settable from Lua,
	/// fields of `Sheet` and `Stats` are not.
	/// This method circumvents that.
	fn force_level(_lua: &mlua::Lua, this: &mut Piece, _: ()) -> mlua::Result<()> {
		this.sheet.level = this.sheet.level.saturating_add(1);
		Ok(())
	}

	pub fn alliance(_lua: &mlua::Lua, this: &mut Piece, _: ()) -> mlua::Result<u32> {
		Ok(this.alliance as u32)
	}

	/// Initializes an effect with the given magnitude, or adds the magnitude to the effect if it already exists.
	pub fn inflict(
		lua: &mlua::Lua,
		this: &mut Piece,
		(key, magnitude): (String, Option<u32>),
	) -> mlua::Result<()> {
		let statuses = lua
			.globals()
			.get::<&str, resource::Handle<Status>>("Status")?;
		let Some(status) = statuses.0.get(key.as_str()).cloned() else {
			return Err(mlua::Error::external(resource::Error::NotFound(key)));
		};
		let entry = this
			.statuses
			.entry(key.into_boxed_str())
			.or_insert_with(|| status);
		if let Some(magnitude) = magnitude {
			entry.add_magnitude(magnitude);
		}
		Ok(())
	}

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, alua::UserData)]
	#[alua(
		method = replace_nouns,
		method = replace_prefixed_nouns,
		method = force_level,
		method = stats,
		method = alliance,
		method = inflict,
	)]
	pub struct Piece {
		#[alua(get)]
		pub sheet: Sheet,

		#[alua(get, set)]
		pub hp: i32,
		#[alua(get, set)]
		pub sp: i32,

		pub statuses: HashMap<Box<str>, Status>,
		pub attacks: Vec<Rc<Attack>>,
		pub spells: Vec<Rc<Spell>>,

		#[alua(get, set)]
		pub x: i32,
		#[alua(get, set)]
		pub y: i32,
		pub next_action: Option<Action>,
		#[alua(get, set)]
		pub player_controlled: bool,
		pub alliance: Alliance,
	}
}

pub use piece::Piece;

impl expression::Variables for Piece {
	fn get(&self, s: &str) -> Result<expression::Integer, expression::Error> {
		match s {
			"hp" => Ok(self.hp as expression::Integer),
			"sp" => Ok(self.sp as expression::Integer),
			_ => self.sheet.get(s),
		}
	}
}

impl Piece {
	pub fn new(sheet: Sheet, resources: &resource::Manager) -> Result<Self> {
		let stats = sheet.stats();
		let hp = stats.heart as i32;
		let sp = stats.soul as i32;
		let attacks = sheet
			.attacks
			.iter()
			.map(|x| resources.get_attack(x).cloned())
			.collect::<Result<_>>()?;
		let spells = sheet
			.spells
			.iter()
			.map(|x| resources.get_spell(x).cloned())
			.collect::<Result<_>>()?;

		Ok(Self {
			sheet,
			hp,
			sp,
			statuses: HashMap::new(),
			attacks,
			spells,
			x: 0,
			y: 0,
			next_action: None,
			player_controlled: false,
			alliance: Alliance::default(),
		})
	}

	pub fn new_turn(&mut self) {
		// Remove any status effects with the duration of one turn.
		self.statuses
			.retain(|_, status| !matches!(status.duration, status::Duration::Turn));
	}

	pub fn rest(&mut self) {
		let stats = self.stats();
		self.restore_hp(stats.heart / 2);
		self.restore_sp(stats.soul);
		// Remove any status effects lasting until the next rest.
		self.statuses
			.retain(|_, status| !matches!(status.duration, status::Duration::Rest));
	}

	pub fn restore_hp(&mut self, amount: u32) {
		self.hp = i32::min(self.hp + amount as i32, self.stats().heart as i32);
	}

	pub fn restore_sp(&mut self, amount: u32) {
		self.sp = i32::min(self.sp + amount as i32, self.stats().soul as i32);
	}
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StatOutcomes {
	pub stats: Stats,
	pub buffs: Stats,
	pub debuffs: Stats,
}

impl Piece {
	pub fn stats(&self) -> Stats {
		self.stat_outcomes().stats
	}

	pub fn stat_outcomes(&self) -> StatOutcomes {
		let buffs = Stats::default();
		let mut debuffs = Stats::default();

		for debuff in self.statuses.values().filter_map(|x| x.on_debuff()) {
			debuffs = debuffs + debuff;
		}

		let mut stats = self.sheet.stats();
		stats.heart = stats.heart.saturating_sub(debuffs.heart) + buffs.heart;
		stats.soul = stats.soul.saturating_sub(debuffs.soul) + buffs.soul;
		stats.power = stats.power.saturating_sub(debuffs.power) + buffs.power;
		stats.defense = stats.defense.saturating_sub(debuffs.defense) + buffs.defense;
		stats.magic = stats.magic.saturating_sub(debuffs.magic) + buffs.magic;
		stats.resistance = stats.resistance.saturating_sub(debuffs.resistance) + buffs.resistance;

		StatOutcomes {
			stats,
			buffs,
			debuffs,
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
#[repr(u32)]
pub enum Alliance {
	Friendly,
	#[default]
	Enemy,
}

mod sheet {
	use super::*;

	fn stats(_lua: &mlua::Lua, this: &mut Sheet, _: ()) -> mlua::Result<Stats> {
		Ok(this.stats())
	}

	fn growth_bonuses() -> Stats {
		use rand::seq::SliceRandom;
		const BONUS_COUNT: usize = 10;

		let mut bonuses = Stats::default();
		let mut stats = [
			&mut bonuses.heart,
			&mut bonuses.soul,
			&mut bonuses.power,
			&mut bonuses.defense,
			&mut bonuses.magic,
			&mut bonuses.resistance,
		];
		let mut rng = rand::thread_rng();

		for _ in 0..BONUS_COUNT {
			let stat = stats
				.choose_mut(&mut rng)
				.expect("stats should not be empty");
			// Prefer skipping stats that are already 0
			if **stat == 0 {
				**stats
					.choose_mut(&mut rng)
					.expect("stats should not be empty") += 1;
			} else {
				**stat += 1;
			}
		}

		bonuses
	}

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, alua::UserData)]
	#[alua(method = stats)]
	pub struct Sheet {
		pub icon: String,
		/// Note that this includes the character's name.
		#[alua(get)]
		pub nouns: Nouns,

		#[alua(get)]
		pub level: u32,
		#[alua(get)]
		#[serde(default)] // There's no reason for most sheets to care about this.
		pub experience: u32,

		#[alua(get)]
		pub bases: Stats,
		#[alua(get)]
		pub growths: Stats,
		#[serde(default = "growth_bonuses")]
		pub growth_bonuses: Stats,

		pub skillset: spell::Skillset,
		#[alua(get)]
		pub speed: Aut,

		#[alua(get)]
		pub attacks: Vec<String>,
		#[alua(get)]
		pub spells: Vec<String>,
	}
}

pub use sheet::Sheet;

impl Sheet {
	pub fn stats(&self) -> Stats {
		const BONUS_WEIGHTS: Stats = Stats {
			heart: 20,
			soul: 20,
			power: 10,
			defense: 10,
			magic: 10,
			resistance: 10,
		};

		self.bases + (self.growths + self.growth_bonuses * BONUS_WEIGHTS) * self.level / 100
	}
}

impl expression::Variables for Sheet {
	fn get(&self, s: &str) -> Result<expression::Integer, expression::Error> {
		match s {
			"level" => Ok(self.level as expression::Integer),
			"speed" => Ok(self.speed as expression::Integer),
			_ => self.stats().get(s),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, alua::UserData)]
pub struct Stats {
	/// Health, or HP; Heart Points
	#[serde(default)]
	#[alua(get)]
	pub heart: u32,
	/// Magic, or SP; Soul Points
	#[serde(default)]
	#[alua(get)]
	pub soul: u32,
	/// Bonus damage applied to physical attacks.
	#[serde(default)]
	#[alua(get)]
	pub power: u32,
	/// Damage reduction when recieving physical attacks.
	#[serde(default)]
	#[alua(get)]
	pub defense: u32,
	/// Bonus damage applied to magical attacks.
	#[serde(default)]
	#[alua(get)]
	pub magic: u32,
	/// Damage reduction when recieving magical attacks.
	/// Also makes harmful spells more likely to fail.
	#[serde(default)]
	#[alua(get)]
	pub resistance: u32,
}

impl std::ops::Add for Stats {
	type Output = Stats;

	fn add(self, rhs: Self) -> Self {
		Stats {
			heart: self.heart + rhs.heart,
			soul: self.soul + rhs.soul,
			power: self.power + rhs.power,
			defense: self.defense + rhs.defense,
			magic: self.magic + rhs.magic,
			resistance: self.resistance + rhs.resistance,
		}
	}
}

impl std::ops::Sub for Stats {
	type Output = Stats;

	fn sub(self, rhs: Self) -> Self {
		Stats {
			heart: self.heart - rhs.heart,
			soul: self.soul - rhs.soul,
			power: self.power - rhs.power,
			defense: self.defense - rhs.defense,
			magic: self.magic - rhs.magic,
			resistance: self.resistance - rhs.resistance,
		}
	}
}

impl std::ops::Mul<u32> for Stats {
	type Output = Stats;

	fn mul(self, rhs: u32) -> Self {
		Stats {
			heart: self.heart * rhs,
			soul: self.soul * rhs,
			power: self.power * rhs,
			defense: self.defense * rhs,
			magic: self.magic * rhs,
			resistance: self.resistance * rhs,
		}
	}
}

impl std::ops::Mul for Stats {
	type Output = Stats;

	fn mul(self, rhs: Self) -> Self {
		Stats {
			heart: self.heart * rhs.heart,
			soul: self.soul * rhs.soul,
			power: self.power * rhs.power,
			defense: self.defense * rhs.defense,
			magic: self.magic * rhs.magic,
			resistance: self.resistance * rhs.resistance,
		}
	}
}

impl std::ops::Div<u32> for Stats {
	type Output = Stats;

	fn div(self, rhs: u32) -> Self {
		Stats {
			heart: self.heart / rhs,
			soul: self.soul / rhs,
			power: self.power / rhs,
			defense: self.defense / rhs,
			magic: self.magic / rhs,
			resistance: self.resistance / rhs,
		}
	}
}

impl expression::Variables for Stats {
	fn get(&self, s: &str) -> Result<expression::Integer, expression::Error> {
		match s {
			"heart" => Ok(self.heart as expression::Integer),
			"soul" => Ok(self.soul as expression::Integer),
			"power" => Ok(self.power as expression::Integer),
			"defense" => Ok(self.defense as expression::Integer),
			"magic" => Ok(self.magic as expression::Integer),
			"resistance" => Ok(self.resistance as expression::Integer),
			_ => Err(expression::Error::MissingVariable(s.into())),
		}
	}
}

const HEART_COLOR: Color = (96, 67, 18, 255);
const SOUL_COLOR: Color = (128, 128, 128, 255);
const POWER_COLOR: Color = (255, 11, 64, 255);
const DEFENSE_COLOR: Color = (222, 120, 64, 255);
const MAGIC_COLOR: Color = (59, 115, 255, 255);
const RESISTANCE_COLOR: Color = (222, 64, 255, 255);

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
