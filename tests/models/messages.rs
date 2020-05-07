use juno::models::{BaseMessage, Value};
use std::collections::HashMap;

#[test]
fn check_types_are_storing_values() {
	let messages = [
		BaseMessage::RegisterModuleRequest {
			request_id: String::from("request_id"),
			module_id: String::from("module_id"),
			version: String::from("version"),
			dependencies: HashMap::new(),
		},
		BaseMessage::RegisterModuleResponse {
			request_id: String::from("request_id"),
		},
		BaseMessage::FunctionCallRequest {
			request_id: String::from("request_id"),
			function: String::from("function"),
			arguments: HashMap::new(),
		},
		BaseMessage::FunctionCallResponse {
			request_id: String::from("request_id"),
			data: Value::Null,
		},
		BaseMessage::RegisterHookRequest {
			request_id: String::from("request_id"),
			hook: String::from("hook"),
		},
		BaseMessage::RegisterHookResponse {
			request_id: String::from("request_id"),
		},
		BaseMessage::TriggerHookRequest {
			request_id: String::from("request_id"),
			hook: String::from("hook"),
			data: Value::Null,
		},
		BaseMessage::TriggerHookResponse {
			request_id: String::from("request_id"),
			hook: Some(String::from("hook")),
			data: Value::Null,
		},
		BaseMessage::DeclareFunctionRequest {
			request_id: String::from("request_id"),
			function: String::from("function"),
		},
		BaseMessage::DeclareFunctionResponse {
			request_id: String::from("request_id"),
			function: String::from("function"),
		},
		BaseMessage::Error {
			request_id: String::from("request_id"),
			error: 0,
		},
		BaseMessage::Unknown {
			request_id: String::from("request_id"),
		},
	];

	for message in messages.iter() {
		match message {
			BaseMessage::RegisterModuleRequest {
				request_id,
				module_id,
				version,
				dependencies,
			} => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(module_id, &String::from("module_id"));
				assert_eq!(version, &String::from("version"));
				assert_eq!(dependencies, &HashMap::new());
			}
			BaseMessage::RegisterModuleResponse { request_id } => {
				assert_eq!(request_id, &String::from("request_id"));
			}
			BaseMessage::FunctionCallRequest {
				request_id,
				function,
				arguments,
			} => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(function, &String::from("function"));
				assert_eq!(arguments, &HashMap::new());
			}
			BaseMessage::FunctionCallResponse { request_id, data } => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(data, &Value::Null);
			}
			BaseMessage::RegisterHookRequest { request_id, hook } => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(hook, &String::from("hook"));
			}
			BaseMessage::RegisterHookResponse { request_id } => {
				assert_eq!(request_id, &String::from("request_id"));
			}
			BaseMessage::TriggerHookRequest {
				request_id,
				hook,
				data,
			} => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(hook, &String::from("hook"));
				assert_eq!(data, &Value::Null);
			}
			BaseMessage::TriggerHookResponse {
				request_id,
				hook,
				data,
			} => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(hook, &Some(String::from("hook")));
				assert_eq!(data, &Value::Null);
			}
			BaseMessage::DeclareFunctionRequest {
				request_id,
				function,
			} => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(function, &String::from("function"));
			}
			BaseMessage::DeclareFunctionResponse {
				request_id,
				function,
			} => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(function, &String::from("function"));
			}
			BaseMessage::Error { request_id, error } => {
				assert_eq!(request_id, &String::from("request_id"));
				assert_eq!(error, &0);
			}
			BaseMessage::Unknown { request_id } => {
				assert_eq!(request_id, &String::from("request_id"));
			}
		}
	}
}
