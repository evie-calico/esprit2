//! Defines a protocol for client/server communication.
//!
//! These types are not used by the internal server (communication has a more programatic API
//! in order to allow clients to embed a server), but it is used by the server binary.
//!
//! Clients communicating with a server binary are expected to use these types.
//! The protocol uses TCP over port 48578 and is dead-simple:
//! Each packet is prefixed by its size in 4 little-endian bytes.
//! Packets should be serializable and deserializable using `rkyv`,
//! assuming both parties are using the same version.
//!
//! For more information about `rkyv`'s data format: [https://rkyv.org/](https://rkyv.org/)

use esprit2::prelude::*;
use percent_encoding::percent_decode_str;
use rkyv::util::AlignedVec;
use std::{io, num::ParseIntError, str::Utf8Error};
use tokio::{
	net::{
		tcp::{OwnedReadHalf, OwnedWriteHalf},
		TcpStream, ToSocketAddrs,
	},
	sync::mpsc,
	task,
};
use url::Url;

/// Default port for esprit servers to listen on.
///
/// This is derived from "esprit", where each letter's index (distance from 'a') was adjusted to a single digit via modulus.
///
/// `(character - 'a') % 10`
pub const DEFAULT_PORT: u16 = 48578;

pub type Checksum = u64;

pub fn checksum(bytes: impl Iterator<Item = u8>) -> Checksum {
	const CHECKSUM_BYTES: usize = Checksum::BITS as usize / 8;
	bytes
		.map(Into::into)
		.array_chunks::<CHECKSUM_BYTES>()
		.map(Checksum::from_le_bytes)
		.reduce(|a, b| a ^ b)
		.unwrap_or(0)
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ClientAuthentication {
	pub username: String,
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ClientRouting {
	pub instance_id: usize,
	pub instance_password: Option<Box<str>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientRoutingError {
	#[error("malformed url: {0}")]
	MalformedUrl(#[from] url::ParseError),
	#[error("missing host")]
	MissingHost,
	#[error("missing instance")]
	MissingInstance,
	#[error("malformed instance: {0}")]
	MalformedInstance(#[from] ParseIntError),
	#[error("malformed password: {0}")]
	MalformedPassword(#[from] Utf8Error),
}

impl ClientRouting {
	pub fn new(url: &str) -> Result<(Self, impl ToSocketAddrs), ClientRoutingError> {
		use ClientRoutingError as E;
		let url = Url::parse(url)?;
		let mut segments = url.path_segments().ok_or(E::MissingInstance)?;
		let instance_id = segments.next().ok_or(E::MissingInstance)?.parse()?;
		let instance_password = segments
			.next()
			.map(|x| percent_decode_str(x).decode_utf8())
			.transpose()?
			.map(|x| x.into());
		Ok((
			Self {
				instance_id,
				instance_password,
			},
			(
				String::from(
					percent_decode_str(url.host_str().ok_or(E::MissingHost)?).decode_utf8()?,
				),
				url.port().unwrap_or(DEFAULT_PORT),
			),
		))
	}
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ClientPacket {
	// Generic packets
	Ping,
	// Root packets
	Authenticate(ClientAuthentication),
	Route(ClientRouting),
	// Instance packets
	Action { action: character::Action },
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ServerPacket<'a> {
	Ping,
	World {
		#[rkyv(with = rkyv::with::Inline)]
		world: &'a world::Manager,
	},
	Message(#[rkyv(with = rkyv::with::Inline)] &'a console::Message),
}

#[derive(Debug)]
pub struct PacketStream {
	pub recv: PacketReceiver,
	pub send: PacketSender,
}

impl PacketStream {
	pub fn new(stream: TcpStream) -> Self {
		let (read, write) = stream.into_split();
		Self {
			recv: PacketReceiver::new(read),
			send: PacketSender::new(write),
		}
	}

	pub async fn send<P>(&self, packet: &P)
	where
		P: for<'a> rkyv::Serialize<
			rkyv::api::high::HighSerializer<
				AlignedVec,
				rkyv::ser::allocator::ArenaHandle<'a>,
				rkyv::rancor::Error,
			>,
		>,
	{
		self.forward(rkyv::to_bytes::<rkyv::rancor::Error>(packet).unwrap())
			.await;
	}

	pub async fn forward(&self, packet: AlignedVec) {
		let _ = self.send.channel.send(packet).await;
	}
}

#[derive(Debug)]
pub struct PacketReceiver {
	pub channel: Option<mpsc::Receiver<AlignedVec>>,
	pub task: task::JoinHandle<io::Result<()>>,
}

impl PacketReceiver {
	pub fn new(read: OwnedReadHalf) -> Self {
		let (send, channel) = mpsc::channel::<AlignedVec>(8);
		let task = task::spawn(async move {
			loop {
				read.readable().await?;
				let mut progress = 0;
				let mut size = [0; 4];
				while progress < size.len() {
					match read.try_read(&mut size) {
						Ok(0) => return Ok(()),
						Ok(n) => progress += n,
						Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
						Err(e) => Err(e)?,
					}
				}
				let size = u32::from_le_bytes(size) as usize;
				let mut progress = 0;
				let mut packet = AlignedVec::with_capacity(size);
				packet.resize(size, 0);
				while progress < packet.len() {
					match read.try_read(packet.as_mut_slice()) {
						Ok(0) => return Ok(()),
						Ok(n) => progress += n,
						Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
						Err(e) => Err(e)?,
					}
				}
				if send.send(packet).await.is_err() {
					break;
				}
			}
			Ok(())
		});
		Self {
			channel: Some(channel),
			task,
		}
	}
}

#[derive(Debug)]
pub struct PacketSender {
	pub channel: mpsc::Sender<AlignedVec>,
	pub task: task::JoinHandle<io::Result<()>>,
}

impl PacketSender {
	pub fn new(write: OwnedWriteHalf) -> Self {
		let (channel, mut recv) = mpsc::channel::<AlignedVec>(8);
		let task = task::spawn(async move {
			while let Some(packet) = recv.recv().await {
				let len_bytes = (packet.len() as u32).to_le_bytes();
				for buffer in [&len_bytes, packet.as_slice()] {
					let mut progress = 0;
					while progress < buffer.len() {
						write.writable().await?;
						progress += write.try_write(&buffer[progress..])?;
					}
				}
			}
			Ok(())
		});
		Self { channel, task }
	}

	pub async fn send<P>(&self, packet: &P)
	where
		P: for<'a> rkyv::Serialize<
			rkyv::api::high::HighSerializer<
				AlignedVec,
				rkyv::ser::allocator::ArenaHandle<'a>,
				rkyv::rancor::Error,
			>,
		>,
	{
		let _ = self
			.channel
			.send(rkyv::to_bytes::<rkyv::rancor::Error>(packet).unwrap())
			.await;
	}
}
