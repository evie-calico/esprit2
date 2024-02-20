use crate::{character, item};
use grid::{grid, Grid};
use uuid::Uuid;

/// This struct contains all information that is relevant during gameplay.
#[derive(Clone, Debug)]
pub struct Manager {
    // I know I'm going to have to change this in the future to add multiple worlds.
    /// Where in the world the characters are.
    pub location: Location,
    /// This is the level pointed to by `location.level`.
    pub current_level: Level,
    /// Always point to the party's pieces, even across floors.
    /// When exiting a dungeon, these sheets will be saved to a party struct.
    // TODO: It might be useful to store a party ID too,
    // so that multiple party members of the same species can be differentiated.
    pub party: Vec<Uuid>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Level {
    pub name: String,
    pub floors: Vec<Floor>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            name: String::from("New Level"),
            floors: vec![Floor::default()],
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Location {
    /// Which level is currently loaded.
    ///
    /// This is usually implicit (see Manager.current_level),
    /// But storing it is important for serialization.
    pub level: String,
    pub floor: usize,
}

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

impl Manager {
    pub fn get_floor(&self) -> &Floor {
        &self.current_level.floors[self.location.floor]
    }

    pub fn get_floor_mut(&mut self) -> &mut Floor {
        &mut self.current_level.floors[self.location.floor]
    }

    // Returns none if no entity with the given uuid is currently loaded.
    // This either mean they no longer exist, or they're on a different floor;
    // either way they cannot be referenced.
    pub fn get_character(&self, id: Uuid) -> Option<&character::Piece> {
        self.get_floor().characters.iter().find(|x| x.id == id)
    }

    // Returns none if no entity with the given uuid is currently loaded.
    // This either means they no longer exist, or they're on a different floor;
    // either way they cannot be referenced.
    pub fn get_character_mut(&mut self, id: Uuid) -> Option<&mut character::Piece> {
        self.get_floor_mut()
            .characters
            .iter_mut()
            .find(|x| x.id == id)
    }

    pub fn next_character(&mut self) -> &mut character::Piece {
        &mut self.get_floor_mut().characters[0]
    }
}
