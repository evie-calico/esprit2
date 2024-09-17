#![feature(maybe_uninit_uninit_array, core_io_borrowed_buf, read_buf)]

use clap::Parser;
use esprit2::prelude::*;
use esprit2_server::*;
use std::io::Write;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
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
	fn new(res: PathBuf) -> Self {
		let (sender, console_reciever) = mpsc::channel();
		let console_handle = Console { sender };
		let server = Server::new(res);
		Self {
			console_reciever,
			console_handle,
			server,
		}
	}
}

#[derive(clap::Parser)]
struct Cli {
	#[clap(short, long)]
	port: Option<u16>,

	resource_directory: PathBuf,
}

fn main() {
	let cli = Cli::parse();
	// Logging initialization.
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::TRACE)
		.with_thread_names(true)
		// Your service manager's logs should already have time.
		.without_time()
		.init();
	let listener = TcpListener::bind((
		Ipv4Addr::new(127, 0, 0, 1),
		cli.port.unwrap_or(protocol::DEFAULT_PORT),
	))
	.unwrap_or_else(|msg| {
		error!("failed to bind listener: {msg}");
		exit(1);
	});
	let mut connections = Vec::new();
	info!(
		"listening for connections on {}",
		// TODO: It might be worth formatting this to a string and putting it in a tracing span.
		listener.local_addr().unwrap()
	);
	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				connections.push(thread::spawn({
					let res = cli.resource_directory.clone();
					move || {
						let _enter = tracing::error_span!(
							"client",
							addr = stream.peer_addr().unwrap().to_string()
						)
						.entered();
						info!("connected");
						connection(stream, res);
					}
				}));

				connections.retain(|x| !x.is_finished());
				info!(
					live_instances = connections.len(),
					"established new connection"
				);
			}
			// TODO: What errors may occur? How should they be handled?
			Err(msg) => error!("failed to read incoming stream: {msg}"),
		}
	}
}

fn connection(mut stream: TcpStream, res: PathBuf) {
	// Create a Lua runtime.
	let lua = mlua::Lua::new();

	lua.globals()
		.get::<&str, mlua::Table>("package")
		.unwrap()
		.set("path", res.join("scripts/?.lua").to_string_lossy())
		.unwrap();

	let scripts = resource::Scripts::open(res.join("scripts"), &lua).unwrap();

	// For now, this spins up a new server for each connection
	// TODO: Route connections to the same instance.
	let mut instance = Instance::new(res);

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

	instance.server.send_ping();
	// TODO: how do we start communication?
	{
		// Give the client an unintial world state.
		let packet = rkyv::to_bytes::<rkyv::rancor::Error>(&protocol::ServerPacket::World {
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
				let packet = rkyv::access::<_, rkyv::rancor::Error>(&packet).unwrap();
				match packet {
					protocol::ArchivedClientPacket::Ping(id) => {
						instance.server.recv_ping();
					}
					protocol::ArchivedClientPacket::Action(action_archive) => {
						let action: character::Action =
							rkyv::deserialize::<_, rkyv::rancor::Error>(action_archive).unwrap();
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
			info!(player = "player", "disconnected by timeout");
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
			let packet = rkyv::to_bytes::<rkyv::rancor::Error>(&protocol::ServerPacket::World {
				world: &instance.server.world,
			})
			.unwrap();
			let packet_len = u32::try_from(packet.len()).unwrap().to_le_bytes();
			stream.write_all(&packet_len).unwrap();
			stream.write_all(&packet).unwrap();
		}

		for i in instance.console_reciever.try_iter() {
			let packet =
				rkyv::to_bytes::<rkyv::rancor::Error>(&protocol::ServerPacket::Message(i)).unwrap();
			let packet_len = u32::try_from(packet.len()).unwrap().to_le_bytes();
			stream.write_all(&packet_len).unwrap();
			stream.write_all(&packet).unwrap();
		}
	}
}
