use crate::{
	connection::{BaseConnection, Buffer},
	models::Value,
	protocol::BaseProtocol,
	utils::{self, Error, Result},
};

use async_std::sync::{Mutex, RwLock};
use futures::channel::oneshot::Sender;
use std::collections::HashMap;

type HookListeners = RwLock<HashMap<String, Vec<fn(Value)>>>;
type Functions = RwLock<HashMap<String, fn(HashMap<String, Value>) -> Value>>;

// Create separate rwlocks for each individual element
// Such that each one of them can be individually read or written independent of the other
pub(crate) struct JunoModuleImpl {
	pub(crate) protocol: RwLock<BaseProtocol>,
	pub(crate) connection: RwLock<Box<dyn BaseConnection + Send + Sync>>,
	pub(crate) requests: RwLock<HashMap<String, Sender<Result<Value>>>>,
	pub(crate) functions: Functions,
	pub(crate) hook_listeners: HookListeners,
	pub(crate) message_buffer: Mutex<Buffer>,
	pub(crate) registered: RwLock<bool>,
}

impl JunoModuleImpl {
	pub(crate) async fn execute_function_call(
		&self,
		function: String,
		arguments: HashMap<String, Value>,
	) -> Result<Value> {
		let functions = self.functions.read().await;
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
			*self.registered.write().await = true;
			let mut buffer = self.message_buffer.lock().await;
			self.connection.write().await.send(buffer.clone()).await?;
			buffer.clear();
		} else if &hook == "juno.deactivated" {
			*self.registered.write().await = true;
		} else {
			let hook_listeners = self.hook_listeners.read().await;
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
