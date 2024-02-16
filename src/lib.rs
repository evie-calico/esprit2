pub mod character;
pub mod console;
pub mod gui;
pub mod item;
pub mod options;
pub mod spell;
pub mod world;

/// Arbitrary Unit of Time.
type Aut = u32;
// 12 is divisible by lots of nice numbers!
const TURN: Aut = 12;
