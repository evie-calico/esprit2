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
use rkyv::util::AlignedVec;
use std::io;

/// Default port for esprit servers to listen on.
///
/// This is derived from "esprit", where each letter's index (distance from 'a') was adjusted to a single digit via modulus.
///
/// `(character - 'a') % 10`
pub const DEFAULT_PORT: u16 = 48578;

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ClientPacket {
	Ping,
	Authenticate(String),

	Action(character::Action),
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ServerPacket<'a> {
	Ping(#[rkyv(with = rkyv::with::InlineAsBox)] &'a str),
	World {
		#[rkyv(with = rkyv::with::Inline)]
		world: &'a world::Manager,
	},
	Message(console::Message),
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

#[derive(Clone, Debug)]
pub struct PacketSender {
	len_progress: usize,
	packet_progress: usize,
	packet: AlignedVec,
}

impl PacketSender {
	pub fn new(packet: &ServerPacket) -> Self {
		let packet = rkyv::to_bytes::<rkyv::rancor::Error>(packet).unwrap();
		PacketSender {
			len_progress: 0,
			packet_progress: 0,
			packet,
		}
	}
	pub fn send(&mut self, mut stream: impl io::Write) -> io::Result<()> {
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
