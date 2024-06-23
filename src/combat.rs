use std::fmt;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Log {
	Hit { magnitude: u32, damage: u32 },
}

impl fmt::Display for Log {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Log::Hit {
				magnitude: _,
				damage,
			} => write!(f, "-{damage} HP"),
		}
	}
}

impl Log {
	pub fn is_weak(&self) -> bool {
		match self {
			Log::Hit {
				magnitude: _,
				damage,
			} => *damage <= 1,
		}
	}
}
