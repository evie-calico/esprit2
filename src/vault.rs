use crate::floor::Tile;
use crate::prelude::*;

pub struct Set {
	pub vaults: Vec<resource::Vault>,
	/// Nodes per floor
	pub density: u32,
	/// ratio of halls to vaults.
	pub hall_ratio: i32,
}

#[derive(Clone, Debug)]
pub struct Vault {
	pub tiles: Vec<Option<Tile>>,
	pub width: usize,

	pub characters: Vec<(i32, i32, resource::Sheet)>,
	pub edges: Vec<(i32, i32)>,
}

#[derive(Clone, Debug, mlua::FromLua)]
pub enum SymbolMeaning {
	Tile(Tile),
	Character { sheet: resource::Sheet, tile: Tile },
	Edge,
	Void,
}

impl mlua::UserData for SymbolMeaning {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("vault is missing a layout section")]
	MissingLayout,
	#[error("unexpected symbol: {0}")]
	UnexpectedSymbol(char),
}

impl Vault {
	pub fn height(&self) -> usize {
		self.tiles.len() / self.width
	}

	/// # Errors
	///
	/// Returns an error if the file could not be opened or parsed.
	pub fn parse(
		source: &str,
		symbols: impl Iterator<Item = &(char, SymbolMeaning)> + Clone,
	) -> Result<Self, Error> {
		let lines = source.lines().skip_while(|line| line.is_empty());
		let width = lines.clone().fold(0, |width, line| width.max(line.len()));

		let mut tiles = Vec::new();
		let mut characters = Vec::new();
		let mut edges = Vec::new();

		for (y, line) in lines.enumerate() {
			for (x, c) in line.chars().enumerate() {
				let default_action = match c {
					// This should define symbols for all Tile variants.
					'.' => Some(SymbolMeaning::Tile(Tile::Floor)),
					'x' => Some(SymbolMeaning::Tile(Tile::Wall)),
					'>' => Some(SymbolMeaning::Tile(Tile::Exit)),
					// ...and for all unit SymbolMeaning variants.
					' ' => Some(SymbolMeaning::Void),
					'E' => Some(SymbolMeaning::Edge),
					_ => None,
				};
				if let Some(action) = symbols
					.clone()
					.find_map(|x| if x.0 == c { Some(&x.1) } else { None })
					.or(default_action.as_ref())
				{
					match action {
						SymbolMeaning::Edge => edges.push((x as i32, y as i32)),
						SymbolMeaning::Character { sheet, tile: _ } => {
							characters.push((x as i32, y as i32, sheet.clone()))
						}
						_ => {}
					}
					tiles.push(match action {
						SymbolMeaning::Edge | SymbolMeaning::Void => None,
						SymbolMeaning::Tile(t) => Some(*t),
						SymbolMeaning::Character { sheet: _, tile } => Some(*tile),
					});
				} else {
					Err(Error::UnexpectedSymbol(c))?
				}
			}
			for _ in 0..(width - line.len()) {
				tiles.push(None);
			}
		}

		Ok(Self {
			tiles,
			width,
			characters,
			edges,
		})
	}
}
