use crate::{
	connection::{BaseConnection, Buffer},
	models::{BaseMessage, Value},
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
pub struct JunoModuleImpl {
	pub protocol: RwLock<BaseProtocol>,
	pub connection: RwLock<Box<dyn BaseConnection + Send + Sync>>,
	pub requests: RwLock<HashMap<String, Sender<Result<Value>>>>,
	pub functions: Functions,
	pub hook_listeners: HookListeners,
	pub message_buffer: Mutex<Buffer>,
	pub registered: RwLock<bool>,
}

impl JunoModuleImpl {
	pub async fn execute_function_call(
		&self,
		function: String,
		arguments: HashMap<String, Value>,
	) -> Result<Value> {
		let functions = self.functions.read().await;
		if !functions.contains_key(&function) {
			return Err(Error::FromJuno(utils::errors::UNKNOWN_FUNCTION));
		}
		let function = functions[&function].clone();
		drop(functions);
		Ok(function(arguments))
	}

	pub async fn execute_hook_triggered(&self, hook: Option<String>, data: Value) -> Result<()> {
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
			let listeners = hook_listeners[&hook].clone();
			drop(hook_listeners);
			for listener in listeners {
				listener(data.clone());
			}
		}
		Ok(())
	}

	pub async fn on_data(&self, buffer: Buffer) {
		let message = self.protocol.read().await.decode(buffer);
		let request_id = message.get_request_id().clone();
		let value = match message {
			BaseMessage::FunctionCallResponse { data, .. } => Ok(data),
			BaseMessage::FunctionCallRequest {
				function,
				arguments,
				..
			} => {
				let result = self.execute_function_call(function, arguments).await;
				let write_buffer = self.protocol.read().await.encode(match result {
					Ok(value) => BaseMessage::FunctionCallResponse {
						request_id: request_id.clone(),
						data: value,
					},
					Err(error) => BaseMessage::Error {
						request_id: request_id.clone(),
						error: match error {
							Error::Internal(_) => 0,
							Error::FromJuno(error_code) => error_code,
						},
					},
				});
				self.connection
					.write()
					.await
					.send(write_buffer)
					.await
					.unwrap();
				Ok(Value::Null)
			}
			BaseMessage::TriggerHookResponse { hook, data, .. } => {
				if let Err(err) = self.execute_hook_triggered(hook, data).await {
					Err(err)
				} else {
					Ok(Value::Null)
				}
			}
			BaseMessage::Error { error, .. } => Err(Error::FromJuno(error)),
			_ => Ok(Value::Null),
		};
		let mut requests = self.requests.write().await;
		if !requests.contains_key(&request_id) {
			drop(requests);
			return;
		}
		if requests.remove(&request_id).unwrap().send(value).is_err() {
			println!("Error sending response of requestId: {}", &request_id);
		}
	}
}
