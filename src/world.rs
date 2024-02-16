use crate::{character, item};
use grid::{grid, Grid};

// Keeping this very light is probably a good idea.
// Decorations, like statues and fountains and such, are sporadic and should be stored seperately.
#[derive(PartialEq, Eq, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Tile {
    Floor,
    #[default]
    Wall,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Floor {
    pub map: Grid<Tile>,
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
