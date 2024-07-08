use crate::character::OrdDir;
use crate::nouns::StrExt;
use crate::prelude::*;
use mlua::LuaSerdeExt;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

pub type CharacterRef = Rc<RefCell<character::Piece>>;

/// This struct contains all information that is relevant during gameplay.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Manager {
	/// Where in the world the characters are.
	pub location: Location,
	pub current_floor: Floor,
	// It might be useful to sort this by remaining action delay to make selecting the next character easier.
	pub characters: VecDeque<CharacterRef>,
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
		resource_manager: &resource::Manager,
		lua: &mlua::Lua,
		options: &Options,
	) -> Result<Self> {
		let mut party = Vec::new();
		let mut characters = VecDeque::new();

		let mut player_controlled = true;

		for PartyReferenceBase {
			sheet,
			accent_color,
		} in party_blueprint
		{
			let sheet = resource_manager.get_sheet(sheet)?;
			let character = Rc::new(RefCell::new(character::Piece {
				player_controlled,
				alliance: character::Alliance::Friendly,
				..character::Piece::new(sheet.clone(), resource_manager)?
			}));
			party.push(world::PartyReference::new(character.clone(), accent_color));
			characters.push_front(character);
			player_controlled = false;
		}

		let console = Console::new(options.ui.colors.console.clone());

		lua.globals().set("Console", console.handle.clone())?;
		lua.globals()
			.set("Status", resource_manager.statuses_handle())?;

		Ok(Manager {
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
		})
	}

	pub fn new_floor(&mut self, resources: &resource::Manager) -> Result<()> {
		self.location.floor += 1;
		self.console
			.print_important(format!("Entering floor {}", self.location.floor));
		self.current_floor = Floor::default();

		self.characters
			.retain(|x| self.party.iter().any(|y| x.as_ptr() == y.piece.as_ptr()));

		self.console
			.print_unimportant("You take some time to rest...".into());
		for i in &self.characters {
			let mut i = i.borrow_mut();
			// Reset positions
			i.x = 0;
			i.y = 0;
			// Rest
			i.rest();
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
		let mut rng = rand::thread_rng();
		self.apply_vault(
			rng.gen_range(1..8),
			rng.gen_range(1..8),
			resources.get_vault("example")?,
			resources,
		)
	}

	pub fn update<'lua>(
		&mut self,
		action_request: Option<ActionRequest<'lua>>,
		lua: &'lua mlua::Lua,
		input_mode: &mut input::Mode,
	) -> mlua::Result<Option<ActionRequest<'lua>>> {
		let outcome = match (action_request, input_mode.clone()) {
			(
				Some(ActionRequest::BeginCursor { callback, .. }),
				input::Mode::Cursor {
					position: (x, y),
					submitted: true,
					..
				},
			) => {
				if let Some(character) = self.get_character_at(x, y) {
					TurnOutcome::poll(lua, callback, character.clone())?
				} else {
					TurnOutcome::Yield
				}
			}
			(
				request @ Some(ActionRequest::BeginCursor { .. }),
				input::Mode::Cursor {
					submitted: false, ..
				},
			) => {
				return Ok(request);
			}
			// If cursor mode is cancelled in any way, the callback will be destroyed.
			(Some(ActionRequest::BeginCursor { .. }), _) => TurnOutcome::Yield,
			(
				Some(ActionRequest::ShowPrompt { callback, .. }),
				input::Mode::Prompt {
					response: Some(response),
					..
				},
			) => TurnOutcome::poll(lua, callback, response)?,
			(
				request @ Some(ActionRequest::ShowPrompt { .. }),
				input::Mode::Prompt { response: None, .. },
			) => return Ok(request),
			(Some(ActionRequest::ShowPrompt { .. }), _) => TurnOutcome::Yield,
			(None, _) => self.next_turn(lua)?,
		};

		match outcome {
			TurnOutcome::Yield => Ok(None),
			TurnOutcome::Action { delay } => {
				#[allow(
					clippy::unwrap_used,
					reason = "next_turn already indexes the first element"
				)]
				let character = self.characters.pop_front().unwrap();
				character.borrow_mut().action_delay = delay;
				// Insert the character into the queue,
				// immediately before the first character to have a higher action delay.
				// This assumes that the queue is sorted.
				self.characters.insert(
					self.characters
						.iter()
						.enumerate()
						.find(|x| x.1.borrow().action_delay > delay)
						.map(|x| x.0)
						.unwrap_or(self.characters.len()),
					character,
				);

				Ok(None)
			}
			TurnOutcome::Request(request) => {
				// Set up any new action requests.
				match &request {
					world::ActionRequest::BeginCursor { x, y, range, .. } => {
						*input_mode = input::Mode::Cursor {
							origin: (*x, *y),
							position: (*x, *y),
							range: *range,
							submitted: false,
							state: input::CursorState::default(),
						};
					}
					world::ActionRequest::ShowPrompt { message, .. } => {
						*input_mode = input::Mode::Prompt {
							response: None,
							message: message.clone(),
						}
					}
				}
				Ok(Some(request))
			}
		}
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

	pub fn apply_vault(
		&mut self,
		x: i32,
		y: i32,
		vault: &Vault,
		resources: &resource::Manager,
	) -> Result<()> {
		self.current_floor.blit_vault(x as usize, y as usize, vault);
		for (xoff, yoff, sheet_name) in &vault.characters {
			let piece = character::Piece {
				x: x + xoff,
				y: y + yoff,
				..character::Piece::new(resources.get_sheet(sheet_name)?.clone(), resources)?
			};
			self.characters.push_front(Rc::new(RefCell::new(piece)));
		}
		Ok(())
	}
}

/// Used to "escape" the world and request extra information, such as inputs.
pub enum ActionRequest<'lua> {
	/// This callback will be called in place of `pop_action` once a position is selected.
	BeginCursor {
		x: i32,
		y: i32,
		range: u32,
		callback: mlua::Thread<'lua>,
	},
	ShowPrompt {
		message: String,
		callback: mlua::Thread<'lua>,
	},
}

impl<'lua> TurnOutcome<'lua> {
	fn from_lua(lua: &'lua mlua::Lua, value: mlua::Value<'lua>) -> mlua::Result<Self> {
		match value {
			mlua::Value::Thread(thread) => TurnOutcome::poll(lua, thread, ()),
			mlua::Value::Nil => Ok(TurnOutcome::Yield),
			mlua::Value::Integer(delay) => Ok(TurnOutcome::Action {
				delay: delay as Aut,
			}),
			_ => Err(mlua::Error::runtime("unexpected return value: {value:?}")),
		}
	}

	fn poll(
		lua: &'lua mlua::Lua,
		thread: mlua::Thread<'lua>,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<Self> {
		#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
		#[serde(tag = "type")]
		pub enum LuaActionRequest {
			Cursor { x: i32, y: i32, range: u32 },
			Prompt { message: String },
		}

		let value = thread.resume(args)?;

		// A resumable thread is expecting an action request response.
		if thread.status() == mlua::ThreadStatus::Resumable {
			Ok(match lua.from_value::<LuaActionRequest>(value)? {
				LuaActionRequest::Cursor { x, y, range } => {
					TurnOutcome::Request(ActionRequest::BeginCursor {
						x,
						y,
						range,
						callback: thread,
					})
				}
				LuaActionRequest::Prompt { message } => {
					TurnOutcome::Request(ActionRequest::ShowPrompt {
						message,
						callback: thread,
					})
				}
			})
		} else {
			TurnOutcome::from_lua(lua, value)
		}
	}
}

pub enum TurnOutcome<'lua> {
	/// No pending action; waiting for player input.
	Yield,
	/// Successful result of an action.
	Action { delay: Aut },
	/// Request extra information for a pending action.
	Request(ActionRequest<'lua>),
}

impl Manager {
	pub fn next_turn<'lua>(&mut self, lua: &'lua mlua::Lua) -> mlua::Result<TurnOutcome<'lua>> {
		let next_character = self.next_character();

		// TODO: Character ordering/timing
		let Some(action) = next_character.borrow_mut().next_action.take() else {
			return Ok(TurnOutcome::Yield);
		};

		let delay = next_character.borrow().action_delay;
		// The delay represents how many auts must pass until this character's next action.
		// If the next character in the queue has a delay higher than 0,
		// then all other characters get their delays decreased as well while the next character "waits" for their action.
		for i in &self.characters {
			let action_delay = &mut i.borrow_mut().action_delay;
			*action_delay = action_delay.saturating_sub(delay);
		}
		// Once an action has been provided, pending turn updates may run.
		next_character.borrow_mut().new_turn();

		match action {
			character::Action::Move(dir) => self.move_piece(lua, next_character, dir),
			character::Action::Attack(attack) => {
				self.attack_piece(lua, attack, next_character, None)
			}
			character::Action::Cast(spell) => {
				match spell.castable_by(&next_character.borrow()) {
					spell::Castable::Yes => {
						let spell = spell.clone();
						// Create a reference for the callback to use.
						let caster = next_character.clone();
						let affinity = spell.affinity(&caster.borrow());

						let chunk = lua.load(spell.on_cast.contents());
						let name = match &spell.on_cast {
							script::MaybeInline::Inline(_) => {
								format!("{} (inline)", spell.name)
							}
							script::MaybeInline::Path(script::Script { path, contents: _ }) => {
								path.clone()
							}
						};
						let globals = lua.globals().clone();

						let parameters = lua.create_table()?;
						for (k, v) in &spell.parameters {
							let k = k.as_ref();
							match v {
								spell::Parameter::Integer(v) => parameters.set(k, *v)?,
								spell::Parameter::Expression(v) => {
									parameters.set(k, u32::evalv(v, &*caster.borrow()))?
								}
							}
						}

						globals.set("parameters", parameters)?;
						globals.set("caster", caster)?;
						// Maybe these should be members of the spell?
						globals.set("level", spell.level)?;
						globals.set("affinity", affinity)?;

						TurnOutcome::from_lua(
							lua,
							chunk.set_name(name).set_environment(globals).eval()?,
						)
					}
					spell::Castable::NotEnoughSP => {
						let message =
							format!("{{Address}} doesn't have enough SP to cast {}.", spell.name)
								.replace_nouns(&next_character.borrow().sheet.nouns);
						self.console.print_system(message);
						Ok(TurnOutcome::Yield)
					}
					spell::Castable::UncastableAffinity => {
						let message =
							format!("{{Address}} has the wrong affinity to cast {}.", spell.name)
								.replace_nouns(&next_character.borrow().sheet.nouns);
						self.console.print_system(message);
						Ok(TurnOutcome::Yield)
					}
				}
			}
		}
	}

	/// # Errors
	///
	/// Returns an error if the target is an ally, or if the user has no attacks.
	pub fn attack_piece<'lua>(
		&self,
		lua: &'lua mlua::Lua,
		attack: Rc<Attack>,
		user: &CharacterRef,
		target: Option<&CharacterRef>,
	) -> mlua::Result<TurnOutcome<'lua>> {
		// Calculate damage
		let magnitude = u32::evalv(&attack.magnitude, &*user.borrow());

		let chunk = lua.load(attack.on_use.contents());
		let name = match &attack.on_use {
			script::MaybeInline::Inline(_) => {
				format!("{} (inline)", attack.name)
			}
			script::MaybeInline::Path(script::Script { path, contents: _ }) => path.clone(),
		};
		let globals = lua.globals().clone();

		globals.set("user", user.clone())?;
		if let Some(target) = target {
			globals.set("target", target.clone())?;
		} else {
			globals.set("target", mlua::Nil)?;
		}
		globals.set("use_time", attack.use_time)?;
		globals.set("magnitude", magnitude)?;

		let value: mlua::Value = chunk.set_name(name).set_environment(globals).eval()?;
		TurnOutcome::from_lua(lua, value)
	}

	/// # Errors
	///
	/// Fails if a wall or void is in the way, or if an implicit attack failed.
	pub fn move_piece<'lua>(
		&self,
		lua: &'lua mlua::Lua,
		character: &CharacterRef,
		dir: OrdDir,
	) -> mlua::Result<TurnOutcome<'lua>> {
		use crate::floor::Tile;

		let (x, y, delay) = {
			let character = character.borrow();
			let (x, y) = dir.as_offset();
			(
				character.x + x,
				character.y + y,
				// Diagonal movement is sqrt(2) times slower
				if x.abs() + y.abs() == 2 {
					SQRT2_TURN
				} else {
					TURN
				},
			)
		};

		// There's a really annoying phenomenon in Pokémon Mystery Dungeon where you can't hit ghosts that are inside of walls.
		// I think that this is super lame, so the attack check comes before any movement.
		if let Some(target_ref) = self.get_character_at(x, y) {
			let Some(attack) = character.borrow().attacks.first().cloned() else {
				self.console
					.print_unimportant("You cannot perform any melee attacks right now.".into());
				return Ok(TurnOutcome::Yield);
			};
			return self.attack_piece(lua, attack, character, Some(target_ref));
		}

		let tile = self.current_floor.map.get(y, x);
		match tile {
			Some(Tile::Floor) | Some(Tile::Exit) => {
				let mut character = character.borrow_mut();
				character.x = x;
				character.y = y;
				Ok(TurnOutcome::Action { delay })
			}
			Some(Tile::Wall) => {
				self.console
					.say(character.borrow().sheet.nouns.name.clone(), "Ouch!".into());
				Ok(TurnOutcome::Yield)
			}
			None => {
				self.console.print_system("You stare out into the void: an infinite expanse of nothingness enclosed within a single tile.".into());
				Ok(TurnOutcome::Yield)
			}
		}
	}
}
