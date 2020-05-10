use crate::{
	connection::{BaseConnection, Buffer},
	utils::{Error, READ_BUFFER_SIZE},
};

use std::{net::Shutdown, os::unix::net::UnixStream, io::{Write, Read}};
use async_trait::async_trait;

pub struct UnixSocketConnection {
	connection_setup: bool,
	socket_path: String,
	client: Option<UnixStream>,
}

impl UnixSocketConnection {
	pub fn new(socket_path: String) -> Self {
		UnixSocketConnection {
			connection_setup: false,
			socket_path,
			client: None,
		}
	}
}

#[async_trait]
impl BaseConnection for UnixSocketConnection {
	async fn setup_connection(&mut self) -> Result<(), Error> {
		if self.connection_setup {
			panic!("Cannot call setup_connection() more than once!");
		}
		let result = UnixStream::connect(&self.socket_path);
		if let Err(err) = result {
			return Err(Error::Internal(format!("{}", err)));
		}
		let client = result.unwrap();
		client.set_nonblocking(true).unwrap();
		self.client = Some(client);

		self.connection_setup = true;
		Ok(())
	}

	async fn close_connection(&mut self) -> Result<(), Error> {
		if !self.connection_setup || self.client.is_none() {
			panic!("Cannot close a connection that hasn't been established yet. Did you forget to call setup_connection()?");
		}
		let result = self.client.as_ref().unwrap().shutdown(Shutdown::Both);
		if let Err(err) = result {
			return Err(Error::Internal(format!("{}", err)));
		}
		Ok(())
	}

	async fn send(&mut self, buffer: Buffer) -> Result<(), Error> {
		if !self.connection_setup || self.client.is_none() {
			panic!("Cannot send data to a connection that hasn't been established yet. Did you forget to await the call to setup_connection()?");
		}
		let result = self.client.as_mut().unwrap().write_all(&buffer);
		if let Err(err) = result {
			return Err(Error::Internal(format!("{}", err)));
		}
		Ok(())
	}

	async fn read_data(&mut self) -> Option<Buffer> {
		if self.client.is_none() {
			None
		} else {
			let client = self.client.as_mut().unwrap();
			let mut buffer = Vec::new();
			let mut read_size = READ_BUFFER_SIZE;

			while read_size > 0 {
				let mut buf = [0u8; READ_BUFFER_SIZE];
				let result = client.read(&mut buf);
				if result.is_err() {
					println!("Error: {}", result.unwrap_err());
					return None;
				}
				read_size = result.unwrap();
				buffer.extend(buf[..read_size].into_iter());
			}
			Some(buffer)
		}
	}
}
