use anyhow::{bail, Context, Result};
#[cfg(windows)]
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{path::Path, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn verify_github_signature(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        verify_windows_signature(path)
    }
    #[cfg(target_os = "macos")]
    {
        verify_macos_signature(path)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = path;
        bail!("当前系统不支持官方应用签名校验")
    }
}

#[cfg(windows)]
fn verify_windows_signature(path: &Path) -> Result<()> {
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Import-Module "$env:SystemRoot\System32\WindowsPowerShell\v1.0\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" -ErrorAction Stop
$signature = Get-AuthenticodeSignature -LiteralPath $env:COPILOT_ZH_VERIFY_PATH
Write-Output ([string]$signature.Status)
if ($signature.SignerCertificate) { Write-Output $signature.SignerCertificate.Subject }
"#;
    let encoded = encode_powershell(script);
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .env("COPILOT_ZH_VERIFY_PATH", path);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.output().context("无法运行 Authenticode 签名检查")?;
    if !output.status.success() {
        bail!("Authenticode 签名检查进程失败")
    }
    parse_signature_output(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
fn verify_macos_signature(path: &Path) -> Result<()> {
    let bundle = crate::macos::bundle_root(path)
        .with_context(|| format!("无法从 {} 定位 .app bundle", path.display()))?;
    let verification = Command::new("codesign")
        .args(["--verify", "--deep", "--strict", "--verbose=2"])
        .arg(&bundle)
        .output()
        .context("无法运行 macOS codesign 校验")?;
    if !verification.status.success() {
        bail!("GitHub Copilot 的 macOS 代码签名校验失败")
    }

    let details = Command::new("codesign")
        .args(["-dv", "--verbose=4"])
        .arg(&bundle)
        .output()
        .context("无法读取 macOS 代码签名信息")?;
    let details = format!(
        "{}{}",
        String::from_utf8_lossy(&details.stdout),
        String::from_utf8_lossy(&details.stderr)
    );
    parse_macos_signature_output(&details)?;

    let assessment = Command::new("spctl")
        .args(["--assess", "--type", "execute", "--verbose=2"])
        .arg(&bundle)
        .output()
        .context("无法运行 macOS Gatekeeper 校验")?;
    if !assessment.status.success() {
        bail!("GitHub Copilot 未通过 macOS Gatekeeper 校验")
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn encode_powershell(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    STANDARD.encode(bytes)
}

pub fn parse_macos_signature_output(output: &str) -> Result<()> {
    if !output
        .lines()
        .any(|line| line.trim() == "Identifier=com.github.githubapp")
    {
        bail!("程序标识不是官方 GitHub Copilot macOS bundle")
    }
    if !crate::macos::is_github_signature(output) {
        bail!("程序签名者或 Team ID 不是 GitHub")
    }
    Ok(())
}

pub fn parse_signature_output(output: &str) -> Result<()> {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let status = lines.next().unwrap_or("Unknown");
    if status != "Valid" {
        bail!("官方程序签名状态无效：{status}")
    }
    let signer = lines.collect::<Vec<_>>().join(" ");
    if !(signer.contains("CN=GitHub, Inc.")
        || signer.contains("O=GitHub, Inc.")
        || signer.contains("CN=\"GitHub, Inc.\"")
        || signer.contains("O=\"GitHub, Inc.\""))
    {
        bail!("程序签名者不是 GitHub, Inc.")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_signature_output;

    #[test]
    fn accepts_valid_github_signature() {
        let output = "Valid\nCN=GitHub, Inc., O=GitHub, Inc., L=San Francisco";
        parse_signature_output(output).unwrap();
    }

    #[test]
    fn accepts_quoted_github_signature_subject() {
        let output = "Valid\nCN=\"GitHub, Inc.\", O=\"GitHub, Inc.\"";
        parse_signature_output(output).unwrap();
    }

    #[test]
    fn rejects_non_github_signer() {
        let output = "Valid\nCN=Example Corp, O=Example Corp";
        let error = parse_signature_output(output).unwrap_err().to_string();
        assert!(error.contains("GitHub"), "unexpected error: {error}");
    }

    #[test]
    fn rejects_invalid_signature_status() {
        let output = "NotSigned\n";
        let error = parse_signature_output(output).unwrap_err().to_string();
        assert!(error.contains("NotSigned"), "unexpected error: {error}");
    }

    #[cfg(windows)]
    #[test]
    fn verifies_the_installed_github_copilot_signature() {
        if let Ok(path) = crate::app::discover_executable(&crate::config::AppConfig::default()) {
            super::verify_github_signature(&path).unwrap();
        }
    }
}
