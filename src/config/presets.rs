//! Known providers, as data.
//!
//! Every endpoint below was checked against the provider's own documentation.
//! A provider that speaks one of the four wire shapes costs a line in this
//! table (or a `[providers.x]` block in user config) — never new code.

use super::Shape;

#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    pub label: &'static str,
    pub shape: Shape,
    pub base: &'static str,
    pub model: &'static str,
    pub key_env: &'static str,
    pub needs_key: bool,
    /// Send `response_format: {"type":"json_object"}` on chat-completions.
    pub json_mode: bool,
    pub note: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "gemini",
        label: "Google Gemini",
        shape: Shape::ChatCompletions,
        base: "https://generativelanguage.googleapis.com/v1beta/openai",
        model: "gemini-3.8-flash",
        key_env: "GEMINI_API_KEY",
        needs_key: true,
        json_mode: true,
        note: "OpenAI-compatible endpoint; no special client needed",
    },
    Preset {
        name: "gemini-lite",
        label: "Google Gemini (cheapest)",
        shape: Shape::ChatCompletions,
        base: "https://generativelanguage.googleapis.com/v1beta/openai",
        model: "gemini-2.5-flash-lite",
        key_env: "GEMINI_API_KEY",
        needs_key: true,
        json_mode: true,
        note: "cheapest Gemini text model; plenty for commit messages",
    },
    Preset {
        name: "claude",
        label: "Anthropic Claude",
        shape: Shape::AnthropicMessages,
        base: "https://api.anthropic.com/v1",
        model: "claude-opus-5-5",
        key_env: "ANTHROPIC_API_KEY",
        needs_key: true,
        json_mode: false,
        note: "native Messages API: Anthropic's OpenAI shim ignores response_format",
    },
    Preset {
        name: "openai",
        label: "OpenAI",
        shape: Shape::Responses,
        base: "https://api.openai.com/v1",
        model: "gpt-5.4-mini",
        key_env: "OPENAI_API_KEY",
        needs_key: true,
        json_mode: true,
        note: "Responses API; set shape=\"chat_completions\" for older models",
    },
    Preset {
        name: "zen",
        label: "OpenCode Zen",
        shape: Shape::Responses,
        base: "https://opencode.ai/zen/v1",
        model: "gpt-5.4-mini",
        key_env: "OPENCODE_API_KEY",
        needs_key: true,
        json_mode: true,
        note: "gateway; older Zen models use chat_completions instead",
    },
    Preset {
        name: "nvidia",
        label: "NVIDIA NIM",
        shape: Shape::ChatCompletions,
        base: "https://integrate.api.nvidia.com/v1",
        model: "nvidia/nemotron-3-super-120b-a12b",
        key_env: "NVIDIA_API_KEY",
        needs_key: true,
        json_mode: false,
        note: "hosted catalog: gpt-oss-120b, nemotron-3-*, kimi-k3, glm5.1 ...",
    },
    Preset {
        name: "ollama",
        label: "Ollama (local)",
        shape: Shape::ChatCompletions,
        base: "http://localhost:11434/v1",
        model: "",
        key_env: "OLLAMA_API_KEY",
        needs_key: false,
        json_mode: false,
        note: "free and private; check `ollama list`, then pass --model",
    },
    Preset {
        name: "lmstudio",
        label: "LM Studio (local)",
        shape: Shape::ChatCompletions,
        base: "http://localhost:1234/v1",
        model: "",
        key_env: "LMSTUDIO_API_KEY",
        needs_key: false,
        json_mode: false,
        note: "any OpenAI-compatible local server works the same way",
    },
    Preset {
        name: "mock",
        label: "Mock (tests)",
        shape: Shape::Mock,
        base: "",
        model: "mock",
        key_env: "WYCKOFF_MOCK_RESPONSE",
        needs_key: false,
        json_mode: false,
        note: "returns a canned response and never touches the network",
    },
];

pub fn find(name: &str) -> Option<&'static Preset> {
    let needle = name.trim().to_lowercase();
    PRESETS.iter().find(|preset| preset.name == needle)
}

pub fn names() -> Vec<&'static str> {
    PRESETS.iter().map(|preset| preset.name).collect()
}

#[cfg(test)]
#[path = "presets_tests.rs"]
mod presets_tests;
