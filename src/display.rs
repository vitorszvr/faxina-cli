use std::io::IsTerminal;
use std::time::SystemTime;

use colored::Colorize;
use dialoguer::Confirm;

use crate::cleaner::CleanResult;
use crate::scanner::StaleProject;

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn days_ago(time: SystemTime) -> String {
    match SystemTime::now().duration_since(time) {
        Ok(duration) => {
            let days = duration.as_secs() / (24 * 3600);
            if days == 1 {
                "1 dia atrás".to_string()
            } else {
                format!("{} dias atrás", days)
            }
        }
        Err(_) => "recente".to_string(),
    }
}

pub fn print_header() {
    println!();
    println!(
        "{}",
        "  🧹 Lixeiro Inteligente — Limpador de Projetos"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "  ─────────────────────────────────────────────"
            .dimmed()
    );
    println!();
}

pub fn print_scan_results(projects: &[StaleProject]) {
    let total_size: u64 = projects.iter().map(|p| p.total_size()).sum();
    let total_dirs: usize = projects.iter().map(|p| p.dep_dirs.len()).sum();

    println!(
        "  {} {} projetos inativos encontrados ({} pastas, {})",
        "📦".to_string(),
        projects.len().to_string().bold().yellow(),
        total_dirs.to_string().bold(),
        format_size(total_size).bold().red()
    );
    println!();

    for project in projects {
        println!(
            "  {} {}",
            "▸".bold().cyan(),
            project.name.bold().white()
        );
        println!(
            "    {}  {}",
            "📂".to_string(),
            project.path.display().to_string().dimmed()
        );
        println!(
            "    {}  Última modificação: {}",
            "🕐".to_string(),
            days_ago(project.last_modified).yellow()
        );

        for dep in &project.dep_dirs {
            let kind_icon = match dep.kind {
                crate::scanner::DepKind::NodeModules => "📦",
                crate::scanner::DepKind::Target => "🦀",
                crate::scanner::DepKind::NextBuild => "▲ ",
                crate::scanner::DepKind::Venv => "🐍",
                crate::scanner::DepKind::Vendor => "📁",
                crate::scanner::DepKind::Build => "🏗️",
            };
            println!(
                "    {} {} {}",
                kind_icon,
                dep.kind.to_string().bold(),
                format_size(dep.size).red()
            );
        }

        println!();
    }
}

pub fn confirm_cleanup(dry_run: bool) -> bool {
    if dry_run {
        println!(
            "  {}",
            "🔍 Modo dry-run: nenhum arquivo será deletado."
                .bold()
                .blue()
        );
        println!();
        return true;
    }

    if !std::io::stdin().is_terminal() {
        eprintln!(
            "  {} Stdin não é interativo. Use a flag {} para pular confirmação.",
            "✗".red().bold(),
            "--yes".bold()
        );
        return false;
    }

    Confirm::new()
        .with_prompt("  🗑️  Deseja remover essas pastas de dependência?")
        .default(false)
        .interact()
        .unwrap_or(false)
}

pub fn print_summary(result: &CleanResult, dry_run: bool) {
    println!();

    if dry_run {
        println!(
            "  {} Simulação concluída. {} seriam liberados de {} pastas.",
            "🔍".to_string(),
            format_size(result.total_freed).bold().green(),
            result.dirs_removed.to_string().bold()
        );
    } else {
        println!(
            "  {} {} {} liberados!",
            "🧹".to_string(),
            "Limpeza concluída.".bold().green(),
            format_size(result.total_freed).bold().green()
        );
        println!(
            "    {} pastas removidas com sucesso.",
            result.dirs_removed.to_string().bold()
        );
    }

    if !result.errors.is_empty() {
        println!();
        println!(
            "  {} {} erros durante a limpeza:",
            "⚠️".to_string(),
            result.errors.len().to_string().bold().red()
        );
        for (path, err) in &result.errors {
            println!("    {} {} — {}", "✗".red(), path.display(), err);
        }
    }

    println!();
}

pub fn print_no_stale_projects(days: u64) {
    println!();
    println!(
        "  {} Nenhum projeto inativo há mais de {} dias encontrado.",
        "✨".to_string(),
        days.to_string().bold()
    );
    println!(
        "  {}",
        "Seu disco está limpo! 🎉".green().bold()
    );
    println!();
}
