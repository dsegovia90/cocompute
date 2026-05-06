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
        .enumerate()
        .map(|(i, tc)| ToolCall {
            id: format!("call_{i}"),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: tc.function.name,
                arguments: tc.function.arguments.to_string(),
            },
        })
        .collect()
}
