use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
mod types;
mod projects;
mod cleaner;
mod display;
mod scanner;

use std::path::PathBuf;

use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
#[command(name = "faxina-cli", version, about, long_about = None)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,

    #[arg(short, long, default_value_t = 30)]
    days: u64,

    #[arg(long)]
    dry_run: bool,

    #[arg(short, long)]
    yes: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root = cli.path.canonicalize()
        .with_context(|| format!("Não foi possível acessar '{}'", cli.path.display()))?;

    if !root.is_dir() {
        bail!("'{}' não é um diretório.", root.display());
    }

    if !cli.quiet {
        display::print_header();
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("  {spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    if !cli.quiet {
        spinner.enable_steady_tick(Duration::from_millis(80));
        spinner.set_message(format!(
            "Varrendo {} (projetos inativos há {}+ dias)...",
            root.display().to_string().bold(),
            cli.days.to_string().bold()
        ));
    }

    let progress_spinner = spinner.clone();
    let root_display = root.display().to_string();
    let days_display = cli.days.to_string();
    let checked_files = Arc::new(AtomicUsize::new(0));
    let checked_files_clone = checked_files.clone();

    let on_progress = move || {
        let count = checked_files_clone.fetch_add(1, Ordering::Relaxed);
        if count % 500 == 0 {
            progress_spinner.set_message(format!(
                "Varrendo {} (projetos inativos há {}+ dias)... {} arquivos",
                root_display.bold(),
                days_display.bold(),
                count.to_string().dimmed()
            ));
        }
    };

    let projects = scanner::scan_projects(&root, cli.days, Some(on_progress));
    spinner.finish_and_clear();

    if projects.is_empty() {
        if !cli.quiet {
            display::print_no_stale_projects(cli.days);
        }
        return Ok(());
    }

    if !cli.quiet {
        display::print_scan_results(&projects);
    }

    if !cli.yes {
        if !display::confirm_cleanup(cli.dry_run) {
            println!();
            println!("  {} Limpeza cancelada.", "↩".dimmed());
            println!();
            return Ok(());
        }
        println!();
    } else if cli.dry_run && !cli.quiet {
        println!(
            "  {}",
            "🔍 Modo dry-run: nenhum arquivo será deletado."
                .bold()
                .blue()
        );
        println!();
    }

    let result = cleaner::clean_projects(&projects, cli.dry_run, cli.verbose);
    display::print_summary(&result, cli.dry_run, cli.quiet);
    
    Ok(())
}
