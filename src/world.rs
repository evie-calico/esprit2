use crate::prelude::*;
use consider::TaggedHeuristics;
use mlua::LuaSerdeExt;
use rand::{seq::SliceRandom, SeedableRng};
use std::collections::VecDeque;

/// This struct contains all information that is relevant during gameplay.
#[derive(
	Debug, serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
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

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
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
	pub sheet: resource::Sheet,
	pub accent_color: Color,
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
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
			let sheet = resource_manager.get(&sheet)?;
			let character = character::Ref::new(character::Piece {
				player_controlled,
				alliance: character::Alliance::Friendly,
				..character::Piece::new(sheet.clone())
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

	pub fn new_floor(&mut self, console: &impl console::Handle) -> Result<()> {
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
			i.rest()?;
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
				debug!("attempting placement at one of: {edges:?}");
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
				let vault = resources.get(vault)?;
				// for every possible edge of the vault (shuffled), check if it fits.
				let mut potential_edges = vault.edges.clone();
				potential_edges.shuffle(&mut rng);
				for (i, (ex, ey)) in potential_edges.iter().enumerate() {
					// adjust the placment position so that px, py and ex, ey overlap.
					let x = px - ex;
					let y = py - ey;
					if self.try_apply_vault(x, y, vault, resources)? {
						debug!(x, y, "placed vault");
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
				..character::Piece::new(resources.get(sheet)?.clone())
			};
			self.characters.push_front(character::Ref::new(piece));
		}

		Ok(true)
	}
}

impl Manager {
	pub fn consider_turn(
		&self,
		resources: &resource::Manager,
		scripts: &resource::Scripts,
	) -> Result<Vec<Consider>> {
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

		for attack_id in next_character.borrow().sheet.attacks.iter() {
			let attack = resources.get(attack_id)?;
			if let Some(on_consider) = &attack.on_consider {
				let attack_heuristics: mlua::Table = scripts
					.sandbox(on_consider)?
					.insert("UseTime", attack.use_time)?
					.insert(
						"Magnitude",
						u32::evalv(&attack.magnitude, &*next_character.borrow())?,
					)?
					.insert("User", next_character.clone())?
					.world(self, ())?;
				for heuristics in attack_heuristics.sequence_values::<mlua::Table>() {
					let heuristics = heuristics?;
					let arguments = scripts.runtime.from_value(heuristics.get("arguments")?)?;
					let heuristics = heuristics.get("heuristics")?;
					considerations.push(Consider {
						action: character::Action::Attack(attack_id.clone(), arguments),
						heuristics,
					})
				}
			}
		}

		for spell_id in next_character.borrow().sheet.spells.iter() {
			let spell = resources.get(spell_id)?;
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
					let arguments = scripts.runtime.from_value(heuristics.get("arguments")?)?;
					let heuristics = heuristics.get("heuristics")?;
					considerations.push(Consider {
						action: character::Action::Cast(spell_id.clone(), arguments),
						heuristics,
					})
				}
			}
		}

		Ok(considerations)
	}

	pub fn consider_action(
		&self,
		scripts: &resource::Scripts,
		character: character::Ref,
		mut considerations: Vec<Consider>,
	) -> Result<character::Action> {
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

	/// Causes the next character in the queue to perform a given action.
	pub fn perform_action(
		&mut self,
		console: impl console::Handle,
		resources: &resource::Manager,
		scripts: &resource::Scripts,
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
		next_character.borrow_mut().new_turn();

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
			character::Action::Attack(attack, arguments) => {
				self.attack(scripts, resources.get(&attack)?, next_character, arguments)?
			}
			character::Action::Cast(spell, arguments) => self.cast(
				resources.get(&spell)?,
				next_character,
				scripts,
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
		spell: &Spell,
		user: character::Ref,
		scripts: &resource::Scripts,
		arguments: character::ActionArgs,
		console: impl console::Handle,
	) -> Result<Option<u32>, Error> {
		let castable = spell.castable_by(&user.borrow());
		Ok(match castable {
			spell::Castable::Yes => {
				let affinity = spell.affinity(&user.borrow());
				let parameters = spell.parameter_table(scripts, &*user.borrow())?;
				let thread = scripts
					.sandbox(&spell.on_cast)?
					.insert("Arguments", scripts.runtime.to_value(&arguments)?)?
					.insert("Parameters", parameters)?
					.insert("User", user)?
					// Maybe these should be members of the spell?
					.insert("Level", spell.level)?
					.insert("Affinity", affinity)?
					.thread()?;
				self.poll::<Option<Aut>>(scripts.runtime, thread, ())?
			}
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
		scripts: &resource::Scripts,
		attack: &Attack,
		user: character::Ref,
		arguments: character::ActionArgs,
	) -> Result<Option<Aut>> {
		// Calculate damage
		let magnitude = u32::evalv(&attack.magnitude, &*user.borrow())?;

		let thread = scripts
			.sandbox(&attack.on_use)?
			.insert("User", user)?
			.insert("Arguments", scripts.runtime.to_value(&arguments)?)?
			.insert("UseTime", attack.use_time)?
			.insert("Magnitude", magnitude)?
			.thread()?;
		Ok(self.poll::<Option<Aut>>(scripts.runtime, thread, ())?)
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
				console.say(character.borrow().sheet.nouns.name.clone(), "Ouch!".into());
				Ok(None)
			}
			None => {
				console.print_system("You stare out into the void: an infinite expanse of nothingness enclosed within a single tile.".into());
				Ok(None)
			}
		}
	}

	pub fn poll<'lua, T: mlua::FromLua<'lua>>(
		&self,
		lua: &'lua mlua::Lua,
		thread: mlua::Thread<'lua>,
		args: impl mlua::IntoLuaMulti<'lua>,
	) -> mlua::Result<T> {
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
			Characters { query: Option<CharacterQuery> },
			Tile { x: i32, y: i32 },
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
						let tile = self.current_floor.get(x, y);
						value = thread.resume(
							tile.map(|x| lua.to_value(&x))
								.transpose()?
								.unwrap_or(mlua::Value::Nil),
						)?;
					}
				}
			} else {
				return T::from_lua(value, lua);
			}
		}
	}
}
