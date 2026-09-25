//! `wyckoff doctor` — is everything wired up?
//!
//! Checks the repo, the config, the key and the model, and returns an error
//! when something needs attention so it can be used in scripts.

use crate::cli::ProviderArgs;
use crate::config::{Config, Settings};
use crate::error::{Error, Result};
use crate::git::Git;
use crate::paths;

pub fn doctor(args: &ProviderArgs) -> Result<()> {
    let mut problems: Vec<String> = Vec::new();

    println!("repo");
    match Git::discover() {
        Ok(git) => {
            println!(
                "  ok   {} (.git in {})",
                git.root().display(),
                paths::display(git.git_dir())
            );
            match git.branch() {
                Ok(branch) => println!("  ok   branch {branch}"),
                Err(error) => println!("  warn could not read the branch: {error}"),
            }
            match git.has_staged() {
                Ok(true) => println!("  ok   something is staged"),
                Ok(false) => println!("  warn nothing is staged (stage work to try a real run)"),
                Err(error) => problems.push(format!("could not read the index: {error}")),
            }
            println!(
                "  ok   {}",
                git.run(&["--version"]).unwrap_or_default().trim()
            );
            let context = crate::context::files::discover(git.root(), &[], 6000);
            if context.is_empty() {
                println!("  warn no .context found — `wyckoff init` drafts one");
            } else {
                let names: Vec<&str> = context.iter().map(|file| file.path.as_str()).collect();
                println!("  ok   context: {}", names.join(", "));
            }
        }
        Err(_) => {
            println!("  warn not inside a git repository");
            problems.push("not inside a git repository".into());
        }
    }

    println!("\nconfig");
    let (config, config_path) = Config::load(args.config.as_deref())?;
    if config_path.exists() {
        println!("  ok   {}", paths::display(&config_path));
    } else {
        println!(
            "  warn {} does not exist yet (`wyckoff config init` writes one)",
            paths::display(&config_path)
        );
    }
    let settings = Settings::from_config(&config);
    println!(
        "  ok   default provider {}, style {}",
        settings.default_provider, settings.style
    );

    println!("\nprovider");
    let name = args
        .provider
        .clone()
        .unwrap_or_else(|| settings.default_provider.clone());
    match config.resolved(&name, args.model.as_deref()) {
        Ok(provider) => {
            println!("  ok   {}", provider.describe());
            println!("  ok   endpoint {}", provider.chat_url());
            match crate::llm::build(&provider).list_models() {
                Ok(models) if models.iter().any(|model| model == &provider.model) => {
                    println!("  ok   model `{}` is available", provider.model);
                    // Some gateways serve the catalogue without auth, so a
                    // passing check here does not prove the key itself is valid.
                    println!(
                        "  note the model list can be public; the key is only proven by a real run (`wyckoff -p {}`)",
                        provider.name
                    );
                }
                Ok(models) => println!(
                    "  warn model `{}` is not among the {} the endpoint lists — run `wyckoff models`",
                    provider.model,
                    models.len()
                ),
                Err(error) => {
                    println!("  warn could not list models: {error}");
                    println!(
                        "       (key problem, endpoint problem, or the server is not running)"
                    );
                }
            }
        }
        Err(error) => {
            println!("  fail {error}");
            problems.push(error.to_string());
        }
    }

    println!("\ntips");
    println!(
        "  · set keys with an env var, or `key_cmd` (e.g. `security find-generic-password -w -s wyckoff-claude`)"
    );
    println!(
        "  · if the provider call fails, wyckoff exits with the provider's own error: it never invents a message"
    );

    if problems.is_empty() {
        println!("\nall checks passed");
        Ok(())
    } else {
        println!("\n{} problem(s) found", problems.len());
        Err(Error::msg("doctor found problems (see above)"))
    }
}
