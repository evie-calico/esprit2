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

use esprit2::prelude::*;
use protocol::{
	ArchivedClientAuthentication, ClientAuthentication, ClientIdentifier, PacketReceiver,
	PacketSender, ServerPacket,
};
use rkyv::rancor;
use rkyv::util::AlignedVec;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{StreamExt, StreamMap};

pub mod protocol;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Engine(#[from] esprit2::Error),
	#[error(transparent)]
	Io(#[from] io::Error),

	#[error("failed to serialize packet: {0}")]
	Ser(rkyv::rancor::BoxedError),
	#[error("failed to read packet: {0}")]
	Access(rkyv::rancor::BoxedError),
	#[error("failed to deserialize packet: {0}")]
	De(rkyv::rancor::BoxedError),
}

#[derive(Debug)]
pub struct Client {
	pub address: Box<str>,
	sender: PacketSender,
	_receiver: PacketReceiver,

	pub ping: Instant,
	pub authentication: Option<ClientAuthentication>,
	pub requested_world: bool,
}

impl Client {
	pub fn new(stream: TcpStream) -> (Self, mpsc::Receiver<AlignedVec>) {
		let address = stream
			.peer_addr()
			.expect("missing peer address")
			.to_string()
			.into_boxed_str();
		let (receiver, sender) = stream.into_split();
		let (receiver, stream) = PacketReceiver::new(receiver);
		(
			Self {
				address,
				sender: PacketSender::new(sender),
				_receiver: receiver,
				ping: Instant::now(),
				authentication: None,
				requested_world: true,
			},
			stream,
		)
	}

	pub async fn ping(&mut self) -> Result<(), Error> {
		self.sender
			.send(&protocol::ServerPacket::Ping)
			.await
			.map_err(Error::Ser)?;
		self.ping = Instant::now();
		Ok(())
	}

	pub async fn authenticate(&mut self, auth: &ArchivedClientAuthentication) -> Result<(), Error> {
		let auth = rkyv::deserialize(auth).map_err(Error::De)?;
		info!(username = auth.username, "authenticated");
		self.authentication = Some(auth);
		Ok(())
	}
}

pub(crate) struct Server {
	pub(crate) resources: resource::Handle,
	pub(crate) world: world::Manager,
}

impl Server {
	pub(crate) fn new(resource_directory: PathBuf) -> esprit2::Result<Self> {
		// Game initialization.
		let resources = match resource::Manager::open(&resource_directory) {
			Ok(resources) => resource::Handle::new(resources.into()),
			Err(msg) => {
				error!("failed to open resource directory: {msg}");
				// TODO: I think these are out-of-date and should be moved to the caller.
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
		world.generate_floor(
			"default seed",
			&vault::Set {
				vaults: vec!["example".into()],
				density: 4,
				hall_ratio: 1,
			},
			&resources,
		)?;

		Ok(Self { resources, world })
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

#[derive(Debug)]
pub struct ClientParty {
	next_id: ClientIdentifier,
	clients: HashMap<ClientIdentifier, Client>,
	receiver: StreamMap<ClientIdentifier, ReceiverStream<AlignedVec>>,
}

impl Default for ClientParty {
	fn default() -> Self {
		Self {
			next_id: ClientIdentifier::default(),
			clients: HashMap::new(),
			receiver: StreamMap::new(),
		}
	}
}

impl ClientParty {
	pub fn join(&mut self, client: Client, receiver: ReceiverStream<AlignedVec>) {
		let id = self.next_id;
		self.clients.insert(id, client);
		self.receiver.insert(id, receiver);
		// I really don't think this will ever be reached,
		// but if it is the thread should just panic.
		self.next_id = self.next_id.checked_add(1).expect("out of client ids");
	}

	pub async fn next(&mut self) -> Option<(ClientIdentifier, &mut Client, AlignedVec)> {
		let (id, packet) = self.receiver.next().await?;
		let client = self
			.get_mut(&id)
			.expect("clients and receivers must have the same keys");
		Some((id, client, packet))
	}

	pub fn take(&mut self, id: ClientIdentifier) -> (Client, ReceiverStream<AlignedVec>) {
		(
			self.clients.remove(&id).expect("id must be valid"),
			self.receiver.remove(&id).expect("id must be valid"),
		)
	}
}

impl std::ops::Deref for ClientParty {
	type Target = HashMap<ClientIdentifier, Client>;

	fn deref(&self) -> &Self::Target {
		&self.clients
	}
}
impl std::ops::DerefMut for ClientParty {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.clients
	}
}

/// # Errors
///
/// Returns an error if the instance cannot be initialized.
pub fn instance(
	mut router: mpsc::Receiver<(Client, ReceiverStream<AlignedVec>)>,
	res: PathBuf,
) -> esprit2::Result<()> {
	// Create a Lua runtime.
	let lua = mlua::Lua::new();

	lua.globals()
		.get::<mlua::Table>("package")?
		.set("path", res.join("scripts/?.lua").to_string_lossy())?;

	let scripts = resource::Scripts::open(res.join("scripts"), &lua)?;

	let (sender, mut console_reciever) = mpsc::unbounded_channel();
	let console_handle = Console { sender };
	let mut server = Server::new(res)?;
	let mut clients = ClientParty::default();

	let handle = server.resources.clone();
	lua.load_from_function::<resource::Handle>(
		"resources",
		lua.create_function(move |_, ()| Ok(handle.clone()))?,
	)?;
	lua.globals()
		.set("Console", console::LuaHandle(console_handle.clone()))?;
	lua.globals()
		.set("Heuristic", consider::HeuristicConstructor)?;
	lua.globals().set("Action", character::ActionConstructor)?;
	lua.globals().set(
		"Consider",
		lua.create_function(|_lua, (action, heuristics)| Ok(Consider { action, heuristics }))?,
	)?;
	lua.globals().set("Log", combat::LogConstructor)?;

	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()?
		.block_on(async move {
			// This function is unusually lenient of errors in order to avoid unexpected shutdowns.
			'server: loop {
				select! {
					Some((client, receiver)) = router.recv() => {
						clients.join(client, receiver);
					}
					Some(i) = console_reciever.recv() => {
						for client in clients.values_mut() {
							if let Err(msg) = client
								.sender
								.send(&protocol::ServerPacket::Message(&i))
								.await
							{
								error!("failed to send console message to client: {msg}");
							}
						}
					}
					Some((_id, client, packet)) = clients.next() => {
						if let Err(msg) = client_tick(
							client,
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

				loop {
					match server
						.world
						.tick(&server.resources, &scripts, &console_handle)
					{
						Ok(true) => (),
						Ok(false) => break,
						Err(msg) => {
							error!("server world tick failed: {msg}");
							break 'server;
						}
					}
				}

				let mut world_packet = None;
				for client in clients.values_mut() {
					if client.requested_world {
						client.requested_world = false;
						let packet = if let Some(packet) = &mut world_packet {
							packet
						} else {
							match rkyv::to_bytes::<rancor::BoxedError>(&ServerPacket::World {
								world: &server.world,
							}) {
								Ok(packet) => world_packet.insert(packet),
								Err(msg) => {
									error!("failed to serialize world: {msg}");
									break 'server;
								}
							}
						};
						// This error is useless; `client.stream.recv.task` would fail first and provides more info.
						let _ = client.sender.forward(packet.clone()).await;
					}
				}

				if clients.clients.is_empty() {
					// TODO: Save to disk
					info!("no clients remain; closing instance");
					break;
				}
			}
		});
	Ok(())
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

	let packet = rkyv::access(&packet).map_err(Error::Access)?;
	match packet {
		protocol::ArchivedClientPacket::Ping => client.ping().await?,
		protocol::ArchivedClientPacket::Action { action } => {
			let action: character::Action = rkyv::deserialize(action).map_err(Error::De)?;
			let console = console_handle;
			let scripts: &resource::Scripts = scripts;
			let next_character = server.world.next_character();
			// TODO: Uuid-based piece ownership.
			// TODO: What happens when a piece isn't owned by anyone (eg: by disconnect)?
			if next_character.borrow().player_controlled {
				server
					.world
					.perform_action(console, &server.resources, scripts, action)?;
			} else {
				warn!("client attempted to move piece it did not own");
				client
					.sender
					.send(&protocol::ServerPacket::World {
						world: &server.world,
					})
					.await
					.map_err(Error::Ser)?;
			}
		}
		protocol::ArchivedClientPacket::Authenticate(auth) => {
			let client_authentication = rkyv::deserialize(auth).map_err(Error::De)?;
			info!(username = client_authentication.username, "authenticated");
			client.authentication = Some(client_authentication);
		}
		// Client is already routed, but a singular server instance without a router may be sent superfluous routing packets.
		// Ignore them and act as usual and clients should connect just fine.
		protocol::ArchivedClientPacket::Instantiate | protocol::ArchivedClientPacket::Route(_) => {}
	}
	Ok(())
}
