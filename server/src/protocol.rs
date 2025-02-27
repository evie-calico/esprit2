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
use rkyv::rancor::ResultExt;
use rkyv::{rancor, util::AlignedVec};
use std::num::IntErrorKind;
use std::{io, num::ParseIntError, str::Utf8Error};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc;
use tokio::task;
use url::Url;

pub type ClientIdentifier = u64;

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
	pub instance_id: u32,
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
	pub fn new(url: &str) -> Result<(Option<Self>, impl ToSocketAddrs), ClientRoutingError> {
		use ClientRoutingError as E;
		let url = Url::parse(url)?;
		let s = url
			.path_segments()
			.and_then(|mut segments| {
				let instance_id = match segments.next()?.parse::<u32>() {
					Ok(i) => i,
					Err(e) => {
						return match e.kind() {
							IntErrorKind::Empty => None,
							_ => Some(Err(E::MalformedInstance(e))),
						}
					}
				};
				let instance_password = match segments
					.next()
					.map(|x| percent_decode_str(x).decode_utf8())
					.transpose()
				{
					Ok(i) => i,
					Err(e) => return Some(Err(E::MalformedPassword(e))),
				}
				.map(|x| x.into());
				Some(Ok(Self {
					instance_id,
					instance_password,
				}))
			})
			.transpose()?;
		Ok((
			s,
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
	Instantiate,
	// Instance packets
	Action { action: character::Action },
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ServerPacket<'a> {
	Ping,
	Register(ClientIdentifier),
	World {
		#[rkyv(with = rkyv::with::Inline)]
		world: &'a world::Manager,
	},
	Message(#[rkyv(with = rkyv::with::Inline)] &'a console::Message),
}

#[derive(Debug)]
pub struct PacketReceiver {
	pub task: task::JoinHandle<io::Result<()>>,
}

impl PacketReceiver {
	pub fn new(read: OwnedReadHalf) -> (Self, mpsc::Receiver<AlignedVec>) {
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
		(Self { task }, channel)
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

	pub async fn send<P>(&self, packet: &P) -> Result<(), rancor::BoxedError>
	where
		P: for<'a> rkyv::Serialize<
			rkyv::api::high::HighSerializer<
				AlignedVec,
				rkyv::ser::allocator::ArenaHandle<'a>,
				rancor::BoxedError,
			>,
		>,
	{
		match rkyv::to_bytes(packet) {
			Ok(packet) => self.forward(packet).await,
			Err(e) => Err(e),
		}
	}

	pub async fn forward(&self, packet: AlignedVec) -> Result<(), rancor::BoxedError> {
		self.channel.send(packet).await.into_error()
	}
}
