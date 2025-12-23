/// Different types of roles a message can have.
// see if it makes sense to add a Other(String) role here.
// incase some providers have a unique role.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

pub enum ContentPart {
    Text(String),
    Binary(String),
    ToolCall(()),
    ToolResponse(()),
}

pub struct Tool {}
pub struct ToolCall {}
pub struct ToolResponse {}

pub struct ChatMessage {}

pub struct ChatRequest {}
pub struct ChatResponse {}
