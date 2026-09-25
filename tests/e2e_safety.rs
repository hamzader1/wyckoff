//! The safety and caching promises, end to end.

mod common;

use common::*;

#[test]
fn secrets_stop_the_run_before_anything_is_sent() {
    let repo = seeded_repo();
    repo.write(
        "src/leak.rs",
        "pub const KEY: &str = \"AKIAIOSFODNN7EXAMPLE\";\n",
    );
    repo.git(&["add", "-A"]);

    let output = repo.wyckoff(&["-p", "mock"]);
    assert!(!output.status.success());
    let diagnostics = stderr(&output);
    assert!(diagnostics.contains("secrets"), "{diagnostics}");
    assert!(diagnostics.contains("src/leak.rs"), "{diagnostics}");
    assert!(
        !diagnostics.contains("AKIAIOSFODNN7EXAMPLE"),
        "the value itself must never be printed: {diagnostics}"
    );

    // The deliberate override still works.
    let output = repo.wyckoff(&["-p", "mock", "--allow-secrets"]);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn env_files_with_real_values_are_caught() {
    let repo = seeded_repo();
    repo.write(".env", "OPENAI_API_KEY=abcdef123456789\n");
    repo.git(&["add", "-A"]);

    let output = repo.wyckoff(&["-p", "mock"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains(".env"), "{}", stderr(&output));
}

#[test]
fn env_example_files_are_not_caught() {
    let repo = seeded_repo();
    repo.write(".env.example", "OPENAI_API_KEY=your_key_here\n");
    repo.git(&["add", "-A"]);

    let output = repo.wyckoff(&["-p", "mock"]);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn the_cache_is_reused_on_the_second_run() {
    let repo = seeded_repo();
    let first = repo.wyckoff(&["-p", "mock", "--stats"]);
    assert!(first.status.success(), "{}", stderr(&first));

    let second = repo.wyckoff(&["-p", "mock", "--stats"]);
    assert!(second.status.success(), "{}", stderr(&second));

    let diagnostics = stderr(&second);
    let hits: usize = diagnostics
        .lines()
        .find_map(|line| line.strip_prefix("cache: "))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse().ok())
        .unwrap_or(0);
    assert!(hits >= 1, "expected a cache hit, got:\n{diagnostics}");
}

#[test]
fn editing_context_invalidates_the_project_cache() {
    let repo = seeded_repo();
    let first = repo.wyckoff(&["-p", "mock", "--dry-run"]);
    assert!(stdout(&first).contains("A tiny btree demo"));

    repo.write(
        ".context",
        "# demo\n\nA tiny btree demo, now with a much longer description.\n",
    );
    repo.git(&["add", "-A"]);

    let second = repo.wyckoff(&["-p", "mock", "--dry-run"]);
    assert!(
        stdout(&second).contains("now with a much longer description"),
        "{}",
        stdout(&second)
    );
}

#[test]
fn cache_commands_report_and_clear() {
    let repo = seeded_repo();
    let run = repo.wyckoff(&["-p", "mock"]);
    assert!(run.status.success());

    let stat = repo.wyckoff(&["cache", "stat"]);
    assert!(stat.status.success());
    assert!(stdout(&stat).contains("context"), "{}", stdout(&stat));

    let dir = repo.wyckoff(&["cache", "dir"]);
    assert!(stdout(&dir).contains("wyckoff-cache"), "{}", stdout(&dir));

    let clean = repo.wyckoff(&["cache", "clean"]);
    assert!(clean.status.success());
    let stat = repo.wyckoff(&["cache", "stat"]);
    assert!(
        stdout(&stat).contains("nothing cached"),
        "{}",
        stdout(&stat)
    );
}

#[test]
fn symlinked_and_nested_context_are_both_read() {
    let repo = seeded_repo();
    repo.write("src/btree/.context", "Page code lives here.\n");
    repo.git(&["add", "-A"]);

    let output = repo.wyckoff(&["-p", "mock", "--dry-run"]);
    let text = stdout(&output);
    assert!(text.contains("A tiny btree demo"), "{text}");
    assert!(text.contains("Page code lives here"), "{text}");
}

#[test]
fn config_init_writes_a_usable_starter() {
    let repo = seeded_repo();
    let output = repo.wyckoff(&["config", "init"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let written = repo.read("wyckoff.toml");
    assert!(written.contains("default_provider"), "{written}");
    assert!(written.contains("[providers.gemini]"), "{written}");

    // And it parses: `config show` reads it back.
    let show = repo.wyckoff(&["config", "show"]);
    assert!(show.status.success(), "{}", stderr(&show));
    assert!(
        stdout(&show).contains("default provider"),
        "{}",
        stdout(&show)
    );
}
