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
use std::{collections::VecDeque, io, net::ToSocketAddrs, num::ParseIntError, str::Utf8Error};
use url::Url;

/// Default port for esprit servers to listen on.
///
/// This is derived from "esprit", where each letter's index (distance from 'a') was adjusted to a single digit via modulus.
///
/// `(character - 'a') % 10`
pub const DEFAULT_PORT: u16 = 48578;

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
	Action(character::Action),
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

#[derive(Clone, Default, Debug)]
pub struct PacketReceiver {
	len: [Option<u8>; 4],
	packet_buffer: rkyv::util::AlignedVec,
}

impl PacketReceiver {
	pub fn packet_len(&self) -> Option<usize> {
		Some(u32::from_le_bytes([self.len[0]?, self.len[1]?, self.len[2]?, self.len[3]?]) as usize)
	}

	pub fn recv(&mut self, stream: impl io::Read, f: impl FnOnce(AlignedVec)) -> io::Result<()> {
		let mut bytes = stream.bytes();
		for (len, byte) in self.len.iter_mut().filter(|x| x.is_none()).zip(&mut bytes) {
			*len = Some(byte?);
		}
		if let Some(packet_len) = self.packet_len() {
			for i in bytes.take(packet_len - self.packet_buffer.len()) {
				self.packet_buffer.push(i?)
			}
			if packet_len == self.packet_buffer.len() {
				let mut packet = rkyv::util::AlignedVec::new();
				std::mem::swap(&mut packet, &mut self.packet_buffer);
				f(packet);
				*self = Self::default();
			}
		}
		Ok(())
	}
}

#[derive(Debug, Default)]
pub struct PacketSender(VecDeque<InProgress>);

impl PacketSender {
	pub fn queue<P>(&mut self, packet: &P)
	where
		P: for<'a> rkyv::Serialize<
			rkyv::api::high::HighSerializer<
				AlignedVec,
				rkyv::ser::allocator::ArenaHandle<'a>,
				rkyv::rancor::Error,
			>,
		>,
	{
		self.0.push_back(InProgress::new(packet));
	}

	pub fn send(&mut self, mut stream: impl io::Write) -> io::Result<()> {
		while let Some(sender) = self.0.front_mut() {
			sender.send(&mut stream)?;
			self.0.pop_front();
		}
		Ok(())
	}
}

#[derive(Debug)]
struct InProgress {
	len_progress: usize,
	packet_progress: usize,
	packet: AlignedVec,
}

impl InProgress {
	fn new<P>(packet: &P) -> Self
	where
		P: for<'a> rkyv::Serialize<
			rkyv::api::high::HighSerializer<
				AlignedVec,
				rkyv::ser::allocator::ArenaHandle<'a>,
				rkyv::rancor::Error,
			>,
		>,
	{
		let packet = rkyv::to_bytes::<rkyv::rancor::Error>(packet).unwrap();
		Self {
			len_progress: 0,
			packet_progress: 0,
			packet,
		}
	}

	fn send(&mut self, mut stream: impl io::Write) -> io::Result<()> {
		let len_bytes = (self.packet.len() as u32).to_le_bytes();
		while self.len_progress < len_bytes.len() {
			self.len_progress += stream.write(&len_bytes[self.len_progress..])?;
		}
		while self.packet_progress < self.packet.len() {
			self.packet_progress += stream.write(&self.packet[self.packet_progress..])?;
		}
		Ok(())
	}
}
