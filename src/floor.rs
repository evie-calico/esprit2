use crate::vault::Vault;
use tracing::warn;

// Keeping this very light is probably a good idea.
// Decorations, like statues and fountains and such, are sporadic and should be stored seperately.
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
pub struct Floor {
	pub width: usize,
	pub map: Box<[Tile]>,
}

impl Default for Floor {
	fn default() -> Self {
		Self {
			// TODO: Decide default grid size.
			width: 32,
			map: Box::new([Tile::Floor; 32 * 32]),
		}
	}
}

impl Floor {
	pub fn get(&self, x: usize, y: usize) -> Option<Tile> {
		self.map
			.get(x.checked_add(y.checked_mul(self.width)?)?)
			.copied()
	}

	pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
		self.map.get_mut(x.checked_add(y.checked_mul(self.width)?)?)
	}

	pub fn width(&self) -> usize {
		self.width
	}

	pub fn height(&self) -> usize {
		self.map.len() / self.width
	}

	pub fn iter(&self) -> impl Iterator<Item = (usize, usize, Tile)> + '_ {
		self.map
			.iter()
			.copied()
			.enumerate()
			.map(|(i, t)| (i % self.width, i / self.width, t))
	}

	pub fn blit_vault(&mut self, mut x: usize, mut y: usize, vault: &Vault) -> bool {
		let mut in_bounds = true;
		for row in vault.tiles.chunks(vault.width) {
			for tile in row {
				if let Some(tile) = tile {
					if let Some(dest) = self.get_mut(x, y) {
						*dest = *tile;
					} else if in_bounds {
						warn!("attempted to place vault out of bounds");
						in_bounds = false;
					}
				}
				x += 1;
			}
			x -= vault.width;
			y += 1;
		}
		in_bounds
	}
}
