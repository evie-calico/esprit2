use crate::vault::Vault;
use tracing::warn;

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

	pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
		if x >= self.width() || y >= self.height() {
			return None;
		}
		self.map.get_mut(x + y * self.width())?.as_mut()
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
