use crate::prelude::*;
use mlua::IntoLuaMulti;
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
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

#[derive(Clone, Debug, mlua::FromLua, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct InnerRef(#[rkyv(with = InlineRefCell)] RefCell<character::Piece>);

#[derive(Clone, Debug, mlua::FromLua, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
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
		fields.add_field_method_get("stats", |lua, this| {
			this.borrow().stats(lua).map_err(mlua::Error::runtime)
		});
		get!(hp, sp, x, y);
		set!(hp, sp, x, y);
	}

	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		methods.add_meta_method("__eq", |_, this, other: Ref| Ok(*this == other));
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
		methods.add_method("force_affinity", force_affinity);
		methods.add_method("attach", |lua, this, (status, value): (Box<str>, Value)| {
			let resources = lua
				.globals()
				.get::<mlua::Table>("package")?
				.get::<mlua::Table>("loaded")?
				.get::<resource::Handle>("esprit.resources")?;
			this.borrow_mut()
				.attach(status, value, &resources, lua)
				.map_err(mlua::Error::external)?;
			Ok(())
		});
		methods.add_method("component", |lua, this, status: mlua::String| {
			this.borrow_mut()
				.statuses
				.get(status.to_str()?.as_ref())
				.map(|x| x.as_lua(lua))
				.transpose()
		});
		methods.add_method("detach", |lua, this, status: mlua::String| {
			let resources = lua
				.globals()
				.get::<mlua::Table>("package")?
				.get::<mlua::Table>("loaded")?
				.get::<resource::Handle>("esprit.resources")?;
			this.borrow_mut()
				.detach(status.to_str()?.as_ref(), &resources, lua)
				.map_err(mlua::Error::external)
		});
	}
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Piece {
	/// Persistent information about the piece.
	///
	/// This represents fields which should be preserved even when the piece is not present on a board,
	/// such as the resource manager, or a party member that is saved but not "in play".
	pub sheet: Sheet,

	pub x: i32,
	pub y: i32,

	/// How far the piece is from being in a "destroyed" state.
	///
	/// This doesn't imply removal from the board, and probably shouldn't!
	/// Enemies might flee until they reach a destination and are removed from the board,
	/// or turn into a corpse object that rots over time or can be destroyed/animated with magic.
	///
	/// For pieces which should not have any health, `sheet.stats().hp` should be 0.
	/// Otherwise health is always displayed, even if `hp` is <= 0.
	///
	/// Should the component system get more mature,
	/// HP and SP might be good candidates for removal from this structure,
	/// though that would involve reconciling the existence of `sheet` too.
	/// I think it's pretty safe to say that these are fundamental enough
	/// that they deserve status as static fields.
	pub hp: i32,
	pub sp: i32,

	/// Additional components of the piece with optional data.
	// TODO: Should this go on sheet? Maybe Duration should detemrine that.
	pub statuses: HashMap<Box<str>, Value>,

	/// How much time has to pass until the piece is allowed to take an action.
	///
	/// This implies that every piece is able to take an action which may or may not be true,
	/// but in the event that one shoudn't, a consideration script which always skips the
	/// piece's turn should be sufficient.
	pub action_delay: Aut,
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
		}
	}

	pub fn new_turn(&mut self, resources: &resource::Manager) {
		// Remove any status effects with the duration of one turn.
		self.statuses.retain(|k, _| {
			resources
				.statuses
				.get(k)
				.ok()
				.map(|status| !matches!(status.duration, status::Duration::Turn))
				.unwrap_or(false)
		});
	}

	pub fn rest(&mut self, resources: &resource::Manager, lua: &mlua::Lua) -> Result<()> {
		let stats = self.stats(lua)?;
		self.hp = i32::min(
			self.hp + (stats.heart as u32 / 2) as i32,
			stats.heart as i32,
		);
		self.sp = i32::min(self.sp + (stats.soul as u32) as i32, stats.soul as i32);
		// Remove any status effects lasting until the next rest.
		self.statuses.retain(|k, _| {
			resources
				.statuses
				.get(k)
				.ok()
				.map(|status| !matches!(status.duration, status::Duration::Rest))
				.unwrap_or(false)
		});
		Ok(())
	}

	pub fn attach(
		&mut self,
		status: Box<str>,
		value: Value,
		_resources: &resource::Manager,
		_lua: &mlua::Lua,
	) -> Result<()> {
		let is_new = !self.statuses.contains_key(&status);
		self.statuses.insert(status, value);
		if is_new {
			// TODO: on_attach
		} else {
			// TODO: on_reattach
		}
		Ok(())
	}

	pub fn detach(
		&mut self,
		status: &str,
		_resources: &resource::Manager,
		_lua: &mlua::Lua,
	) -> Result<Option<Value>> {
		let value = self.statuses.remove(status);
		// TODO: on_detach
		Ok(value)
	}
}

#[derive(Clone, Debug, Default)]
pub struct StatOutcomes {
	pub stats: Stats,
	pub buffs: Stats,
	pub debuffs: Stats,
}

impl Piece {
	pub fn stats(&self, lua: &mlua::Lua) -> Result<Stats> {
		self.stat_outcomes(lua).map(|x| x.stats)
	}

	pub fn stat_outcomes(&self, lua: &mlua::Lua) -> Result<StatOutcomes> {
		let buffs = Stats::default();
		let mut debuffs = Stats::default();
		let resources: resource::Handle =
			lua.load(mlua::chunk!(require "esprit.resources")).eval()?;

		for (status_id, value) in &self.statuses {
			if let Ok(status) = resources.statuses.get(status_id.as_ref())
				&& let Some(on_debuff) = &status.on_debuff
			{
				let debuff = on_debuff.call(value.as_lua(lua)?)?;
				debuffs = debuffs + debuff;
			}
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
	Attack(Box<str>, Value),
	Cast(Box<str>, Value),
}

impl mlua::UserData for Action {}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Sheet {
	pub icon: Box<str>,
	/// Note that this includes the character's name.
	pub nouns: Nouns,

	pub level: u16,
	pub experience: u32,

	pub bases: Stats,
	pub growths: Stats,
	pub growth_bonuses: Stats,

	pub skillset: spell::Skillset,
	pub speed: Aut,

	pub attacks: Vec<Box<str>>,
	pub spells: Vec<Box<str>>,

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

#[derive(
	Clone, Copy, Debug, Default, mlua::FromLua, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Stats {
	/// Health, or HP; Heart Points
	pub heart: u16,
	/// Magic, or SP; Soul Points
	pub soul: u16,
	/// Bonus damage applied to physical attacks.
	pub power: u16,
	/// Damage reduction when recieving physical attacks.
	pub defense: u16,
	/// Bonus damage applied to magical attacks.
	pub magic: u16,
	/// Damage reduction when recieving magical attacks.
	/// Also makes harmful spells more likely to fail.
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

impl mlua::UserData for Stats {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("heart", |_, this| Ok(this.heart));
		fields.add_field_method_get("soul", |_, this| Ok(this.soul));
		fields.add_field_method_get("power", |_, this| Ok(this.power));
		fields.add_field_method_get("defense", |_, this| Ok(this.defense));
		fields.add_field_method_get("magic", |_, this| Ok(this.magic));
		fields.add_field_method_get("resistance", |_, this| Ok(this.resistance));
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
