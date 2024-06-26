#![feature(ascii_char, path_file_prefix, lint_reasons, let_chains, once_cell_try)]
#![warn(
	clippy::missing_errors_doc,
	clippy::module_name_repetitions,
	clippy::items_after_statements,
	clippy::inconsistent_struct_constructor
)]

pub mod attack;
pub mod character;
pub mod combat;
pub mod console;
pub mod draw;
pub mod expression;
pub mod floor;
pub mod gui;
pub mod input;
pub mod item;
pub mod nouns;
pub mod options;
pub mod resource_manager;
pub mod script;
pub mod soul;
pub mod spell;
pub mod spell_menu;
pub mod typography;
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

type Color = (u8, u8, u8, u8);

pub mod prelude {
	pub use super::*;

	// Import redundant module::Struct names.
	pub use attack::Attack;
	pub use console::Console;
	pub use expression::Expression;
	pub use floor::Floor;
	pub use item::Item;
	pub use nouns::Nouns;
	pub use options::Options;
	pub use resource_manager::ResourceManager;
	pub use script::Script;
	pub use soul::Soul;
	pub use spell::Spell;
	pub use typography::Typography;
	pub use vault::Vault;

	// Export common traits
	pub use expression::Evaluate;
	pub use rand::Rng;
}
