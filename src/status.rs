use crate::prelude::*;
use mlua::LuaSerdeExt;

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub enum Duration {
	Rest,
	Turn,
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
pub struct Debuff {
	#[serde(skip)]
	magnitude: u32,
	on_debuff: String,
}

impl Debuff {
	pub fn get(&self) -> Result<character::Stats> {
		thread_local! { static LUA: mlua::Lua = mlua::Lua::new() }
		LUA.with(|lua| {
			lua.globals().set("magnitude", self.magnitude)?;
			let stats = lua.from_value(lua.load(&self.on_debuff).eval()?)?;
			Ok(stats)
		})
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
pub enum Effect {
	StaticDebuff(character::Stats),
	Debuff(Debuff),
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
pub struct Status {
	pub name: String,
	pub icon: String,
	pub duration: Duration,
	pub effect: Effect,
}

impl Status {
	pub fn add_magnitude(&mut self, amount: u32) {
		match &mut self.effect {
			Effect::Debuff(Debuff { magnitude, .. }) => {
				*magnitude = magnitude.saturating_add(amount)
			}
			Effect::StaticDebuff(_) => {
				warn!(name = self.name, "increased magnitude of a static debuff");
			}
		}
	}

	pub fn on_debuff(&self) -> Result<Option<character::Stats>> {
		match &self.effect {
			Effect::Debuff(debuff) => debuff.get().map(Some),
			Effect::StaticDebuff(debuff) => Ok(Some(*debuff)),
		}
	}
}

impl mlua::UserData for Status {}
