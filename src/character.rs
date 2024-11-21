use crate::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Used for debugging.
fn force_affinity(_lua: &mlua::Lua, this: &Ref, index: u32) -> mlua::Result<()> {
	this.borrow_mut().sheet.skillset = match index {
		0 => spell::Skillset::EnergyMajor {
			major: spell::Energy::Positive,
			minor: None,
		},
		1 => spell::Skillset::EnergyMajor {
			major: spell::Energy::Positive,
			minor: Some(spell::Harmony::Chaos),
		},
		2 => spell::Skillset::EnergyMajor {
			major: spell::Energy::Positive,
			minor: Some(spell::Harmony::Order),
		},
		3 => spell::Skillset::EnergyMajor {
			major: spell::Energy::Negative,
			minor: None,
		},
		4 => spell::Skillset::EnergyMajor {
			major: spell::Energy::Negative,
			minor: Some(spell::Harmony::Chaos),
		},
		5 => spell::Skillset::EnergyMajor {
			major: spell::Energy::Negative,
			minor: Some(spell::Harmony::Order),
		},
		6 => spell::Skillset::HarmonyMajor {
			major: spell::Harmony::Chaos,
			minor: None,
		},
		7 => spell::Skillset::HarmonyMajor {
			major: spell::Harmony::Chaos,
			minor: Some(spell::Energy::Positive),
		},
		8 => spell::Skillset::HarmonyMajor {
			major: spell::Harmony::Chaos,
			minor: Some(spell::Energy::Negative),
		},
		9 => spell::Skillset::HarmonyMajor {
			major: spell::Harmony::Order,
			minor: None,
		},
		10 => spell::Skillset::HarmonyMajor {
			major: spell::Harmony::Order,
			minor: Some(spell::Energy::Positive),
		},
		11 => spell::Skillset::HarmonyMajor {
			major: spell::Harmony::Order,
			minor: Some(spell::Energy::Negative),
		},
		_ => {
			return Err(mlua::Error::runtime("invalid affinity index"));
		}
	};
	Ok(())
}

/// Initializes an effect with the given magnitude, or adds the magnitude to the effect if it already exists.
fn inflict(
	lua: &mlua::Lua,
	this: &Ref,
	(key, magnitude): (String, Option<u32>),
) -> mlua::Result<()> {
	let resources = lua
		.globals()
		.get::<mlua::Table>("package")?
		.get::<mlua::Table>("loaded")?
		.get::<resource::Handle>("resources")?;
	let status = resources
		.statuses
		.get(&key)
		.cloned()
		.map_err(mlua::Error::external)?;
	let mut entry = this.borrow_mut();
	let entry = entry
		.statuses
		.entry(key.into_boxed_str())
		.or_insert_with(|| status);
	if let Some(magnitude) = magnitude {
		entry.add_magnitude(magnitude);
	}
	Ok(())
}

use mlua::IntoLuaMulti;
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};

pub struct InlineRefCell;

impl<F: rkyv::Archive> ArchiveWith<RefCell<F>> for InlineRefCell {
	type Archived = F::Archived;
	type Resolver = F::Resolver;

	fn resolve_with(
		field: &RefCell<F>,
		resolver: Self::Resolver,
		out: rkyv::Place<Self::Archived>,
	) {
		(*field.borrow()).resolve(resolver, out);
	}
}

impl<F: rkyv::Serialize<S>, S: rkyv::rancor::Fallible + ?Sized> SerializeWith<RefCell<F>, S>
	for InlineRefCell
{
	#[inline]
	fn serialize_with(field: &RefCell<F>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
		(*field.borrow()).serialize(serializer)
	}
}

impl<F: rkyv::Archive, D: rkyv::rancor::Fallible + ?Sized>
	DeserializeWith<F::Archived, RefCell<F>, D> for InlineRefCell
where
	F::Archived: rkyv::Deserialize<F, D>,
{
	#[inline]
	fn deserialize_with(field: &F::Archived, deserializer: &mut D) -> Result<RefCell<F>, D::Error> {
		use rkyv::Deserialize;
		match field.deserialize(deserializer) {
			Ok(val) => Ok(RefCell::new(val)),
			Err(a) => Err(a),
		}
	}
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	mlua::FromLua,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
struct InnerRef(#[rkyv(with = InlineRefCell)] RefCell<character::Piece>);

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	mlua::FromLua,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Ref(Rc<InnerRef>);

impl Ref {
	pub fn new(character: character::Piece) -> Self {
		Self(Rc::new(InnerRef(RefCell::new(character))))
	}
}

impl PartialEq for Ref {
	fn eq(&self, other: &Self) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}

impl Eq for Ref {}

impl std::ops::Deref for Ref {
	type Target = RefCell<character::Piece>;

	fn deref(&self) -> &Self::Target {
		&self.0 .0
	}
}

impl mlua::UserData for Ref {
	fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
		macro_rules! get {
			($field:ident) => {
				fields.add_field_method_get(stringify!($field), |_, this| Ok(this.borrow().$field.clone()));
			};
			($($field:ident),+$(,)?) => {
				$( get! { $field } )+
			}
		}
		macro_rules! set {
			($field:ident) => {
				fields.add_field_method_set(stringify!($field), |_, this, value| {
					this.borrow_mut().$field = value;
					Ok(())
				});
			};
			($($field:ident),+$(,)?) => {
				$( set! { $field } )+
			}
		}
		fields.add_field_method_get("level", |_, this| Ok(this.borrow().sheet.level));
		fields.add_field_method_get("stats", |_, this| {
			this.borrow().stats().map_err(mlua::Error::runtime)
		});
		fields.add_field_method_get("alliance", |_, this| Ok(this.borrow().alliance as u32));
		get!(hp, sp, x, y);
		set!(hp, sp, x, y);
	}

	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		methods.add_function("attacks", |_, this: mlua::AnyUserData| {
			Ok((
				this.metatable()?.get::<mlua::Function>("__next_attack")?,
				this,
				mlua::Nil,
			))
		});
		methods.add_meta_method("__next_attack", |lua, this, index: mlua::Value| {
			let index = index.as_usize().unwrap_or(0);
			if let Some(attack) = this.borrow().sheet.attacks.get(index) {
				lua.pack_multi((index + 1, attack.clone()))
			} else {
				mlua::Nil.into_lua_multi(lua)
			}
		});

		methods.add_function("spells", |_, this: mlua::AnyUserData| {
			Ok((
				this.metatable()?.get::<mlua::Function>("__next_spell")?,
				this,
				mlua::Nil,
			))
		});
		methods.add_meta_method("__next_spell", |lua, this, index: mlua::Value| {
			let index = index.as_usize().unwrap_or(0);
			if let Some(spell) = this.borrow().sheet.spells.get(index) {
				lua.pack_multi((index + 1, spell.clone()))
			} else {
				mlua::Nil.into_lua_multi(lua)
			}
		});

		methods.add_method("replace_nouns", |_, this, s: String| {
			Ok(s.replace_nouns(&this.borrow().sheet.nouns))
		});
		methods.add_method(
			"replace_prefixed_nouns",
			|_, this, (prefix, string): (String, String)| {
				Ok(string.replace_prefixed_nouns(&this.borrow().sheet.nouns, &prefix))
			},
		);
		methods.add_method("force_level", |_, this, ()| {
			let level = &mut this.borrow_mut().sheet.level;
			*level = level.saturating_add(1);
			Ok(())
		});
		// TODO: Make these functions into Rust methods of Piece.
		methods.add_method("force_affinity", force_affinity);
		methods.add_method("inflict", inflict);
	}
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Piece {
	pub sheet: Sheet,

	pub hp: i32,
	pub sp: i32,

	pub statuses: HashMap<Box<str>, Status>,

	pub x: i32,
	pub y: i32,
	pub action_delay: Aut,
	pub player_controlled: bool,
	pub alliance: Alliance,
}

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
	pub fn new(sheet: Sheet) -> Self {
		let stats = sheet.stats();
		let hp = stats.heart as i32;
		let sp = stats.soul as i32;

		Self {
			sheet,
			hp,
			sp,
			statuses: HashMap::new(),
			x: 0,
			y: 0,
			action_delay: 0,
			player_controlled: false,
			alliance: Alliance::default(),
		}
	}

	pub fn new_turn(&mut self) {
		// Remove any status effects with the duration of one turn.
		self.statuses
			.retain(|_, status| !matches!(status.duration, status::Duration::Turn));
	}

	pub fn rest(&mut self) -> Result<()> {
		let stats = self.stats()?;
		self.restore_hp(stats.heart as u32 / 2)?;
		self.restore_sp(stats.soul as u32)?;
		// Remove any status effects lasting until the next rest.
		self.statuses
			.retain(|_, status| !matches!(status.duration, status::Duration::Rest));
		Ok(())
	}

	pub fn restore_hp(&mut self, amount: u32) -> Result<()> {
		self.hp = i32::min(self.hp + amount as i32, self.stats()?.heart as i32);
		Ok(())
	}

	pub fn restore_sp(&mut self, amount: u32) -> Result<()> {
		self.sp = i32::min(self.sp + amount as i32, self.stats()?.soul as i32);
		Ok(())
	}
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StatOutcomes {
	pub stats: Stats,
	pub buffs: Stats,
	pub debuffs: Stats,
}

impl Piece {
	pub fn stats(&self) -> Result<Stats> {
		self.stat_outcomes().map(|x| x.stats)
	}

	pub fn stat_outcomes(&self) -> Result<StatOutcomes> {
		let buffs = Stats::default();
		let mut debuffs = Stats::default();

		for debuff in self
			.statuses
			.values()
			.filter_map(|x| x.on_debuff().transpose())
		{
			debuffs = debuffs + debuff?;
		}

		let mut stats = self.sheet.stats();
		stats.heart = stats.heart.saturating_sub(debuffs.heart) + buffs.heart;
		stats.soul = stats.soul.saturating_sub(debuffs.soul) + buffs.soul;
		stats.power = stats.power.saturating_sub(debuffs.power) + buffs.power;
		stats.defense = stats.defense.saturating_sub(debuffs.defense) + buffs.defense;
		stats.magic = stats.magic.saturating_sub(debuffs.magic) + buffs.magic;
		stats.resistance = stats.resistance.saturating_sub(debuffs.resistance) + buffs.resistance;

		Ok(StatOutcomes {
			stats,
			buffs,
			debuffs,
		})
	}
}

/// Anything a character piece can "do".
///
/// This is the only way that character logic or player input should communicate with pieces.
/// The information here should be enough to perform the action, but in the event it isn't
/// (from an incomplete player input), an `ActionRequest` will be yielded to fill in the missing information.
#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, mlua::FromLua)]
pub enum Action {
	Wait(Aut),
	Move(i32, i32),
	Attack(resource::Attack, Value),
	Cast(resource::Spell, Value),
}

impl mlua::UserData for Action {}

pub struct ActionConstructor;

impl mlua::UserData for ActionConstructor {
	fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
		methods.add_function("wait", |_, time| Ok(Action::Wait(time)));
		methods.add_function("move", |_, (x, y)| Ok(Action::Move(x, y)));
		methods.add_function("attack", |_, (attack, args)| {
			Ok(Action::Attack(attack, args))
		});
		methods.add_function("cast", |_, (spell, args)| Ok(Action::Cast(spell, args)));
	}
}

#[derive(
	Copy,
	PartialEq,
	Eq,
	Clone,
	Debug,
	Default,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[repr(u32)]
pub enum Alliance {
	Friendly,
	#[default]
	Enemy,
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

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Sheet {
	pub icon: Box<str>,
	/// Note that this includes the character's name.
	pub nouns: Nouns,

	pub level: u16,
	#[serde(default)] // There's no reason for most sheets to care about this.
	pub experience: u32,

	pub bases: Stats,
	pub growths: Stats,
	#[serde(default = "growth_bonuses")]
	pub growth_bonuses: Stats,

	pub skillset: spell::Skillset,
	pub speed: Aut,

	pub attacks: Vec<resource::Attack>,
	pub spells: Vec<resource::Spell>,

	/// Script to decide on an action from a list of considerations
	pub on_consider: Box<str>,
}

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

fn as_table(lua: &mlua::Lua, this: &mut Stats, _: ()) -> mlua::Result<mlua::Table> {
	lua.create_table_from([
		("heart", this.heart),
		("soul", this.soul),
		("power", this.power),
		("defense", this.defense),
		("magic", this.magic),
		("resistance", this.resistance),
	])
}

#[derive(
	Clone,
	Copy,
	Debug,
	Default,
	serde::Serialize,
	serde::Deserialize,
	alua::UserData,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[alua(method = as_table)]
pub struct Stats {
	/// Health, or HP; Heart Points
	#[serde(default)]
	#[alua(get)]
	pub heart: u16,
	/// Magic, or SP; Soul Points
	#[serde(default)]
	#[alua(get)]
	pub soul: u16,
	/// Bonus damage applied to physical attacks.
	#[serde(default)]
	#[alua(get)]
	pub power: u16,
	/// Damage reduction when recieving physical attacks.
	#[serde(default)]
	#[alua(get)]
	pub defense: u16,
	/// Bonus damage applied to magical attacks.
	#[serde(default)]
	#[alua(get)]
	pub magic: u16,
	/// Damage reduction when recieving magical attacks.
	/// Also makes harmful spells more likely to fail.
	#[serde(default)]
	#[alua(get)]
	pub resistance: u16,
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

impl std::ops::Mul<u16> for Stats {
	type Output = Stats;

	fn mul(self, rhs: u16) -> Self {
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

impl std::ops::Div<u16> for Stats {
	type Output = Stats;

	fn div(self, rhs: u16) -> Self {
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
