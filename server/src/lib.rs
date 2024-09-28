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
use protocol::{PacketReceiver, PacketSender};
use std::collections::VecDeque;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{io, thread};

#[derive(Clone, Debug, Eq, PartialEq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ClientIdentification(String);

pub struct Client {
	pub stream: TcpStream,
	pub receiver: PacketReceiver,
	pub senders: VecDeque<PacketSender>,

	pub ping: Instant,
	/// This field remains `None` until the client performs authentication.
	/// In this state, most client packets will be discarded without response.
	pub identifier: Option<ClientIdentification>,
	pub owned_pieces: Vec<character::Ref>,
}

impl Client {
	pub fn send(&mut self) -> io::Result<()> {
		while let Some(sender) = self.senders.front_mut() {
			sender.send(&mut self.stream)?;
			self.senders.pop_front();
		}
		Ok(())
	}
}

/// Server state
///
/// These fields are public for now but it might make sense to better encapsulate the server in the future.
pub struct Server {
	pub clients: Vec<Client>,

	pub resources: resource::Manager,
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
				sheet: "luvui".into(),
				accent_color: (0xDA, 0x2D, 0x5C, 0xFF),
			},
			world::PartyReferenceBase {
				sheet: "aris".into(),
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

		Self {
			resources,
			clients: Vec::new(),
			world,
		}
	}

	pub fn tick(
		&mut self,
		scripts: &resource::Scripts,
		console: impl console::Handle,
	) -> esprit2::Result<bool> {
		let character = self.world.next_character();
		if !character.borrow().player_controlled {
			let considerations = self.world.consider_turn(&self.resources, scripts)?;
			let action = self
				.world
				.consider_action(scripts, character.clone(), considerations)?;
			self.world
				.perform_action(&console, &self.resources, scripts, action)?;
			Ok(true)
		} else {
			Ok(false)
		}
	}
}

/// Recieve operations
// TODO: Multiple clients.
impl Server {
	pub fn recv_ping(&self, client: &mut Client) {
		let ms = client.ping.elapsed().as_millis();
		if ms > 50 {
			info!(client = "client", ms, "recieved late ping")
		}
		client.ping = Instant::now();
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
	pub fn send_ping(&mut self, client: &mut Client) -> Option<()> {
		client.ping = Instant::now();
		Some(())
	}

	/// Returns an archived version of the world state, as an array of bytes.
	pub fn send_world(&self) -> Option<&world::Manager> {
		Some(&self.world)
	}
}

#[derive(Clone, Debug)]
pub struct Console {
	sender: mpsc::Sender<console::Message>,
}

impl console::Handle for Console {
	fn send_message(&self, message: console::Message) {
		let _ = self.sender.send(message);
	}
}

pub fn connection(router: mpsc::Receiver<TcpStream>, res: PathBuf) {
	const TIMEOUT: Duration = Duration::from_secs(10);

	// Create a Lua runtime.
	let lua = mlua::Lua::new();

	lua.globals()
		.get::<&str, mlua::Table>("package")
		.unwrap()
		.set("path", res.join("scripts/?.lua").to_string_lossy())
		.unwrap();

	let scripts = resource::Scripts::open(res.join("scripts"), &lua).unwrap();

	let (sender, console_reciever) = mpsc::channel();
	let console_handle = Console { sender };
	// For now, this spins up a new server for each connection
	// TODO: Route connections to the same instance.
	let mut server = Server::new(res);

	lua.globals()
		.set("Console", console::LuaHandle(console_handle.clone()))
		.unwrap();
	lua.globals()
		.set("Status", server.resources.statuses_handle())
		.unwrap();
	lua.globals()
		.set("Heuristic", consider::HeuristicConstructor)
		.unwrap();
	lua.globals().set("Log", combat::LogConstructor).unwrap();

	loop {
		for stream in router.try_iter() {
			stream.set_nonblocking(true).unwrap();
			let client = Client {
				stream,
				receiver: protocol::PacketReceiver::default(),
				// Give the client an intial world state.
				// TODO: don't special-case this.
				senders: VecDeque::from([PacketSender::new(
					&(protocol::ServerPacket::World {
						world: &server.world,
					}),
				)]),

				ping: Instant::now(),
				identifier: None,
				owned_pieces: Vec::new(),
			};
			server.clients.push(client);
		}

		for client in &mut server.clients {
			match client.send() {
				Ok(()) => {}
				Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
				Err(e) => {
					error!("failed to write to client stream: {e}");
				}
			}
			let result = client.receiver.recv(&mut client.stream, |packet| {
				let packet = rkyv::access::<_, rkyv::rancor::Error>(&packet).unwrap();
				match packet {
					protocol::ArchivedClientPacket::Ping => {
						let ms = client.ping.elapsed().as_millis();
						if ms > 50 {
							info!(
								client = client.identifier.as_ref().map(|x| &x.0),
								ms, "recieved late ping"
							)
						}
						client.ping = Instant::now();
					}
					protocol::ArchivedClientPacket::Authenticate(username) => {
						client.identifier = Some(ClientIdentification(
							rkyv::deserialize::<_, rkyv::rancor::Error>(username).unwrap(),
						));
					}
					protocol::ArchivedClientPacket::Action(action_archive) => {
						let action: character::Action =
							rkyv::deserialize::<_, rkyv::rancor::Error>(action_archive).unwrap();
						let console = &console_handle;
						let scripts: &resource::Scripts = &scripts;
						let next_character = server.world.next_character();
						if next_character.borrow().player_controlled
							// TODO: What happens when a piece isn't owned by anyone (eg: by disconnect)?
							&& client.owned_pieces.contains(next_character)
						{
							server
								.world
								.perform_action(console, &server.resources, scripts, action)
								.unwrap();
						}
					}
				}
			});
			match result {
				Ok(()) => {}
				Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
				Err(e) => {
					error!("failed to read client stream: {e}");
				}
			}

			// This check has to happen after recieving packets to be as charitable to the client as possible.
			if client.ping.elapsed() > TIMEOUT {
				info!(player = "player", "disconnected by timeout");
			}
			for i in console_reciever.try_iter() {
				client
					.senders
					.push_back(PacketSender::new(&protocol::ServerPacket::Message(i)));
			}
		}

		if server.tick(&scripts, &console_handle).unwrap() {
			for client in &mut server.clients {
				client
					.senders
					.push_back(PacketSender::new(&protocol::ServerPacket::World {
						world: &server.world,
					}));
			}
		} else {
			// Very short sleep, just to avoid busy waiting.
			// Please let me know if there's a way I can wait for TCP traffic.
			thread::sleep(Duration::from_millis(1));
		}
	}
}
