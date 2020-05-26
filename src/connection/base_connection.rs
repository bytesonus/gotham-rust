use crate::{connection::Buffer, juno_module_impl::JunoModuleImpl, utils::Error};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait BaseConnection {
	async fn setup_connection(&mut self) -> Result<(), Error>;
	async fn close_connection(&mut self) -> Result<(), Error>;
	async fn send(&mut self, buffer: Buffer) -> Result<(), Error>;

	fn set_data_listener(&mut self, listener: Arc<JunoModuleImpl>);
	fn get_data_listener(&self) -> &Option<Arc<JunoModuleImpl>>;
}
