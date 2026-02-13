use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("faxina-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Faxina CLI"));
}

#[test]
fn test_dry_run_no_projects() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("faxina-cli").unwrap();

    cmd.arg(temp.path())
        .arg("--days").arg("0")
        .arg("--dry-run")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nenhum projeto inativo"));
}

#[test]
fn test_dry_run_with_projects() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Setup: Node Project
    let node_proj = root.join("node-proj");
    fs::create_dir_all(node_proj.join("node_modules")).unwrap();
    fs::write(node_proj.join("package.json"), "{}").unwrap();
    fs::write(node_proj.join("node_modules/bin"), "data").unwrap(); // Simula conteúdo

    // Setup: Rust Project
    let rust_proj = root.join("rust-proj");
    fs::create_dir_all(rust_proj.join("target")).unwrap();
    fs::write(rust_proj.join("Cargo.toml"), "[package]").unwrap();
    fs::write(rust_proj.join("target/debug"), "data").unwrap();

    let mut cmd = Command::cargo_bin("faxina-cli").unwrap();
    
    // Executa dry-run
    cmd.arg(root)
        .arg("--days").arg("0")
        .arg("--dry-run")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("2 projetos inativos encontrados"))
        .stdout(predicate::str::contains("node-proj"))
        .stdout(predicate::str::contains("rust-proj"));

    // Verifica que NADA foi apagado
    assert!(node_proj.join("node_modules").exists());
    assert!(rust_proj.join("target").exists());
}

#[test]
fn test_clean_execution() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Setup: Node Project a ser limpo
    let node_proj = root.join("node-proj");
    fs::create_dir_all(node_proj.join("node_modules")).unwrap();
    fs::write(node_proj.join("package.json"), "{}").unwrap();
    
    // Executa limpeza real
    let mut cmd = Command::cargo_bin("faxina-cli").unwrap();
    cmd.arg(root)
        .arg("--days").arg("0")
        .arg("--yes") // Confirma
        .assert()
        .success();

    // Verifica que foi APAGADO
    assert!(!node_proj.join("node_modules").exists());
}
