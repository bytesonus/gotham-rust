use async_std::{io::BufReader, os::unix::net::UnixStream, prelude::*, task};
use async_trait::async_trait;

use futures::{
	channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
	future::{self, Either},
};
use futures_util::SinkExt;

use crate::connection::{BaseConnection, Buffer};

pub struct UnixSocketConnection {
	read_data_receiver: Option<UnboundedReceiver<Vec<u8>>>,
	write_data_sender: Option<UnboundedSender<Vec<u8>>>,
	close_sender: Option<UnboundedSender<()>>,
	socket_path: String,
}

impl UnixSocketConnection {
	pub fn new(socket_path: String) -> Self {
		UnixSocketConnection {
			read_data_receiver: None,
			write_data_sender: None,
			close_sender: None,
			socket_path,
		}
	}
}

async fn read_data_from_socket(
	socket_path: String,
	mut read_sender: UnboundedSender<Vec<u8>>,
	mut write_receiver: UnboundedReceiver<Vec<u8>>,
	mut close_receiver: UnboundedReceiver<()>,
) {
	let result = UnixStream::connect(socket_path).await;
	if let Err(err) = result {
		panic!("Error while connecting to socket: {}", err);
	}
	let client = result.unwrap();
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
					let result = read_sender.send(line.as_bytes().to_vec()).await;
					if let Err(err) = result {
						println!("Error queing data from the socket to the module: {}", err);
					}
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
	let result = read_sender.close().await;
	if let Err(err) = result {
		println!("Error closing the MPSC sender to queue data: {}", err);
	}
	write_receiver.close();
	close_receiver.close();
}

#[async_trait]
impl BaseConnection for UnixSocketConnection {
	async fn setup_connection(&mut self) {
		let (read_data_sender, read_data_receiver) = unbounded::<Vec<u8>>();
		let (write_data_sender, write_data_receiver) = unbounded::<Vec<u8>>();
		let (close_sender, close_receiver) = unbounded::<()>();

		self.read_data_receiver = Some(read_data_receiver);
		self.write_data_sender = Some(write_data_sender);
		self.close_sender = Some(close_sender);
		let socket_path = self.socket_path.clone();

		task::spawn(async {
			read_data_from_socket(
				socket_path,
				read_data_sender,
				write_data_receiver,
				close_receiver,
			)
			.await;
		});
	}

	async fn close_connection(&mut self) {
		if self.close_sender.is_none() {
			println!("Cannot close a connection that hasn't been established yet");
		}
		let mut sender = &self.close_sender.as_ref().unwrap().clone();
		if let Err(err) = sender.send(()).await {
			println!("Error attempting to close connection: {}", err);
		}
	}

	async fn send(&mut self, buffer: Buffer) {
		if self.write_data_sender.is_none() {
			println!("Cannot send data to a connection that hasn't been established yet");
		}
		let mut sender = &self.write_data_sender.as_ref().unwrap().clone();
		if let Err(err) = sender.send(buffer).await {
			println!("Error attempting to send data to connection: {}", err);
		}
	}

	fn get_data_receiver(&mut self) -> UnboundedReceiver<Buffer> {
		self.read_data_receiver.take().unwrap()
	}
}
