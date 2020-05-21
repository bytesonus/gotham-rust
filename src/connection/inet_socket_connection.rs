use crate::{
	connection::{BaseConnection, Buffer},
	juno_module_impl::JunoModuleImpl,
	utils::Error,
};
use std::sync::Arc;

use async_std::{io::BufReader, net::TcpStream, prelude::*, task};
use async_trait::async_trait;
use future::Either;
use futures::{
	channel::{
		mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
		oneshot::{channel, Sender},
	},
	future, SinkExt,
};

pub struct InetSocketConnection {
	connection_setup: bool,
	socket_path: String,
	on_data_handler: Option<Arc<JunoModuleImpl>>,
	write_data_sender: Option<UnboundedSender<Buffer>>,
	close_sender: Option<UnboundedSender<()>>,
}

impl InetSocketConnection {
	pub fn new(socket_path: String) -> Self {
		InetSocketConnection {
			connection_setup: false,
			socket_path,
			on_data_handler: None,
			write_data_sender: None,
			close_sender: None,
		}
	}
}

#[async_trait]
impl BaseConnection for InetSocketConnection {
	async fn setup_connection(&mut self) -> Result<(), Error> {
		if self.connection_setup {
			panic!("Cannot call setup_connection() more than once!");
		}
		let (write_data_sender, write_data_receiver) = unbounded::<Vec<u8>>();
		let (close_sender, close_receiver) = unbounded::<()>();
		let (init_sender, init_receiver) = channel::<Result<(), Error>>();

		self.write_data_sender = Some(write_data_sender);
		self.close_sender = Some(close_sender);
		let socket_path = self.socket_path.clone();
		let juno_module_impl = self.on_data_handler.as_ref().unwrap().clone();

		task::spawn(async {
			read_data_from_socket(
				socket_path,
				init_sender,
				juno_module_impl,
				write_data_receiver,
				close_receiver,
			)
			.await;
		});

		self.connection_setup = true;
		init_receiver.await.unwrap()
	}

	async fn close_connection(&mut self) -> Result<(), Error> {
		if !self.connection_setup || self.close_sender.is_none() {
			panic!("Cannot close a connection that hasn't been established yet. Did you forget to call setup_connection()?");
		}
		self.close_sender.as_ref().unwrap().send(()).await.unwrap();
		Ok(())
	}

	async fn send(&mut self, buffer: Buffer) -> Result<(), Error> {
		if !self.connection_setup || self.write_data_sender.is_none() {
			panic!("Cannot send data to a connection that hasn't been established yet. Did you forget to await the call to setup_connection()?");
		}
		self.write_data_sender
			.as_ref()
			.unwrap()
			.send(buffer)
			.await
			.unwrap();
		Ok(())
	}

	fn set_data_listener(&mut self, listener: Arc<JunoModuleImpl>) {
		self.on_data_handler = Some(listener);
	}

	fn get_data_listener(&self) -> &Option<Arc<JunoModuleImpl>> {
		&self.on_data_handler
	}
}

async fn read_data_from_socket(
	socket_path: String,
	init_sender: Sender<Result<(), Error>>,
	juno_impl: Arc<JunoModuleImpl>,
	mut write_receiver: UnboundedReceiver<Vec<u8>>,
	mut close_receiver: UnboundedReceiver<()>,
) {
	let result = TcpStream::connect(socket_path).await;
	if let Err(err) = result {
		init_sender
			.send(Err(Error::Internal(format!("{}", err))))
			.unwrap_or(());
		return;
	}
	let client = result.unwrap();
	init_sender.send(Ok(())).unwrap_or(());
	let reader = BufReader::new(&client);
	let mut lines = reader.lines();
	let mut read_future = lines.next();
	let mut write_future = write_receiver.next();
	let mut close_future = close_receiver.next();
	let mut read_or_write_future = future::select(read_future, write_future);
	while let Either::Left((read_write_future, next_close_future)) =
		future::select(read_or_write_future, close_future).await
	{
		// Either a read or a write event has happened
		close_future = next_close_future;
		match read_write_future {
			Either::Left((read_future_result, next_write_future)) => {
				// Read event has happened
				read_future = lines.next();
				write_future = next_write_future;
				read_or_write_future = future::select(read_future, write_future);
				// Send the read data to the MPSC sender
				if let Some(Ok(line)) = read_future_result {
					let juno_impl = juno_impl.clone();
					task::spawn(async move {
						juno_impl.on_data(line.as_bytes().to_vec()).await;
					});
				}
			}
			Either::Right((write_future_result, next_read_future)) => {
				// Write event has happened
				read_future = next_read_future;
				write_future = write_receiver.next();
				read_or_write_future = future::select(read_future, write_future);
				// Write the recieved bytes to the socket
				if let Some(bytes) = write_future_result {
					let mut socket = &client;
					if let Err(err) = socket.write_all(&bytes).await {
						println!("Error while sending data to socket: {}", err);
					}
				}
			}
		}
	}
	// Either a read, nor a write event has happened.
	// This means the socket close event happened. Shutdown the socket and close any mpsc channels
	drop(lines);
	write_receiver.close();
	close_receiver.close();
}
