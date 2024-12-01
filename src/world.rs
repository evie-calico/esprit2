use crate::prelude::*;
use rand::{seq::SliceRandom, SeedableRng};
use std::{collections::VecDeque, rc::Rc};

/// This struct contains all information that is relevant during gameplay.
#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Manager {
	/// Where in the world the characters are.
	pub location: Location,
	pub current_floor: Floor,
	// It might be useful to sort this by remaining action delay to make selecting the next character easier.
	pub characters: VecDeque<character::Ref>,
	pub items: Vec<item::Piece>,
	/// Always point to the party's pieces, even across floors.
	/// When exiting a dungeon, these sheets will be saved to a party struct.
	pub party: Vec<PartyReference>,
	pub inventory: Vec<String>,
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct PartyReference {
	/// The piece that is being used by this party member.
	pub piece: character::Ref,
	/// Displayed on the pamphlet.
	pub accent_color: Color,
}

impl PartyReference {
	pub fn new(piece: character::Ref, accent_color: Color) -> Self {
		Self {
			piece,
			accent_color,
		}
	}
}

// this is probably uneccessary and just makes main.rs look nicer
pub struct PartyReferenceBase {
	pub sheet: Box<str>,
	pub accent_color: Color,
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Location {
	/// Which level is currently loaded.
	pub level: String,
	pub floor: usize,
}

impl Manager {
	pub fn new(
		party_blueprint: impl Iterator<Item = PartyReferenceBase>,
		resources: &resource::Manager,
	) -> Result<Self> {
		let mut party = Vec::new();
		let mut characters = VecDeque::new();

		let mut player_controlled = true;

		for PartyReferenceBase {
			sheet,
			accent_color,
		} in party_blueprint
		{
			let sheet = resources.sheets.get(&sheet)?;
			let character = character::Ref::new(character::Piece {
				player_controlled,
				alliance: character::Alliance::Friendly,
				..character::Piece::new((**sheet).clone())
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

	pub fn new_floor(
		&mut self,
		resources: &resource::Manager,
		lua: &mlua::Lua,
		console: &impl console::Handle,
	) -> Result<()> {
		self.location.floor += 1;
		console.print_important(format!("Entering floor {}", self.location.floor));
		self.current_floor = Floor::default();

		self.characters
			.retain(|x| self.party.iter().any(|y| x.as_ptr() == y.piece.as_ptr()));

		console.print_unimportant("You take some time to rest...");
		for i in &self.characters {
			let mut i = i.borrow_mut();
			// Reset positions
			i.x = 0;
			i.y = 0;
			// Rest
			i.rest(resources, lua)?;
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
		// TODO: Generate floor
		Ok(())
	}

	pub fn next_character(&self) -> &character::Ref {
		&self.characters[0]
	}

	pub fn get_character_at(&self, x: i32, y: i32) -> Option<&character::Ref> {
		self.characters.iter().find(|p| {
			let p = p.borrow();
			p.x == x && p.y == y
		})
	}

	pub fn generate_floor(
		&mut self,
		seed: &str,
		set: &vault::Set,
		resources: &resource::Manager,
	) -> Result<()> {
		const SEED_LENGTH: usize = 32;

		let _enter = tracing::error_span!("level gen", seed).entered();
		let mut seed_slice = [0; SEED_LENGTH];
		for (str_byte, seed_byte) in seed.bytes().take(SEED_LENGTH).zip(seed_slice.iter_mut()) {
			*seed_byte = str_byte;
		}
		let mut rng = rand::rngs::StdRng::from_seed(seed_slice);

		let mut edges = vec![(4, 4)];

		'placement: for _ in 0..set.density {
			// This loop allows for retries each time placement fails.
			// These retries are safe because edges are always discarded whether or not they succeed,
			// meaning a full board will eventually exhaust its edges.
			'edges: loop {
				// partial_shuffle swaps the randomly selected edge with the last edge,
				// returning the remaining halves in reverse order.
				let (placement_edge, _) = edges.partial_shuffle(&mut rng, 1);
				// This slice should only ever be 0 or 1 elements.
				let Some((px, py)) = placement_edge.first().copied() else {
					// If there are no remaining edges, we cannot place any more vaults.
					break 'placement;
				};
				// Remove the placement edge we chose.
				edges.pop();
				let Some(vault) = set.vaults.choose(&mut rng) else {
					warn!("set has no vaults");
					break 'placement;
				};
				let vault = resources.vaults.get(vault)?;
				// for every possible edge of the vault (shuffled), check if it fits.
				let mut potential_edges = vault.edges.clone();
				potential_edges.shuffle(&mut rng);
				for (i, (ex, ey)) in potential_edges.iter().enumerate() {
					// adjust the placment position so that px, py and ex, ey overlap.
					let x = px - ex;
					let y = py - ey;
					if self.try_apply_vault(x, y, vault, resources)? {
						for (px, py) in potential_edges
							.iter()
							.take(i)
							.chain(potential_edges.iter().skip(i + 1))
						{
							edges.push((x + px, y + py));
						}
						break 'edges;
					}
				}
			}
		}

		Ok(())
	}

	pub fn try_apply_vault(
		&mut self,
		x: i32,
		y: i32,
		vault: &Vault,
		resources: &resource::Manager,
	) -> Result<bool> {
		for (row, y) in vault
			.tiles
			.chunks(vault.width)
			.zip(y..(y + vault.height() as i32))
		{
			for (tile, x) in row.iter().zip(x..(x + vault.width as i32)) {
				if tile.is_some() && self.current_floor.get(x, y).is_some() {
					return Ok(false);
				}
			}
		}

		for (row, y) in vault
			.tiles
			.chunks(vault.width)
			.zip(y..(y + vault.height() as i32))
		{
			for (tile, x) in row.iter().zip(x..(x + vault.width as i32)) {
				if let Some(tile) = tile {
					*self.current_floor.get_mut(x, y) = Some(*tile);
				}
			}
		}

		for (xoff, yoff, sheet) in &vault.characters {
			let piece = character::Piece {
				x: x + xoff,
				y: y + yoff,
				..character::Piece::new((**resources.sheets.get(sheet)?).clone())
			};
			self.characters.push_front(character::Ref::new(piece));
		}

		Ok(true)
	}
}

impl Manager {
	/// Returns whether or not world has permission to perform an action internally.
	/// If false, no work is dpne.
	pub fn tick(
		&mut self,
		resources: &resource::Manager,
		lua: &mlua::Lua,
		console: impl console::Handle,
	) -> Result<bool> {
		let character = self.next_character();
		if !character.borrow().player_controlled {
			let action = self.consider_action(resources, lua, character.clone())?;
			self.perform_action(&console, resources, lua, action)?;
			Ok(true)
		} else {
			Ok(false)
		}
	}

	pub fn consider_action(
		&self,
		resources: &resource::Manager,
		lua: &mlua::Lua,
		character: character::Ref,
	) -> Result<character::Action> {
		let thread = lua.create_thread(
			resources
				.functions
				.get(&character.borrow().sheet.on_consider)?
				.clone(),
		)?;
		Ok(self
			.poll::<Option<Consider>>(lua, thread, character)?
			.map(|x| x.action)
			.unwrap_or(character::Action::Wait(TURN)))
	}

	/// Causes the next character in the queue to perform a given action.
	pub fn perform_action(
		&mut self,
		console: impl console::Handle,
		resources: &resource::Manager,
		lua: &mlua::Lua,
		action: character::Action,
	) -> Result<()> {
		let next_character = self.next_character().clone();

		let delay = next_character.borrow().action_delay;
		// The delay represents how many auts must pass until this character's next action.
		// If the next character in the queue has a delay higher than 0,
		// then all other characters get their delays decreased as well while the next character "waits" for their action.
		for i in &self.characters {
			let action_delay = &mut i.borrow_mut().action_delay;
			*action_delay = action_delay.saturating_sub(delay);
		}
		// Once an action has been provided, pending turn updates may run.
		next_character.borrow_mut().new_turn(resources);

		let delay = match action {
			character::Action::Wait(delay) => Some(delay),
			character::Action::Move(target_x, target_y) => {
				let (x, y) = {
					let next_character = next_character.borrow();
					(next_character.x, next_character.y)
				};
				// For distances of 1 tile, don't bother using a dijkstra map.
				if let Some(direction) = OrdDir::from_offset(target_x - x, target_y - y) {
					self.move_piece(&next_character, direction, console)?
				} else {
					let mut dijkstra = astar::Floor::target(&[(target_x, target_y)]);
					dijkstra.explore(x, y, |x, y, base| {
						if let Some(character) = self.get_character_at(x, y)
							&& character.as_ptr() != next_character.as_ptr()
							&& character.borrow().alliance == next_character.borrow().alliance
						{
							return astar::IMPASSABLE;
						}
						match self.current_floor.get(x, y) {
							Some(floor::Tile::Floor) | Some(floor::Tile::Exit) => base + 1,
							Some(floor::Tile::Wall) | None => astar::IMPASSABLE,
						}
					});
					if let Some(direction) = dijkstra.step(x, y) {
						self.move_piece(&next_character, direction, console)?
					} else {
						None
					}
				}
			}
			character::Action::Attack(attack, arguments) => self.attack(
				lua,
				resources.attacks.get(&attack)?.clone(),
				next_character,
				arguments,
			)?,
			character::Action::Cast(spell, arguments) => self.cast(
				resources.spells.get(&spell)?.clone(),
				next_character,
				lua,
				arguments,
				console,
			)?,
		};

		// Remove dead characters.
		// TODO: Does this belong here?
		self.characters
			.retain(|character| character.borrow().hp > 0);

		let character = self
			.characters
			.pop_front()
			.expect("next_turn's element should still exist");
		// TODO: A turn should never result in a None. earlier versions of the engine used this to cancel actions.
		let delay = delay.unwrap_or(TURN);
		character.borrow_mut().action_delay = delay;
		// Insert the character into the queue,
		// immediately before the first character to have a higher action delay.
		// self.world assumes that the queue is sorted.
		self.characters.insert(
			self.characters
				.iter()
				.enumerate()
				.find(|x| x.1.borrow().action_delay > delay)
				.map(|x| x.0)
				.unwrap_or(self.characters.len()),
			character,
		);
		Ok(())
	}

	fn cast(
		&mut self,
		spell: Rc<Spell>,
		user: character::Ref,
		lua: &mlua::Lua,
		argument: Value,
		console: impl console::Handle,
	) -> Result<Option<u32>, Error> {
		let castable = spell.castable_by(&user.borrow());
		Ok(match castable {
			spell::Castable::Yes => self.poll::<Option<Aut>>(
				lua,
				lua.create_thread(spell.on_cast.clone())?,
				(user, spell, argument),
			)?,
			spell::Castable::NotEnoughSP => {
				let message = format!("{{Address}} doesn't have enough SP to cast {}.", spell.name)
					.replace_nouns(&user.borrow().sheet.nouns);
				console.print_system(message);
				None
			}
			spell::Castable::UncastableAffinity => {
				let message = format!("{{Address}} has the wrong affinity to cast {}.", spell.name)
					.replace_nouns(&user.borrow().sheet.nouns);
				console.print_system(message);
				None
			}
		})
	}

	pub fn attack(
		&self,
		lua: &mlua::Lua,
		attack: Rc<Attack>,
		user: character::Ref,
		argument: Value,
	) -> Result<Option<Aut>> {
		let thread = lua.create_thread(attack.on_use.clone())?;
		Ok(self.poll::<Option<Aut>>(lua, thread, (user, attack, argument))?)
	}

	pub fn move_piece(
		&self,
		character: &character::Ref,
		dir: OrdDir,
		console: impl console::Handle,
	) -> Result<Option<Aut>> {
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

		let tile = self.current_floor.get(x, y);
		match tile {
			Some(Tile::Floor) | Some(Tile::Exit) => {
				let mut character = character.borrow_mut();
				character.x = x;
				character.y = y;
				Ok(Some(delay))
			}
			Some(Tile::Wall) => {
				console.say(character.borrow().sheet.nouns.name.clone(), "Ouch!");
				Ok(None)
			}
			None => {
				console.print_system(
					"You stare out into the void: an infinite expanse of nothingness enclosed within a single tile."
				);
				Ok(None)
			}
		}
	}

	pub fn poll<T: mlua::FromLua>(
		&self,
		lua: &mlua::Lua,
		thread: mlua::Thread,
		args: impl mlua::IntoLuaMulti,
	) -> mlua::Result<T> {
		let mut value = thread.resume(args)?;
		loop {
			// A resumable thread is expecting an action request response.
			if thread.status() == mlua::ThreadStatus::Resumable {
				match <LuaRequest as mlua::FromLua>::from_lua(value, lua)? {
					LuaRequest::Characters { query } => {
						value = match query {
							Some(LuaCharacterQuery::Within { x, y, range }) => thread.resume(
								lua.create_sequence_from(
									self.characters
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
								lua.create_sequence_from(self.characters.iter().cloned())?,
							)?,
						}
					}
					LuaRequest::Tile { x, y } => {
						value = thread.resume(self.current_floor.get(x, y))?;
					}
				}
			} else {
				return T::from_lua(value, lua);
			}
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) enum LuaCharacterQuery {
	Within { x: i32, y: i32, range: u32 },
}

/// Handle requests for extra information from a lua function.
#[derive(Clone, Debug, mlua::FromLua)]
pub(crate) enum LuaRequest {
	// World manager communication
	Characters { query: Option<LuaCharacterQuery> },
	Tile { x: i32, y: i32 },
}

impl mlua::UserData for LuaRequest {}
