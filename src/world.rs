use crate::character::OrdDir;
use crate::nouns::StrExt;
use crate::prelude::*;
use parking_lot::RwLock;
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::sync::Arc;
use uuid::Uuid;

const DEFAULT_ATTACK_MESSAGE: &str = "{self_Address} attacked {target_indirect}";

pub type CharacterRef = Arc<RwLock<character::Piece>>;

/// This struct contains all information that is relevant during gameplay.
#[derive(Clone, Debug)]
pub struct Manager {
	// I know I'm going to have to change this in the future to add multiple worlds.
	/// Where in the world the characters are.
	pub location: Location,
	/// This is the level pointed to by `location.level`.
	pub current_level: Arc<RwLock<Level>>,
	pub current_floor: Floor,
	// It might be useful to sort this by remaining action delay to make selecting the next character easier.
	pub characters: Vec<CharacterRef>,
	pub items: Vec<item::Piece>,
	/// Always point to the party's pieces, even across floors.
	/// When exiting a dungeon, these sheets will be saved to a party struct.
	pub party: Vec<PartyReference>,
	pub inventory: Vec<String>,
	pub console: Arc<RwLock<Console>>,
}

/// Contains information about what should generate on each floor.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, alua::UserData)]
pub struct Level {
	#[alua(get, set)]
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
		self.characters.iter().find(|x| x.read().id == id)
	}

	pub fn next_character(&self) -> &CharacterRef {
		&self.characters[0]
	}

	pub fn get_character_at(&self, x: i32, y: i32) -> Option<&CharacterRef> {
		self.characters.iter().find(|p| {
			let p = p.read();
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
			self.characters.push(Arc::new(RwLock::new(piece)));
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
		let mut console = self.console.write();

		let Some(action) = next_character.write().next_action.take() else {
			return;
		};
		match action {
			character::Action::Move(dir) => match self.move_piece(next_character, dir) {
				Ok(MovementResult::Attack(AttackResult::Hit { message, weak })) => {
					let colors = &console.colors;
					let color = if weak {
						colors.unimportant
					} else {
						colors.normal
					};
					console.print_colored(message, color);
				}
				Ok(_) => (),
				Err(MovementError::HitWall) => {
					let name = next_character.read().sheet.read().nouns.read().name.clone();
					console.say(name, "Ouch!".into());
				}
				Err(MovementError::HitVoid) => {
					console.print_system("You stare out into the void: an infinite expanse of nothingness enclosed within a single tile.".into());
				}
				Err(MovementError::Attack(AttackError::Ally)) => {
					self.console
						.write()
						.print_system("You can't attack your allies!".into());
				}
				Err(MovementError::Attack(AttackError::NoAttacks)) => {
					self.console
						.write()
						.print_system("You cannot perform any melee attacks right now.".into());
				}
			},
			character::Action::Cast(spell) => {
				let castable = {
					let mut next_character = next_character.write();
					if spell.castable_by(&next_character) {
						next_character.sp -= spell.level as i32;
						true
					} else {
						false
					}
				};
				if castable {
					// Pass write access to the console to the lua runtime.
					drop(console);
					let lua = mlua::Lua::new();
					lua.globals()
						.set("Level", self.current_level.clone())
						.unwrap();
					lua.globals().set("Console", self.console.clone()).unwrap();
					let chunk =
						lua.load("local caster = ...; Console:print(caster.sheet.nouns.name..\" fired a magic missile.\")");
					let () = chunk.call(next_character.clone()).unwrap();
				} else {
					let message =
						format!("{{Address}} doesn't have enough SP to cast {}.", spell.name)
							.replace_nouns(&next_character.read().sheet.read().nouns.read());
					console.print_system(message);
				}
			}
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
			let character = character_ref.read();
			let character_sheet = character.sheet.read();
			let target = target_ref.read();
			let target_sheet = target.sheet.read();

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
			let damage =
				u32::evalv(&attack.damage, &*character).saturating_sub(target_sheet.stats.defense);
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
				.replace_prefixed_nouns(&character_sheet.nouns.read(), "self_")
				.replace_prefixed_nouns(&target_sheet.nouns.read(), "target_");
			message.push_str(damage_punctuation);
			message.push_str(&format!(" (-{damage} HP)"));

			(damage, message, is_weak_attack)
		};

		// This is where the damage is actually dealt
		target_ref.write().hp -= damage as i32;

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
			let (x, y) = dir.as_offset();
			let character = character_ref.read();
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
				let mut character = character_ref.write();
				character.x = x;
				character.y = y;
				Ok(MovementResult::Move)
			}
			Some(Tile::Wall) => Err(MovementError::HitWall),
			None => Err(MovementError::HitVoid),
		}
	}
}
