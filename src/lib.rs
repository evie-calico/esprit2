#![feature(
	ascii_char,
	path_file_prefix,
	let_chains,
	once_cell_try,
	maybe_uninit_fill,
	anonymous_lifetime_in_impl_trait
)]

pub mod astar;
pub mod attack;
pub mod character;
pub mod combat;
pub mod consider;
pub mod console;
pub mod expression;
pub mod floor;
pub mod item;
pub mod nouns;
pub mod resource;
pub mod soul;
pub mod spell;
pub mod status;
pub mod vault;
pub mod world;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Io(#[from] std::io::Error),
	#[error(transparent)]
	Toml(#[from] toml::de::Error),
	#[error(transparent)]
	Lua(#[from] mlua::Error),

	#[error("lua function requested user input when it was unavailable")]
	IllegalActionRequest,

	#[error("{0}")]
	Sdl(String),

	#[error(transparent)]
	Vault(#[from] vault::Error),
	#[error(transparent)]
	Resource(#[from] resource::Error),
	#[error(transparent)]
	Expression(#[from] expression::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Arbitrary Unit of Time.
pub type Aut = u32;
/// The length of a "turn".
///
/// This is arbitrary, but it effectively makes Auts a fixed-point fraction,
/// which is useful for dividing by common values like 2, 3, 4, and 6.
// 12 is divisible by lots of nice numbers!
pub const TURN: Aut = 12;
/// For diagonal movement.
/// sqrt(2) * 12 = 16.9705627485, which we round to 17.
pub const SQRT2_TURN: Aut = 17;

pub type Color = (u8, u8, u8, u8);

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DirectionType {
	Cardinal,
	Ordinal,
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum CardDir {
	Up,
	Right,
	Down,
	Left,
}

impl CardDir {
	pub fn all() -> impl Iterator<Item = Self> {
		[CardDir::Up, CardDir::Right, CardDir::Down, CardDir::Left].into_iter()
	}

	pub fn as_offset(self) -> (i32, i32) {
		match self {
			CardDir::Up => (0, -1),
			CardDir::Right => (1, 0),
			CardDir::Down => (0, 1),
			CardDir::Left => (-1, 0),
		}
	}

	pub fn from_offset(x: i32, y: i32) -> Option<Self> {
		match (x, y) {
			(0, -1) => Some(CardDir::Up),
			(1, 0) => Some(CardDir::Right),
			(0, 1) => Some(CardDir::Down),
			(-1, 0) => Some(CardDir::Left),
			_ => None,
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
	pub fn all() -> impl Iterator<Item = Self> {
		[
			OrdDir::Up,
			OrdDir::UpRight,
			OrdDir::Right,
			OrdDir::DownRight,
			OrdDir::Down,
			OrdDir::DownLeft,
			OrdDir::Left,
			OrdDir::UpLeft,
		]
		.into_iter()
	}

	pub fn as_offset(self) -> (i32, i32) {
		match self {
			OrdDir::Up => (0, -1),
			OrdDir::UpRight => (1, -1),
			OrdDir::Right => (1, 0),
			OrdDir::DownRight => (1, 1),
			OrdDir::Down => (0, 1),
			OrdDir::DownLeft => (-1, 1),
			OrdDir::Left => (-1, 0),
			OrdDir::UpLeft => (-1, -1),
		}
	}

	pub fn from_offset(x: i32, y: i32) -> Option<Self> {
		match (x, y) {
			(0, -1) => Some(OrdDir::Up),
			(1, -1) => Some(OrdDir::UpRight),
			(1, 0) => Some(OrdDir::Right),
			(1, 1) => Some(OrdDir::DownRight),
			(0, 1) => Some(OrdDir::Down),
			(-1, 1) => Some(OrdDir::DownLeft),
			(-1, 0) => Some(OrdDir::Left),
			(-1, -1) => Some(OrdDir::UpLeft),
			_ => None,
		}
	}
}

pub mod prelude {
	pub use super::*;

	// Import redundant module::Struct names.
	pub use attack::Attack;
	pub use consider::Consider;
	pub use expression::Expression;
	pub use floor::Floor;
	pub use item::Item;
	pub use nouns::Nouns;
	pub use soul::Soul;
	pub use spell::Spell;
	pub use status::Status;
	pub use vault::Vault;

	// Export common traits
	pub use console::Handle;
	pub use expression::Evaluate;
	pub use nouns::StrExt;
	pub use rand::Rng;

	pub use tracing::{debug, error, info, warn};
}
