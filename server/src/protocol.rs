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
use std::io;

/// Default port for esprit servers to listen on.
///
/// This is derived from "esprit", where each letter's index (distance from 'a') was adjusted to a single digit via modulus.
///
/// `(character - 'a') % 10`
pub const DEFAULT_PORT: u16 = 48578;

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum ClientPacket {
	Ping(String),
	Action(character::Action),
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum ServerPacket<'a> {
	Ping(#[with(rkyv::with::RefAsBox)] &'a str),
	World {
		#[with(rkyv::with::Inline)]
		world: &'a world::Manager,
	},
	Message(console::Message),
}

#[derive(Clone, Default, Debug)]
pub struct PacketReciever {
	len: [Option<u8>; 4],
	packet_buffer: rkyv::AlignedVec,
}

impl PacketReciever {
	pub fn packet_len(&self) -> Option<usize> {
		Some(u32::from_le_bytes([self.len[0]?, self.len[1]?, self.len[2]?, self.len[3]?]) as usize)
	}

	pub fn recv(
		&mut self,
		stream: impl io::Read,
		f: impl FnOnce(rkyv::AlignedVec),
	) -> io::Result<()> {
		let mut bytes = stream.bytes();
		for (len, byte) in self.len.iter_mut().filter(|x| x.is_none()).zip(&mut bytes) {
			*len = Some(byte?);
		}
		if let Some(packet_len) = self.packet_len() {
			for i in bytes.take(packet_len - self.packet_buffer.len()) {
				self.packet_buffer.push(i?)
			}
			if packet_len == self.packet_buffer.len() {
				let mut packet = rkyv::AlignedVec::new();
				std::mem::swap(&mut packet, &mut self.packet_buffer);
				f(packet);
				*self = Self::default();
			}
		}
		Ok(())
	}
}
