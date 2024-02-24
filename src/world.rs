use crate::character::OrdDir;
use crate::nouns::StrExt;
use crate::prelude::*;
use grid::{grid, Grid};
use rand::{seq::SliceRandom, thread_rng, Rng};
use uuid::Uuid;

const DEFAULT_ATTACK_MESSAGE: &str = "{self_Address} attacked {target_indirect}";

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
	pub party: Vec<PartyReference>,
	pub inventory: Vec<String>,
	pub console: Console,
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
pub struct PartyReference {
	/// The piece that is being used by this party member.
	pub piece: Uuid,
	/// This party member's ID within the party.
	/// Used for saving data.
	pub member: Uuid,
}

impl PartyReference {
	pub fn new(piece: Uuid, member: Uuid) -> Self {
		Self { piece, member }
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

macro_rules! get_floor_mut {
	($self:ident) => {
		&mut $self.current_level.floors[$self.location.floor]
	};
}
// Returns none if no entity with the given uuid is currently loaded.
// This either means they no longer exist, or they're on a different floor;
// either way they cannot be referenced.
macro_rules! get_character_mut {
	($self:ident, $id:expr) => {
		$self
			.get_floor_mut()
			.characters
			.iter_mut()
			.find(|x| x.id == $id)
	};
}

macro_rules! get_character_at_mut {
	($self:ident, $x:expr, $y:expr) => {
		get_floor_mut!($self)
			.characters
			.iter_mut()
			.find(|p| p.x == $x && p.y == $y)
	};
}

impl Manager {
	pub fn get_floor(&self) -> &Floor {
		&self.current_level.floors[self.location.floor]
	}

	pub fn get_floor_mut(&mut self) -> &mut Floor {
		get_floor_mut!(self)
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
		get_character_mut!(self, id)
	}

	pub fn next_character(&mut self) -> &mut character::Piece {
		&mut self.get_floor_mut().characters[0]
	}

	pub fn get_character_at(&self, x: i32, y: i32) -> Option<&character::Piece> {
		self.get_floor()
			.characters
			.iter()
			.find(|p| p.x == x && p.y == y)
	}

	pub fn get_character_at_mut(&mut self, x: i32, y: i32) -> Option<&mut character::Piece> {
		get_character_at_mut!(self, x, y)
	}
}

impl Manager {
	pub fn pop_action(&mut self) {
		let next_character = self.next_character();
		let next_character_id = next_character.id;

		let Some(action) = next_character.next_action.take() else {
			return;
		};
		match action {
			character::Action::Move(dir) => self.move_piece(next_character_id, dir),
		}
	}

	pub fn move_piece(&mut self, id: Uuid, dir: OrdDir) {
		let Some(character) = self.get_character(id) else {
			return;
		};
		let (x, y) = match dir {
			OrdDir::Up => (0, -1),
			OrdDir::UpRight => (1, -1),
			OrdDir::Right => (1, 0),
			OrdDir::DownRight => (1, 1),
			OrdDir::Down => (0, 1),
			OrdDir::DownLeft => (-1, 1),
			OrdDir::Left => (-1, 0),
			OrdDir::UpLeft => (-1, -1),
		};
		let x = character.x + x;
		let y = character.y + y;

		if let Some(target) = self.get_character_at(x, y) {
			if target.alliance != character.alliance {
				if let Some(mut attack) = character.attacks.first() {
					let mut rng = thread_rng();
					let max_attack_weight = character.attacks.iter().fold(0, |a, x| a + x.weight);
					let mut point = rng.gen_range(0..max_attack_weight);
					for i in &character.attacks {
						if point < i.weight {
							attack = i;
							break;
						} else {
							point -= i.weight;
						}
					}

					// Calculate damage
					let damage = 0.max(
						((character.sheet.stats.power + attack.bonus) as i32)
							- (target.sheet.stats.defense as i32),
					);
					let is_weak_attack = damage <= 1;

					// TODO: Change this depending on the proportional amount of damage dealt.
					let damage_punctuation = match damage {
						20.. => "!!!",
						10.. => "!!",
						5.. => "!",
						_ => ".",
					};
					let message_pool = attack
						.messages
						.low
						.as_ref()
						.filter(|_| is_weak_attack)
						.unwrap_or(&attack.messages.high);
					let message = message_pool.choose(&mut rng);

					let mut message = message
						.map(|s| s.as_str())
						.unwrap_or(DEFAULT_ATTACK_MESSAGE)
						.replace_prefixed_nouns(&character.sheet.nouns, "self_")
						.replace_prefixed_nouns(&target.sheet.nouns, "target_");
					message.push_str(damage_punctuation);

					// Mutable time.
					let target = self.get_character_mut(target.id).unwrap();
					target.hp -= damage;
					self.console.print(message);
				}
			}
		} else {
			let character = self.get_character_mut(id).unwrap();
			character.x = x;
			character.y = y;
		}
	}
}
