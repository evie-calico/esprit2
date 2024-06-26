use crate::character::OrdDir;
use crate::nouns::StrExt;
use crate::prelude::*;
use mlua::LuaSerdeExt;
use rand::seq::SliceRandom;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::error;

const DEFAULT_ATTACK_MESSAGE: &str = "{self_Address} attacked {target_indirect}";

pub type CharacterRef = Rc<RefCell<character::Piece>>;

/// This struct contains all information that is relevant during gameplay.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Manager {
	/// Where in the world the characters are.
	pub location: Location,
	pub current_floor: Floor,
	// It might be useful to sort this by remaining action delay to make selecting the next character easier.
	pub characters: Vec<CharacterRef>,
	pub items: Vec<item::Piece>,
	/// Always point to the party's pieces, even across floors.
	/// When exiting a dungeon, these sheets will be saved to a party struct.
	pub party: Vec<PartyReference>,
	pub inventory: Vec<String>,
	#[serde(skip)]
	pub console: Console,
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

/// Anything not strictly tied to the party reference's "logic",
/// but still associated with its rendering
#[derive(Clone, Default, Debug)]
pub struct PartyReferenceDrawState {
	pub cloud: draw::CloudState,
	pub cloud_trail: draw::CloudTrail,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PartyReference {
	/// The piece that is being used by this party member.
	pub piece: CharacterRef,
	/// Displayed on the pamphlet.
	pub accent_color: Color,
	#[serde(skip)]
	pub draw_state: PartyReferenceDrawState,
}

impl PartyReference {
	pub fn new(piece: CharacterRef, accent_color: Color) -> Self {
		Self {
			piece,
			accent_color,
			draw_state: PartyReferenceDrawState::default(),
		}
	}
}

// this is probably uneccessary and just makes main.rs look nicer
pub struct PartyReferenceBase {
	pub sheet: &'static str,
	pub accent_color: Color,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Location {
	/// Which level is currently loaded.
	pub level: String,
	pub floor: usize,
}

impl Manager {
	pub fn new(
		party_blueprint: impl Iterator<Item = PartyReferenceBase>,
		resource_manager: &ResourceManager,
		lua: &mlua::Lua,
		options: &Options,
	) -> Self {
		let mut party = Vec::new();
		let mut characters = Vec::new();

		let mut player_controlled = true;

		for PartyReferenceBase {
			sheet,
			accent_color,
		} in party_blueprint
		{
			let sheet = resource_manager.get_sheet(sheet).unwrap();
			let character = Rc::new(RefCell::new(character::Piece {
				player_controlled,
				alliance: character::Alliance::Friendly,
				..character::Piece::new(sheet.clone(), resource_manager)
			}));
			party.push(world::PartyReference::new(character.clone(), accent_color));
			characters.push(character);
			player_controlled = false;
		}

		let console = Console::new(options.ui.colors.console.clone());

		lua.globals().set("Console", console.handle()).unwrap();

		Manager {
			location: world::Location {
				level: String::from("New Level"),
				floor: 0,
			},
			current_floor: Floor::default(),
			characters,
			items: Vec::new(),

			party,
			inventory: vec![
				"items/aloe".into(),
				"items/apple".into(),
				"items/blinkfruit".into(),
				"items/fabric_shred".into(),
				"items/grapes".into(),
				"items/ice_cream".into(),
				"items/lily".into(),
				"items/pear_on_a_stick".into(),
				"items/pear".into(),
				"items/pepper".into(),
				"items/purefruit".into(),
				"items/raspberry".into(),
				"items/reviver_seed".into(),
				"items/ring_alt".into(),
				"items/ring".into(),
				"items/scarf".into(),
				"items/slimy_apple".into(),
				"items/super_pepper".into(),
				"items/twig".into(),
				"items/water_chestnut".into(),
				"items/watermelon".into(),
			],

			console,
		}
	}

	pub fn new_floor(&mut self, resources: &ResourceManager) {
		self.location.floor += 1;
		self.console
			.print_important(format!("Entering floor {}", self.location.floor));
		self.current_floor = Floor::default();

		let party_pieces: Vec<_> = self.party.iter().map(|x| x.piece.clone()).collect();
		self.characters.clear();

		self.console
			.print_unimportant("You take some time to rest...".into());
		for i in &party_pieces {
			let mut i = i.borrow_mut();
			// Reset positions
			i.x = 0;
			i.y = 0;
			// Rest
			let stats = i.sheet.stats();
			i.hp = stats.heart.min(i.hp as u32 + stats.heart / 2) as i32;
			i.sp = stats.soul.min(i.sp as u32 + stats.soul / 2) as i32;
			// Award experience
			i.sheet.experience += 40;
			while i.sheet.experience >= 100 {
				i.sheet.experience -= 100;
				i.sheet.level = i.sheet.level.saturating_add(1);
				self.console.print_special(
					format!("{{Address}}'s level increased to {}!", i.sheet.level)
						.replace_nouns(&i.sheet.nouns),
				);
			}
		}
		self.characters = party_pieces;
		let mut rng = rand::thread_rng();
		self.apply_vault(
			rng.gen_range(1..8),
			rng.gen_range(1..8),
			resources.get_vault("example").unwrap(),
			resources,
		);
	}

	pub fn update<'lua>(
		&mut self,
		action_request: Option<world::ActionRequest<'lua>>,
		lua: &'lua mlua::Lua,
		input_mode: &mut input::Mode,
	) -> Option<world::ActionRequest<'lua>> {
		let (renew_action, action_request) = match action_request {
			Some(world::ActionRequest::BeginCursor { x, y, callback }) => {
				match *input_mode {
					input::Mode::Cursor {
						x,
						y,
						submitted: true,
						..
					} => {
						*input_mode = input::Mode::Normal;
						if let Some(character) = self.get_character_at(x, y) {
							(
								true,
								ActionRequest::poll(lua, callback, character.clone()).unwrap(),
							)
						} else {
							(false, None)
						}
					}
					input::Mode::Cursor {
						submitted: false, ..
					} => {
						// This match statement currently has ownership of `action_request`
						// since the callback is `FnOnce`.
						// Because of this, `action_request` needs to be reconstructed in all match arms,
						// even if this is a no-op.
						(
							false,
							Some(world::ActionRequest::BeginCursor { x, y, callback }),
						)
					}
					_ => {
						// If cursor mode is cancelled in any way, the callback will be destroyed.
						(false, None)
					}
				}
			}
			None => (true, self.pop_action(lua)),
		};

		if renew_action {
			// Set up any new action requests.
			if let Some(world::ActionRequest::BeginCursor { x, y, callback: _ }) = action_request {
				*input_mode = input::Mode::Cursor {
					x,
					y,
					submitted: false,
					state: input::CursorState::default(),
				};
			}
		}

		action_request
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
			self.characters.push(Rc::new(RefCell::new(piece)));
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
pub struct AttackResult {
	/// Flavor text to explain the attack.
	message: String,
	/// Hard info about the attack, like damage.
	log: combat::Log,
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum AttackError {
	#[error("attempted to attack an ally")]
	Ally,
	#[error("attacker has no attacks defined")]
	NoAttacks,
}

/// Used to "escape" the world and request extra information, such as inputs.
pub enum ActionRequest<'lua> {
	/// This callback will be called in place of `pop_action` once a position is selected.
	BeginCursor {
		x: i32,
		y: i32,
		callback: mlua::Thread<'lua>,
	},
}

impl<'lua> ActionRequest<'lua> {
	fn poll(
		lua: &'lua mlua::Lua,
		thread: mlua::Thread<'lua>,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<Option<ActionRequest<'lua>>> {
		#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
		#[serde(tag = "type")]
		pub enum LuaActionRequest {
			Cursor { x: i32, y: i32 },
		}

		let action: Option<LuaActionRequest> = lua.from_value(thread.resume(args)?)?;
		Ok(action.map(
			|LuaActionRequest::Cursor { x, y }| ActionRequest::BeginCursor {
				x,
				y,
				callback: thread,
			},
		))
	}
}

impl Manager {
	pub fn pop_action<'lua>(&mut self, lua: &'lua mlua::Lua) -> Option<ActionRequest<'lua>> {
		let next_character = self.next_character();

		let action = next_character.borrow_mut().next_action.take()?;
		match action {
			character::Action::Move(dir) => match self.move_piece(next_character, dir) {
				Ok(MovementResult::Attack(AttackResult { message, log })) => {
					self.console.combat_log(message, log);
				}
				Ok(_) => (),
				Err(MovementError::HitWall) => {
					let name = next_character.borrow().sheet.nouns.name.clone();
					self.console.say(name, "Ouch!".into());
				}
				Err(MovementError::HitVoid) => {
					self.console.print_system("You stare out into the void: an infinite expanse of nothingness enclosed within a single tile.".into());
				}
				Err(MovementError::Attack(AttackError::Ally)) => {
					self.console
						.print_system("You can't attack your allies!".into());
				}
				Err(MovementError::Attack(AttackError::NoAttacks)) => {
					self.console
						.print_system("You cannot perform any melee attacks right now.".into());
				}
			},
			character::Action::Cast(spell) => {
				if spell.castable_by(&next_character.borrow()) {
					let spell = spell.clone();
					// TODO: this is awful. just move targeting into scripts.
					match spell.parameters.clone() {
						spell::Parameters::Target {
							magnitude,
							pierce_threshold,
						} => {
							// Create a reference for the callback to use.
							let caster = next_character.clone();
							let affinity = spell.affinity(&caster.borrow());
							let magnitude = magnitude
								.as_ref()
								.map(|x| affinity.magnitude(u32::evalv(x, &*caster.borrow())));

							let chunk = lua.load(spell.on_cast.contents());
							let name = match &spell.on_cast {
								spell::ScriptOrInline::Inline(_) => {
									format!("{} (inline)", spell.name)
								}
								spell::ScriptOrInline::Path(spell::Script {
									path,
									contents: _,
								}) => path.clone(),
							};
							let globals = lua.globals();

							globals.set("caster", caster).unwrap();
							// Maybe these should be members of the spell?
							globals.set("magnitude", magnitude).unwrap();
							globals.set("pierce_threshold", pierce_threshold).unwrap();
							globals.set("level", spell.level).unwrap();
							globals.set("affinity", affinity).unwrap();

							let value: mlua::Value = chunk
								.set_name(name)
								.set_environment(globals)
								.eval()
								.unwrap();

							match value {
								mlua::Value::Thread(thread) => {
									return ActionRequest::poll(lua, thread, ()).unwrap();
								}
								mlua::Value::Nil => (),
								_ => error!("unexpected return value"),
							}
						}
					}
				} else {
					let message =
						format!("{{Address}} doesn't have enough SP to cast {}.", spell.name)
							.replace_nouns(&next_character.borrow().sheet.nouns);
					self.console.print_system(message);
				}
			}
		};

		None
	}

	/// # Errors
	///
	/// Returns an error if the target is an ally, or if the user has no attacks.
	pub fn attack_piece(
		&self,
		character_ref: &CharacterRef,
		target_ref: &CharacterRef,
	) -> Result<AttackResult, AttackError> {
		let character = character_ref.borrow();
		let target = target_ref.borrow();

		if target.alliance == character.alliance {
			return Err(AttackError::Ally);
		}

		let mut rng = rand::thread_rng();

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
		let target_stats = target.sheet.stats();
		let magnitude = u32::evalv(&attack.damage, &*character);
		let damage = magnitude.saturating_sub(target_stats.defense);
		let is_miss = damage == 0;

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
			.filter(|_| is_miss)
			.unwrap_or(&attack.messages.high);
		let message = message_pool.choose(&mut rng);

		let mut message = message
			.map(|s| s.as_str())
			.unwrap_or(DEFAULT_ATTACK_MESSAGE)
			.replace_prefixed_nouns(&character.sheet.nouns, "self_")
			.replace_prefixed_nouns(&target.sheet.nouns, "target_");
		message.push_str(damage_punctuation);

		drop(target);

		// This is where the damage is actually dealt
		target_ref.borrow_mut().hp -= damage as i32;

		let log = if is_miss {
			combat::Log::Miss
		} else {
			combat::Log::Hit { damage }
		};

		// `self` is not mutable, so the message needs to be passed up to the manager,
		// where printing can occur.
		Ok(AttackResult { message, log })
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
			Some(Tile::Floor) | Some(Tile::Exit) => {
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
