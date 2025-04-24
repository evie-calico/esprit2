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

use esprit2::anyhow::Context;
use esprit2::prelude::*;
use protocol::{
	ArchivedClientAuthentication, ClientAuthentication, ClientIdentifier, PacketReceiver,
	PacketSender, ServerPacket,
};
use rkyv::rancor;
use rkyv::util::AlignedVec;
use std::collections::HashMap;
use std::path::Path;
use std::process::exit;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{StreamExt, StreamMap};

pub mod protocol;

pub use esprit2::anyhow;

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

	pub async fn ping(&mut self) -> anyhow::Result<()> {
		self.sender
			.send(&protocol::ServerPacket::Ping)
			.await
			.context("failed to send packet")?;
		self.ping = Instant::now();
		Ok(())
	}

	pub async fn authenticate(
		&mut self,
		auth: &ArchivedClientAuthentication,
	) -> anyhow::Result<()> {
		let auth =
			rkyv::deserialize::<_, rancor::Error>(auth).context("failed to recieve packet")?;
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
	pub(crate) fn new(
		resource_directory: impl AsRef<Path>,
		lua: &mlua::Lua,
	) -> anyhow::Result<Self> {
		let modules = resource_directory
			.as_ref()
			.read_dir()
			.context("failed to read contents of resource directory")?
			.filter_map(|x| {
				let x = x.ok()?;
				if x.metadata().ok()?.is_dir() {
					Some(x.path().into_boxed_path())
				} else {
					None
				}
			})
			.collect::<Box<[Box<Path>]>>();
		let (resources, errors) =
			resource::open(lua, modules.iter().map(|x| x.as_ref()), |_, _, init| init());
		let resources = resource::Handle::new(resources.into());
		for (module, error) in errors
			.into_iter()
			.flat_map(|x| <Box<[_]> as IntoIterator>::into_iter(x.errors).map(move |e| (x.name, e)))
		{
			error!(module, "{error:?}");
		}

		// Create a piece for the player, and register it with the world manager.
		let party_blueprint = [
			world::PartyReferenceBase {
				sheet: "esprit:luvui".into(),
				accent_color: (0xDA, 0x2D, 0x5C, 0xFF),
			},
			world::PartyReferenceBase {
				sheet: "esprit:aris".into(),
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
				vaults: vec!["esprit:example".into()],
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
	res: impl AsRef<Path>,
) -> anyhow::Result<()> {
	let lua = esprit2::lua::init()?;

	let (sender, mut console_reciever) = mpsc::unbounded_channel();
	let console = Console { sender };
	let mut server = Server::new(res, &lua)?;
	let mut clients = ClientParty::default();

	let resources = server.resources.clone();
	lua.load_from_function::<mlua::Value>(
		"runtime.resources",
		lua.create_function(move |_, ()| Ok(resources.clone()))?,
	)?;
	let console_handle = console.clone();
	lua.load_from_function::<mlua::Value>(
		"runtime.console",
		lua.create_function(move |_, ()| Ok(console::LuaHandle(console_handle.clone())))?,
	)?;

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
							&console,
							&lua,
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
					match server.world.tick(&server.resources, &lua, &console) {
						// TODO: infinite loop when the player dies please fix. (how)
						Ok(true) => (),
						Ok(false) => break,
						Err(msg) => {
							error!("server world tick failed: {msg:?}");
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
	lua: &mlua::Lua,
	server: &mut Server,
) -> anyhow::Result<()> {
	let span = tracing::error_span!(
		"client",
		addr = client.address,
		username = tracing::field::Empty,
	);
	if let Some(auth) = &client.authentication {
		span.record("username", &auth.username);
	}
	let _span = span.entered();

	let packet = rkyv::access::<_, rancor::Error>(&packet).context("failed to read packet")?;
	match packet {
		protocol::ArchivedClientPacket::Ping => client.ping().await?,
		protocol::ArchivedClientPacket::Action { action } => {
			let action: character::Action = rkyv::deserialize::<_, rancor::Error>(action)
				.context("failed to deserialize action packet")?;
			let console = console_handle;
			let next_character = server.world.next_character();
			// TODO: Uuid-based piece ownership.
			// TODO: What happens when a piece isn't owned by anyone (eg: by disconnect)?
			if next_character
				.borrow()
				.components
				.contains_key(":conscious")
			{
				server
					.world
					.perform_action(console, &server.resources, lua, action)?;
			} else {
				warn!("client attempted to move piece it did not own");
			}
		}
		protocol::ArchivedClientPacket::Authenticate(auth) => {
			let client_authentication = rkyv::deserialize::<_, rancor::Error>(auth)
				.context("failed to deserialize client authentication packet")?;
			info!(username = client_authentication.username, "authenticated");
			client.authentication = Some(client_authentication);
		}
		// Client is already routed, but a singular server instance without a router may be sent superfluous routing packets.
		// Ignore them and act as usual and clients should connect just fine.
		protocol::ArchivedClientPacket::Instantiate | protocol::ArchivedClientPacket::Route(_) => {}
	}
	Ok(())
}
