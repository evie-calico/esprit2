use crate::prelude::*;
use esprit2::prelude::*;
use esprit2_server::{protocol, Server};
use std::io;
use std::io::{prelude::*, BorrowedBuf};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;

/// Encapsulate resources can be (but aren't necessarily!) shared by both the client and the server.
///
/// This special handling is necessary because the server may be an internal process,
/// or an external program connected over TCP.
///
/// When connected to an external server, it's necessary for us to have a local copy of most information,
/// such as the lua runtime, console, resource directory, and even the world manager.
/// The server is expected to communicate with the client so that everything is kept in sync,
/// but in the cast of desyncs the world manager should be easy to replace.
///
/// Because both the server and client have a copy of the game's resources,
/// and these copies may not be exactly the same,
/// client and server state may become desyncronized; this is expected.
/// The client should simulate changes to its cache of the world manager and send a checksum
/// along with whatever action it performed. If this checksum differs from the server's copy,
/// it will send an complete copy of the world state back to the client, which it *must* accept.
pub enum ServerHandle {
	Internal {
		server: Server,
	},
	External {
		stream: TcpStream,
		packet_reciever: protocol::PacketReciever,
		world_cache: world::Manager,
		resources: resource::Manager,
	},
}

impl ServerHandle {
	/// Create an internal server.
	pub fn internal(resource_directory: PathBuf) -> Self {
		let mut server = Server::new(resource_directory);
		server.send_ping();
		Self::Internal { server }
	}

	/// Connect to an external server.
	pub fn external(address: impl ToSocketAddrs, resource_directory: PathBuf) -> Self {
		let mut stream = TcpStream::connect(address).unwrap();
		let mut packet_len = [0; 4];
		// Read a ping back, reusing the buffers we used to send it.
		stream.read_exact(&mut packet_len).unwrap();
		let mut packet = Box::new_uninit_slice(u32::from_le_bytes(packet_len) as usize);
		let mut packet_buf = BorrowedBuf::from(&mut *packet);
		stream
			.set_nonblocking(true)
			.expect("failed to set nonblocking");
		stream.read_buf_exact(packet_buf.unfilled()).unwrap();
		// SAFETY: read_buf_exact always fills the entire buffer.
		let packet = unsafe { packet.assume_init() };
		// Parse the ping and print its message
		let packet =
			rkyv::access::<protocol::ArchivedServerPacket, rkyv::rancor::Failure>(&packet).unwrap();
		let world_cache = match packet {
			protocol::ArchivedServerPacket::World { world } => {
				rkyv::deserialize::<world::Manager, rkyv::rancor::Error>(world).unwrap()
			}
			_ => {
				todo!();
			}
		};
		Self::External {
			stream,
			packet_reciever: protocol::PacketReciever::default(),
			world_cache,
			resources: resource::Manager::open(resource_directory).unwrap(),
		}
	}

	/// Access the currently cached world manager.
	///
	/// This is NOT necessarily the same as the server's copy!
	/// The client must be careful to communicate its expected state with the server,
	/// and replace its state in the event of a desync.
	pub fn world(&self) -> &world::Manager {
		match self {
			Self::Internal { server } => &server.world,
			Self::External { world_cache, .. } => world_cache,
		}
	}

	/// Access resources.
	///
	/// Both servers and clients keep track of game resources, and they may not be the same.
	/// However, the client can share its resources with the server when it is running internally.
	pub fn resources(&self) -> &resource::Manager {
		match self {
			Self::Internal { server } => &server.resources,
			Self::External { resources, .. } => resources,
		}
	}

	pub fn tick(
		&mut self,
		scripts: &resource::Scripts,
		console: &mut Console,
	) -> esprit2::Result<()> {
		match self {
			Self::Internal { server } => {
				server.recv_ping();
				server.tick(scripts, &console.handle)?;
			}
			Self::External {
				stream,
				packet_reciever,
				world_cache,
				..
			} => {
				let packet = rkyv::to_bytes::<rkyv::rancor::Error>(&protocol::ClientPacket::Ping(
					"meow".into(),
				))
				.unwrap();
				stream
					.write_all(&(packet.len() as u32).to_le_bytes())
					.unwrap();
				stream.write_all(&packet).unwrap();
				match packet_reciever.recv(stream, |packet| {
					let packet = rkyv::access::<_, rkyv::rancor::Error>(&packet).unwrap();
					match packet {
						protocol::ArchivedServerPacket::Ping(_) => todo!(),
						protocol::ArchivedServerPacket::World { world } => {
							*world_cache =
								rkyv::deserialize::<world::Manager, rkyv::rancor::Error>(world)
									.unwrap()
						}
						protocol::ArchivedServerPacket::Message(message) => {
							console.history.push(
								rkyv::deserialize::<_, rkyv::rancor::Error>(message).unwrap(),
							);
						}
					}
				}) {
					Ok(()) => {}
					Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
					Err(e) => Err(e)?,
				}
			}
		}
		Ok(())
	}
}

/// Send operations.
impl ServerHandle {
	pub fn send_action(
		&mut self,
		console: impl console::Handle,
		scripts: &resource::Scripts,
		action: character::Action,
	) -> esprit2::Result<()> {
		match self {
			Self::Internal { server } => server.recv_action(console, scripts, action),
			Self::External {
				stream,
				world_cache,
				resources,
				..
			} => {
				let packet = rkyv::to_bytes::<rkyv::rancor::Error>(
					&protocol::ClientPacket::Action(action.clone()),
				)
				.unwrap();
				stream
					.write_all(&(packet.len() as u32).to_le_bytes())
					.unwrap();
				stream.write_all(&packet).unwrap();

				world_cache.perform_action(console_impl::Dummy, resources, scripts, action)
			}
		}
	}
}
