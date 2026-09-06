use anyhow::{bail, Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
};

pub const COPILOT_BUNDLE_ID: &str = "com.github.githubapp";
pub const GITHUB_TEAM_ID: &str = "VEKTX9H2N7";

pub fn bundle_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

pub fn is_github_bundle(path: &Path) -> bool {
    bundle_root(path)
        .and_then(|bundle| {
            bundle
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .is_some_and(|name| name.eq_ignore_ascii_case("GitHub Copilot.app"))
}

pub fn bundle_executable_path(path: &Path) -> PathBuf {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
    {
        path.join("Contents").join("MacOS").join("github")
    } else {
        path.to_path_buf()
    }
}

pub fn resource_base_dir(executable_path: &Path) -> PathBuf {
    let Some(macos_dir) = executable_path.parent() else {
        return PathBuf::from(".");
    };
    if macos_dir.file_name().is_some_and(|name| name == "MacOS")
        && macos_dir
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "Contents")
    {
        return macos_dir
            .parent()
            .expect("Contents parent was checked")
            .join("Resources");
    }
    macos_dir.to_path_buf()
}

pub fn default_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/Applications/GitHub Copilot.app"),
        PathBuf::from("/Applications/GitHub Copilot.app/Contents/MacOS/github"),
    ];
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("Applications/GitHub Copilot.app"));
        candidates.push(home.join("Applications/GitHub Copilot.app/Contents/MacOS/github"));
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

pub fn inspector_environment(port: u16) -> Vec<(String, String)> {
    let address = format!("127.0.0.1:{port}");
    vec![
        ("WEBKIT_INSPECTOR_SERVER".to_owned(), address.clone()),
        ("WEBKIT_INSPECTOR_HTTP_SERVER".to_owned(), address),
    ]
}

pub fn inspector_endpoint_paths(port: u16) -> Vec<String> {
    vec![
        format!("http://127.0.0.1:{port}/json/list"),
        format!("http://127.0.0.1:{port}/json"),
    ]
}

pub fn is_github_signature(output: &str) -> bool {
    let has_team = output.lines().any(|line| {
        line.trim()
            .strip_prefix("TeamIdentifier=")
            .is_some_and(|team| team == GITHUB_TEAM_ID)
    });
    let has_authority = output.lines().any(|line| {
        line.contains("Authority=Developer ID Application: GitHub, Inc.")
            && line.contains(GITHUB_TEAM_ID)
    });
    has_team && has_authority
}

pub fn product_version(path: &Path) -> Result<String> {
    let bundle = bundle_root(path).context("GitHub Copilot .app bundle 无效")?;
    let plist_path = bundle.join("Contents").join("Info.plist");
    let value = plist::Value::from_file(&plist_path)
        .with_context(|| format!("无法读取 {}", plist_path.display()))?;
    let dictionary = value.as_dictionary().context("Info.plist 不是有效字典")?;
    for key in ["CFBundleShortVersionString", "CFBundleVersion"] {
        if let Some(version) = dictionary.get(key).and_then(plist::Value::as_string) {
            if !version.trim().is_empty() {
                return Ok(version.to_owned());
            }
        }
    }
    bail!("Info.plist 中没有 GitHub Copilot 版本")
}

pub fn unsupported_inspector_message() -> &'static str {
    "当前官方 macOS 版使用 WKWebView，但没有提供可用的本地 Inspector/CDP 接口。未注入汉化层；请使用官方应用，或等待官方开放可调试接口。"
}
