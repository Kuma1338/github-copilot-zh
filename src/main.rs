#![cfg_attr(not(test), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use copilot_zh::{app, config::AppConfig, signature, ui};
use std::{env, path::PathBuf};

fn main() {
    if let Err(error) = run() {
        ui::show_error("GitHub Copilot 中文版", &format!("{error:#}"));
    }
}

fn run() -> Result<()> {
    let base_dir = env::current_exe()
        .context("无法确定启动器路径")?
        .parent()
        .map(PathBuf::from)
        .context("启动器路径无效")?;
    let config = AppConfig::load(&base_dir)?;
    let executable = app::discover_executable(&config)?;
    if app::is_copilot_running(&executable) {
        ui::show_info(
            "GitHub Copilot 中文版",
            "GitHub Copilot 正在运行。请先正常关闭应用，然后重新打开中文快捷方式。",
        );
        return Ok(());
    }
    signature::verify_github_signature(&executable)?;
    ui::show_info(
        "GitHub Copilot 中文版",
        "启动器基础组件已安装，汉化注入模块正在构建中。",
    );
    Ok(())
}
