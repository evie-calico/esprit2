use crate::prelude::*;

#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	Debug,
	Default,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[archive(check_bytes)]
// Keeping this very light is probably a good idea.
// Decorations, like statues and fountains and such, are sporadic and should be stored seperately.
// Don't go over 255 variants (reserve one for Option::None), and don't add members; they'll bloat the size of the map.
#[repr(u8)]
pub enum Tile {
	Floor,
	#[default]
	Wall,
	Exit,
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[archive(check_bytes)]
pub struct Floor {
	pub width: usize,
	pub map: Box<[Option<Tile>]>,
}

impl Default for Floor {
	fn default() -> Self {
		Self {
			// TODO: Decide default grid size.
			width: 32,
			map: Box::new([None; 32 * 32]),
		}
	}
}

impl Floor {
	pub fn get(&self, x: usize, y: usize) -> Option<Tile> {
		if x >= self.width() || y >= self.height() {
			return None;
		}
		self.map.get(x + y * self.width()).copied()?
	}

	pub fn set(&mut self, x: usize, y: usize, tile: impl Into<Option<Tile>>) {
		if x >= self.width() || y >= self.height() {
			warn!("vaults cannot be placed at negative coordinates yet");
		} else if let Some(dest) = self.map.get_mut(x + y * self.width()) {
			*dest = tile.into();
		} else {
			panic!("attempted to place a tile out of bounds");
		}
	}

	pub fn width(&self) -> usize {
		self.width
	}

	pub fn height(&self) -> usize {
		self.map.len() / self.width
	}

	pub fn iter_tiles(&self) -> impl Iterator<Item = (usize, usize, Tile)> + '_ {
		self.iter_grid()
			.filter_map(|(x, y, t)| t.map(|t| (x, y, t)))
	}

	pub fn iter_grid(&self) -> impl Iterator<Item = (usize, usize, Option<Tile>)> + '_ {
		self.map
			.iter()
			.copied()
			.enumerate()
			.map(|(i, t)| (i % self.width, i / self.width, t))
	}
}
