use std::fmt::Display;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not inside a git repository (or `git` is not on PATH)")]
    NotARepo,

    #[error("nothing is staged. stage your work first (`git add -p`), then run wyckoff again")]
    NothingStaged,

    #[error("`git {cmd}` failed: {stderr}")]
    Git { cmd: String, stderr: String },

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{0}")]
    Plain(String),

    #[error("config {path}: {msg}")]
    Config { path: PathBuf, msg: String },

    #[error("unknown provider `{name}`. known providers: {known}")]
    UnknownProvider { name: String, known: String },

    #[error(
        "provider `{name}` has no API key.\n  set ${env} in your shell, or add `key_cmd = \"...\"` to [providers.{name}] in {config}."
    )]
    NoKey {
        name: String,
        env: String,
        config: String,
    },

    #[error("provider `{name}` needs `{field}` in config")]
    ProviderField { name: String, field: &'static str },

    #[error("HTTP {status} from `{provider}`: {body}")]
    ProviderHttp {
        provider: String,
        status: u16,
        body: String,
    },

    #[error("could not reach {url}: {msg}")]
    Http { url: String, msg: String },

    #[error("could not read a commit message out of the model output. raw output was:\n{0}")]
    ModelOutput(String),

    #[error(
        "the staged content looks like it contains secrets:\n{0}\n\
         nothing was sent anywhere. unstage it, or re-run with --allow-secrets if you know what you are doing."
    )]
    Secrets(String),

    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }

    pub fn msg(text: impl Display) -> Self {
        Error::Plain(text.to_string())
    }
}
