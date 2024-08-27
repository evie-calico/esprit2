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
use tracing::{error, warn};

pub struct Player {
	// TODO: controlled_characters.
	prospective_action: Option<character::Action>,
	ping: Option<Instant>,
}

/// Server state
///
/// These fields are public for now but it might make sense to better encapsulate the server in the future.
pub struct Server {
	resource_directory: PathBuf,

	pub resources: resource::Manager,
	pub players: Player,

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
			// Start with no players/connections.
			players: Player {
				prospective_action: None,
				ping: None,
			},

			console,
			world,
		}
	}

	pub fn tick(&mut self, scripts: &resource::Scripts) -> esprit2::Result<()> {
		if let Some(action) = self.players.prospective_action.take() {
			self.world
				.perform_action(&self.console, &self.resources, scripts, action)?;
		}

		Ok(())
	}
}

/// Recieve operations
// TODO: Multiple clients.
impl Server {
	pub fn recv_ping(&mut self) {
		if let Some(ping) = &mut self.players.ping {
			let ms = ping.elapsed().as_millis();
			if ms > 50 {
				info!("recieved ping after {ms}ms (slow) from {{client}}")
			}
			*ping = Instant::now();
		} else {
			warn!("recieved unexpected ping packet");
		}
	}

	pub fn recv_action(&mut self, action: character::Action) {
		self.players.prospective_action = Some(action);
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
		self.players.ping.get_or_insert(Instant::now());
		Some(())
	}

	/// Returns an archived version of the world state, as an array of bytes.
	pub fn send_world(&self) -> Option<&world::Manager> {
		Some(&self.world)
	}
}
