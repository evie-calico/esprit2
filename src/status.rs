use std::cell::Cell;

use crate::prelude::*;
use mlua::LuaSerdeExt;
use tracing::{error, warn};

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
	StaticDebuff(character::Stats),
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
			Effect::StaticDebuff(_) => {
				warn!(
					"attempted to increase the magnitude of \"{}\" but it had none",
					self.name
				);
			}
		}
	}

	pub fn on_debuff(&self) -> Option<character::Stats> {
		match &self.effect {
			Effect::Debuff(debuff) => debuff.get(),
			Effect::StaticDebuff(debuff) => Some(*debuff),
		}
	}

	pub fn tip(&self) -> String {
		use std::fmt::Write;

		fn print_stats(tip: &mut String, stats: &character::Stats) {
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

		let mut tip = self.name.to_string();

		match &self.effect {
			Effect::Debuff(debuff) => {
				if let Some(stats) = debuff.get() {
					print_stats(&mut tip, &stats);
				}
			}
			Effect::StaticDebuff(stats) => print_stats(&mut tip, stats),
		}

		tip
	}

	pub fn color(&self) -> (u8, u8, u8, u8) {
		match &self.effect {
			Effect::Debuff(_) | Effect::StaticDebuff(_) => (255, 0, 0, 255),
		}
	}
}

impl mlua::UserData for Status {}
