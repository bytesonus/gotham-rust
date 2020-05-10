use crate::{
	connection::{BaseConnection, Buffer},
	models::Value,
	protocol::BaseProtocol,
	utils::{self, Error, Result},
};

use async_std::sync::{Mutex, RwLock};
use futures::channel::oneshot::Sender;
use std::collections::HashMap;

// Create separate rwlocks for each individual element
// Such that each one of them can be individually read or written independent of the other
pub(crate) struct JunoModuleImpl {
	pub(crate) protocol: Mutex<BaseProtocol>,
	pub(crate) connection: RwLock<Box<dyn BaseConnection + Send + Sync>>,
	pub(crate) requests: Mutex<HashMap<String, Sender<Result<Value>>>>,
	pub(crate) functions: Mutex<HashMap<String, fn(HashMap<String, Value>) -> Value>>,
	pub(crate) hook_listeners: Mutex<HashMap<String, Vec<fn(Value)>>>,
	pub(crate) message_buffer: Mutex<Buffer>,
	pub(crate) registered: Mutex<bool>,
}

impl JunoModuleImpl {
	pub(crate) async fn execute_function_call(
		&self,
		function: String,
		arguments: HashMap<String, Value>,
	) -> Result<Value> {
		let functions = self.functions.lock().await;
		if !functions.contains_key(&function) {
			return Err(Error::FromJuno(utils::errors::UNKNOWN_FUNCTION));
		}
		Ok(functions[&function](arguments))
	}

	pub(crate) async fn execute_hook_triggered(
		&self,
		hook: Option<String>,
		data: Value,
	) -> Result<()> {
		if hook.is_none() {
			// This module triggered the hook.
			return Ok(());
		}

		let hook = hook.unwrap();
		if hook == "juno.activated" {
			*self.registered.lock().await = true;
			let mut buffer = self.message_buffer.lock().await;
			self.connection.write().await.send(buffer.clone()).await?;
			buffer.clear();
		} else if &hook == "juno.deactivated" {
			*self.registered.lock().await = true;
		} else {
			let hook_listeners = self.hook_listeners.lock().await;
			if !hook_listeners.contains_key(&hook) {
				todo!("Wtf do I do now? Need to propogate errors. How do I do that?");
			}
			for listener in &hook_listeners[&hook] {
				listener(data.clone());
			}
		}
		Ok(())
	}
}
