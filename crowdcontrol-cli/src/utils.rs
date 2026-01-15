use anyhow::{anyhow, Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::time::Duration;

pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "!".yellow(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

pub fn create_progress_bar(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Extract Claude Code credentials from macOS Keychain
#[cfg(target_os = "macos")]
pub fn extract_keychain_credentials() -> Result<String> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-a",
            &whoami::username(),
            "-w",
        ])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        return Err(anyhow!(
            "No Claude Code credentials found in keychain"
        ));
    }

    let credentials = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in keychain credentials")?
        .trim()
        .to_string();

    if credentials.is_empty() {
        return Err(anyhow!("Empty credentials returned from keychain"));
    }

    Ok(credentials)
}

#[cfg(not(target_os = "macos"))]
pub fn extract_keychain_credentials() -> Result<String> {
    Err(anyhow!("Keychain extraction is only supported on macOS"))
}
