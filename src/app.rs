use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
    process::{Child, Command},
};
use sysinfo::System;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn discover_executable(config: &AppConfig) -> Result<PathBuf> {
    discover_executable_from_candidates(config, default_candidates())
}

pub fn discover_executable_from_candidates(
    config: &AppConfig,
    candidates: Vec<PathBuf>,
) -> Result<PathBuf> {
    if let Some(path) = &config.copilot_executable {
        if path.is_file() {
            return Ok(path.clone());
        }
    }

    if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
        return Ok(path);
    }

    bail!("未找到 GitHub Copilot App。请在 config.json 中设置 copilot_executable。")
}

pub fn default_candidates() -> Vec<PathBuf> {
    let mut paths = running_github_paths();
    paths.extend(registry_install_paths());
    paths.push(PathBuf::from(r"C:\Path\To\github.exe"));

    if let Some(local) = env::var_os("LOCALAPPDATA") {
        paths.push(PathBuf::from(&local).join(r"Programs\GitHub Copilot\github.exe"));
        paths.push(PathBuf::from(&local).join(r"GitHub Copilot\github.exe"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles") {
        paths.push(PathBuf::from(program_files).join(r"GitHub Copilot\github.exe"));
    }

    paths.sort();
    paths.dedup();
    paths
}

pub fn is_copilot_running(executable: &Path) -> bool {
    let expected = executable
        .canonicalize()
        .unwrap_or_else(|_| executable.to_path_buf());
    running_github_paths()
        .into_iter()
        .any(|path| path.canonicalize().unwrap_or(path) == expected)
}

fn running_github_paths() -> Vec<PathBuf> {
    let system = System::new_all();
    system
        .processes()
        .values()
        .filter(|process| process.name().eq_ignore_ascii_case("github.exe"))
        .filter_map(|process| process.exe().map(Path::to_path_buf))
        .collect()
}

fn registry_install_paths() -> Vec<PathBuf> {
    let script = r#"
$roots = @(
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
Get-ItemProperty $roots -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -eq 'GitHub Copilot App' } |
  ForEach-Object {
    if ($_.InstallLocation) { Join-Path $_.InstallLocation 'github.exe' }
    elseif ($_.DisplayIcon) { $_.DisplayIcon -replace ',\d+$','' }
  }
"#;
    hidden_powershell(script)
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

pub fn launch_copilot(path: &Path, port: u16) -> Result<Child> {
    let mut command = Command::new(path);
    let existing = env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let args = webview_arguments(&existing, port);
    command.env("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", args);
    #[cfg(windows)]
    command.creation_flags(0);
    command
        .spawn()
        .with_context(|| format!("无法启动 {}", path.display()))
}

pub fn webview_arguments(existing: &str, port: u16) -> String {
    let required =
        format!("--remote-debugging-port={port} --remote-allow-origins=http://127.0.0.1:{port}");
    if existing.trim().is_empty() {
        required
    } else {
        format!("{} {}", existing.trim(), required)
    }
}

fn hidden_powershell(script: &str) -> Result<std::process::Output> {
    let encoded = crate::signature::encode_powershell(script);
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-EncodedCommand",
        &encoded,
    ]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command.output().context("无法查询 GitHub Copilot 安装信息")
}

#[cfg(test)]
mod tests {
    use super::{discover_executable_from_candidates, webview_arguments};
    use crate::config::AppConfig;
    use std::fs;

    #[test]
    fn configured_existing_path_takes_precedence() {
        let temp = tempfile::tempdir().unwrap();
        let configured = temp.path().join("configured.exe");
        let fallback = temp.path().join("fallback.exe");
        fs::write(&configured, b"configured").unwrap();
        fs::write(&fallback, b"fallback").unwrap();
        let config = AppConfig {
            copilot_executable: Some(configured.clone()),
        };

        let found = discover_executable_from_candidates(&config, vec![fallback]).unwrap();
        assert_eq!(found, configured);
    }

    #[test]
    fn missing_candidates_return_actionable_error() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing.exe");
        let error = discover_executable_from_candidates(&AppConfig::default(), vec![missing])
            .unwrap_err()
            .to_string();
        assert!(error.contains("config.json"), "unexpected error: {error}");
    }

    #[test]
    fn webview_arguments_preserve_existing_flags() {
        let args = webview_arguments("--disable-features=Example", 32123);
        assert_eq!(
            args,
            "--disable-features=Example --remote-debugging-port=32123 --remote-allow-origins=http://127.0.0.1:32123"
        );
    }
}
