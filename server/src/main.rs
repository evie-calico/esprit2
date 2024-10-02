#![feature(
	maybe_uninit_uninit_array,
	maybe_uninit_fill,
	core_io_borrowed_buf,
	read_buf
)]

use clap::Parser;
use esprit2::prelude::*;
use esprit2_server::*;
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

#[derive(clap::Parser)]
struct Cli {
	#[clap(short, long)]
	port: Option<u16>,
	#[clap(long, default_value = "256")]
	instances: usize,

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

	let _span =
		tracing::error_span!("router", addr = listener.local_addr().unwrap().to_string()).entered();

	let mut instances = Box::new_uninit_slice(cli.instances);
	let instances = MaybeUninit::fill_with(&mut instances, || None);

	info!("listening");
	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let (router, reciever) = mpsc::channel();
				if let Some((i, instance)) = instances
					.iter_mut()
					.enumerate()
					.find(|(_, x)| x.as_ref().is_none_or(JoinHandle::is_finished))
				{
					*instance = Some(
						thread::Builder::new()
							// TODO: Identify instances by their file name on disk
							.name(format!("instance {i}"))
							.spawn({
								let res = cli.resource_directory.clone();
								move || {
									esprit2_server::instance(reciever, res);
								}
							})
							.unwrap(),
					);
					info!(peer = stream.peer_addr().unwrap().to_string(), "connected");
					router.send(Client::new(stream)).unwrap();
				} else {
					todo!()
				}
			}
			// TODO: What errors may occur? How should they be handled?
			Err(msg) => error!("failed to read incoming stream: {msg}"),
		}
	}
}
