use std::fmt;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, mlua::FromLua)]
#[serde(tag = "type")]
pub enum Log {
	/// An attack that dealt damage
	Hit { damage: u32 },
	/// An attack that succeeded
	Success,
	/// An attack that failed to do damage.
	Miss,
	/// An attack that dealt too little damage to pierce.
	Glance,
}

impl mlua::UserData for Log {}

impl fmt::Display for Log {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Log::Hit { damage } => write!(f, "-{damage} HP"),
			Log::Success => write!(f, "Success"),
			Log::Miss => write!(f, "Miss"),
			Log::Glance => write!(f, "Glancing Blow"),
		}
	}
}

impl Log {
	pub fn is_weak(&self) -> bool {
		match self {
			Log::Hit { .. } | Log::Success => false,
			Log::Miss | Log::Glance => true,
		}
	}
}

pub struct LogConstructor;

impl mlua::UserData for LogConstructor {
	fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
		macro_rules! units {
			($unit:ident) => {
				fields.add_field(stringify!($unit), Log::$unit);
			};
		}
		units!(Success);
		units!(Miss);
		units!(Glance);
	}

	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
		methods.add_function("Hit", |_, damage| Ok(Log::Hit { damage }));
	}
}
