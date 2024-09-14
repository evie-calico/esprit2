//! Esprit 2's server implementation
//!
//! A server defines an interface for interacting with the underlying
//! esprit 2 engine.
//! This server may do anything with its connections: manage multiple players,
//! allow spectators to watch a game, or run a chat room.
//!
//! Keep in mind that this is just *a* server implementation,
//! and it may be freely swapped out for other server protocols as needed.
//! (though, clients will need to be adapted to support new server implementations
//! if the protocol changes)

#![feature(anonymous_lifetime_in_impl_trait, once_cell_try)]

pub mod protocol;

use esprit2::prelude::*;
use std::path::PathBuf;
use std::process::exit;
use std::time::Instant;

pub struct Player {
	pub ping: Instant,
}

/// Server state
///
/// These fields are public for now but it might make sense to better encapsulate the server in the future.
pub struct Server {
	pub resources: resource::Manager,
	pub players: Player,

	// These fields should be kept in sync with the client.
	pub world: world::Manager,
}

impl Server {
	pub fn new(resource_directory: PathBuf) -> Self {
		// Game initialization.
		let resources = match resource::Manager::open(&resource_directory) {
			Ok(resources) => resources,
			Err(msg) => {
				error!("failed to open resource directory: {msg}");
				exit(1);
			}
		};

		// Create a piece for the player, and register it with the world manager.
		let party_blueprint = [
			world::PartyReferenceBase {
				sheet: "luvui",
				accent_color: (0xDA, 0x2D, 0x5C, 0xFF),
			},
			world::PartyReferenceBase {
				sheet: "aris",
				accent_color: (0x0C, 0x94, 0xFF, 0xFF),
			},
		];
		let mut world = world::Manager::new(party_blueprint.into_iter(), &resources)
			.unwrap_or_else(|msg| {
				error!("failed to initialize world manager: {msg}");
				exit(1);
			});
		world
			.generate_floor(
				"default seed",
				&vault::Set {
					vaults: vec!["example".into()],
					density: 4,
					hall_ratio: 1,
				},
				&resources,
			)
			.unwrap();
		world.characters[2].borrow_mut().x = 4;
		world.characters[2].borrow_mut().y = 4;

		Self {
			resources,
			// Start with no players/connections.
			players: Player {
				ping: Instant::now(),
			},
			world,
		}
	}

	pub fn tick(
		&mut self,
		scripts: &resource::Scripts,
		console: impl console::Handle,
	) -> esprit2::Result<()> {
		let character = self.world.next_character();
		if !character.borrow().player_controlled {
			let considerations = self.world.consider_turn(&self.resources, scripts)?;
			let action = self
				.world
				.consider_action(scripts, character.clone(), considerations)?;
			self.world
				.perform_action(&console, &self.resources, scripts, action)?;
		}
		Ok(())
	}
}

/// Recieve operations
// TODO: Multiple clients.
impl Server {
	pub fn recv_ping(&mut self) {
		let ms = self.players.ping.elapsed().as_millis();
		if ms > 50 {
			info!("recieved late ping after {ms}ms from {{client}}")
		}
		self.players.ping = Instant::now();
	}

	pub fn recv_action(
		&mut self,
		console: impl console::Handle,
		scripts: &resource::Scripts,
		action: character::Action,
	) -> esprit2::Result<()> {
		if self.world.next_character().borrow().player_controlled {
			self.world
				.perform_action(&console, &self.resources, scripts, action)?;
		}
		Ok(())
	}
}

/// Send operations.
// TODO: Multiple clients.
impl Server {
	/// Check if the server is ready to ping this client.
	///
	/// # Returns
	/// `Some(())` if a ping packet should be sent.
	pub fn send_ping(&mut self) -> Option<()> {
		self.players.ping = Instant::now();
		Some(())
	}

	/// Returns an archived version of the world state, as an array of bytes.
	pub fn send_world(&self) -> Option<&world::Manager> {
		Some(&self.world)
	}
}
