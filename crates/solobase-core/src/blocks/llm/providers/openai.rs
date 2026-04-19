//! OpenAI native API wire format.
//!
//! Encoder: translates a wafer-core `ChatRequest` into a `POST /v1/chat/completions`
//! payload. Decoder: parses OpenAI's SSE frames (`data: {json}\n\n` + terminal
//! `data: [DONE]`) into `ChatChunk` events.
//!
//! See <https://platform.openai.com/docs/api-reference/chat/create> for the
//! reference request/response shapes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wafer_core::interfaces::llm::service::{
    ChatContent, ChatMessage, ChatRequest, ChatRole, ContentPart, ResponseFormat, ToolCall,
    ToolDefinition,
};

use super::config::ProviderConfig;

/// Build an `(url, headers, body)` triple for a streaming OpenAI chat
/// completion. Callers POST the body with the given headers.
///
/// Returns `Err` only if the configured provider is missing the required
/// `api_key` — we never silently omit `Authorization` on the OpenAI native
/// protocol, unlike `openai_compatible` which may.
pub fn encode_chat_request(
    req: &ChatRequest,
    provider: &ProviderConfig,
    resolved_api_key: Option<&str>,
) -> Result<(String, HashMap<String, String>, Vec<u8>), EncodeError> {
    let url = format!("{}/chat/completions", provider.endpoint.trim_end_matches('/'));

    let mut headers = HashMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    match resolved_api_key {
        Some(key) => {
            headers.insert("Authorization".into(), format!("Bearer {key}"));
        }
        None => return Err(EncodeError::MissingApiKey),
    }

    let body = OpenAiRequest::from_chat_request(req);
    let bytes = serde_json::to_vec(&body).map_err(|e| EncodeError::Serialize(e.to_string()))?;
    Ok((url, headers, bytes))
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("missing api_key for openai provider")]
    MissingApiKey,
    #[error("serialize request body: {0}")]
    Serialize(String),
}

// ---------- Wire format types ----------

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAiResponseFormat<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OpenAiTool<'a>>,
    /// When `true`, OpenAI includes a usage frame as the last SSE event
    /// (`stream_options.include_usage`). We always ask for it so our decoder
    /// can emit a terminal `ChatChunk` with `TokenUsage`.
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

impl<'a> OpenAiRequest<'a> {
    fn from_chat_request(req: &'a ChatRequest) -> Self {
        let stop = if req.params.stop_sequences.is_empty() {
            None
        } else {
            Some(req.params.stop_sequences.as_slice())
        };
        let response_format = req.params.response_format.as_ref().map(encode_response_format);
        let tools = req.tools.iter().map(encode_tool).collect::<Vec<_>>();

        Self {
            model: &req.model,
            messages: req.messages.iter().map(encode_message).collect(),
            stream: true,
            temperature: req.params.temperature,
            max_tokens: req.params.max_tokens,
            top_p: req.params.top_p,
            seed: req.params.seed,
            stop,
            response_format,
            tools,
            stream_options: Some(StreamOptions {
                include_usage: true,
            }),
        }
    }
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<OpenAiContent<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<OpenAiToolCall<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenAiContent<'a> {
    Text(&'a str),
    Parts(Vec<OpenAiContentPart<'a>>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum OpenAiContentPart<'a> {
    #[serde(rename = "text")]
    Text { text: &'a str },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OpenAiImageUrl<'a> },
}

#[derive(Serialize)]
struct OpenAiImageUrl<'a> {
    url: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>,
}

#[derive(Serialize)]
struct OpenAiToolCall<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    function: OpenAiToolCallFunction<'a>,
}

#[derive(Serialize)]
struct OpenAiToolCallFunction<'a> {
    name: &'a str,
    /// Wire format is a JSON-encoded string, not a nested object.
    arguments: String,
}

#[derive(Serialize)]
struct OpenAiTool<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    function: OpenAiToolFunction<'a>,
}

#[derive(Serialize)]
struct OpenAiToolFunction<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAiResponseFormat<'a> {
    Text,
    #[serde(rename = "json_object")]
    Json,
    #[serde(rename = "json_schema")]
    JsonSchema { json_schema: &'a serde_json::Value },
}

// ---------- Translation helpers ----------

fn encode_role(role: ChatRole) -> &'static str {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
        // `ChatRole` is `#[non_exhaustive]` — fall back to user for any future
        // role variant, since OpenAI doesn't recognize arbitrary roles.
        _ => "user",
    }
}

fn encode_message(m: &ChatMessage) -> OpenAiMessage<'_> {
    let content = match &m.content {
        ChatContent::Text(s) => Some(OpenAiContent::Text(s)),
        ChatContent::Parts(parts) => Some(OpenAiContent::Parts(
            parts.iter().map(encode_part).collect(),
        )),
        // `ChatContent` is `#[non_exhaustive]` — unknown content kinds skip.
        _ => None,
    };
    // Assistant messages invoking tools send `tool_calls` and may omit content.
    let content = if content.is_some() && matches!(&m.content, ChatContent::Text(s) if s.is_empty())
        && !m.tool_calls.is_empty()
    {
        None
    } else {
        content
    };

    OpenAiMessage {
        role: encode_role(m.role),
        content,
        tool_call_id: m.tool_call_id.as_deref(),
        tool_calls: m.tool_calls.iter().map(encode_tool_call).collect(),
    }
}

fn encode_part(p: &ContentPart) -> OpenAiContentPart<'_> {
    match p {
        ContentPart::Text(s) => OpenAiContentPart::Text { text: s },
        ContentPart::ImageUrl { url, detail } => OpenAiContentPart::ImageUrl {
            image_url: OpenAiImageUrl {
                url,
                detail: detail.as_deref(),
            },
        },
        // OpenAI's image API accepts data URLs, but encoding bytes here would
        // need a base64 dep that solobase-core doesn't already carry. Callers
        // that want to send raw bytes can encode them into a data URL upstream
        // and use `ImageUrl`. Fall back to a text part labeling what happened.
        ContentPart::ImageBytes { .. } => OpenAiContentPart::Text {
            text: "[image bytes unsupported — encode as data URL in ImageUrl]",
        },
        // `ContentPart` is `#[non_exhaustive]`.
        _ => OpenAiContentPart::Text {
            text: "[unknown content part]",
        },
    }
}

fn encode_tool_call(call: &ToolCall) -> OpenAiToolCall<'_> {
    OpenAiToolCall {
        id: &call.id,
        kind: "function",
        function: OpenAiToolCallFunction {
            name: &call.name,
            arguments: call.arguments.to_string(),
        },
    }
}

fn encode_tool(t: &ToolDefinition) -> OpenAiTool<'_> {
    OpenAiTool {
        kind: "function",
        function: OpenAiToolFunction {
            name: &t.name,
            description: &t.description,
            parameters: &t.parameters,
        },
    }
}

fn encode_response_format(r: &ResponseFormat) -> OpenAiResponseFormat<'_> {
    match r {
        ResponseFormat::Text => OpenAiResponseFormat::Text,
        ResponseFormat::Json => OpenAiResponseFormat::Json,
        ResponseFormat::JsonSchema(v) => OpenAiResponseFormat::JsonSchema { json_schema: v },
        // `ResponseFormat` is `#[non_exhaustive]` — fall back to Text for any
        // future variants the spec adds.
        _ => OpenAiResponseFormat::Text,
    }
}

#[cfg(test)]
mod tests {
    use wafer_core::interfaces::llm::service::{
        ChatMessage, ChatParams, ChatRequest, ChatRole, ToolDefinition,
    };

    use super::super::config::{ProviderConfig, ProviderProtocol};
    use super::*;

    fn openai_provider() -> ProviderConfig {
        ProviderConfig::new(
            "openai-main",
            ProviderProtocol::OpenAi,
            "https://api.openai.com/v1",
        )
    }

    #[test]
    fn encodes_simple_chat_request() {
        let req = ChatRequest::new("openai-main", "gpt-4o-mini", vec![ChatMessage::user("hi")]);
        let (url, headers, body) = encode_chat_request(&req, &openai_provider(), Some("sk-test"))
            .expect("encode");
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(
            headers.get("Authorization").map(String::as_str),
            Some("Bearer sk-test")
        );
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["model"], "gpt-4o-mini");
        assert_eq!(json["stream"], true);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "hi");
        assert_eq!(json["stream_options"]["include_usage"], true);
    }

    #[test]
    fn encodes_multi_turn_with_system() {
        let req = ChatRequest::new(
            "openai-main",
            "gpt-4o",
            vec![
                ChatMessage::system("be terse"),
                ChatMessage::user("what is 2+2?"),
                ChatMessage::assistant("4"),
                ChatMessage::user("now divide by 2"),
            ],
        );
        let (_, _, body) =
            encode_chat_request(&req, &openai_provider(), Some("sk")).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let msgs = json["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[2]["role"], "assistant");
    }

    #[test]
    fn encodes_params() {
        let mut params = ChatParams::default();
        params.temperature = Some(0.3);
        params.max_tokens = Some(512);
        params.top_p = Some(0.9);
        params.seed = Some(42);
        params.stop_sequences = vec!["END".into()];
        let mut req =
            ChatRequest::new("openai-main", "gpt-4o", vec![ChatMessage::user("hi")]);
        req.params = params;

        let (_, _, body) =
            encode_chat_request(&req, &openai_provider(), Some("sk")).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["temperature"], 0.3);
        assert_eq!(json["max_tokens"], 512);
        assert_eq!(json["top_p"], 0.9);
        assert_eq!(json["seed"], 42);
        assert_eq!(json["stop"][0], "END");
    }

    #[test]
    fn encodes_tools() {
        let mut req =
            ChatRequest::new("openai-main", "gpt-4o", vec![ChatMessage::user("hi")]);
        // ToolDefinition is #[non_exhaustive]; round-trip through serde.
        let tool: ToolDefinition = serde_json::from_value(serde_json::json!({
            "name": "lookup",
            "description": "look up a thing",
            "parameters": {
                "type": "object",
                "properties": {"x": {"type": "string"}}
            }
        }))
        .unwrap();
        req.tools = vec![tool];
        let (_, _, body) =
            encode_chat_request(&req, &openai_provider(), Some("sk")).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["tools"][0]["type"], "function");
        assert_eq!(json["tools"][0]["function"]["name"], "lookup");
        assert_eq!(
            json["tools"][0]["function"]["parameters"]["properties"]["x"]["type"],
            "string"
        );
    }

    #[test]
    fn rejects_missing_api_key() {
        let req = ChatRequest::new("openai-main", "gpt-4o", vec![ChatMessage::user("hi")]);
        assert!(matches!(
            encode_chat_request(&req, &openai_provider(), None),
            Err(EncodeError::MissingApiKey)
        ));
    }
}
