use grid::Grid;

use crate::vault::Vault;

// Keeping this very light is probably a good idea.
// Decorations, like statues and fountains and such, are sporadic and should be stored seperately.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Tile {
	Floor,
	#[default]
	Wall,
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
	pub fn blit_vault(&mut self, mut x: usize, mut y: usize, vault: &Vault) {
		for row in vault.tiles.chunks(vault.width) {
			for tile in row {
				if let Some(tile) = tile {
					*self.map.get_mut(x, y).unwrap() = *tile;
				}
				y += 1;
			}
			y -= vault.width;
			x += 1;
		}
	}
}
