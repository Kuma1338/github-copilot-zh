use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppConfig {
    pub copilot_executable: Option<PathBuf>,
}

impl AppConfig {
    pub fn load(base_dir: &Path) -> Result<Self> {
        let path = base_dir.join("config.json");
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents =
            fs::read_to_string(&path).with_context(|| format!("无法读取 {}", path.display()))?;
        serde_json::from_str(&contents).with_context(|| format!("无法解析 {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use std::{fs, path::PathBuf};

    #[test]
    fn missing_config_uses_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let config = AppConfig::load(temp.path()).unwrap();
        assert_eq!(config.copilot_executable, None);
    }

    #[test]
    fn explicit_executable_path_is_loaded() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("config.json"),
            r#"{"copilot_executable":"C:\\Path\\To\\github.exe"}"#,
        )
        .unwrap();
        let config = AppConfig::load(temp.path()).unwrap();
        assert_eq!(
            config.copilot_executable,
            Some(PathBuf::from(r"C:\Path\To\github.exe"))
        );
    }

    #[test]
    fn malformed_config_reports_the_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");
        fs::write(&path, "{").unwrap();
        let error = AppConfig::load(temp.path()).unwrap_err().to_string();
        assert!(error.contains("config.json"), "unexpected error: {error}");
    }
}
