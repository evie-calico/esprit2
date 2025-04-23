#![feature(
	anonymous_lifetime_in_impl_trait,
	ascii_char,
	int_roundings,
	let_chains,
	maybe_uninit_fill,
	once_cell_try,
	path_file_prefix,
	try_blocks
)]

pub mod astar;
pub mod attack;
pub mod character;
pub mod combat;
pub mod component;
pub mod consider;
pub mod console;
pub mod expression;
pub mod floor;
pub mod item;
pub mod lua;
pub mod nouns;
pub mod resource;
pub mod spell;
pub mod value;
pub mod vault;
pub mod world;

// Deferring to anyhow feels unfortunate, but it's also usually *correct*.
// Most areas of the engine have to mix a few error types together,
// and want to provide a context trace for why an error occured.
// The only errors worth introspecting on (lua and resources) heavily benefit from a context trace,
// any anyhow lets you downcast if you really need to anyways.
pub use anyhow;

pub use value::Value;
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

#[derive(Copy, Clone, Debug)]
pub enum DirectionType {
	Cardinal,
	Ordinal,
}

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
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
	pub use component::Component;
	pub use consider::Consider;
	pub use expression::Expression;
	pub use floor::Floor;
	pub use item::Item;
	pub use nouns::Nouns;
	pub use spell::Spell;
	pub use vault::Vault;

	// Export common traits
	pub use console::Handle;
	pub use expression::Evaluate;
	pub use nouns::StrExt;

	pub use tracing::{debug, error, info, warn};
}
