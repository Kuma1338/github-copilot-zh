use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Dictionary {
    pub version: u32,
    pub tested_app_versions: Vec<String>,
    pub exact: BTreeMap<String, String>,
    #[serde(default)]
    pub patterns: Vec<Pattern>,
    #[serde(default)]
    pub excluded_selectors: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Pattern {
    pub source: String,
    pub target: String,
    #[serde(default, rename = "parameterRules")]
    pub parameter_rules: BTreeMap<String, String>,
}

impl Dictionary {
    pub fn load(path: &Path) -> Result<Self> {
        let contents =
            fs::read_to_string(path).with_context(|| format!("无法读取 {}", path.display()))?;
        serde_json::from_str(&contents).with_context(|| format!("无法解析 {}", path.display()))
    }

    pub fn supports_version(&self, version: &str) -> bool {
        self.tested_app_versions.iter().any(|item| item == version)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, Dictionary};
    use std::{collections::BTreeMap, fs, path::PathBuf};

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
            r#"{"copilot_executable":"C:\\Apps\\GitHub Copilot\\github.exe"}"#,
        )
        .unwrap();
        let config = AppConfig::load(temp.path()).unwrap();
        assert_eq!(
            config.copilot_executable,
            Some(PathBuf::from(r"C:\Apps\GitHub Copilot\github.exe"))
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

    #[test]
    fn dictionary_loads_camel_case_fields() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("zh-CN.json"),
            r#"{"version":1,"testedAppVersions":["1.1.15"],"exact":{"Home":"主页"},"patterns":[],"excludedSelectors":["code"]}"#,
        )
        .unwrap();
        let dictionary = Dictionary::load(&temp.path().join("zh-CN.json")).unwrap();
        assert_eq!(dictionary.tested_app_versions, vec!["1.1.15"]);
        assert_eq!(
            dictionary.exact.get("Home").map(String::as_str),
            Some("主页")
        );
    }

    #[test]
    fn dictionary_identifies_tested_versions_exactly() {
        let dictionary = Dictionary {
            version: 1,
            tested_app_versions: vec!["1.1.15".into()],
            exact: BTreeMap::new(),
            patterns: Vec::new(),
            excluded_selectors: Vec::new(),
        };
        assert!(dictionary.supports_version("1.1.15"));
        assert!(!dictionary.supports_version("1.1.16"));
    }
}
