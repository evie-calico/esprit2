use crate::prelude::*;
use consider::TaggedHeuristics;
use mlua::LuaSerdeExt;
use rand::{seq::SliceRandom, Rng, SeedableRng};
use std::collections::VecDeque;
use std::rc::Rc;

pub use character::Ref as CharacterRef;

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
	pub piece: CharacterRef,
	/// Displayed on the pamphlet.
	pub accent_color: Color,
}

impl PartyReference {
	pub fn new(piece: CharacterRef, accent_color: Color) -> Self {
		Self {
			piece,
			accent_color,
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
			let character = CharacterRef::new(character::Piece {
				player_controlled,
				alliance: character::Alliance::Friendly,
				..character::Piece::new(sheet.clone(), resource_manager)?
			});
			party.push(world::PartyReference::new(character.clone(), accent_color));
			characters.push_front(character);
			player_controlled = false;
		}

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
		})
	}

	pub fn new_floor(&mut self, resources: &resource::Manager, console: &Console) -> Result<()> {
		self.location.floor += 1;
		console.print_important(format!("Entering floor {}", self.location.floor));
		self.current_floor = Floor::default();

		self.characters
			.retain(|x| self.party.iter().any(|y| x.as_ptr() == y.piece.as_ptr()));

		console.print_unimportant("You take some time to rest...".into());
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
				console.print_special(
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
		action: PartialAction<'lua>,
		scripts: &'lua resource::Scripts,
		console: &Console,
		input_mode: &mut input::Mode,
	) -> Result<Option<ActionRequest<'lua>>> {
		let outcome = match (action, input_mode.clone()) {
			// Handle targetted cursor submission
			(
				PartialAction::Request(ActionRequest::BeginTargetCursor { callback, .. }),
				input::Mode::Cursor {
					position: (x, y),
					submitted: true,
					..
				},
			) => {
				if let Some(character) = self.get_character_at(x, y) {
					TurnOutcome::poll(self, scripts.runtime, callback, character.clone())?
				} else {
					// If the cursor hasn't selected a character,
					// cancel the request altogther.
					// This destroys the lua callback.
					TurnOutcome::Yield
				}
			}
			// Handle positional cursor submission
			(
				PartialAction::Request(ActionRequest::BeginCursor { callback, .. }),
				input::Mode::Cursor {
					position: (x, y),
					submitted: true,
					..
				},
			) => TurnOutcome::poll(self, scripts.runtime, callback, (x, y))?,
			// An unsubmitted cursor yields the same action request.
			(
				PartialAction::Request(
					request @ (ActionRequest::BeginCursor { .. }
					| ActionRequest::BeginTargetCursor { .. }),
				),
				input::Mode::Cursor {
					submitted: false, ..
				},
			) => {
				return Ok(Some(request));
			}
			// Prompt with submitted response
			(
				PartialAction::Request(ActionRequest::ShowPrompt { callback, .. }),
				input::Mode::Prompt {
					response: Some(response),
					..
				},
			) => TurnOutcome::poll(self, scripts.runtime, callback, response)?,
			// Prompt with unsubmitted response
			(
				PartialAction::Request(request @ ActionRequest::ShowPrompt { .. }),
				input::Mode::Prompt { response: None, .. },
			) => return Ok(Some(request)),
			// Direction prompt with submitted response
			(
				PartialAction::Request(ActionRequest::ShowDirectionPrompt { callback, .. }),
				input::Mode::DirectionPrompt {
					response: Some(response),
					..
				},
			) => TurnOutcome::poll(
				self,
				scripts.runtime,
				callback,
				scripts.runtime.to_value(&response),
			)?,
			// Direction prompt with unsubmitted response
			(
				PartialAction::Request(request @ ActionRequest::ShowDirectionPrompt { .. }),
				input::Mode::DirectionPrompt { response: None, .. },
			) => return Ok(Some(request)),
			// If the input mode is invalid in any way, the callback will be destroyed.
			(
				PartialAction::Request(
					ActionRequest::BeginCursor { .. }
					| ActionRequest::BeginTargetCursor { .. }
					| ActionRequest::ShowPrompt { .. }
					| ActionRequest::ShowDirectionPrompt { .. },
				),
				_,
			) => TurnOutcome::Yield,
			// If there is no pending request, pop a turn off the character queue.
			(PartialAction::Action(action), _) => self.next_turn(console, scripts, action)?,
		};

		let player_controlled = self.next_character().borrow().player_controlled;
		let mut apply_delay = |delay| {
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
		};

		match outcome {
			TurnOutcome::Yield => {
				if !player_controlled {
					apply_delay(TURN);
				}
				Ok(None)
			}
			TurnOutcome::Action { delay } => {
				apply_delay(delay);
				Ok(None)
			}
			TurnOutcome::Request(request) => {
				// Set up any new action requests.
				match &request {
					world::ActionRequest::BeginCursor {
						x,
						y,
						range,
						radius,
						callback: _,
					} => {
						*input_mode = input::Mode::Cursor {
							origin: (*x, *y),
							position: (*x, *y),
							range: *range,
							radius: *radius,
							submitted: false,
							state: input::CursorState::default(),
						};
					}
					world::ActionRequest::BeginTargetCursor {
						x,
						y,
						range,
						callback: _,
					} => {
						*input_mode = input::Mode::Cursor {
							origin: (*x, *y),
							position: (*x, *y),
							range: *range,
							radius: None,
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
					world::ActionRequest::ShowDirectionPrompt { message, .. } => {
						*input_mode = input::Mode::DirectionPrompt {
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

	pub fn generate_floor(&mut self, seed: &str, set: &vault::Set, resources: &resource::Manager) {
		const SEED_LENGTH: usize = 32;

		let _enter = tracing::error_span!("level gen", seed).entered();
		let mut seed_slice = [0; SEED_LENGTH];
		for (str_byte, seed_byte) in seed.bytes().take(SEED_LENGTH).zip(seed_slice.iter_mut()) {
			*seed_byte = str_byte;
		}
		let mut rng = rand::rngs::StdRng::from_seed(seed_slice);
		for _ in 0..set.density {
			let x = rng.gen_range(0..self.current_floor.map.cols() as i32);
			let y = rng.gen_range(0..self.current_floor.map.rows() as i32);
			match set.vaults.choose(&mut rng).map(|k| {
				resources
					.get_vault(k)
					.and_then(|vault| self.apply_vault(x, y, vault, resources))
			}) {
				Some(Ok(())) => (),
				Some(Err(msg)) => error!("{msg}"),
				None => error!("vault set has no vaults"),
			}
		}
	}

	pub fn apply_vault(
		&mut self,
		x: i32,
		y: i32,
		vault: &Vault,
		resources: &resource::Manager,
	) -> Result<()> {
		if self.current_floor.blit_vault(x as usize, y as usize, vault) {
			for (xoff, yoff, sheet_name) in &vault.characters {
				let piece = character::Piece {
					x: x + xoff,
					y: y + yoff,
					..character::Piece::new(resources.get_sheet(sheet_name)?.clone(), resources)?
				};
				self.characters.push_front(character::Ref::new(piece));
			}
		}
		Ok(())
	}
}

pub enum PartialAction<'lua> {
	Action(character::Action<'lua>),
	Request(ActionRequest<'lua>),
}

/// Used to "escape" the world and request extra information, such as inputs.
pub enum ActionRequest<'lua> {
	/// Returns a position to the callback.
	///
	/// This callback will be called in place of `pop_action` once a position is selected.
	BeginCursor {
		x: i32,
		y: i32,
		range: u32,
		radius: Option<u32>,
		callback: mlua::Thread<'lua>,
	},
	/// Returns a character piece to the callback.
	///
	/// This callback will be called in place of `pop_action` once a position is selected.
	BeginTargetCursor {
		x: i32,
		y: i32,
		range: u32,
		callback: mlua::Thread<'lua>,
	},
	ShowPrompt {
		message: String,
		callback: mlua::Thread<'lua>,
	},
	ShowDirectionPrompt {
		message: String,
		callback: mlua::Thread<'lua>,
	},
}

pub enum TurnOutcome<'lua> {
	/// No pending action; waiting for player input.
	Yield,
	/// Successful result of an action.
	Action { delay: Aut },
	/// Request extra information for a pending action.
	Request(ActionRequest<'lua>),
}

impl<'lua> TurnOutcome<'lua> {
	fn poll(
		world: &Manager,
		lua: &'lua mlua::Lua,
		thread: mlua::Thread<'lua>,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<Self> {
		match ThreadOutcome::poll(world, lua, thread, args)? {
			ThreadOutcome::Value(value) => match value {
				mlua::Value::Thread(thread) => TurnOutcome::poll(world, lua, thread, ()),
				mlua::Value::Nil => Ok(TurnOutcome::Yield),
				mlua::Value::Integer(delay) => Ok(TurnOutcome::Action {
					delay: delay as Aut,
				}),
				_ => Err(mlua::Error::runtime("unexpected return value: {value:?}")),
			},
			ThreadOutcome::Request(request) => Ok(TurnOutcome::Request(request)),
		}
	}
}

pub enum ThreadOutcome<'lua> {
	Value(mlua::Value<'lua>),
	Request(ActionRequest<'lua>),
}

impl<'lua> ThreadOutcome<'lua> {
	pub fn poll(
		world: &Manager,
		lua: &'lua mlua::Lua,
		thread: mlua::Thread<'lua>,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<Self> {
		#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
		pub enum CharacterQuery {
			Within { x: i32, y: i32, range: u32 },
		}

		// Handle requests for extra information from the lua function.
		// These may or may not be inputs.
		#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
		#[serde(tag = "type")]
		pub enum LuaRequest {
			// World manager communication
			Characters {
				query: Option<CharacterQuery>,
			},
			Tile {
				x: i32,
				y: i32,
			},
			// Input
			TargetCursor {
				x: i32,
				y: i32,
				range: u32,
			},
			Cursor {
				x: i32,
				y: i32,
				range: u32,
				radius: Option<u32>,
			},
			Prompt {
				message: String,
			},
			Direction {
				message: String,
			},
		}

		let mut value = thread.resume(args)?;
		loop {
			// A resumable thread is expecting an action request response.
			if thread.status() == mlua::ThreadStatus::Resumable {
				match lua.from_value::<LuaRequest>(value)? {
					LuaRequest::Characters { query } => {
						value = match query {
							Some(CharacterQuery::Within { x, y, range }) => thread.resume(
								lua.create_sequence_from(
									world
										.characters
										.iter()
										.filter(|character| {
											let character = character.borrow();
											(character.x - x)
												.unsigned_abs()
												.max((character.y - y).unsigned_abs())
												<= range
										})
										.cloned(),
								)?,
							)?,
							None => thread.resume(
								lua.create_sequence_from(world.characters.iter().cloned())?,
							)?,
						}
					}
					LuaRequest::Tile { x, y } => {
						let tile = world.current_floor.map.get(y, x).copied();
						value = thread.resume(
							tile.map(|x| lua.to_value(&x))
								.transpose()?
								.unwrap_or(mlua::Value::Nil),
						)?;
					}
					LuaRequest::TargetCursor { x, y, range } => {
						return Ok(ThreadOutcome::Request(ActionRequest::BeginTargetCursor {
							x,
							y,
							range,
							callback: thread,
						}));
					}
					LuaRequest::Cursor {
						x,
						y,
						range,
						radius,
					} => {
						return Ok(ThreadOutcome::Request(ActionRequest::BeginCursor {
							x,
							y,
							range,
							radius,
							callback: thread,
						}));
					}
					LuaRequest::Prompt { message } => {
						return Ok(ThreadOutcome::Request(ActionRequest::ShowPrompt {
							message,
							callback: thread,
						}));
					}
					LuaRequest::Direction { message } => {
						return Ok(ThreadOutcome::Request(ActionRequest::ShowDirectionPrompt {
							message,
							callback: thread,
						}));
					}
				}
			} else {
				return Ok(ThreadOutcome::Value(value));
			}
		}
	}
}

impl Manager {
	pub fn consider_turn<'lua>(
		&mut self,
		scripts: &'lua resource::Scripts,
	) -> Result<Vec<Consider<'lua>>> {
		let next_character = self.next_character();

		let mut considerations = Vec::new();

		for character in self
			.characters
			.iter()
			.filter(|x| x.borrow().alliance != next_character.borrow().alliance)
		{
			let character = character.borrow();
			let x = character.x;
			let y = character.y;
			considerations.push(Consider {
				action: character::Action::Move(x, y),
				heuristics: vec![consider::Heuristic::Move { x, y }],
			})
		}

		for attack in &next_character.borrow().attacks {
			if let Some(on_consider) = &attack.on_consider {
				let attack_heuristics: mlua::Table = scripts
					.sandbox(on_consider)?
					.insert("UseTime", attack.use_time)?
					.insert(
						"Magnitude",
						u32::evalv(&attack.magnitude, &*next_character.borrow()),
					)?
					.insert("User", next_character.clone())?
					.insert("Heuristic", consider::HeuristicConstructor)?
					.world(self, ())?;
				for heuristics in attack_heuristics.sequence_values::<mlua::Table>() {
					let heuristics = heuristics?;
					let arguments = heuristics.get("arguments")?;
					let heuristics = heuristics.get("heuristics")?;
					considerations.push(Consider {
						action: character::Action::Attack(attack.clone(), Some(arguments)),
						heuristics,
					})
				}
			}
		}

		for spell in &next_character.borrow().spells {
			if let (spell::Castable::Yes, Some(on_consider)) = (
				spell.castable_by(&next_character.borrow()),
				&spell.on_consider,
			) {
				let parameters = spell.parameter_table(scripts, &*next_character.borrow())?;
				let spell_heuristics: mlua::Table = scripts
					.sandbox(on_consider)?
					.insert("Parameters", parameters)?
					.insert("User", next_character.clone())?
					// Maybe these should be members of the spell?
					.insert("Level", spell.level)?
					.insert("Affinity", spell.affinity(&next_character.borrow()))?
					.world(self, ())?;
				for heuristics in spell_heuristics.sequence_values::<mlua::Table>() {
					let heuristics = heuristics?;
					let arguments = heuristics.get("arguments")?;
					let heuristics = heuristics.get("heuristics")?;
					considerations.push(Consider {
						action: character::Action::Cast(spell.clone(), Some(arguments)),
						heuristics,
					})
				}
			}
		}

		Ok(considerations)
	}

	pub fn consider_action<'lua>(
		&self,
		scripts: &'lua resource::Scripts,
		character: CharacterRef,
		mut considerations: Vec<Consider<'lua>>,
	) -> Result<character::Action<'lua>> {
		Ok(scripts
			.sandbox(&character.borrow().sheet.on_consider)?
			.insert("User", character.clone())?
			.call::<Option<usize>>(
				scripts
					.runtime
					.create_sequence_from(considerations.iter().map(TaggedHeuristics::new))?,
			)?
			.map(|index| considerations.remove(index - 1).action)
			.unwrap_or(character::Action::Wait(TURN)))
	}

	pub fn next_turn<'lua>(
		&mut self,
		console: &Console,
		scripts: &'lua resource::Scripts,
		action: character::Action<'lua>,
	) -> Result<TurnOutcome<'lua>> {
		let next_character = self.next_character();

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
			character::Action::Wait(delay) => Ok(TurnOutcome::Action { delay }),
			character::Action::Move(target_x, target_y) => {
				let (x, y) = {
					let next_character = next_character.borrow();
					(next_character.x, next_character.y)
				};
				// For distances of 1 tile, don't bother using a dijkstra map.
				if let Some(direction) = OrdDir::from_offset(target_x - x, target_y - y) {
					self.move_piece(next_character, direction, console)
				} else {
					let mut dijkstra = astar::DijkstraMap::target(
						self.current_floor.map.cols(),
						self.current_floor.map.rows(),
						&[(target_x, target_y)],
					);
					if let Ok(x) = x.try_into()
						&& let Ok(y) = y.try_into()
					{
						dijkstra.explore(x, y, |x, y, base| {
							if let Some(character) = self.get_character_at(x as i32, y as i32)
								&& character.as_ptr() != next_character.as_ptr()
								&& character.borrow().alliance == next_character.borrow().alliance
							{
								return astar::IMPASSABLE;
							}
							match self.current_floor.map.get(y, x) {
								Some(floor::Tile::Floor) | Some(floor::Tile::Exit) => base + 1,
								Some(floor::Tile::Wall) | None => astar::IMPASSABLE,
							}
						});
					}
					if let Some(direction) = dijkstra.step(x, y) {
						self.move_piece(next_character, direction, console)
					} else {
						Ok(TurnOutcome::Yield)
					}
				}
			}
			character::Action::Attack(attack, arguments) => {
				Ok(self.attack_piece(scripts, attack, next_character, arguments)?)
			}
			character::Action::Cast(spell, arguments) => {
				let castable = spell.castable_by(&next_character.borrow());
				match castable {
					spell::Castable::Yes => {
						let affinity = spell.affinity(&next_character.borrow());

						let thread = scripts
							.sandbox(&spell.on_cast)?
							.insert("Arguments", arguments)?
							.insert(
								"Parameters",
								spell.parameter_table(scripts, &*next_character.borrow())?,
							)?
							.insert("User", next_character.clone())?
							// Maybe these should be members of the spell?
							.insert("Level", spell.level)?
							.insert("Affinity", affinity)?
							.thread()?;
						Ok(TurnOutcome::poll(self, scripts.runtime, thread, ())?)
					}
					spell::Castable::NotEnoughSP => {
						let message =
							format!("{{Address}} doesn't have enough SP to cast {}.", spell.name)
								.replace_nouns(&next_character.borrow().sheet.nouns);
						console.print_system(message);
						Ok(TurnOutcome::Yield)
					}
					spell::Castable::UncastableAffinity => {
						let message =
							format!("{{Address}} has the wrong affinity to cast {}.", spell.name)
								.replace_nouns(&next_character.borrow().sheet.nouns);
						console.print_system(message);
						Ok(TurnOutcome::Yield)
					}
				}
			}
		}
	}

	pub fn attack_piece<'lua>(
		&self,
		scripts: &'lua resource::Scripts,
		attack: Rc<Attack>,
		user: &CharacterRef,
		arguments: Option<mlua::Table<'lua>>,
	) -> Result<TurnOutcome<'lua>> {
		// Calculate damage
		let magnitude = u32::evalv(&attack.magnitude, &*user.borrow());

		let thread = scripts
			.sandbox(&attack.on_use)?
			.insert("User", user.clone())?
			.insert("Arguments", arguments)?
			.insert("UseTime", attack.use_time)?
			.insert("Magnitude", magnitude)?
			.thread()?;
		Ok(TurnOutcome::poll(self, scripts.runtime, thread, ())?)
	}

	pub fn move_piece<'lua>(
		&self,
		character: &CharacterRef,
		dir: OrdDir,
		console: &Console,
	) -> Result<TurnOutcome<'lua>> {
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

		let tile = self.current_floor.map.get(y, x);
		match tile {
			Some(Tile::Floor) | Some(Tile::Exit) => {
				let mut character = character.borrow_mut();
				character.x = x;
				character.y = y;
				Ok(TurnOutcome::Action { delay })
			}
			Some(Tile::Wall) => {
				console.say(character.borrow().sheet.nouns.name.clone(), "Ouch!".into());
				Ok(TurnOutcome::Yield)
			}
			None => {
				console.print_system("You stare out into the void: an infinite expanse of nothingness enclosed within a single tile.".into());
				Ok(TurnOutcome::Yield)
			}
		}
	}
}
