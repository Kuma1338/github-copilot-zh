#![cfg_attr(not(test), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use copilot_zh::{
    app,
    cdp::{self, InjectionStats},
    config::{AppConfig, Dictionary},
    logging, signature, ui,
};
use std::{env, fs, path::PathBuf};

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
            "GitHub Copilot 正在运行。请先从系统托盘退出应用，然后重新打开中文快捷方式。",
        );
        return Ok(());
    }
    signature::verify_github_signature(&executable)?;

    let localization_dir = base_dir.join("localization");
    let dictionary = Dictionary::load(&localization_dir.join("zh-CN.json"))?;
    let runtime = fs::read_to_string(localization_dir.join("runtime.js"))
        .context("无法读取 localization\\runtime.js")?;
    let source = cdp::build_injection_source(&runtime, &dictionary)?;
    let version = app::product_version(&executable)?;
    let tested_version = dictionary.supports_version(&version);
    if !tested_version {
        ui::show_info(
            "GitHub Copilot 中文版 - 兼容模式",
            &format!(
                "当前 GitHub Copilot 版本为 {version}，尚未列入已验证清单。\n\n汉化将使用精确匹配兼容模式；无法识别的文字会保持英文。"
            ),
        );
    }

    let port = cdp::allocate_loopback_port()?;
    let mut child = app::launch_copilot(&executable, port)?;
    let log_dir = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| base_dir.clone())
        .join("GitHubCopilotZh");

    match cdp::run_injector(port, &source, child.id()) {
        Ok(stats) => {
            let _ = logging::write_session_log(&log_dir, &version, tested_version, stats);
        }
        Err(error) => {
            let _ = logging::write_session_log(
                &log_dir,
                &version,
                tested_version,
                InjectionStats {
                    targets_seen: 0,
                    targets_injected: 0,
                    errors: 1,
                },
            );
            return Err(error);
        }
    }
    let _ = child.wait();
    Ok(())
}
