//! Provider-agnostic canonical request/response shapes.
//!
//! The engine ([`crate::engine::ChatEngine`]) speaks these types.
//! Each [`crate::adapter::ChatProviderAdapter`] translates to/from
//! its provider's wire format. OpenAI Chat Completions is the
//! reference shape — fields here mirror that ABI.
//!
//! ## Multimodal content
//!
//! [`Message::content`] is a [`MessageContent`] enum: a `Text(String)`
//! variant for the common single-text case (system / assistant /
//! tool-result messages, and text-only user prompts) plus a
//! `Parts(Vec<ContentPart>)` variant for multimodal user input. The
//! per-provider adapters branch on this when encoding to their wire
//! format — text-only paths read [`MessageContent::as_text`], parts-
//! aware paths handle each [`ContentPart`] explicitly.

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct NormalizedChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    /// JSON Schema for the structured response. `None` = free-form text
    /// (provider-side schema enforcement is skipped).
    pub response_schema: Option<Value>,
    pub strict_response: bool,
    /// Function-tool definitions exposed to the model.
    pub tools: Vec<ToolDef>,
    pub tool_choice: ToolChoiceWire,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_completion_tokens: Option<u32>,
    pub seed: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
    /// For `Role::Assistant` messages that emitted tool calls.
    pub tool_calls: Vec<ToolCall>,
    /// For `Role::Tool` messages, the id of the tool_call this answers.
    pub tool_call_id: Option<String>,
}

/// Per-message content. Multimodal input lands in
/// [`MessageContent::Parts`]; the [`MessageContent::Text`] variant
/// covers the common text-only case (and is what every adapter has
/// historically encoded).
#[derive(Debug, Clone)]
pub enum MessageContent {
    /// Plain text — the default for system / assistant / tool-result
    /// messages and text-only user prompts.
    Text(String),
    /// Mixed content: text + image + audio + file in declared order.
    /// Currently produced only for [`Role::User`] messages by the
    /// engine when the binding spec declares
    /// `prompt.image_inputs` / `audio_inputs` / `file_inputs`.
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Convenience: extract just the text. For `Parts`, concatenates
    /// every `ContentPart::Text` and ignores media. Used by adapters
    /// on text-only paths (system-prompt collation, tool-result
    /// echo, assistant turns).
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Parts(parts) => {
                let mut out = String::new();
                for p in parts {
                    if let ContentPart::Text(s) = p {
                        if !out.is_empty() {
                            out.push('\n');
                        }
                        out.push_str(s);
                    }
                }
                out
            }
        }
    }

    /// `true` when this is `Text` or a `Parts` whose only entries
    /// are `ContentPart::Text`. Adapters use this to skip the
    /// multimodal-encoding path entirely.
    pub fn is_text_only(&self) -> bool {
        match self {
            Self::Text(_) => true,
            Self::Parts(parts) => parts.iter().all(|p| matches!(p, ContentPart::Text(_))),
        }
    }

    /// `true` when the content carries no text and no parts.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(s) => s.is_empty(),
            Self::Parts(parts) => parts.is_empty(),
        }
    }
}

/// One element of a [`MessageContent::Parts`] message.
#[derive(Debug, Clone)]
pub enum ContentPart {
    Text(String),
    Image(ImageContent),
    Audio(AudioContent),
    File(FileContent),
}

#[derive(Debug, Clone)]
pub struct ImageContent {
    pub source: ImageSource,
    /// OpenAI: `auto` | `high` | `low`. Anthropic / Gemini: ignored.
    pub detail: Option<ImageDetail>,
}

#[derive(Debug, Clone)]
pub enum ImageSource {
    /// HTTP(S) URL. OpenAI passes through; Anthropic / Gemini fetch
    /// and convert to base64 via [`crate::multimodal`].
    Url(String),
    /// Inline base64-encoded bytes with declared MIME type.
    Base64 { mime_type: String, data: String },
    /// `mcpg-resource://<id>` URI. The engine resolves through
    /// [`mcpg_plugin_protocol::BackendHost::fetch_content`] before
    /// the adapter encodes — adapters never see this variant unless
    /// resolution failed.
    McpResource(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDetail {
    Auto,
    High,
    Low,
}

#[derive(Debug, Clone)]
pub struct AudioContent {
    pub source: AudioSource,
    pub format: AudioFormat,
}

#[derive(Debug, Clone)]
pub enum AudioSource {
    Url(String),
    Base64 { data: String },
    McpResource(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Mp3,
    Wav,
    Flac,
    Ogg,
    Aac,
    Pcm,
}

impl AudioFormat {
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Flac => "audio/flac",
            Self::Ogg => "audio/ogg",
            Self::Aac => "audio/aac",
            Self::Pcm => "audio/pcm",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileContent {
    pub source: FileSource,
    pub mime_type: String,
    /// Operator-supplied filename hint surfaced to the model. Some
    /// providers display this in the conversation transcript.
    pub filename: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FileSource {
    Url(String),
    Base64 { data: String },
    McpResource(String),
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(content.into()),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Multimodal user message — text plus media in declared order.
    /// Used by the engine when the binding spec wired
    /// `prompt.image_inputs` / `audio_inputs` / `file_inputs`.
    pub fn user_parts(parts: Vec<ContentPart>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Parts(parts),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    pub fn assistant_tool_calls(calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(String::new()),
            tool_calls: calls,
            tool_call_id: None,
        }
    }

    /// Anthropic-style assistant turn with both text and tool_use
    /// blocks. The engine uses this when the model returns a
    /// non-empty text segment before/after its tool calls — common on
    /// Claude where the model often narrates ("let me check the
    /// database…") before invoking a tool. OpenAI usually returns
    /// empty content alongside tool_calls; either path is supported.
    pub fn assistant_text_and_tool_calls(text: impl Into<String>, calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(text.into()),
            tool_calls: calls,
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: MessageContent::Text(content.into()),
            tool_calls: vec![],
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema for the tool's input.
    pub parameters: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolChoiceWire {
    Auto,
    Required,
    None,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Arguments as the model emitted them — JSON, but not yet validated
    /// against the child binding's input schema.
    pub arguments: Value,
}

#[derive(Debug, Clone)]
pub struct NormalizedChatResponse {
    /// Final text content (may be empty if the assistant only emitted
    /// tool calls).
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
    Other,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_text_constructors_produce_text_variant() {
        let m = Message::system("rules");
        assert!(matches!(m.content, MessageContent::Text(ref s) if s == "rules"));

        let m = Message::user("hello");
        assert!(matches!(m.content, MessageContent::Text(ref s) if s == "hello"));

        let m = Message::tool_result("t1", "{\"ok\":true}");
        assert!(matches!(m.content, MessageContent::Text(ref s) if s == "{\"ok\":true}"));
    }

    #[test]
    fn user_parts_constructor_produces_parts_variant() {
        let m = Message::user_parts(vec![
            ContentPart::Text("look at this".into()),
            ContentPart::Image(ImageContent {
                source: ImageSource::Url("https://example.com/a.png".into()),
                detail: Some(ImageDetail::High),
            }),
        ]);
        assert!(matches!(m.content, MessageContent::Parts(_)));
        assert_eq!(m.content.as_text(), "look at this");
        assert!(!m.content.is_text_only());
    }

    #[test]
    fn as_text_concatenates_text_parts_only() {
        let c = MessageContent::Parts(vec![
            ContentPart::Text("first".into()),
            ContentPart::Image(ImageContent {
                source: ImageSource::Base64 {
                    mime_type: "image/png".into(),
                    data: "abc".into(),
                },
                detail: None,
            }),
            ContentPart::Text("second".into()),
        ]);
        assert_eq!(c.as_text(), "first\nsecond");
    }

    #[test]
    fn is_text_only_true_for_only_text_parts() {
        let c = MessageContent::Parts(vec![ContentPart::Text("x".into())]);
        assert!(c.is_text_only());
        let c = MessageContent::Parts(vec![
            ContentPart::Text("x".into()),
            ContentPart::Audio(AudioContent {
                source: AudioSource::Base64 { data: "abc".into() },
                format: AudioFormat::Mp3,
            }),
        ]);
        assert!(!c.is_text_only());
    }

    #[test]
    fn audio_format_mime_types() {
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
        assert_eq!(AudioFormat::Wav.mime_type(), "audio/wav");
        assert_eq!(AudioFormat::Flac.mime_type(), "audio/flac");
        assert_eq!(AudioFormat::Ogg.mime_type(), "audio/ogg");
    }
}
