use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{path::Path, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn verify_github_signature(path: &Path) -> Result<()> {
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

pub(crate) fn encode_powershell(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    STANDARD.encode(bytes)
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
        let path = std::path::Path::new(r"C:\Path\To\github.exe");
        if path.exists() {
            super::verify_github_signature(path).unwrap();
        }
    }
}
