use crate::character::OrdDir;
use crate::nouns::StrExt;
use crate::prelude::*;
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::cell::RefCell;
use uuid::Uuid;

const DEFAULT_ATTACK_MESSAGE: &str = "{self_Address} attacked {target_indirect}";

pub type CharacterRef = RefCell<character::Piece>;

/// This struct contains all information that is relevant during gameplay.
#[derive(Clone, Debug)]
pub struct Manager {
	// I know I'm going to have to change this in the future to add multiple worlds.
	/// Where in the world the characters are.
	pub location: Location,
	/// This is the level pointed to by `location.level`.
	pub current_level: Level,
	pub current_floor: Floor,
	// It might be useful to sort this by remaining action delay to make selecting the next character easier.
	pub characters: Vec<CharacterRef>,
	pub items: Vec<item::Piece>,
	/// Always point to the party's pieces, even across floors.
	/// When exiting a dungeon, these sheets will be saved to a party struct.
	pub party: Vec<PartyReference>,
	pub inventory: Vec<String>,
	pub console: Console,
}

/// Contains information about what should generate on each floor.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Level {
	pub name: String,
}

impl Default for Level {
	fn default() -> Self {
		Self {
			name: String::from("New Level"),
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

impl Manager {
	// Returns none if no entity with the given uuid is currently loaded.
	// This either mean they no longer exist, or they're on a different floor;
	// either way they cannot be referenced.
	pub fn get_character(&self, id: Uuid) -> Option<&CharacterRef> {
		self.characters.iter().find(|x| x.borrow().id == id)
	}

	pub fn next_character(&self) -> &CharacterRef {
		&self.characters[0]
	}

	pub fn get_character_at(&self, x: i32, y: i32) -> Option<&CharacterRef> {
		self.characters.iter().find(|p| {
			let p = p.borrow();
			p.x == x && p.y == y
		})
	}

	pub fn apply_vault(&mut self, x: i32, y: i32, vault: &Vault, resources: &ResourceManager) {
		self.current_floor.blit_vault(x as usize, y as usize, vault);
		for (xoff, yoff, sheet_name) in &vault.characters {
			let piece = character::Piece {
				x: x + xoff,
				y: y + yoff,
				..character::Piece::new(resources.get_sheet(sheet_name).unwrap().clone(), resources)
			};
			self.characters.push(RefCell::new(piece));
		}
	}
}

#[derive(Clone, Debug)]
pub enum MovementResult {
	Move,
	Attack(AttackResult),
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum MovementError {
	#[error("hit a wall")]
	HitWall,
	#[error("hit the void")]
	HitVoid,
	#[error(transparent)]
	Attack(#[from] AttackError),
}

#[derive(Clone, Debug)]
pub enum AttackResult {
	Hit { message: String, weak: bool },
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum AttackError {
	#[error("attempted to attack an ally")]
	Ally,
	#[error("attacker has no attacks defined")]
	NoAttacks,
}

impl Manager {
	pub fn pop_action(&mut self) {
		let next_character = self.next_character();

		let Some(action) = next_character.borrow_mut().next_action.take() else {
			return;
		};
		match action {
			character::Action::Move(dir) => match self.move_piece(next_character, dir) {
				Ok(MovementResult::Attack(AttackResult::Hit { message, weak })) => {
					let colors = &self.console.colors;
					self.console.print_colored(
						message,
						if weak {
							colors.unimportant
						} else {
							colors.normal
						},
					)
				}
				Ok(_) => (),
				Err(MovementError::HitWall) => {
					let name = next_character.borrow().sheet.nouns.name.clone();
					self.console.say(name, "Ouch!");
				}
				Err(MovementError::HitVoid) => {
					self.console.print_system("You stare out into the void: an infinite expanse of nothingness enclosed within a single tile.");
				}
				Err(MovementError::Attack(AttackError::Ally)) => {
					self.console.print_system("You can't attack your allies!");
				}
				Err(MovementError::Attack(AttackError::NoAttacks)) => {
					self.console
						.print_system("You cannot perform any melee attacks right now.");
				}
			},
		};
	}

	/// # Errors
	///
	/// Returns an error if the target is an ally, or if the user has no attacks.
	pub fn attack_piece(
		&self,
		character_ref: &CharacterRef,
		target_ref: &CharacterRef,
	) -> Result<AttackResult, AttackError> {
		let (damage, message, weak) = {
			let character = character_ref.borrow();
			let target = target_ref.borrow();

			if target.alliance == character.alliance {
				return Err(AttackError::Ally);
			}

			let mut rng = thread_rng();

			let mut attack = character.attacks.first().ok_or(AttackError::NoAttacks)?;
			let max_attack_weight = character.attacks.iter().fold(0, |a, x| a + x.weight);

			let mut point = rng.gen_range(0..max_attack_weight);
			for i in &character.attacks {
				if point < i.weight {
					attack = i;
					break;
				}
				point -= i.weight;
			}

			// Calculate damage
			let damage = 0.max(
				((character.sheet.stats.power + attack.bonus) as i32)
					- (target.sheet.stats.defense as i32),
			);
			let is_weak_attack = damage == 0;

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

			(damage, message, is_weak_attack)
		};

		// This is where the damage is actually dealt
		target_ref.borrow_mut().hp -= damage;

		// `self` is not mutable, so the message needs to be passed up to the manager,
		// where printing can occur.
		Ok(AttackResult::Hit { message, weak })
	}

	/// # Errors
	///
	/// Fails if a wall or void is in the way, or if an implicit attack failed.
	pub fn move_piece(
		&self,
		character_ref: &CharacterRef,
		dir: OrdDir,
	) -> Result<MovementResult, MovementError> {
		use crate::floor::Tile;

		let (x, y) = {
			let (x, y) = dir_to_vector(dir);
			let character = character_ref.borrow();
			(character.x + x, character.y + y)
		};

		// There's a really annoying phenomenon in Pokémon Mystery Dungeon where you can't hit ghosts that are inside of walls.
		// I think that this is super lame, so the attack check comes before any movement.
		if let Some(target_ref) = self.get_character_at(x, y) {
			return Ok(MovementResult::Attack(
				self.attack_piece(character_ref, target_ref)?,
			));
		}

		let tile = self.current_floor.map.get(y, x);
		match tile {
			Some(Tile::Floor) => {
				let mut character = character_ref.borrow_mut();
				character.x = x;
				character.y = y;
				Ok(MovementResult::Move)
			}
			Some(Tile::Wall) => Err(MovementError::HitWall),
			None => Err(MovementError::HitVoid),
		}
	}
}

fn dir_to_vector(dir: OrdDir) -> (i32, i32) {
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
	(x, y)
}
