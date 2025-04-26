use crate::prelude::*;
use mlua::IntoLuaMulti;
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
		std::ptr::eq(self.as_ptr(), other.as_ptr())
	}
}

impl Eq for Ref {}

impl std::ops::Deref for Ref {
	type Target = RefCell<character::Piece>;

	fn deref(&self) -> &Self::Target {
		&self.0 .0
	}
}

// TODO: Use `try_borrow(_mut)?` methods to catch immutability violations in scripts (such as `Ability`'s usable)
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
		fields.add_field_method_get("stats", |lua, this| {
			this.borrow().stats(lua).map_err(mlua::Error::runtime)
		});
		get!(hp, sp, x, y);
		set!(hp, sp, x, y);
	}

	fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
		methods.add_meta_method("__eq", |_, this, other: Ref| Ok(*this == other));

		methods.add_function("abilities", |_, this: mlua::AnyUserData| {
			Ok((
				this.metatable()?.get::<mlua::Function>("__next_ability")?,
				this,
				mlua::Nil,
			))
		});
		methods.add_meta_method("__next_ability", |lua, this, index: mlua::Value| {
			let index = index.as_usize().unwrap_or(0);
			if let Some(ability) = this.borrow().sheet.abilities.get(index) {
				lua.pack_multi((index + 1, ability.clone()))
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
		methods.add_method(
			"attach",
			|lua, this, (component_id, value): (Box<str>, Value)| {
				let resources = lua
					.globals()
					.get::<mlua::Table>("package")?
					.get::<mlua::Table>("loaded")?
					.get::<resource::Handle>("runtime.resources")?;
				let component = resources
					.component
					.get(&component_id)
					.map_err(mlua::Error::external)?;
				let previous = this.borrow_mut().components.insert(component_id, value);
				if let Some(on_attach) = &component.on_attach {
					on_attach.call::<()>((this.clone(), previous))?;
				}
				Ok(())
			},
		);
		methods.add_method("component", |lua, this, component_id: mlua::String| {
			this.borrow()
				.components
				.get(component_id.to_str()?.as_ref())
				.map(|x| x.as_lua(lua))
				.transpose()
		});
		methods.add_method(
			"detach",
			|lua, this, (component_id, annotation): (mlua::String, mlua::Value)| {
				let resources = lua
					.globals()
					.get::<mlua::Table>("package")?
					.get::<mlua::Table>("loaded")?
					.get::<resource::Handle>("runtime.resources")?;
				let component_id = component_id.to_str()?;
				let component = resources
					.component
					.get(component_id.as_ref())
					.map_err(mlua::Error::external)?;
				let previous = this.borrow_mut().components.remove(component_id.as_ref());
				if let Some(on_detach) = &component.on_detach {
					on_detach.call::<()>((this.clone(), previous, annotation))?;
				}
				Ok(())
			},
		)
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
	pub components: HashMap<Box<str>, Value>,

	/// How much time has to pass until the piece is allowed to take an action.
	///
	/// This implies that every piece is able to take an action which may or may not be true,
	/// but in the event that one shoudn't, a consideration script which always skips the
	/// piece's turn should be sufficient.
	pub action_delay: Aut,
}

// Don't add stupid methods to this!
// Anything useful should be operating on a Ref!!!
impl Piece {
	pub fn new(sheet: Sheet) -> Self {
		let hp = sheet.stats.heart as i32;
		let sp = sheet.stats.soul as i32;

		Self {
			sheet,
			hp,
			sp,
			components: HashMap::new(),
			x: 0,
			y: 0,
			action_delay: 0,
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct StatOutcomes {
	pub stats: Stats,
	pub buffs: Stats,
	pub debuffs: Stats,
}

impl Piece {
	pub fn stats(&self, lua: &mlua::Lua) -> mlua::Result<Stats> {
		self.stat_outcomes(lua).map(|x| x.stats)
	}

	pub fn stat_outcomes(&self, lua: &mlua::Lua) -> mlua::Result<StatOutcomes> {
		let buffs = Stats::default();
		let mut debuffs = Stats::default();
		let resources: resource::Handle =
			lua.load(mlua::chunk!(require "runtime.resources")).eval()?;

		for (component_id, value) in &self.components {
			if let Ok(component) = resources.component.get(component_id.as_ref())
				&& let Some(on_debuff) = &component.on_debuff
			{
				let debuff = on_debuff.call(value.as_lua(lua)?)?;
				debuffs = debuffs + debuff;
			}
		}

		let mut stats = self.sheet.stats;
		stats.heart = stats
			.heart
			.saturating_sub(debuffs.heart)
			.saturating_add(buffs.heart);
		stats.soul = stats
			.soul
			.saturating_sub(debuffs.soul)
			.saturating_add(buffs.soul);
		stats.power = stats
			.power
			.saturating_sub(debuffs.power)
			.saturating_add(buffs.power);
		stats.defense = stats
			.defense
			.saturating_sub(debuffs.defense)
			.saturating_add(buffs.defense);
		stats.magic = stats
			.magic
			.saturating_sub(debuffs.magic)
			.saturating_add(buffs.magic);
		stats.resistance = stats
			.resistance
			.saturating_sub(debuffs.resistance)
			.saturating_add(buffs.resistance);

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
	Move(i32, i32),
	Ability(Box<str>, Value),
}

impl mlua::UserData for Action {}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Sheet {
	/// The identifier of the resource this sheet originated from.
	/// The sheet this refers to is not guaranteed to be the same as the sheet is is a member of,
	/// since a piece's sheet may mutate over the course of a game.
	pub id: Box<str>,

	/// Note that this includes the character's name.
	pub nouns: Nouns,

	pub stats: Stats,

	pub abilities: Vec<Box<str>>,

	/// Script to decide on an action from a list of considerations
	pub on_consider: Box<str>,
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
