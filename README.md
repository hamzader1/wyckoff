# wyckoff

Commit messages written from your staged diff, in your own voice.

Stage your edits, run `wyckoff`, get one line that sounds like you wrote it:

```
$ git add -p
$ wyckoff
Refactor B-Tree page access to use failable cell count retrieval and unified page types
```

No `feat:`. No `fix(parser):`. No template. No 72-character nursery rhyme.
One plain sentence, up to 120 characters, in the imperative, using the real names
of the things you actually changed.

```sh
cargo install --path .        # or: cargo build --release
wyckoff doctor                # checks the repo, config, key and model
wyckoff hook install          # optional: prefill the editor on `git commit`
```

## Why another commit tool

Most of them send the diff to a model and hope. That fails in four specific ways,
and each one has a cheap, deterministic fix:

| Failure | What wyckoff does instead |
|---|---|
| The diff says *what*, never *why* | reads a `.context` file from your repo, plus nested ones next to the code you touched |
| Fifty recent commits drown the signal | measures your style in code, then shows the model **three** examples chosen by path overlap |
| A lockfile bump is 90% of the diff | classifies every path, replaces noise with a one-line effect, budgets the rest |
| Models invent and template | one targeted repair pass, then a deterministic validator |

## How it works

```
git facts ─┬─> [1] PROJECT   .context chain + manifests + repo layout     ── cached
           ├─> [2] STYLE     last N subjects -> measurements + exemplars   ── cached
           ├─> [3] BRANCH    feat|fix|remake/JIRA-123/slug -> weak prior
           └─> [4] DIFF      parse -> classify -> budget -> symbols
                    │
              [5] COMPOSE   one small request, structured JSON out
                    │
              [6] VALIDATE  no prefix, <=120 chars, imperative, no invented paths,
                            no invented vocabulary, no secrets leaving the machine
                    │
                    └── fail? one repair pass, then fix locally what is fixable
```

Layers 1 and 2 are **cached** in `~/.cache/wyckoff/<repo>/`, keyed by content:
the project digest by a stat-signature of the files it read, the style profile by
the log window and the staged paths. Editing `.context` invalidates it
immediately.

The point of these layers is not local CPU — reading git's log takes 5ms. It is
**prompt size**: a curated request of ~3k tokens instead of 40k. Cheaper, faster,
and measurably better output, because the model is not trying to read a lockfile
while guessing what you meant.

## The honest rules

These are in the prompt *and* enforced after the fact:

- the diff is the only source of truth about what changed
- the branch name is a hint; if it disagrees with the diff, the diff wins
- never name a file, symbol or dependency that is not in the diff
- if the reason is not visible, do not invent one: set confidence low and say so
- no filler ("This commit", "Various improvements"), no marketing
- **no type prefixes**, unless your repository really does use them

Prefixes come back only when your own history is at least 60% templated. That is
the smart behaviour: match the house style of the repo you are in, never impose
one. `--style plain` forces a bare sentence; `--style conventional` forces the
opposite.

## Style, not templates

`wyckoff` measures your last commits (40 by default) and sends the *measurements*:
typical length, whether you capitalise, whether you use the imperative, how often
you mention a ticket, which language you write in, and the verbs you reach for
most. It also shows three of your real messages — picked by which of them touched
the same paths as today's change, so the example is about your filing system, not
about whatever landed yesterday.

That is how you get `Remake Btree page implementation to use generic byte access`
instead of `chore: refactor btree module`.

## Providers

Any of these, or your own. `wyckoff models` lists what your endpoint actually
serves; `wyckoff doctor` tells you what is wrong when something is not working.

| name | shape | key env | notes |
|---|---|---|---|
| `gemini` (default) | chat completions | `GEMINI_API_KEY` | OpenAI-compatible endpoint |
| `gemini-lite` | chat completions | `GEMINI_API_KEY` | cheapest Gemini text model |
| `claude` | native messages | `ANTHROPIC_API_KEY` | native API: their OpenAI shim ignores `response_format` |
| `openai` | responses | `OPENAI_API_KEY` | `/v1/responses` |
| `zen` | responses | `OPENCODE_API_KEY` | OpenCode Zen gateway |
| `nvidia` | chat completions | `NVIDIA_API_KEY` | NVIDIA NIM catalog |
| `ollama`, `lmstudio` | chat completions | — (local) | free and private; pick a model with `--model` |
| `mock` | — | — | canned output, used by the tests and `--dry-run` |

So: `export NVIDIA_API_KEY=...` and `wyckoff -p nvidia` works. No login, no config
file, no daemon.

Adding one is config, not code:

```toml
[providers.my-gateway]
shape = "chat_completions"      # chat_completions | responses | anthropic_messages
base  = "https://gateway.internal/v1"
model = "some-model"
key_env = "MY_GATEWAY_KEY"
```

Keys are resolved in this order: `--api-key` → `$KEY_ENV` → `key_cmd` →
`api_key` in config. `key_cmd` keeps secrets out of files entirely:

```toml
[providers.claude]
key_cmd = "security find-generic-password -w -s wyckoff-claude"
```

Keys are never printed, never logged, and headers are never echoed.

## No key, no message

There is no offline mode and no local generator. If wyckoff cannot reach the
provider, or cannot read a message out of what came back, it **exits with the
provider's own error** and prints nothing. A silently invented line is worse than
a clear failure you can fix in ten seconds.

```sh
$ wyckoff
wyckoff: provider `gemini` has no API key.
  set $GEMINI_API_KEY in your shell, or add `key_cmd = "..."` to [providers.gemini] in ~/.config/wyckoff/config.toml.
```

The one exception is the git hook, which must never fail a commit: the editor
opens empty and stderr carries the one-line reason.

Gateways are handled tolerantly on the way in, because they all disagree:
responses wrapped in an envelope (`{"data": {"choices": [...]}}`), the answer
fenced in ```` ```json ````, a separate `reasoning` field — all read correctly.

## The `.context` file

The diff cannot tell you why. This can:

```sh
wyckoff init                # drafts one from your repo
```

```markdown
# wyckoff

Commit messages written from the staged diff, in your own voice.

## Architecture

How the pieces fit: the main modules, the data flow, where the tricky parts live.

## Conventions

Rules a newcomer would get wrong: error handling, naming, where tests go.

## Domain vocabulary

Names that mean something specific here, and the word you would use in a commit.

## Current work

What is being built or refactored right now.
```

It is also read from `.context/` directories and from any `.context` next to the
files you changed, so a monorepo can have one per package. If you already have
`AGENTS.md` or `CLAUDE.md`, that is used as a fallback.

## Interactive by default

Run `wyckoff` with a terminal attached and it reports what it did, then asks:

```
◇  📁  Detected 3 staged files
      src/backend/executor/filter.rs
      src/backend/executor/scan_guard.rs
      src/backend/executor/tablescan.rs
│
◇  ✅  Changes analyzed in 7.8s
      cline · minimax/minimax-m2.5 · ~3k tokens
│

  Rename scan modes to Safe/Unsafe, add Filter::into_child and rename predicate fields

? Use this commit message? ›
❯ Yes, commit this
  No, abort
  Edit in $EDITOR
  Retry, ask again
```

- **Yes** commits with it (your `extra_commit_args` and `--amend` still apply)
- **No** aborts; nothing is committed
- **Edit** opens `$VISUAL`/`$EDITOR` (vim when unset) on the message; your text is
  then used as-is, without further review
- **Retry** asks the model again, *telling it which message you rejected*, so it
  writes something different instead of repeating itself

Nothing decorative appears when nobody is watching: if stdout is not a terminal
(a pipe, a script, a test, the git hook), wyckoff prints the message and exits.
`--yes`, `wyckoff commit`, `--json` and `-q` all skip the question too.

## Telling it *why*: `-I "..."`

The diff contains *what* changed, never *why*. Without intent the model is only
allowed to describe what it can see. `-I` (long form `--intent`) drops your own
words into the request as a separate, authoritative section:

```
$ wyckoff -p gemini --dry-run -I "unify page types so the reader stops guessing"
...
## The author's own words about this change
unify page types so the reader stops guessing
(prefer this over anything you infer)

## Staged change
...
```

It is not a template and not a summary of the diff — it is the sentence you would
say to a colleague. One clause is enough, and it is never stored anywhere:

```sh
wyckoff -I "wallet sync was double counting on retry"
wyckoff -I "renaming src/btree to src/page"
wyckoff -I "the free list walker could skip a page"
```

How it interacts with the honesty rules: your words win for *why*, the diff still
wins for *what actually changed*, and neither lets the model mention code that is
not in the diff.

## Commands

```
wyckoff [OPTIONS] [-- <PATHS>...]     generate and print           (the normal one)
wyckoff commit [OPTIONS]              generate, then git commit
wyckoff explain [OPTIONS]             prose description instead of a message
wyckoff init [--print] [--path <p>] [--force]      draft a .context
wyckoff models [-p NAME] [-m ID]      what the provider actually serves
wyckoff doctor [-p NAME] [-m ID]      what is misconfigured (exit 1 if it is)
wyckoff config path|show|init [--force]
wyckoff cache stat|dir|clean
wyckoff hook install [--force] | uninstall | status
```

Options for `wyckoff`, `wyckoff commit` and `wyckoff explain`:

| flag | what it does |
|---|---|
| `-p, --provider <NAME>` | provider to use (gemini, gemini-lite, claude, openai, zen, nvidia, ollama, lmstudio, mock, or one from your config) |
| `-m, --model <ID>` | model id, overriding the provider default |
| `-I, --intent <TEXT>` | your own words about why (see above) |
| `--style <auto\|plain\|conventional>` | auto follows your history; plain never uses prefixes; conventional always does |
| `--language <auto\|en\|ar\|fr\|...>` | language of the message |
| `-b, --body` | allow a short bullet body (single line by default) |
| `--max-len <N>` | subject length cap (default 120) |
| `--commit` | commit with the message (same as the `commit` subcommand) |
| `--amend` | amend the previous commit (implies `--commit`) |
| `-y, --yes` | commit without asking (skips the review prompt) |
| `--timeout <SECS>` | HTTP timeout for the provider call (default 120) |
| `--dry-run` | print the exact request that would be sent, send nothing |
| `--json` | machine-readable output |
| `--copy` | also copy the message to the clipboard |
| `--stats` | provider, token and cache details on stderr |
| `-v, --verbose` | notes, confidence and repair details |
| `-q, --quiet` | print nothing (used by the hook) |
| `--unstaged` | use the working tree (`git diff`) instead of the index |
| `--all` | everything since HEAD (`git diff HEAD`) |
| `--context-file <PATH>` | extra `.context`-style file to read (e.g. `README.md`) |
| `--exclude <GLOB>` | never send paths matching this (repeatable, glob-lite) |
| `--no-cache` | neither read nor write the cache |
| `--refresh` | recompute cached values |
| `--allow-secrets` | send the diff even if it looks like it contains secrets |
| `--config <PATH>` | config file to use instead of the default location |
| `-- <PATHS>...` | limit the diff to these paths |

`models` and `doctor` take `-p`, `-m`, `--config` and `-v`.

## Secrets

Because this tool's job is to send your diff somewhere, it checks first. Known
key formats (AWS, GitHub, Slack, NVIDIA, OpenAI, Anthropic, Google, GitLab,
private key blocks) and real-looking assignments are detected in **added lines
only**, and the run stops before anything leaves the machine. Placeholders like
`your_key_here` and `.env.example` are ignored. Override with `--allow-secrets`.

Only the kind of secret and the file are ever printed — never the value.

## Configuration

`~/.config/wyckoff/config.toml` (or `$WYCKOFF_CONFIG`). Everything is optional;
`wyckoff config init` writes a commented starter. Highlights:

```toml
default_provider = "gemini"
style            = "auto"      # auto | plain | conventional
language         = "auto"      # auto | en | ar | fr ...
history_window   = 40
exemplars        = 3
max_subject_len  = 120
include_body     = false
max_diff_tokens  = 24000
skip             = ["*.lock", "*.min.js", "dist/*", "*.snap"]
prompt_extra     = "..."       # appended to the system prompt
extra_commit_args = ["--signoff"]
```

Env overrides: `WYCKOFF_PROVIDER`, `WYCKOFF_CONFIG`, `WYCKOFF_CACHE`,
`WYCKOFF_NO_CACHE`, `WYCKOFF_API_KEY`.

## The hook

```sh
wyckoff hook install
```

Installs `prepare-commit-msg`, which git calls before opening the editor. It runs
only when you did not supply a message yourself, and it steps aside for merges,
rebases, squashes, cherry-picks and `-m`. Clear the message to write your own;
the hook never fails a commit.

(`wyckoff hook-fill <file>` is what the hook itself calls. You never need it by
hand, but it is what to run if you want to debug the hook path.)

## Speed

wyckoff's own work is milliseconds; the wait is the model. Measured on this
machine with 17 staged files:

```
wyckoff --version          3.1 ms
git diff --cached --raw    ~10 ms
git log -n40 --name-only   ~15 ms
```

So when a run takes a minute, that minute is the provider thinking. The spinner
shows the elapsed time while you wait, and `--stats` prints it afterwards as
`analysis: 63.4s`. In order of effect:

1. **Use a model that does not reason, for a one-line message.** This is the big
   one: `-m gemini-2.5-flash-lite`, `-m gpt-5.4-nano`, `-m openai/gpt-oss-20b`.
   Reasoning models spend most of their time in a `reasoning` field you never see.
2. **Turn the thinking down** where the gateway supports it:
   ```toml
   [providers.cline.params]
   reasoning_effort = "low"
   ```
3. **Cap the wait:** `--timeout 30` instead of sitting on the 120s default.
4. **Keep the cache warm:** the first run of a session pays for the project and
   style layers; afterwards you get `cache: 2 hit(s)` and a smaller prompt.

## Limitations, honestly

- **Intent is not in the diff.** `.context`, the branch name and `-I "..."` are
  the only ways to tell it why. Without them you get what the diff shows.
- **Small local models are worse at this** than frontier models. Ollama works; it
  will sometimes write something bland.
- **The style profile is a heuristic.** If your history is inconsistent, so is its
  idea of your voice; `--style plain` is the escape hatch.
- **Model ids move.** Nothing is hardcoded beyond a default: `wyckoff models`
  asks your endpoint, and config wins.
- **A failed request fails the run.** There is no invented fallback: if the
  provider is down you get an error, not a plausible-sounding line.
- **First run costs more.** Cache misses are the expensive case; steady state is
  one small call per commit.

## Development

```sh
cargo test                     # 206 tests (181 unit + 25 end to end), no network
cargo clippy --all-targets     # clean
cargo fmt --check
```

The suite includes end-to-end tests that create real git repositories, stage real
changes and run the real binary against the mock provider.

MIT.



# 
