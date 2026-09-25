//! Tests for the command line surface.

use super::*;
use clap::CommandFactory;

#[test]
fn cli_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn bare_invocation_means_generate() {
    let cli = Cli::parse_from(["wyckoff"]);
    assert!(cli.command.is_none());
    assert!(cli.generate.provider.is_none());
    assert!(!cli.generate.commit);
}

#[test]
fn flags_parse_on_the_default_command() {
    let cli = Cli::parse_from([
        "wyckoff",
        "-p",
        "nvidia",
        "--intent",
        "fix the race",
        "--body",
        "--stats",
    ]);
    assert_eq!(cli.generate.provider.as_deref(), Some("nvidia"));
    assert_eq!(cli.generate.intent.as_deref(), Some("fix the race"));
    assert!(cli.generate.body);
    assert!(cli.generate.stats);
}

#[test]
fn commit_subcommand_and_pathspecs_parse() {
    let cli = Cli::parse_from(["wyckoff", "commit", "--amend", "--", "src/diff"]);
    match cli.command {
        Some(Command::Commit(args)) => {
            assert!(args.amend);
            assert_eq!(args.paths, vec!["src/diff"]);
        }
        other => panic!("expected commit, got {other:?}"),
    }
}

#[test]
fn nested_actions_parse() {
    let cli = Cli::parse_from(["wyckoff", "hook", "install", "--force"]);
    assert!(matches!(
        cli.command,
        Some(Command::Hook {
            action: HookAction::Install { force: true }
        })
    ));

    let cli = Cli::parse_from(["wyckoff", "cache", "clean"]);
    assert!(matches!(
        cli.command,
        Some(Command::Cache {
            action: CacheAction::Clean
        })
    ));

    let cli = Cli::parse_from(["wyckoff", "config", "path"]);
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            action: ConfigAction::Path
        })
    ));

    let cli = Cli::parse_from(["wyckoff", "models", "-p", "ollama"]);
    match cli.command {
        Some(Command::Models(args)) => assert_eq!(args.provider.as_deref(), Some("ollama")),
        other => panic!("expected models, got {other:?}"),
    }
}

#[test]
fn hook_fill_is_hidden_but_usable() {
    let cli = Cli::parse_from(["wyckoff", "hook-fill", "/tmp/msg"]);
    assert!(matches!(
        cli.command,
        Some(Command::HookFill { file, .. }) if file == *"/tmp/msg"
    ));
}

#[test]
fn style_choices_are_constrained() {
    let cli = Cli::parse_from(["wyckoff", "--style", "plain"]);
    assert_eq!(cli.generate.style, Some(StyleChoice::Plain));
    assert!(Cli::try_parse_from(["wyckoff", "--style", "nonsense"]).is_err());
    assert_eq!(StyleChoice::Conventional.as_str(), "conventional");
}

#[test]
fn diff_source_flags_are_exclusive_in_intent() {
    let cli = Cli::parse_from(["wyckoff", "--all"]);
    assert!(cli.generate.everything);
    let cli = Cli::parse_from(["wyckoff", "--unstaged"]);
    assert!(cli.generate.unstaged);
}
