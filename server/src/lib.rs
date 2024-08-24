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

use esprit2::prelude::*;
use std::path::PathBuf;
use std::process::exit;
use tracing::error;

/// Server state
///
/// These fields are public for now but it might make sense to better encapsulate the server in the future.
pub struct Server {
	resource_directory: PathBuf,

	pub resources: resource::Manager,

	// These fields should be kept in sync with the client.
	pub console: console::Handle,
	pub world: world::Manager,
}

impl Server {
	pub fn new(console: console::Handle, resource_directory: PathBuf) -> Self {
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
		world.generate_floor(
			"default seed",
			&vault::Set {
				vaults: vec!["example".into()],
				density: 4,
				hall_ratio: 1,
			},
			&resources,
		);

		Self {
			resource_directory,

			resources,

			console,
			world,
		}
	}

	pub fn act(
		&mut self,
		scripts: &resource::Scripts,
		action: character::Action,
	) -> esprit2::Result<()> {
		let delay = self
			.world
			.next_turn(&self.console, scripts, action)?
			.unwrap_or(TURN);
		self.world
			.characters
			.retain(|character| character.borrow().hp > 0);

		let character = self
			.world
			.characters
			.pop_front()
			.expect("next_turn's element should still exist");
		character.borrow_mut().action_delay = delay;
		// Insert the character into the queue,
		// immediately before the first character to have a higher action delay.
		// self.world assumes that the queue is sorted.
		self.world.characters.insert(
			self.world
				.characters
				.iter()
				.enumerate()
				.find(|x| x.1.borrow().action_delay > delay)
				.map(|x| x.0)
				.unwrap_or(self.world.characters.len()),
			character,
		);
		Ok(())
	}
}
