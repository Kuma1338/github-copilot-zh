use std::{fs, path::Path};

use copilot_zh::macos::{
    bundle_executable_path, inspector_endpoint_paths, inspector_environment, is_github_bundle,
    is_github_signature, product_version, resource_base_dir,
};
use copilot_zh::signature::parse_macos_signature_output;

#[test]
fn resolves_the_official_app_bundle_executable() {
    assert_eq!(
        bundle_executable_path(Path::new("/Applications/GitHub Copilot.app")),
        Path::new("/Applications/GitHub Copilot.app/Contents/MacOS/github")
    );
    let executable = Path::new("/Applications/GitHub Copilot.app/Contents/MacOS/github");
    assert_eq!(bundle_executable_path(executable), executable);
}

#[test]
fn only_github_copilot_bundles_are_process_candidates() {
    assert!(is_github_bundle(Path::new(
        "/Applications/GitHub Copilot.app/Contents/MacOS/github",
    )));
    assert!(!is_github_bundle(Path::new(
        "/Applications/Calculator.app/Contents/MacOS/Calculator",
    )));
}

#[test]
fn uses_bundle_resources_for_a_macos_launcher() {
    assert_eq!(
        resource_base_dir(Path::new(
            "/Applications/GitHub Copilot 中文版.app/Contents/MacOS/copilot-zh",
        )),
        Path::new("/Applications/GitHub Copilot 中文版.app/Contents/Resources")
    );
    assert_eq!(
        resource_base_dir(Path::new("C:/GitHubCopilotZh/copilot-zh.exe")),
        Path::new("C:/GitHubCopilotZh")
    );
}

#[test]
fn recognizes_github_developer_id_and_team_id() {
    let details = "Authority=Developer ID Application: GitHub, Inc. (VEKTX9H2N7)\nTeamIdentifier=VEKTX9H2N7\nIdentifier=com.github.githubapp";
    assert!(is_github_signature(details));
    assert!(!is_github_signature(
        "Authority=Developer ID Application: Example Corp (AAAAAAAAAA)\nTeamIdentifier=AAAAAAAAAA"
    ));
}

#[test]
fn builds_loopback_inspector_environment_and_endpoints() {
    assert_eq!(
        inspector_environment(43123),
        vec![
            (
                "WEBKIT_INSPECTOR_SERVER".to_owned(),
                "127.0.0.1:43123".to_owned()
            ),
            (
                "WEBKIT_INSPECTOR_HTTP_SERVER".to_owned(),
                "127.0.0.1:43123".to_owned()
            )
        ]
    );
    assert_eq!(
        inspector_endpoint_paths(43123),
        vec![
            "http://127.0.0.1:43123/json/list".to_owned(),
            "http://127.0.0.1:43123/json".to_owned()
        ]
    );
}

#[test]
fn parses_macos_codesign_output_as_a_github_bundle() {
    let details = "Executable=/Applications/GitHub Copilot.app/Contents/MacOS/github\nIdentifier=com.github.githubapp\nTeamIdentifier=VEKTX9H2N7\nAuthority=Developer ID Application: GitHub, Inc. (VEKTX9H2N7)\nAuthority=Developer ID Certification Authority";
    parse_macos_signature_output(details).unwrap();

    let error = parse_macos_signature_output(
        "Identifier=com.example.app\nTeamIdentifier=AAAAAAAAAA\nAuthority=Developer ID Application: Example Corp (AAAAAAAAAA)",
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("GitHub"));
}

#[test]
fn reads_the_short_bundle_version_before_the_build_version() {
    let temp = tempfile::tempdir().unwrap();
    let bundle = temp.path().join("GitHub Copilot.app");
    let contents = bundle.join("Contents");
    let macos = contents.join("MacOS");
    fs::create_dir_all(&macos).unwrap();
    fs::write(
        contents.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleVersion</key><string>build-42</string>
<key>CFBundleShortVersionString</key><string>1.1.15</string>
</dict></plist>"#,
    )
    .unwrap();
    let executable = macos.join("github");
    fs::write(&executable, b"placeholder").unwrap();

    assert_eq!(product_version(&executable).unwrap(), "1.1.15");
}
