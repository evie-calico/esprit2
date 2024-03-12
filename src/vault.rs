use crate::floor::Tile;
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct Vault {
	pub tiles: Vec<Tile>,
	pub width: usize,
}

impl Vault {
	pub fn open(path: impl AsRef<Path>) -> Self {
		let mut width = 0;

		let vault_text = fs::read_to_string(path).unwrap();

		// Before we can do anything, we need to know how wide this vault is.
		for line in vault_text.lines() {
			width = width.max(line.len());
		}

		let mut tiles = Vec::new();

		for line in vault_text.lines() {
			for c in line.chars() {
				tiles.push(match c {
					' ' | '.' => Tile::Floor,
					'x' => Tile::Wall,
					_ => todo!(),
				});
			}
			for _ in 0..(width - line.len()) {
				// TODO: this should be None. vaults don't have to be square.
				tiles.push(Tile::Floor);
			}
		}

		Self { tiles, width }
	}
}
