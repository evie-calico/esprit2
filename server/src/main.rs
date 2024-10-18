#![feature(
	maybe_uninit_uninit_array,
	maybe_uninit_fill,
	core_io_borrowed_buf,
	read_buf
)]

use clap::Parser;
use esprit2::prelude::*;
use esprit2_server::Error;
use esprit2_server::*;
use rkyv::util::AlignedVec;
use std::mem::MaybeUninit;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::exit;
use std::thread;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(clap::Parser)]
struct Cli {
	#[clap(short, long)]
	port: Option<u16>,
	#[clap(long, default_value = "256")]
	instances: u32,

	resource_directory: PathBuf,
}

struct Instance {
	handle: thread::JoinHandle<esprit2::Result<()>>,
	router: mpsc::Sender<(Client, ReceiverStream<AlignedVec>)>,
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	// Logging initialization.
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::TRACE)
		// Your service manager's logs should already have time.
		.without_time()
		.init();

	let listener = TcpListener::bind((
		Ipv4Addr::new(127, 0, 0, 1),
		cli.port.unwrap_or(protocol::DEFAULT_PORT),
	))
	.await
	.unwrap_or_else(|msg| {
		error!("failed to bind listener: {msg}");
		exit(1);
	});

	let _span = tracing::error_span!(
		"router",
		addr = listener
			.local_addr()
			.expect("missing local address")
			.to_string()
	)
	.entered();

	let mut instances = Box::new_uninit_slice(cli.instances as usize);
	let instances: &mut [Option<Instance>] = MaybeUninit::fill_with(&mut instances, || None);
	let mut clients = ClientParty::default();

	info!("listening");
	loop {
		select! {
			stream = listener.accept() => {
				match stream {
					Ok((stream, address)) => {
						info!(peer = address.to_string(), "connected");
						let (client, receiver) = Client::new(stream);
						clients.join(client, ReceiverStream::new(receiver));
					}
					// TODO: What errors may occur? How should they be handled?
					Err(msg) => error!("failed to read incoming stream: {msg}"),
				}
			}
			Some((id, client, packet)) = clients.next() => {
				let span = tracing::error_span!(
					"client",
					addr = client.address,
					username = tracing::field::Empty,
				);
				if let Some(auth) = &client.authentication {
					span.record("username", &auth.username);
				}
				let _span = span.entered();

				let packet = rkyv::access(&packet).map_err(Error::Access).unwrap();
				match packet {
					protocol::ArchivedClientPacket::Ping => client.ping().await.unwrap(),
					protocol::ArchivedClientPacket::Authenticate(auth) => client.authenticate(auth).await.unwrap(),
					protocol::ArchivedClientPacket::Instantiate => {
						if let Some((i, instance)) = instances.iter_mut().enumerate().find(|(_, x)| x.as_ref().is_none_or(|x| x.handle.is_finished())) {
							let (router, reciever) = mpsc::channel(4);
							router.send(clients.take(id)).await.unwrap();
							*instance = Some(Instance {
								handle: thread::Builder::new()
									.name(format!("instance {i}"))
									.spawn({
										let res = cli.resource_directory.clone();
										move || esprit2_server::instance(reciever, res)
									})
									.expect("failed to spawn instance thread"),
								router,
							});
						}
					}
					protocol::ArchivedClientPacket::Route(routing) => {
						if let Some(Some(instance)) = instances.get(routing.instance_id.to_native() as usize) {
							instance.router.send(clients.take(id)).await.unwrap();
						} else {
							todo!()
						}
					},
					protocol::ArchivedClientPacket::Action { .. } => todo!(),
				}
			}
		}
	}
}
