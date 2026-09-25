//! Wiring the CLI to the pipeline, and turning an outcome into output.

mod hook_runner;

use crate::cli::{self, Command, GenerateArgs};
use crate::config::{Config, Settings};
use crate::error::{Error, Result};
use crate::generate::{self, Mode, Options};
use crate::git::Git;
use crate::{init, output, paths};

pub use hook_runner::hook_fill;

pub fn dispatch(cli: cli::Cli) -> Result<i32> {
    match cli.command {
        Some(Command::Hook { action }) => {
            crate::commands::hook_command(&action)?;
            Ok(0)
        }
        Some(Command::HookFill {
            file,
            config,
            verbose,
        }) => hook_fill(&file, config.as_deref(), verbose),
        Some(Command::Init(args)) => init_command(&args),
        Some(Command::Models(args)) => {
            crate::commands::models(&args)?;
            Ok(0)
        }
        Some(Command::Doctor(args)) => {
            crate::commands::doctor(&args)?;
            Ok(0)
        }
        Some(Command::Config { action }) => {
            crate::commands::config_command(&action)?;
            Ok(0)
        }
        Some(Command::Cache { action }) => {
            crate::commands::cache_command(&action)?;
            Ok(0)
        }
        Some(Command::Commit(args)) => generate_command(&args, true, Mode::Message),
        Some(Command::Explain(args)) => generate_command(&args, false, Mode::Explain),
        None => {
            let commit = cli.generate.commit || cli.generate.amend;
            generate_command(&cli.generate, commit, Mode::Message)
        }
    }
}

fn generate_command(args: &GenerateArgs, commit: bool, mode: Mode) -> Result<i32> {
    let (config, _) = Config::load(args.config.as_deref())?;
    let settings = Settings::from_config(&config);
    let mut options = Options::from_args(args, &settings);
    options.mode = mode;

    let git = Git::discover()?;
    let outcome = generate::run(&git, &config, &options)?;

    // --dry-run: show the request and send nothing.
    if let Some(request) = &outcome.request {
        println!("=== system ===\n{}\n", request.system);
        println!("=== user ===\n{}\n", request.user);
        eprintln!("dry run: nothing was sent (provider {})", options.provider);
        return Ok(0);
    }

    if mode == Mode::Explain {
        if args.json {
            println!("{}", serde_json::to_string_pretty(&outcome.text)?);
        } else {
            output::print_message(&outcome.text, args.quiet);
        }
        if args.stats {
            output::print_meta(&outcome.meta);
        }
        return Ok(0);
    }

    if args.json {
        println!("{}", output::json(&outcome.msg, &outcome.meta)?);
    } else {
        output::print_message(&outcome.text, args.quiet);
    }

    output::print_notes(&outcome.msg, &outcome.issues, args.verbose);

    if outcome.repaired && args.verbose {
        eprintln!("note: the first answer broke a rule and was rewritten once");
    }
    if outcome.truncated {
        eprintln!("note: the model stopped at its output limit, so the message may be short");
    }
    if args.stats {
        output::print_meta(&outcome.meta);
    }
    if args.copy {
        output::copy_to_clipboard(&outcome.text)?;
        if !args.quiet {
            eprintln!("copied to the clipboard");
        }
    }

    if !commit {
        return Ok(0);
    }

    // Never commit a message the validator cannot stand behind: an empty or
    // broken subject is worse than asking the human to try once more.
    let blocking = crate::validate::blocking(&outcome.issues);
    if outcome.msg.is_empty() || !blocking.is_empty() {
        let reasons = if blocking.is_empty() {
            "the subject came back empty".to_string()
        } else {
            blocking.join("\n  ")
        };
        return Err(Error::msg(format!(
            "not committing: the message did not pass the checks\n  {reasons}\n\
             run `wyckoff` to see it, or `wyckoff -I \"what you were doing\"` for a better one"
        )));
    }

    let mut extra = settings.extra_commit_args.clone();
    if args.amend {
        extra.push("--amend".to_string());
    }
    output::commit(&git, &outcome.text, &extra)?;
    Ok(0)
}

fn init_command(args: &cli::InitArgs) -> Result<i32> {
    let git = Git::discover()?;
    let text = init::draft(&git)?;

    if args.print {
        println!("{text}");
        return Ok(0);
    }

    let path = args
        .path
        .clone()
        .unwrap_or_else(|| git.root().join(".context"));
    init::write(&path, &text, args.force)?;
    println!("wrote {}", paths::display(&path));
    println!(
        "the Architecture, Conventions and Current work sections are what make the \
         messages good — give them two minutes"
    );
    Ok(0)
}
