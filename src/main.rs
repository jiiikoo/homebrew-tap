//! `sshelf` — a TUI SSH host manager.
//!
//! The binary runs in one of two modes:
//!  - normal: the interactive TUI (default) or a subcommand (`list`, `add`, `import`);
//!  - askpass: when invoked by `ssh` via `SSH_ASKPASS` (detected by the `SSHELF_ASKPASS`
//!    env var). Implemented in M5.

mod app;
mod askpass;
mod config;
mod import;
mod model;
mod paths;
mod search;
mod secrets;
mod ssh;
mod state;
mod store;
mod ui;
mod vault;

use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

use crate::config::Config;
use crate::paths::{CONFIG_ENV, Paths};
use crate::state::FrecencyState;
use anyhow::Context;

#[derive(Parser)]
#[command(name = "sshelf", version, about = "A TUI SSH host manager")]
struct Cli {
    /// Use a specific config file (default: ~/.config/sshelf/config.toml).
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List saved hosts (sorted by frecency).
    List,
    /// Add a host via the wizard.
    Add,
    /// Import hosts from ~/.ssh/config (read-only).
    Import {
        /// Show what would be imported without writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Store a password (read from stdin) for a host, by name or id.
    SetPassword {
        /// Host name or id.
        host: String,
    },
    /// Print shell completions to stdout (for packaging / `source <(sshelf completions bash)`).
    Completions {
        /// Shell: bash, zsh, fish, elvish, or powershell.
        shell: clap_complete::Shell,
    },
    /// Print the man page (roff) to stdout.
    Man,
}

fn main() -> Result<()> {
    // ssh invokes us as `sshelf "<prompt>"` with SSHELF_ASKPASS=1 in the environment.
    // This must be checked before clap, since the prompt is a positional arg, not a flag.
    if std::env::var_os("SSHELF_ASKPASS").is_some() {
        let prompt = std::env::args().nth(1).unwrap_or_default();
        std::process::exit(askpass::run(&prompt));
    }

    let cli = Cli::parse();
    // A `--config` flag is plumbed to all paths via the env var (so subcommands and Paths
    // resolution see it uniformly). Set before any Paths::resolve().
    if let Some(path) = &cli.config {
        // SAFETY: set once at startup, before any threads are spawned.
        unsafe {
            std::env::set_var(CONFIG_ENV, path);
        }
    }
    match cli.command {
        Some(Command::List) => cmd_list(),
        Some(Command::Add) => {
            println!("`sshelf add` arrives in M4. For now, edit hosts.toml directly.");
            Ok(())
        }
        Some(Command::Import { dry_run }) => cmd_import(dry_run),
        Some(Command::SetPassword { host }) => cmd_set_password(&host),
        Some(Command::Completions { shell }) => {
            clap_complete::generate(shell, &mut Cli::command(), "sshelf", &mut std::io::stdout());
            Ok(())
        }
        Some(Command::Man) => clap_mangen::Man::new(Cli::command())
            .render(&mut std::io::stdout())
            .context("rendering man page"),
        None => app::run(),
    }
}

fn cmd_import(dry_run: bool) -> Result<()> {
    let path = import::default_config_path().context("HOME is not set")?;
    if !path.exists() {
        anyhow::bail!("no ssh config at {}", path.display());
    }
    let result = import::parse_file(&path)?;
    println!(
        "Parsed {} host(s) from {}",
        result.hosts.len(),
        path.display()
    );
    for w in &result.warnings {
        println!("  warning: {w}");
    }

    let paths = Paths::resolve()?;
    paths.ensure_dirs()?;
    let cfg = Config::load(&paths.config_file())?;
    let hosts_path = cfg.hosts_path(&paths);
    let mut file = store::load_hosts(&hosts_path)?;
    let to_add = import::new_hosts(&result.hosts, &file.hosts)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    if to_add.is_empty() {
        println!("Nothing new to import (all names already exist).");
        return Ok(());
    }
    println!("{} new host(s):", to_add.len());
    for h in &to_add {
        println!("  {:<20} {}", h.name, h.endpoint());
    }
    if dry_run {
        println!("(dry run — nothing written)");
        return Ok(());
    }
    file.hosts.extend(to_add);
    store::save_hosts(&hosts_path, &file)?;
    println!("Imported into {}", hosts_path.display());
    Ok(())
}

fn cmd_set_password(host_ref: &str) -> Result<()> {
    use std::io::BufRead;
    let paths = Paths::resolve()?;
    let cfg = Config::load(&paths.config_file())?;
    let hosts = store::load_hosts(&cfg.hosts_path(&paths))?.hosts;
    let host = hosts
        .iter()
        .find(|h| h.id == host_ref || h.name == host_ref)
        .with_context(|| format!("no host with name or id '{host_ref}'"))?;

    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("reading password from stdin")?;
    let password = line.trim_end_matches(['\n', '\r']);
    if password.is_empty() {
        anyhow::bail!("empty password; nothing stored");
    }
    secrets::store_password(&paths.vault_file(), &host.id, password)?;
    println!("stored password for \"{}\" ({})", host.name, host.id);
    Ok(())
}

fn cmd_list() -> Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure_dirs()?;
    let _ = Config::ensure_default_file(&paths.config_file()); // best-effort
    let cfg = Config::load(&paths.config_file())?;
    let hosts_path = cfg.hosts_path(&paths);
    let hosts = store::load_hosts(&hosts_path)?.hosts;
    let st = FrecencyState::load(&paths.state_file())?;

    if hosts.is_empty() {
        println!("No hosts yet. Add one with `sshelf add`, or create:");
        println!("  {}", hosts_path.display());
        return Ok(());
    }

    let order = search::rank(&hosts, "", &st, cfg.decay_rate, cfg.default_sort);
    for &i in &order {
        let h = &hosts[i];
        let tags = if h.tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", h.tags.join(", "))
        };
        println!(
            "{:<20}  {:<28}  {}{}",
            h.name,
            h.endpoint(),
            h.auth.as_str(),
            tags
        );
    }
    Ok(())
}
