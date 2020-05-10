use crate::{connection::Buffer, utils::Error};
use async_trait::async_trait;

#[async_trait]
pub trait BaseConnection {
	async fn setup_connection(&mut self) -> Result<(), Error>;
	async fn close_connection(&mut self) -> Result<(), Error>;
	async fn send(&mut self, buffer: Buffer) -> Result<(), Error>;
	async fn read_data(&mut self) -> Option<Buffer>;
}
