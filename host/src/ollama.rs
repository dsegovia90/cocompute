use common::protocols::chat::{ChatMessage, ToolCall, ToolCallFunction, ToolDefinition};
use ollama_rs::generation::{
    chat::{ChatMessage as OllamaChatMessage, MessageRole},
    tools::{ToolCall as OllamaToolCall, ToolFunctionInfo, ToolInfo, ToolType},
};

/// Strip the `@dev` suffix from model names in debug builds.
/// In release builds, this is a no-op (the suffix is never added).
pub(crate) fn ollama_model_name(model: &str) -> String {
    if cfg!(debug_assertions) {
        model.strip_suffix("@dev").unwrap_or(model).to_string()
    } else {
        model.to_string()
    }
}

/// Convert protocol `ChatMessage`s into Ollama `ChatMessage`s.
pub(crate) fn convert_messages(messages: Vec<ChatMessage>) -> Vec<OllamaChatMessage> {
    messages
        .into_iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            let mut msg = OllamaChatMessage::new(role, m.content);
            if !m.images.is_empty() {
                let images = m
                    .images
                    .into_iter()
                    .map(ollama_rs::generation::images::Image::from_base64)
                    .collect();
                msg = msg.with_images(images);
            }
            msg
        })
        .collect()
}

/// Convert protocol `ToolDefinition`s into Ollama `ToolInfo`s.
/// Tools whose `parameters` JSON is invalid are silently dropped (with a warning).
pub(crate) fn convert_tools(tools: &[ToolDefinition]) -> Vec<ToolInfo> {
    tools
        .iter()
        .filter_map(|t| {
            let params: serde_json::Value = match serde_json::from_str(&t.function.parameters) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("tool '{}' has invalid parameters JSON: {e}", t.function.name);
                    return None;
                }
            };
            let schema: schemars::Schema = match serde_json::from_value(params) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("tool '{}' has invalid JSON Schema: {e}", t.function.name);
                    return None;
                }
            };
            Some(ToolInfo {
                tool_type: ToolType::Function,
                function: ToolFunctionInfo {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: schema,
                },
            })
        })
        .collect()
}

/// Convert Ollama tool-call results into protocol `ToolCall`s.
pub(crate) fn convert_tool_calls(tool_calls: Vec<OllamaToolCall>) -> Vec<ToolCall> {
    tool_calls
        .into_iter()
        .map(|tc| ToolCall {
            id: format!("call_{}", uuid::Uuid::new_v4().simple()),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: tc.function.name,
                arguments: tc.function.arguments.to_string(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ollama_rs::generation::tools::{
        ToolCall as OllamaToolCall, ToolCallFunction as OllamaToolCallFunction,
    };

    fn ollama_call(name: &str) -> OllamaToolCall {
        OllamaToolCall {
            function: OllamaToolCallFunction {
                name: name.into(),
                arguments: serde_json::json!({}),
            },
        }
    }

    #[test]
    fn ids_are_unique_within_one_response() {
        let out = convert_tool_calls(vec![
            ollama_call("list_runs"),
            ollama_call("summarize_run"),
            ollama_call("compare_runs"),
        ]);
        let ids: std::collections::HashSet<_> = out.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.len(), 3, "expected 3 distinct ids, got {ids:?}");
    }

    #[test]
    fn ids_do_not_collide_across_calls() {
        // Simulates two separate assistant turns each emitting one tool call.
        // The old enumerate-based scheme produced `call_0` for both, which
        // crashed Mastra Studio's tool-trace renderer on duplicate React keys.
        let turn_a = convert_tool_calls(vec![ollama_call("list_runs")]);
        let turn_b = convert_tool_calls(vec![ollama_call("summarize_run")]);
        assert_ne!(turn_a[0].id, turn_b[0].id);
    }

    #[test]
    fn id_uses_openai_call_prefix() {
        let out = convert_tool_calls(vec![ollama_call("noop")]);
        assert!(
            out[0].id.starts_with("call_"),
            "id `{}` should start with `call_`",
            out[0].id
        );
    }

    #[test]
    fn function_name_and_arguments_are_preserved() {
        let mut tc = ollama_call("do_thing");
        tc.function.arguments = serde_json::json!({"x": 1, "y": "two"});
        let out = convert_tool_calls(vec![tc]);
        assert_eq!(out[0].function.name, "do_thing");
        // arguments is stored as a JSON-encoded string in the protocol.
        let parsed: serde_json::Value = serde_json::from_str(&out[0].function.arguments).unwrap();
        assert_eq!(parsed["x"], 1);
        assert_eq!(parsed["y"], "two");
    }
}
