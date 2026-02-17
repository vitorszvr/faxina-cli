use std::collections::HashMap;
use std::io::IsTerminal;
use std::time::SystemTime;

use colored::Colorize;
use dialoguer::Confirm;

use crate::cleaner::CleanResult;
use crate::types::{StaleProject, DepKind};

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
        "  🧹 Faxina CLI — Lixeiro Inteligente de Projetos"
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
            println!(
                "    {} {} {}",
                dep.kind.icon(),
                dep.kind.to_string().bold(),
                format_size(dep.size).red()
            );
        }

        println!();
    }
}

pub fn print_stats(projects: &[StaleProject]) {
    let mut stats: HashMap<DepKind, (usize, u64)> = HashMap::new();

    for project in projects {
        for dep in &project.dep_dirs {
            let entry = stats.entry(dep.kind.clone()).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += dep.size;
        }
    }

    // Convert to vector for sorting
    let mut stats_vec: Vec<(DepKind, usize, u64)> = stats
        .into_iter()
        .map(|(k, (count, size))| (k, count, size))
        .collect();

    // Sort by size (descending)
    stats_vec.sort_by(|a, b| b.2.cmp(&a.2));

    println!();
    println!("  {}", "📊 Estatísticas por Tipo de Projeto:".bold().yellow());
    println!();

    for (kind, count, size) in stats_vec {
        println!(
            "  {} {:<15} {} projetos, {}",
            kind.icon(),
            kind.to_string().bold(),
            count.to_string().bold().cyan(),
            format_size(size).red()
        );
    }
    println!();
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

pub fn print_summary(result: &CleanResult, dry_run: bool, quiet: bool) {
    if quiet {
        println!("{}", crate::display::format_size(result.total_freed));
        return;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1536), "2 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn test_days_ago_recent() {
        let now = SystemTime::now();
        assert_eq!(days_ago(now), "0 dias atrás");
    }
}
