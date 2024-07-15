#![feature(ascii_char, path_file_prefix, let_chains, once_cell_try)]
#![warn(
	clippy::module_name_repetitions,
	clippy::items_after_statements,
	clippy::inconsistent_struct_constructor,
	clippy::unwrap_used
)]

pub mod astar;
pub mod attack;
pub mod character;
pub mod combat;
pub mod consider;
pub mod console;
pub mod draw;
pub mod expression;
pub mod floor;
pub mod gui;
pub mod input;
pub mod item;
pub mod nouns;
pub mod options;
pub mod resource;
pub mod script;
pub mod soul;
pub mod spell;
pub mod status;
pub mod typography;
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
type Aut = u32;
/// The length of a "turn".
///
/// This is arbitrary, but it effectively makes Auts a fixed-point fraction,
/// which is useful for dividing by common values like 2, 3, 4, and 6.
// 12 is divisible by lots of nice numbers!
const TURN: Aut = 12;
/// For diagonal movement.
/// sqrt(2) * 12 = 16.9705627485, which we round to 17.
const SQRT2_TURN: Aut = 17;

type Color = (u8, u8, u8, u8);

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
		let (x, y) = match self {
			OrdDir::Up => (0, -1),
			OrdDir::UpRight => (1, -1),
			OrdDir::Right => (1, 0),
			OrdDir::DownRight => (1, 1),
			OrdDir::Down => (0, 1),
			OrdDir::DownLeft => (-1, 1),
			OrdDir::Left => (-1, 0),
			OrdDir::UpLeft => (-1, -1),
		};
		(x, y)
	}
}

pub mod prelude {
	pub use super::*;

	// Import redundant module::Struct names.
	pub use attack::Attack;
	pub use consider::Consider;
	pub use console::Console;
	pub use expression::Expression;
	pub use floor::Floor;
	pub use item::Item;
	pub use nouns::Nouns;
	pub use options::Options;
	pub use script::Script;
	pub use soul::Soul;
	pub use spell::Spell;
	pub use status::Status;
	pub use typography::Typography;
	pub use vault::Vault;

	// Export common traits
	pub use expression::Evaluate;
	pub use rand::Rng;

	pub use tracing::{debug, error, info};
}
