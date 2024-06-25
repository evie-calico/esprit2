use std::fmt;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Log {
	/// An attack that dealt damage
	Hit { damage: u32 },
	/// An attack that failed to do damage.
	Miss,
	/// An attack that dealt too little damage to pierce.
	Glance,
}

impl fmt::Display for Log {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Log::Hit { damage } => write!(f, "-{damage} HP"),
			Log::Miss => write!(f, "Miss"),
			Log::Glance => write!(f, "Glancing Blow"),
		}
	}
}

impl Log {
	pub fn is_weak(&self) -> bool {
		match self {
			Log::Hit { .. } => false,
			Log::Miss | Log::Glance => true,
		}
	}
}
