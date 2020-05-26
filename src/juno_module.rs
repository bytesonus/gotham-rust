extern crate async_std;
extern crate futures;
extern crate futures_util;
extern crate serde_json;

use crate::juno_module_impl::JunoModuleImpl;
pub use serde_json::json;

use crate::{
	connection::{BaseConnection, InetSocketConnection},
	models::{BaseMessage, Value},
	protocol::BaseProtocol,
	utils::{Error, Result},
};

#[cfg(target_family = "unix")]
use crate::connection::UnixSocketConnection;

use async_std::sync::{Arc, Mutex, RwLock};
use futures::channel::oneshot::channel;
use std::{
	collections::HashMap,
	net::{AddrParseError, SocketAddr},
};

pub struct JunoModule {
	pub(crate) module_impl: Arc<JunoModuleImpl>,
}

impl JunoModule {
	pub fn default(connection_path: &str) -> Self {
		let is_ip: std::result::Result<SocketAddr, AddrParseError> =
			connection_path.to_string().parse();
		if let Ok(ip) = is_ip {
			Self::from_inet_socket(&format!("{}", ip.ip()), ip.port())
		} else {
			Self::from_unix_socket(connection_path)
		}
	}

	#[cfg(target_family = "windows")]
	pub fn from_unix_socket(_: &str) -> Self {
		panic!("Unix sockets are not supported on windows");
	}

	#[cfg(target_family = "unix")]
	pub fn from_unix_socket(socket_path: &str) -> Self {
		JunoModule::new(
			BaseProtocol::default(),
			Box::new(UnixSocketConnection::new(socket_path.to_string())),
		)
	}

	pub fn from_inet_socket(host: &str, port: u16) -> Self {
		JunoModule::new(
			BaseProtocol::default(),
			Box::new(InetSocketConnection::new(format!("{}:{}", host, port))),
		)
	}

	pub fn new(protocol: BaseProtocol, connection: Box<dyn BaseConnection + Send + Sync>) -> Self {
		JunoModule {
			module_impl: Arc::new(JunoModuleImpl {
				protocol: RwLock::new(protocol),
				connection: RwLock::new(connection),
				requests: RwLock::new(HashMap::new()),
				functions: RwLock::new(HashMap::new()),
				hook_listeners: RwLock::new(HashMap::new()),
				message_buffer: Mutex::new(vec![]),
				registered: RwLock::new(false),
			}),
		}
	}

	pub async fn initialize(
		&mut self,
		module_id: &str,
		version: &str,
		dependencies: HashMap<String, String>,
	) -> Result<()> {
		self.setup_connections().await?;

		let request = self.module_impl.protocol.write().await.initialize(
			String::from(module_id),
			String::from(version),
			dependencies,
		);
		self.send_request(request).await?;
		Ok(())
	}

	pub async fn declare_function(
		&mut self,
		fn_name: &str,
		function: fn(HashMap<String, Value>) -> Value,
	) -> Result<()> {
		let fn_name = fn_name.to_string();
		self.module_impl
			.functions
			.write()
			.await
			.insert(fn_name.clone(), function);

		let request = self
			.module_impl
			.protocol
			.read()
			.await
			.declare_function(fn_name);
		self.send_request(request).await?;
		Ok(())
	}

	pub async fn call_function(
		&mut self,
		fn_name: &str,
		args: HashMap<String, Value>,
	) -> Result<Value> {
		let fn_name = fn_name.to_string();
		let request = self
			.module_impl
			.protocol
			.read()
			.await
			.call_function(fn_name, args);
		self.send_request(request).await
	}

	pub async fn register_hook(&mut self, hook: &str, callback: fn(Value)) -> Result<()> {
		let hook = hook.to_string();
		let mut hook_listeners = self.module_impl.hook_listeners.write().await;
		if hook_listeners.contains_key(&hook) {
			hook_listeners.get_mut(&hook).unwrap().push(callback);
		} else {
			hook_listeners.insert(hook.clone(), vec![callback]);
		}
		drop(hook_listeners);

		let request = self.module_impl.protocol.read().await.register_hook(hook);
		self.send_request(request).await?;
		Ok(())
	}

	pub async fn trigger_hook(&mut self, hook: &str, data: Value) -> Result<()> {
		let hook = hook.to_string();
		let request = self
			.module_impl
			.protocol
			.read()
			.await
			.trigger_hook(hook, data);
		self.send_request(request).await?;
		Ok(())
	}

	pub async fn close(&mut self) -> Result<()> {
		self.module_impl
			.connection
			.write()
			.await
			.close_connection()
			.await
	}

	async fn setup_connections(&mut self) -> Result<()> {
		self.module_impl
			.connection
			.write()
			.await
			.set_data_listener(self.module_impl.clone());
		self.module_impl
			.connection
			.write()
			.await
			.setup_connection()
			.await?;

		Ok(())
	}

	async fn send_request(&mut self, request: BaseMessage) -> Result<Value> {
		if let BaseMessage::RegisterModuleRequest { .. } = request {
			if *self.module_impl.registered.read().await {
				return Err(Error::Internal(String::from("Module already registered")));
			}
		}

		let request_type = request.get_type();
		let request_id = request.get_request_id().clone();
		let mut encoded = self.module_impl.protocol.read().await.encode(request);

		let (sender, receiver) = channel::<Result<Value>>();

		self.module_impl
			.requests
			.write()
			.await
			.insert(request_id, sender);

		if *self.module_impl.registered.read().await || request_type == 1 {
			self.module_impl
				.connection
				.write()
				.await
				.send(encoded)
				.await?;
		} else {
			self.module_impl
				.message_buffer
				.lock()
				.await
				.append(&mut encoded);
		}

		match receiver.await {
			Ok(value) => value,
			Err(_) => Err(Error::Internal(String::from(
				"Request sender was dropped before data could be retrieved",
			))),
		}
	}
}
