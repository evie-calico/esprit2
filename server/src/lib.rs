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

#![feature(anonymous_lifetime_in_impl_trait, once_cell_try, iter_array_chunks)]

pub mod protocol;

use esprit2::prelude::*;
use protocol::{ClientAuthentication, PacketStream, ServerPacket};
use rkyv::util::AlignedVec;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::{select, task};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Io(#[from] io::Error),

	#[error("failed to serialize packet: {0}")]
	Ser(rkyv::rancor::Error),
	#[error("failed to read packet: {0}")]
	Access(rkyv::rancor::Error),
	#[error("failed to deserialize packet: {0}")]
	De(rkyv::rancor::Error),
}

#[derive(Debug)]
pub struct Client {
	pub address: Box<str>,
	pub stream: PacketStream,

	pub ping: Instant,
	pub authentication: Option<ClientAuthentication>,
	pub requested_world: bool,
}

impl Client {
	pub fn new(stream: TcpStream) -> Self {
		let address = stream.peer_addr().unwrap().to_string().into_boxed_str();
		Self {
			address,
			stream: PacketStream::new(stream),
			ping: Instant::now(),
			authentication: None,
			requested_world: true,
		}
	}
}

pub(crate) struct Server {
	pub(crate) resources: resource::Manager,
	pub(crate) world: world::Manager,
}

impl Server {
	pub(crate) fn new(resource_directory: PathBuf) -> Self {
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

		Self { resources, world }
	}
}

#[derive(Clone, Debug)]
struct Console {
	sender: mpsc::UnboundedSender<console::Message>,
}

impl console::Handle for Console {
	fn send_message(&self, message: console::Message) {
		let _ = self.sender.send(message);
	}
}

type ClientIdentifier = u64;

#[derive(Debug)]
struct ClientParty {
	next_id: u64,
	clients: HashMap<ClientIdentifier, (Client, task::JoinHandle<()>)>,
	packet_sender: mpsc::Sender<(ClientIdentifier, AlignedVec)>,
	packet_reciever: mpsc::Receiver<(ClientIdentifier, AlignedVec)>,
}

impl Default for ClientParty {
	fn default() -> Self {
		let (packet_sender, packet_reciever) = mpsc::channel(64);
		Self {
			next_id: 0,
			clients: HashMap::new(),
			packet_sender,
			packet_reciever,
		}
	}
}

impl ClientParty {
	fn join(&mut self, mut client: Client) {
		let mut reciever = client.stream.recv.channel.take().unwrap();
		let sender = self.packet_sender.clone();
		let id = self.next_id;
		let task = task::spawn(async move {
			loop {
				sender
					.send((id, reciever.recv().await.unwrap()))
					.await
					.unwrap();
			}
		});
		self.clients.insert(id, (client, task));
		// I really don't think this will ever be reached,
		// but if it is the thread should just panic.
		self.next_id = self.next_id.checked_add(1).expect("out of client ids");
	}

	fn iter_mut(&mut self) -> impl Iterator<Item = &mut Client> {
		self.clients.values_mut().map(|(client, _task)| client)
	}
}

pub fn instance(mut router: mpsc::Receiver<Client>, res: PathBuf) {
	// Create a Lua runtime.
	let lua = mlua::Lua::new();

	lua.globals()
		.get::<&str, mlua::Table>("package")
		.unwrap()
		.set("path", res.join("scripts/?.lua").to_string_lossy())
		.unwrap();

	let scripts = resource::Scripts::open(res.join("scripts"), &lua).unwrap();

	let (sender, mut console_reciever) = mpsc::unbounded_channel();
	let console_handle = Console { sender };
	let mut server = Server::new(res);
	let mut clients = ClientParty::default();

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

	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async move {
			loop {
				select! {
					Some(client) = router.recv() => {
						client.stream.send(&protocol::ServerPacket::Ping).await;
						clients.join(client);
					}
					Some(i) = console_reciever.recv() => {
						for client in clients.iter_mut() {
							client
								.stream
								.send(&protocol::ServerPacket::Message(&i))
								.await;
						}
					}
					Some((id, packet)) = clients.packet_reciever.recv() => {
						if let Err(msg) = client_tick(
							&mut clients.clients.get_mut(&id).unwrap().0,
							packet,
							&console_handle,
							&scripts,
							&mut server,
						)
						.await {
							error!("client action failed: {msg}");
						}
					}
					_ = tokio::time::sleep(Duration::from_millis(1)) => {
					}
				}

				while server
					.world
					.tick(&server.resources, &scripts, &console_handle)
					.unwrap()
				{}

				let mut world_packet = None;
				for client in clients.iter_mut() {
					if client.requested_world {
						debug!("");
						client.requested_world = false;
						client
							.stream
							.forward(
								world_packet
									.get_or_insert_with(|| {
										to_bytes(&ServerPacket::World {
											world: &server.world,
										})
										.unwrap()
									})
									.clone(),
							)
							.await;
					}
				}

				if clients.clients.is_empty() {
					// TODO: Save to disk
					info!("no clients remain; closing instance");
					break;
				}
			}
		});
}

async fn client_tick(
	client: &mut Client,
	packet: AlignedVec,
	console_handle: &Console,
	scripts: &resource::Scripts<'_>,
	server: &mut Server,
) -> Result<(), Error> {
	let span = tracing::error_span!(
		"client",
		addr = client.address,
		username = tracing::field::Empty,
	);
	if let Some(auth) = &client.authentication {
		span.record("username", &auth.username);
	}
	let _span = span.entered();

	let packet = access(&packet).map_err(Error::Access)?;
	match packet {
		protocol::ArchivedClientPacket::Ping => {
			client.stream.send(&protocol::ServerPacket::Ping).await;
			client.ping = Instant::now();
		}
		protocol::ArchivedClientPacket::Action { action } => {
			let action: character::Action = deserialize(action).map_err(Error::De)?;
			let console = console_handle;
			let scripts: &resource::Scripts = scripts;
			let next_character = server.world.next_character();
			// TODO: Uuid-based piece ownership.
			// TODO: What happens when a piece isn't owned by anyone (eg: by disconnect)?
			if next_character.borrow().player_controlled {
				server
					.world
					.perform_action(console, &server.resources, scripts, action)
					.unwrap();
			} else {
				warn!("client attempted to move piece it did not own");
				client
					.stream
					.send(&protocol::ServerPacket::World {
						world: &server.world,
					})
					.await;
			}
		}
		protocol::ArchivedClientPacket::Authenticate(auth) => {
			let client_authentication = deserialize(auth).map_err(Error::De)?;
			info!(username = client_authentication.username, "authenticated");
			client.authentication = Some(client_authentication);
		}
		// Client is already routed!
		protocol::ArchivedClientPacket::Route(_route) => {}
	}
	Ok(())
}
