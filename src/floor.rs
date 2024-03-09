use crate::prelude::*;
use crate::world::Tile;
use grid::{grid, Grid};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Floor {
	pub map: Grid<Tile>,
	// It might be useful to sort this by remaining action delay to make selecting the next character easier.
	pub characters: Vec<character::Piece>,
	pub items: Vec<item::Piece>,
}

impl Default for Floor {
	fn default() -> Self {
		// thanks rustfmt :/
		let map = grid![
			[Tile::Wall, Tile::Wall, Tile::Wall, Tile::Wall, Tile::Wall]
			[
				Tile::Wall,
				Tile::Floor,
				Tile::Floor,
				Tile::Floor,
				Tile::Wall
			]
			[
				Tile::Wall,
				Tile::Floor,
				Tile::Floor,
				Tile::Floor,
				Tile::Wall
			]
			[
				Tile::Wall,
				Tile::Floor,
				Tile::Floor,
				Tile::Floor,
				Tile::Wall
			]
			[Tile::Wall, Tile::Wall, Tile::Wall, Tile::Wall, Tile::Wall]
		];

		Self {
			map,
			characters: Vec::new(),
			items: Vec::new(),
		}
	}
}
