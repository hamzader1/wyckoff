//! Subcommands and the hook, end to end.

mod common;

use common::*;

#[test]
fn hook_install_status_and_uninstall() {
    let repo = seeded_repo();

    let installed = repo.wyckoff(&["hook", "install"]);
    assert!(installed.status.success(), "{}", stderr(&installed));
    let hook = repo.read(".git/hooks/prepare-commit-msg");
    assert!(hook.contains("wyckoff"), "{hook}");
    assert!(hook.contains("case \"$2\""), "{hook}");

    let status = repo.wyckoff(&["hook", "status"]);
    assert!(
        stdout(&status).contains("installed yes"),
        "{}",
        stdout(&status)
    );

    let removed = repo.wyckoff(&["hook", "uninstall"]);
    assert!(removed.status.success());
    let status = repo.wyckoff(&["hook", "status"]);
    assert!(
        stdout(&status).contains("installed no"),
        "{}",
        stdout(&status)
    );
}

#[test]
fn hook_fill_writes_a_message_only_when_the_human_said_nothing() {
    let repo = seeded_repo();

    let empty = repo.path().join("msg-empty");
    std::fs::write(&empty, "# Please enter the commit message\n#\n").unwrap();
    let output = repo.wyckoff(&["hook-fill", empty.to_str().unwrap()]);
    assert!(output.status.success(), "the hook must never fail a commit");
    let written = std::fs::read_to_string(&empty).unwrap();
    assert!(!written.trim().is_empty(), "expected a generated message");
    assert!(!written.contains("Please enter"), "{written}");

    let typed = repo.path().join("msg-typed");
    std::fs::write(&typed, "Fix the thing by hand\n").unwrap();
    let output = repo.wyckoff(&["hook-fill", typed.to_str().unwrap()]);
    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(&typed).unwrap(),
        "Fix the thing by hand\n",
        "a message the human wrote must be left alone"
    );
}

#[test]
fn commit_subcommand_commits_the_generated_message() {
    let repo = seeded_repo();
    let output = repo.wyckoff(&["commit", "-p", "mock"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let subject = repo.git(&["log", "-1", "--format=%s"]);
    let subject = subject.trim();
    assert!(!subject.is_empty());
    assert!(!subject.contains("feat:"), "{subject}");
    assert!(subject.chars().count() <= 120, "{subject}");
}

#[test]
fn init_drafts_a_context_file() {
    let repo = Repo::new();
    let printed = repo.wyckoff(&["init", "--print"]);
    assert!(printed.status.success(), "{}", stderr(&printed));
    let text = stdout(&printed);
    assert!(text.contains("## Architecture"), "{text}");
    assert!(text.contains("## Current work"), "{text}");
}

#[test]
fn models_and_doctor_run_against_the_mock_provider() {
    let repo = seeded_repo();

    let models = repo.wyckoff(&["models", "-p", "mock"]);
    assert!(models.status.success(), "{}", stderr(&models));
    assert!(stdout(&models).contains("mock"), "{}", stdout(&models));

    let doctor = repo.wyckoff(&["doctor", "-p", "mock"]);
    let report = format!("{}{}", stdout(&doctor), stderr(&doctor));
    assert!(report.contains("repo"), "{report}");
    assert!(report.contains("provider"), "{report}");
    assert!(
        doctor.status.success(),
        "doctor should pass in a seeded repo: {report}"
    );
}

#[test]
fn a_branch_hint_is_passed_through_as_a_weak_prior() {
    let repo = seeded_repo();
    repo.git(&[
        "checkout",
        "-q",
        "-b",
        "remake/PROJ-4512/btree-generic-bytes",
    ]);

    let output = repo.wyckoff(&["-p", "mock", "--dry-run"]);
    let text = stdout(&output);
    assert!(
        text.contains("remake/PROJ-4512/btree-generic-bytes"),
        "{text}"
    );
    assert!(text.contains("author's own label: remake"), "{text}");
    assert!(text.contains("ticket: PROJ-4512"), "{text}");
    assert!(text.contains("weak hint only"), "{text}");
}

#[test]
fn a_repository_without_history_still_works() {
    let repo = Repo::new();
    repo.write("first.rs", "pub fn hello() -> u8 {\n    1\n}\n");
    repo.git(&["add", "-A"]);

    let output = repo.wyckoff(&["-p", "mock", "--stats"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("0 commits sampled"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn explain_mode_prints_prose_without_validation() {
    let repo = seeded_repo();
    let output = repo.wyckoff(&["explain", "-p", "mock"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!stdout(&output).trim().is_empty());
}

#[test]
fn pathspecs_narrow_the_diff() {
    let repo = seeded_repo();
    repo.write("docs/notes.md", "notes\n");
    repo.git(&["add", "-A"]);

    let output = repo.wyckoff(&["-p", "mock", "--dry-run", "--", "docs"]);
    let text = stdout(&output);
    assert!(text.contains("docs/notes.md"), "{text}");
    assert!(
        !text.contains("src/btree/page.rs"),
        "the pathspec should have excluded it: {text}"
    );
}
