use crate::{
	connection::Buffer,
	models::{BaseMessage, Value},
	protocol::json_protocol,
};

use std::{
	collections::HashMap,
	time::{SystemTime, UNIX_EPOCH},
};

pub enum BaseProtocol {
	JsonProtocol { module_id: String },
	MsgPackProtocol { module_id: String },
}

impl BaseProtocol {
	pub fn default() -> Self {
		json_protocol::default()
	}

	pub fn from(other: &Self) -> Self {
		match other {
			BaseProtocol::JsonProtocol { .. } => json_protocol::from(other),
			_ => panic!("Currently, only JsonProtocol is supported"),
		}
	}

	pub fn generate_request_id(&self) -> String {
		format!(
			"{}-{}",
			self.get_module_id(),
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.expect("Time went backwards. Wtf?")
				.as_nanos()
		)
	}

	pub fn get_module_id(&self) -> &String {
		match self {
			BaseProtocol::JsonProtocol { module_id } => module_id,
			_ => panic!("Currently, only JsonProtocol is supported"),
		}
	}

	pub fn set_module_id(&mut self, new_module_id: String) {
		match self {
			BaseProtocol::JsonProtocol { ref mut module_id } => {
				*module_id = new_module_id;
			}
			_ => panic!("Currently, only JsonProtocol is supported"),
		}
	}

	pub fn initialize(
		&mut self,
		module_id: String,
		version: String,
		dependencies: HashMap<String, String>,
	) -> BaseMessage {
		self.set_module_id(module_id);
		BaseMessage::RegisterModuleRequest {
			request_id: self.generate_request_id(),
			module_id: self.get_module_id().clone(),
			version,
			dependencies,
		}
	}

	pub fn register_hook(&self, hook: String) -> BaseMessage {
		BaseMessage::RegisterHookRequest {
			request_id: self.generate_request_id(),
			hook,
		}
	}

	pub fn trigger_hook(&self, hook: String, data: Value) -> BaseMessage {
		BaseMessage::TriggerHookRequest {
			request_id: self.generate_request_id(),
			hook,
			data,
		}
	}

	pub fn declare_function(&self, function: String) -> BaseMessage {
		BaseMessage::DeclareFunctionRequest {
			request_id: self.generate_request_id(),
			function,
		}
	}

	pub fn call_function(
		&self,
		function: String,
		arguments: HashMap<String, Value>,
	) -> BaseMessage {
		BaseMessage::FunctionCallRequest {
			request_id: self.generate_request_id(),
			function,
			arguments,
		}
	}

	pub fn encode(&self, req: BaseMessage) -> Buffer {
		match self {
			BaseProtocol::JsonProtocol { .. } => json_protocol::encode(&self, req),
			_ => panic!("Currently, only JsonProtocol is supported"),
		}
	}

	pub fn decode(&self, data: &[u8]) -> BaseMessage {
		match self {
			BaseProtocol::JsonProtocol { .. } => json_protocol::decode(&self, data),
			_ => panic!("Currently, only JsonProtocol is supported"),
		}
	}
}
