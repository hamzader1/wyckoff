//! The smaller subcommands.

pub mod doctor;

pub use doctor::doctor;

use crate::cli::{CacheAction, ConfigAction, HookAction, ProviderArgs};
use crate::config::{self, Config, Settings};
use crate::error::Result;
use crate::git::Git;
use crate::paths;

/// `wyckoff models` — ask the provider what it actually serves.
pub fn models(args: &ProviderArgs) -> Result<()> {
    let (config, _) = Config::load(args.config.as_deref())?;
    let settings = Settings::from_config(&config);
    let name = args
        .provider
        .clone()
        .unwrap_or_else(|| settings.default_provider.clone());
    let provider = config.resolved(&name, args.model.as_deref())?;
    let client = crate::llm::build(&provider);

    println!("{} — {}", provider.label, provider.models_url());
    if let Some(preset) = crate::config::presets::find(&provider.name) {
        println!("({})", preset.note);
    }
    let names = client.list_models()?;
    if names.is_empty() {
        println!("(the endpoint returned no models)");
        return Ok(());
    }
    for model in &names {
        let marker = if model == &provider.model { "*" } else { " " };
        println!("{marker} {model}");
    }
    println!(
        "\n{} model(s); `*` is the one wyckoff would use.",
        names.len()
    );
    Ok(())
}

pub fn config_command(action: &ConfigAction) -> Result<()> {
    let (config, path) = Config::load(None)?;
    match action {
        ConfigAction::Path => {
            println!("{}", path.display());
            if !path.exists() {
                println!("(does not exist yet — `wyckoff config init` writes a starter)");
            }
            Ok(())
        }
        ConfigAction::Init { force } => {
            config::write_starter(&path, *force)?;
            println!("wrote {}", paths::display(&path));
            Ok(())
        }
        ConfigAction::Show => {
            let settings = Settings::from_config(&config);
            println!("config file      {}", paths::display(&path));
            println!("default provider {}", settings.default_provider);
            println!("style            {}", settings.style);
            println!("language         {}", settings.language);
            println!("history window   {}", settings.history_window);
            println!("exemplars        {}", settings.exemplars);
            println!("max subject      {}", settings.max_subject_len);
            println!("include body     {}", settings.include_body);
            println!("max diff tokens  {}", settings.max_diff_tokens);
            println!("repair           {}", settings.repair);
            println!("diff context     {}", settings.diff_context_lines);
            println!("cache            {}", settings.cache);
            println!("skip patterns    {}", settings.skip.join(", "));
            println!("providers        {}", config.known_providers().join(", "));
            Ok(())
        }
    }
}

pub fn cache_command(action: &CacheAction) -> Result<()> {
    let repo = Git::discover()
        .map(|git| git.root().display().to_string())
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_default()
                .display()
                .to_string()
        });
    let cache = crate::cache::Cache::new(&repo, true);

    match action {
        CacheAction::Dir => {
            println!("{}", cache.root().display());
            println!("this repo: {}", cache.repo_dir().display());
            Ok(())
        }
        CacheAction::Stat => {
            let stats = cache.stats();
            if stats.is_empty() {
                println!("nothing cached for {repo}");
                return Ok(());
            }
            let (mut total_entries, mut total_bytes) = (0, 0);
            for (namespace, entries, bytes) in stats {
                total_entries += entries;
                total_bytes += bytes;
                println!("{namespace:<10} {entries:>4} entries {bytes:>8} bytes");
            }
            println!(
                "{:<10} {total_entries:>4} entries {total_bytes:>8} bytes",
                "total"
            );
            Ok(())
        }
        CacheAction::Clean => {
            let (entries, bytes) = cache.clean()?;
            println!("removed {entries} entries ({bytes} bytes) for {repo}");
            Ok(())
        }
    }
}

pub fn hook_command(action: &HookAction) -> Result<()> {
    let git = Git::discover()?;
    match action {
        HookAction::Install { force } => {
            let path = crate::hook::install(&git, *force)?;
            println!("installed {}", paths::display(&path));
            println!("a plain `git commit` now opens the editor with a generated message");
            Ok(())
        }
        HookAction::Uninstall => {
            if crate::hook::uninstall(&git)? {
                println!("removed the wyckoff hook");
            } else {
                println!("no wyckoff hook was installed");
            }
            Ok(())
        }
        HookAction::Status => {
            let (path, installed) = crate::hook::status(&git);
            println!("hook file {}", paths::display(&path));
            println!("installed {}", if installed { "yes" } else { "no" });
            Ok(())
        }
    }
}
