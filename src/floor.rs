use grid::Grid;
use tracing::warn;

use crate::vault::Vault;

// Keeping this very light is probably a good idea.
// Decorations, like statues and fountains and such, are sporadic and should be stored seperately.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Tile {
	Floor,
	#[default]
	Wall,
	Exit,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Floor {
	pub map: Grid<Tile>,
}

impl Default for Floor {
	fn default() -> Self {
		Self {
			// TODO: Decide default grid size.
			// 32x32 is ¼ the size of Esprit 1 (64x64)
			map: Grid::init(32, 32, Tile::Floor),
		}
	}
}

impl Floor {
	pub fn blit_vault(&mut self, mut x: usize, mut y: usize, vault: &Vault) -> bool {
		let mut in_bounds = true;
		for row in vault.tiles.chunks(vault.width) {
			for tile in row {
				if let Some(tile) = tile {
					if let Some(dest) = self.map.get_mut(y, x) {
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
