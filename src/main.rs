use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
mod types;
mod projects;
mod cleaner;
mod display;
mod scanner;
mod config;

use std::path::PathBuf;
use config::{Config, ConfigError};

use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, debug};

#[derive(Parser, Debug)]
#[command(name = "faxina-cli", version, about, long_about = None)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,

    #[arg(short, long)]
    days: Option<u64>,

    #[arg(long)]
    dry_run: bool,

    #[arg(short, long)]
    yes: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,

    #[arg(long)]
    stats: bool, // Exibe estatísticas e sai sem limpar

    #[arg(short, long)]
    interactive: bool, // Modo interativo de seleção

    #[arg(long, value_delimiter = ',')]
    excluded_dirs: Option<Vec<String>>, // Pastas para ignorar (separadas por vírgula)

    #[arg(long)]
    config: Option<PathBuf>, // Arquivo de configuração personalizado
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let result = run();
    pause_on_windows();
    result
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let root = cli.path.canonicalize()
        .with_context(|| format!("Não foi possível acessar '{}'", cli.path.display()))?;

    // Carrega configuração com tratamento de erro robusto
    let config_result = if let Some(path) = &cli.config {
        Config::load_from_path(path)
    } else {
        Config::load()
    };

    let config = match config_result {
        Ok(c) => c,
        Err(e) => match e {
            ConfigError::NotFound => {
                if cli.config.is_some() {
                    bail!("Arquivo de configuração especificado não encontrado: {}", cli.config.unwrap().display());
                }
                debug!("Arquivo de configuração não encontrado, usando defaults.");
                Config { days: None, excluded_dirs: None, auto_confirm: None }
            },
            ConfigError::ParseError(path, msg) => {
                error!("Erro fatal no arquivo de configuração '{}': {}", path.display(), msg);
                bail!("Erro fatal no arquivo de configuração '{}': {}", path.display(), msg);
            },
            ConfigError::IoError(err) => {
                error!("Erro de I/O ao ler configuração: {}", err);
                Config { days: None, excluded_dirs: None, auto_confirm: None }
            }
        }
    };

    let days = cli.days.or(config.days).unwrap_or(30);
    let auto_confirm = cli.yes || config.auto_confirm.unwrap_or(false);

    let mut ignored_paths: Vec<PathBuf> = config.excluded_dirs
        .unwrap_or_default()
        .iter()
        .map(|p| PathBuf::from(p))
        .collect();

    if let Some(cli_excludes) = cli.excluded_dirs {
        for p in cli_excludes {
            ignored_paths.push(PathBuf::from(p));
        }
    }

    let ignored_paths: Vec<PathBuf> = ignored_paths.into_iter()
        .map(|p| if p.is_absolute() { p } else { std::env::current_dir().unwrap_or_default().join(p) })
        .collect();

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
            days.to_string().bold()
        ));
    }

    let progress_spinner = spinner.clone();
    let root_display = root.display().to_string();
    let days_display = days.to_string();
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

    let mut projects = scanner::scan_projects(&root, days, &ignored_paths, Some(on_progress));
    spinner.finish_and_clear();

    if projects.is_empty() {
        if !cli.quiet {
            display::print_no_stale_projects(days);
        }
        return Ok(());
    }

    // Calcula tamanhos dos diretórios de dependência (fase separada para performance)
    let size_spinner = ProgressBar::new_spinner();
    size_spinner.set_style(
        ProgressStyle::with_template("  {spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    if !cli.quiet {
        size_spinner.enable_steady_tick(Duration::from_millis(80));
        size_spinner.set_message(format!(
            "Calculando tamanhos de {} projetos...",
            projects.len().to_string().bold()
        ));
    }
    scanner::calculate_sizes(&mut projects);
    size_spinner.finish_and_clear();

    if cli.stats {
        if !cli.quiet {
            display::print_stats(&projects);
        }
        return Ok(());
    }

    // Modo Interativo
    if cli.interactive {
        use dialoguer::{theme::ColorfulTheme, MultiSelect};

        println!();
        println!("  {}", "Selecione os projetos para limpar (Espaço para selecionar, Enter para confirmar):".bold());

        let project_labels: Vec<String> = projects.iter().map(|p| {
            let size = p.total_size();
            let size_str = display::format_size(size);
            format!("{} ({}) - {}", p.path.display(), size_str, p.dep_dirs.iter().map(|d| d.kind.icon()).collect::<Vec<_>>().join(" "))
        }).collect();

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .items(&project_labels)
            .interact()?;

        if selections.is_empty() {
             println!();
             println!("  {} Nenhum projeto selecionado.", "↩".dimmed());
             println!();
             return Ok(());
        }

        projects = selections.into_iter().map(|i| projects[i].clone()).collect();
    }

    if !cli.quiet {
        display::print_scan_results(&projects);
    }

    if !auto_confirm {
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

fn pause_on_windows() {
    #[cfg(target_os = "windows")]
    {
        use std::io::{self, Read};
        // Só pausa se não for quiet e se for terminal interativo, ou se der erro?
        // O original pausava sempre. Manter comportamento.
        println!("\nPressione Enter para sair...");
        let _ = io::stdin().read(&mut [0u8]);
    }
}
