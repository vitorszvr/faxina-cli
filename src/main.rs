mod types;
mod projects;
mod cleaner;
mod display;
mod scanner;

use std::path::PathBuf;
use std::process;
use std::time::Duration;

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

fn main() {
    let cli = Cli::parse();

    let root = match cli.path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "  {} Não foi possível acessar '{}': {}",
                "✗".red().bold(),
                cli.path.display(),
                e
            );
            process::exit(1);
        }
    };

    if !root.is_dir() {
        eprintln!(
            "  {} '{}' não é um diretório.",
            "✗".red().bold(),
            root.display()
        );
        process::exit(1);
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

    let projects = scanner::scan_projects(&root, cli.days);
    spinner.finish_and_clear();

    if projects.is_empty() {
        if !cli.quiet {
            display::print_no_stale_projects(cli.days);
        }
        return;
    }

    if !cli.quiet {
        display::print_scan_results(&projects);
    }

    if !cli.yes {
        if !display::confirm_cleanup(cli.dry_run) {
            println!();
            println!("  {} Limpeza cancelada.", "↩".dimmed());
            println!();
            return;
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
}
