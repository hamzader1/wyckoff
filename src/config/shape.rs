//! The four wire shapes wyckoff can speak.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// OpenAI-style `/chat/completions`. Almost every gateway speaks this.
    ChatCompletions,
    /// OpenAI's `/responses`, now the primary surface for their newest models.
    Responses,
    /// Anthropic's native `/messages`. Needed because Anthropic's OpenAI
    /// compatibility layer ignores `response_format`, so JSON mode is lost there.
    AnthropicMessages,
    /// Canned output, for tests and dry runs.
    Mock,
}

impl Shape {
    pub fn as_str(self) -> &'static str {
        match self {
            Shape::ChatCompletions => "chat_completions",
            Shape::Responses => "responses",
            Shape::AnthropicMessages => "anthropic_messages",
            Shape::Mock => "mock",
        }
    }

    pub fn parse(value: &str) -> Option<Shape> {
        match value.trim().to_lowercase().as_str() {
            "chat_completions" | "chat" | "openai" | "completions" => Some(Shape::ChatCompletions),
            "responses" | "openai_responses" => Some(Shape::Responses),
            "anthropic_messages" | "anthropic" | "messages" | "claude" => {
                Some(Shape::AnthropicMessages)
            }
            "mock" | "test" => Some(Shape::Mock),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shape_aliases() {
        assert_eq!(
            Shape::parse("chat_completions"),
            Some(Shape::ChatCompletions)
        );
        assert_eq!(Shape::parse("Responses"), Some(Shape::Responses));
        assert_eq!(Shape::parse("claude"), Some(Shape::AnthropicMessages));
        assert_eq!(Shape::parse("mock"), Some(Shape::Mock));
        assert_eq!(Shape::parse("nonsense"), None);
    }

    #[test]
    fn round_trips_through_str() {
        for shape in [
            Shape::ChatCompletions,
            Shape::Responses,
            Shape::AnthropicMessages,
            Shape::Mock,
        ] {
            assert_eq!(Shape::parse(shape.as_str()), Some(shape));
        }
    }
}
