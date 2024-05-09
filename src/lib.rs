#![feature(
	ascii_char,
	path_file_prefix,
	lint_reasons,
	lazy_cell,
	let_chains,
	once_cell_try
)]
#![warn(
	clippy::missing_errors_doc,
	clippy::module_name_repetitions,
	clippy::items_after_statements,
	clippy::inconsistent_struct_constructor
)]

pub mod attack;
pub mod character;
pub mod console;
pub mod expression;
pub mod floor;
pub mod gui;
pub mod input;
pub mod item;
pub mod nouns;
pub mod options;
pub mod resource_manager;
pub mod soul;
pub mod spell;
pub mod spell_menu;
pub mod vault;
pub mod world;

/// Arbitrary Unit of Time.
type Aut = u32;
/// The length of a "turn".
///
/// This is arbitrary, but it effectively makes Auts a fixed-point fraction,
/// which is useful for dividing by common values like 2, 3, 4, and 6.
// 12 is divisible by lots of nice numbers!
#[allow(unused)] // I'm not using this anywhere yet, but it's useful to have written down.
const TURN: Aut = 12;

pub mod prelude {
	pub use super::*;
	pub use attack::Attack;
	pub use console::Console;
	pub use expression::{Evaluate, Expression};
	pub use floor::Floor;
	pub use item::Item;
	pub use nouns::Nouns;
	pub use options::Options;
	pub use resource_manager::ResourceManager;
	pub use soul::Soul;
	pub use spell::Spell;
	pub use vault::Vault;
}
