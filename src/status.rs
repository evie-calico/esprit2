use std::cell::Cell;

use crate::prelude::*;
use mlua::LuaSerdeExt;
use tracing::error;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Duration {
	Rest,
	Turn,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Debuff {
	#[serde(skip)]
	magnitude: u32,
	on_debuff: script::MaybeInline,

	#[serde(skip)]
	cache: Cell<Option<(u32, character::Stats)>>,
}

impl Debuff {
	fn get_script(&self) -> Result<character::Stats> {
		// TODO: OnceCell
		let lua = mlua::Lua::new();
		lua.globals().set("magnitude", self.magnitude)?;
		let stats = lua.from_value(lua.load(self.on_debuff.contents()).eval()?)?;
		Ok(stats)
	}

	fn get(&self) -> Option<character::Stats> {
		if let Some((last_magnitude, cache)) = self.cache.get() {
			if self.magnitude == last_magnitude {
				return Some(cache);
			}
		}
		let stats = self
			.get_script()
			.map_err(|msg| error!("failed to calculate debuff: {msg}"))
			.unwrap_or_default();
		self.cache.set(Some((self.magnitude, stats)));
		Some(stats)
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum Effect {
	Debuff(Debuff),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Status {
	pub name: String,
	pub duration: Duration,
	effect: Effect,
}

impl Status {
	pub fn add_magnitude(&mut self, amount: u32) {
		match &mut self.effect {
			Effect::Debuff(Debuff { magnitude, .. }) => {
				*magnitude = magnitude.saturating_add(amount)
			}
		}
	}

	pub fn on_debuff(&self) -> Option<character::Stats> {
		match &self.effect {
			Effect::Debuff(debuff) => debuff.get(),
		}
	}

	pub fn tip(&self) -> String {
		use std::fmt::Write;

		let mut tip = self.name.to_string();

		match &self.effect {
			Effect::Debuff(debuff) => {
				if let Some(stats) = debuff.get() {
					for (name, value) in [
						("Heart", stats.heart),
						("Soul", stats.soul),
						("Power", stats.power),
						("Defense", stats.defense),
						("Magic", stats.magic),
						("Resistance", stats.resistance),
					] {
						if value > 0 {
							let _ = write!(tip, " -{value} {name}");
						}
					}
				}
			}
		}

		tip
	}
}

impl mlua::UserData for Status {}
