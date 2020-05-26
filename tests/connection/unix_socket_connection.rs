use async_std::{fs::remove_file, io::Result, os::unix::net::UnixListener, prelude::*};
use futures::future;
use juno::connection::{BaseConnection, UnixSocketConnection};

#[test]
fn connection_object_should_create_successfully() {
	let _socket_connection = UnixSocketConnection::new(String::from("socket_path"));
}

#[async_std::test]
async fn should_connect_async() -> Result<()> {
	// Setup to try and connect to socket server
	let mut connection = UnixSocketConnection::new(String::from("./temp-1.sock"));

	// Listen for unix socket connections
	let socket = UnixListener::bind("./temp-1.sock").await?;
	let mut incoming = socket.incoming();
	let connection_listener = incoming.next();

	let (..) = future::join(connection_listener, connection.setup_connection()).await;

	drop(socket);
	remove_file("./temp-1.sock").await?;

	Ok(())
}

#[async_std::test]
async fn should_connect_and_send_data_async() -> Result<()> {
	// Setup to try and connect to socket server
	let mut connection = UnixSocketConnection::new(String::from("./temp-2.sock"));

	// Listen for unix socket connections
	let socket = UnixListener::bind("./temp-2.sock").await?;
	let mut incoming = socket.incoming();
	let connection_listener = incoming.next();

	let (stream, _) = future::join(connection_listener, connection.setup_connection()).await;

	let mut read_buffer = [0; 10];
	let (_, read_result) = futures::future::join(
		connection.send(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]),
		stream.unwrap()?.read(&mut read_buffer),
	)
	.await;
	read_result?;

	assert_eq!(read_buffer.to_vec(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);

	drop(socket);
	remove_file("./temp-2.sock").await?;

	Ok(())
}

#[async_std::test]
async fn should_connect_and_read_data_async() -> Result<()> {
	// Setup to try and connect to socket server
	let mut connection = UnixSocketConnection::new(String::from("./temp-3.sock"));

	// Listen for unix socket connections
	let socket = UnixListener::bind("./temp-3.sock").await?;
	let mut incoming = socket.incoming();
	let connection_listener = incoming.next();

	let (stream, _) = future::join(connection_listener, connection.setup_connection()).await;

	let write_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0, b'\n'];
	let mut stream = stream.unwrap()?;

	let write_result = stream.write_all(write_data.as_slice()).await;
	write_result?;

	let mut receiver = connection.get_data_receiver();
	let read_result = receiver.next().await;
	let read_buffer = read_result.unwrap();

	assert_eq!(read_buffer, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);

	drop(socket);
	remove_file("./temp-3.sock").await?;

	Ok(())
}

#[async_std::test]
async fn should_connect_and_send_data_from_cloned_sender_async() -> Result<()> {
	// Setup to try and connect to socket server
	let mut connection = UnixSocketConnection::new(String::from("./temp-4.sock"));

	// Listen for unix socket connections
	let socket = UnixListener::bind("./temp-4.sock").await?;
	let mut incoming = socket.incoming();
	let connection_listener = incoming.next();

	let (stream, _) = future::join(connection_listener, connection.setup_connection()).await;

	let mut read_buffer = [0; 10];
	let (write_result, read_result) = futures::future::join(
		connection
			.clone_write_sender()
			.send(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]),
		stream.unwrap()?.read(&mut read_buffer),
	)
	.await;
	write_result.unwrap();
	read_result?;

	assert_eq!(read_buffer.to_vec(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);

	drop(socket);
	remove_file("./temp-4.sock").await?;

	Ok(())
}

#[async_std::test]
#[should_panic]
async fn should_send_data_without_connection_and_panic() {
	let connection = UnixSocketConnection::new(String::from("./test.sock"));
	connection.send(vec![]).await;
}

#[async_std::test]
#[should_panic]
async fn should_close_connection_without_setup_and_panic() {
	let mut connection = UnixSocketConnection::new(String::from("./test.sock"));
	connection.close_connection().await;
}

#[test]
#[should_panic]
fn should_get_data_receiver_without_setup_and_panic() {
	let mut connection = UnixSocketConnection::new(String::from("./test.sock"));
	connection.get_data_receiver();
}

#[test]
#[should_panic]
fn should_clone_write_sender_without_setup_and_panic() {
	let connection = UnixSocketConnection::new(String::from("./test.sock"));
	connection.clone_write_sender();
}

#[async_std::test]
#[should_panic]
async fn should_setup_connection_twice_and_panic_async() {
	// Setup to try and connect to socket server
	let mut connection = UnixSocketConnection::new(String::from("./temp-5.sock"));

	// Listen for unix socket connections
	let socket = UnixListener::bind("./temp-5.sock").await.unwrap();
	let mut incoming = socket.incoming();
	let (_stream, result) = future::join(incoming.next(), connection.setup_connection()).await;
	result.unwrap();
	let (_, result) = future::join(incoming.next(), connection.setup_connection()).await;
	result.unwrap();
	drop(socket);
}
