#![feature(maybe_uninit_uninit_array, core_io_borrowed_buf, read_buf)]

use esprit2::prelude::*;
use esprit2_server::*;
use rkyv::Deserialize;
use std::io::Write;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::process::exit;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub struct Console {
	sender: mpsc::Sender<console::Message>,
}

impl console::Handle for Console {
	fn send_message(&self, message: console::Message) {
		let _ = self.sender.send(message);
	}
}

struct Instance {
	console_reciever: mpsc::Receiver<console::Message>,
	console_handle: Console,
	server: Server,
}

impl Instance {
	fn new() -> Self {
		let (sender, console_reciever) = mpsc::channel();
		let console_handle = Console { sender };
		let server = Server::new("res/".into());
		Self {
			console_reciever,
			console_handle,
			server,
		}
	}
}

fn main() {
	tracing_subscriber::fmt::init();
	let listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), protocol::DEFAULT_PORT))
		.unwrap_or_else(|msg| {
			error!("failed to bind listener: {msg}");
			exit(1);
		});
	let mut connections = Vec::new();
	info!(
		"listening for connections on {}",
		listener.local_addr().unwrap()
	);
	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				connections.push(thread::spawn(move || {
					let _enter = tracing::error_span!(
						"client",
						addr = stream.peer_addr().unwrap().to_string()
					)
					.entered();
					info!("connected");
					connection(stream)
				}));

				connections.retain(|x| !x.is_finished());
				info!("{} live instances", connections.len());
			}
			// TODO: What errors may occur? How should they be handled?
			Err(msg) => error!("failed to read incoming stream: {msg}"),
		}
	}
}

fn connection(mut stream: TcpStream) {
	// For now, this spins up a new server for each connection
	// TODO: Route connections to the same instance.
	let mut instance = Instance::new();
	// Create a Lua runtime.
	let lua = mlua::Lua::new();

	lua.globals()
		.get::<&str, mlua::Table>("package")
		.unwrap()
		.set("path", "res/scripts/?.lua")
		.unwrap();
	lua.globals()
		.set(
			"Console",
			console::LuaHandle(instance.console_handle.clone()),
		)
		.unwrap();
	lua.globals()
		.set("Status", instance.server.resources.statuses_handle())
		.unwrap();
	lua.globals()
		.set("Heuristic", consider::HeuristicConstructor)
		.unwrap();
	lua.globals().set("Log", combat::LogConstructor).unwrap();

	let scripts = resource::Scripts::open("res/scripts/", &lua).unwrap();
	instance.server.send_ping();
	// TODO: how do we start communication?
	{
		// Give the client an unintial world state.
		let packet = rkyv::to_bytes::<_, 4096>(&protocol::ServerPacket::World {
			world: &instance.server.world,
		})
		.unwrap();
		let packet_len = u32::try_from(packet.len()).unwrap().to_le_bytes();
		stream.write_all(&packet_len).unwrap();
		stream.write_all(&packet).unwrap();
	}
	let mut packet_reciever = protocol::PacketReciever::default();
	let mut awaiting_input = false;
	loop {
		packet_reciever
			.recv(&mut stream, |packet| {
				let packet = rkyv::check_archived_root::<protocol::ClientPacket>(&packet).unwrap();
				match packet {
					protocol::ArchivedClientPacket::Ping(id) => {
						instance.server.recv_ping();
					}
					protocol::ArchivedClientPacket::Action(action_archive) => {
						let mut deserializer = rkyv::de::deserializers::SharedDeserializeMap::new();
						let action: character::Action =
							action_archive.deserialize(&mut deserializer).unwrap();
						instance
							.server
							.recv_action(&instance.console_handle, &scripts, action)
							.unwrap();
						awaiting_input = false;
					}
				}
			})
			.unwrap();
		// This check has to happen after recieving packets to be as charitable to the client as possible.
		if instance.server.players.ping.elapsed() > TIMEOUT {
			info!("{{player}} disconnected by timeout");
			return;
		}
		instance
			.server
			.tick(&scripts, &instance.console_handle)
			.unwrap();
		if instance
			.server
			.world
			.next_character()
			.borrow()
			.player_controlled
			&& !awaiting_input
		{
			awaiting_input = true;
			let packet = rkyv::to_bytes::<_, 4096>(&protocol::ServerPacket::World {
				world: &instance.server.world,
			})
			.unwrap();
			let packet_len = u32::try_from(packet.len()).unwrap().to_le_bytes();
			stream.write_all(&packet_len).unwrap();
			stream.write_all(&packet).unwrap();
		}

		for i in instance.console_reciever.try_iter() {
			let packet = rkyv::to_bytes::<_, 4096>(&protocol::ServerPacket::Message(i)).unwrap();
			let packet_len = u32::try_from(packet.len()).unwrap().to_le_bytes();
			stream.write_all(&packet_len).unwrap();
			stream.write_all(&packet).unwrap();
		}
	}
}
