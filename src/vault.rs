use crate::floor::Tile;
use std::{collections::HashMap, fs, path::Path};

#[derive(Clone, Debug)]
pub struct Vault {
	pub tiles: Vec<Tile>,
	pub width: usize,

	pub characters: Vec<(i32, i32, String)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SymbolMeaning {
	Tile(Tile),
	Character(String),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
	symbols: HashMap<char, SymbolMeaning>,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
	#[error("vault is missing a layout section")]
	MissingLayout,
	#[error("failed to parse metadata: {0}")]
	Toml(#[from] toml::de::Error),
}

impl Vault {
	pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
		let mut width = 0;

		let vault_text = fs::read_to_string(path).unwrap();

		let (metadata, layout) = vault_text
			.split_once("# Layout\n")
			.ok_or(Error::MissingLayout)?;

		let metadata: Metadata = toml::from_str(metadata)?;

		// Before we can do anything, we need to know how wide this vault is.
		for line in layout.lines() {
			width = width.max(line.len());
		}

		let mut tiles = Vec::new();
		let mut characters = Vec::new();

		for (y, line) in layout.lines().enumerate() {
			for (x, c) in line.chars().enumerate() {
				if let Some(action) = metadata.symbols.get(&c) {
					match action {
						SymbolMeaning::Tile(t) => tiles.push(*t),
						SymbolMeaning::Character(sheet) => {
							characters.push((x as i32, y as i32, sheet.clone()));
							tiles.push(Tile::Floor);
						}
					}
				} else {
					tiles.push(match c {
						' ' | '.' => Tile::Floor,
						'x' => Tile::Wall,
						_ => todo!(),
					});
				}
			}
			for _ in 0..(width - line.len()) {
				// TODO: this should be None. vaults don't have to be square.
				tiles.push(Tile::Floor);
			}
		}

		Ok(Self {
			tiles,
			width,
			characters,
		})
	}
}
