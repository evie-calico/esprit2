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
				let (router, reciever) = mpsc::channel();
				connections.push(thread::spawn({
					let res = cli.resource_directory.clone();
					move || {
						connection(reciever, res);
					}
				}));
				info!(
					addr = stream.peer_addr().unwrap().to_string(),
					"client connected"
				);
				router.send(stream).unwrap();

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
