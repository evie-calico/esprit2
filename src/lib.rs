#![feature(path_file_prefix)]

pub mod character;
pub mod console;
pub mod gui;
pub mod item;
pub mod options;
pub mod resource_manager;
pub mod spell;
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
