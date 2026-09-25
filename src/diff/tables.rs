//! Path pattern tables for [`super::classify`].
//!
//! Pure data on purpose: keeping the lists here makes the classification
//! logic readable and makes it obvious where to add a language or a tool.

pub const LOCKFILES: &[&str] = &[
    "cargo.lock",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "poetry.lock",
    "pipfile.lock",
    "uv.lock",
    "pdm.lock",
    "go.sum",
    "composer.lock",
    "gemfile.lock",
    "flake.lock",
    "mix.lock",
    "packages.lock.json",
];

pub const ASSET_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "icns", "tiff", "svg", "pdf", "woff",
    "woff2", "ttf", "otf", "eot", "mp3", "wav", "ogg", "flac", "m4a", "mp4", "mov", "webm", "avi",
    "zip", "gz", "xz", "zst", "bz2", "7z", "tar", "rar", "so", "dylib", "dll", "rlib", "wasm",
    "db", "sqlite", "sqlite3", "class", "jar", "apk", "ipa", "keystore", "p12", "pfx", "der",
    "pem", "key", "crt", "cer",
];

pub const SOURCE_EXTS: &[&str] = &[
    "rs", "py", "pyi", "ts", "tsx", "js", "jsx", "mjs", "cjs", "go", "java", "kt", "kts", "c", "h",
    "cc", "cpp", "cxx", "hpp", "hh", "cs", "rb", "php", "swift", "m", "mm", "sh", "bash", "zsh",
    "fish", "ps1", "sql", "lua", "zig", "ex", "exs", "erl", "hrl", "scala", "sc", "dart", "vue",
    "svelte", "astro", "html", "htm", "css", "scss", "sass", "less", "proto", "graphql", "gql",
    "tf", "nix", "vim", "el", "clj", "cljs", "r", "jl", "hs", "ml", "fs", "vb", "asm", "s",
];

pub const DOC_EXTS: &[&str] = &["md", "mdx", "rst", "adoc", "txt", "textile", "org"];

pub const CONFIG_EXTS: &[&str] = &[
    "toml",
    "yaml",
    "yml",
    "json",
    "json5",
    "jsonc",
    "ini",
    "cfg",
    "conf",
    "properties",
    "env",
    "plist",
];

pub const GENERATED_DIRS: &[&str] = &[
    "target/",
    "dist/",
    "build/",
    "out/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    ".turbo/",
    ".parcel-cache/",
    ".terraform/",
    "coverage/",
    "__pycache__/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".gradle/",
    ".venv/",
    "venv/",
    "generated/",
    "gen/",
];

pub const VENDOR_DIRS: &[&str] = &[
    "vendor/",
    "third_party/",
    "node_modules/",
    "deps/",
    "subprojects/",
];

pub const CI_DIRS: &[&str] = &[
    ".github/",
    ".gitlab/",
    ".circleci/",
    ".buildkite/",
    ".woodpecker/",
];

pub const CI_FILES: &[&str] = &[
    ".gitlab-ci.yml",
    "jenkinsfile",
    "azure-pipelines.yml",
    ".travis.yml",
    "bitbucket-pipelines.yml",
    "appveyor.yml",
];

pub const CONFIG_FILES: &[&str] = &[
    "dockerfile",
    "makefile",
    "justfile",
    "procfile",
    "rakefile",
    "gemfile",
    "brewfile",
    ".editorconfig",
    ".gitignore",
    ".gitattributes",
    ".dockerignore",
    ".npmrc",
    ".nvmrc",
    ".tool-versions",
    ".rustfmt.toml",
    "rust-toolchain.toml",
    "clippy.toml",
    "deny.toml",
    "codecov.yml",
];

pub const DOC_FILES: &[&str] = &[
    "readme",
    "license",
    "licence",
    "changelog",
    "contributing",
    "authors",
    "notice",
    "security",
    "code_of_conduct",
];

pub const DOC_DIRS: &[&str] = &["docs/", "doc/"];

pub const TEST_DIRS: &[&str] = &[
    "tests/",
    "test/",
    "__tests__/",
    "spec/",
    "specs/",
    "benches/",
];
