use crate::cdp::InjectionStats;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn format_session_log(
    app_version: &str,
    tested_version: bool,
    stats: InjectionStats,
) -> String {
    format!(
        "app_version={app_version}\ntested_version={tested_version}\ntargets_seen={}\ntargets_injected={}\nerrors={}\n",
        stats.targets_seen, stats.targets_injected, stats.errors
    )
}

pub fn write_session_log(
    base_dir: &Path,
    app_version: &str,
    tested_version: bool,
    stats: InjectionStats,
) -> Result<PathBuf> {
    let logs = base_dir.join("logs");
    fs::create_dir_all(&logs).with_context(|| format!("无法创建日志目录 {}", logs.display()))?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let path = logs.join(format!("session-{timestamp}.log"));
    fs::write(
        &path,
        format_session_log(app_version, tested_version, stats),
    )
    .with_context(|| format!("无法写入日志 {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::format_session_log;
    use crate::cdp::InjectionStats;

    #[test]
    fn session_log_contains_metadata_only() {
        let log = format_session_log(
            "1.1.15",
            true,
            InjectionStats {
                targets_seen: 2,
                targets_injected: 2,
                errors: 0,
            },
        );
        assert!(log.contains("app_version=1.1.15"));
        assert!(log.contains("tested_version=true"));
        assert!(log.contains("targets_injected=2"));
        assert!(!log.contains("dom_text"));
    }
}
