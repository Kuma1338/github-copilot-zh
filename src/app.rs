use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
    process::{Child, Command},
};
use sysinfo::System;

#[cfg(windows)]
use std::{
    ffi::OsString,
    mem::{size_of, zeroed},
    os::windows::{ffi::OsStrExt, process::CommandExt},
    ptr::{null, null_mut},
};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree, HANDLE, WAIT_FAILED},
    Security::{
        Authorization::ConvertStringSidToSidW, DuplicateTokenEx, GetLengthSid, GetSidSubAuthority,
        GetSidSubAuthorityCount, GetTokenInformation, SecurityImpersonation, SetTokenInformation,
        TokenIntegrityLevel, TokenPrimary, SID_AND_ATTRIBUTES, TOKEN_ADJUST_DEFAULT,
        TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
    },
    System::Threading::{
        CreateProcessWithTokenW, GetCurrentProcess, OpenProcessToken, WaitForSingleObject,
        CREATE_UNICODE_ENVIRONMENT, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
    },
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const MEDIUM_INTEGRITY_RID: u32 = 0x2000;
#[cfg(windows)]
const MAXIMUM_ALLOWED: u32 = 0x0200_0000;
#[cfg(windows)]
const SE_GROUP_INTEGRITY: u32 = 0x20;

pub enum CopilotProcess {
    Standard(Child),
    #[cfg(windows)]
    Token {
        handle: HANDLE,
        id: u32,
    },
}

impl CopilotProcess {
    pub fn id(&self) -> u32 {
        match self {
            Self::Standard(child) => child.id(),
            #[cfg(windows)]
            Self::Token { id, .. } => *id,
        }
    }

    pub fn wait(&mut self) -> Result<()> {
        match self {
            Self::Standard(child) => {
                child.wait().context("等待 GitHub Copilot 退出失败")?;
                Ok(())
            }
            #[cfg(windows)]
            Self::Token { handle, .. } => {
                let result = unsafe { WaitForSingleObject(*handle, INFINITE) };
                if result == WAIT_FAILED {
                    return Err(std::io::Error::last_os_error())
                        .context("等待 GitHub Copilot 退出失败");
                }
                Ok(())
            }
        }
    }

    pub fn terminate(&mut self) -> Result<()> {
        match self {
            Self::Standard(child) => {
                child.kill().context("无法关闭 GitHub Copilot")?;
                child.wait().context("等待 GitHub Copilot 关闭失败")?;
                Ok(())
            }
            #[cfg(windows)]
            Self::Token { handle, .. } => {
                if unsafe { windows_sys::Win32::System::Threading::TerminateProcess(*handle, 1) }
                    == 0
                {
                    return Err(std::io::Error::last_os_error()).context("无法关闭 GitHub Copilot");
                }
                Ok(())
            }
        }
    }
}

#[cfg(windows)]
impl Drop for CopilotProcess {
    fn drop(&mut self) {
        if let Self::Token { handle, .. } = self {
            unsafe {
                CloseHandle(*handle);
            }
        }
    }
}

#[cfg(windows)]
struct WinHandle(HANDLE);

#[cfg(windows)]
impl Drop for WinHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

pub fn discover_executable(config: &AppConfig) -> Result<PathBuf> {
    discover_executable_from_candidates(config, default_candidates())
}

pub fn resource_base_dir(launcher_path: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if launcher_path.parent().is_none() {
            bail!("启动器路径无效");
        }
        Ok(crate::macos::resource_base_dir(launcher_path))
    }
    #[cfg(not(target_os = "macos"))]
    {
        launcher_path
            .parent()
            .map(PathBuf::from)
            .context("启动器路径无效")
    }
}

pub fn data_directory(fallback: &Path) -> PathBuf {
    #[cfg(windows)]
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_app_data).join("GitHubCopilotZh");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("GitHubCopilotZh");
    }
    fallback.join("GitHubCopilotZh")
}

pub fn discover_executable_from_candidates(
    config: &AppConfig,
    candidates: Vec<PathBuf>,
) -> Result<PathBuf> {
    if let Some(path) = &config.copilot_executable {
        let normalized = normalize_candidate(path);
        if normalized.is_file() {
            return Ok(normalized);
        }
    }

    if let Some(path) = candidates
        .into_iter()
        .map(|path| normalize_candidate(&path))
        .find(|path| path.is_file())
    {
        return Ok(path);
    }

    bail!("未找到 GitHub Copilot App。请在 config.json 中设置 copilot_executable。")
}

pub fn default_candidates() -> Vec<PathBuf> {
    let mut paths = running_github_paths();
    #[cfg(windows)]
    {
        paths.extend(registry_install_paths());

        if let Some(local) = env::var_os("LOCALAPPDATA") {
            paths.push(PathBuf::from(&local).join(r"Programs\GitHub Copilot\github.exe"));
            paths.push(PathBuf::from(&local).join(r"GitHub Copilot\github.exe"));
        }
        if let Some(program_files) = env::var_os("ProgramFiles") {
            paths.push(PathBuf::from(program_files).join(r"GitHub Copilot\github.exe"));
        }
    }
    #[cfg(target_os = "macos")]
    paths.extend(crate::macos::default_candidates());

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
        .filter(|process| {
            #[cfg(windows)]
            {
                process.name().eq_ignore_ascii_case("github.exe")
            }
            #[cfg(target_os = "macos")]
            {
                process.exe().is_some_and(crate::macos::is_github_bundle)
            }
            #[cfg(not(any(windows, target_os = "macos")))]
            {
                false
            }
        })
        .filter_map(|process| process.exe().map(Path::to_path_buf))
        .collect()
}

fn normalize_candidate(path: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        crate::macos::bundle_executable_path(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        path.to_path_buf()
    }
}

#[cfg(windows)]
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
$shell = New-Object -ComObject WScript.Shell
@(
  [Environment]::GetFolderPath('Desktop'),
  [Environment]::GetFolderPath('CommonDesktopDirectory')
) | Where-Object { $_ } | Select-Object -Unique | ForEach-Object {
  $shortcut = Join-Path $_ 'GitHub Copilot.lnk'
  if (Test-Path -LiteralPath $shortcut -PathType Leaf) {
    try { $shell.CreateShortcut($shortcut).TargetPath } catch {}
  }
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

pub fn launch_copilot(path: &Path, port: u16) -> Result<CopilotProcess> {
    #[cfg(windows)]
    {
        let existing = env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
        let args = webview_arguments(&existing, port);

        if needs_medium_integrity_launch(current_integrity_rid()?) {
            return launch_with_medium_integrity(path, &args);
        }

        let mut command = Command::new(path);
        command.env("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", args);
        command.creation_flags(0);
        command
            .spawn()
            .map(CopilotProcess::Standard)
            .with_context(|| format!("无法启动 {}", path.display()))
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new(path);
        for (key, value) in crate::macos::inspector_environment(port) {
            command.env(key, value);
        }
        if let Some(parent) = path.parent() {
            command.current_dir(parent);
        }
        command
            .spawn()
            .map(CopilotProcess::Standard)
            .with_context(|| format!("无法启动 {}", path.display()))
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (path, port);
        bail!("当前系统暂不支持 GitHub Copilot 中文启动器")
    }
}

#[cfg(windows)]
fn needs_medium_integrity_launch(integrity_rid: u32) -> bool {
    integrity_rid > MEDIUM_INTEGRITY_RID
}

#[cfg(windows)]
fn current_integrity_rid() -> Result<u32> {
    let mut raw_token = null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0 {
        return Err(std::io::Error::last_os_error()).context("无法读取当前进程权限级别");
    }
    let token = WinHandle(raw_token);
    let mut required = 0;
    unsafe {
        GetTokenInformation(token.0, TokenIntegrityLevel, null_mut(), 0, &mut required);
    }
    if required == 0 {
        return Err(std::io::Error::last_os_error()).context("无法读取当前进程完整性信息");
    }

    let mut buffer = vec![0u8; required as usize];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("无法读取当前进程完整性信息");
    }

    let label =
        unsafe { std::ptr::read_unaligned(buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
    let count = unsafe { *GetSidSubAuthorityCount(label.Label.Sid) };
    if count == 0 {
        bail!("当前进程完整性 SID 无效")
    }
    Ok(unsafe { *GetSidSubAuthority(label.Label.Sid, u32::from(count - 1)) })
}

#[cfg(windows)]
fn launch_with_medium_integrity(path: &Path, webview_args: &str) -> Result<CopilotProcess> {
    let mut raw_source = null_mut();
    let access = TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT;
    if unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut raw_source) } == 0 {
        return Err(std::io::Error::last_os_error()).context("无法打开启动器进程令牌");
    }
    let source = WinHandle(raw_source);

    let mut raw_token = null_mut();
    if unsafe {
        DuplicateTokenEx(
            source.0,
            MAXIMUM_ALLOWED,
            null(),
            SecurityImpersonation,
            TokenPrimary,
            &mut raw_token,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("无法创建 GitHub Copilot 进程令牌");
    }
    let token = WinHandle(raw_token);

    let medium_sid_text = OsString::from("S-1-16-8192")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut medium_sid = null_mut();
    if unsafe { ConvertStringSidToSidW(medium_sid_text.as_ptr(), &mut medium_sid) } == 0 {
        return Err(std::io::Error::last_os_error()).context("无法创建中等完整性 SID");
    }
    let label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: medium_sid,
            Attributes: SE_GROUP_INTEGRITY,
        },
    };
    let label_size =
        size_of::<TOKEN_MANDATORY_LABEL>() + unsafe { GetLengthSid(medium_sid) } as usize;
    let set_result = unsafe {
        SetTokenInformation(
            token.0,
            TokenIntegrityLevel,
            (&label as *const TOKEN_MANDATORY_LABEL).cast(),
            label_size as u32,
        )
    };
    unsafe {
        LocalFree(medium_sid);
    }
    if set_result == 0 {
        return Err(std::io::Error::last_os_error())
            .context("无法降低 GitHub Copilot 进程完整性级别");
    }

    let application = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut command_line = OsString::from(format!("\"{}\"", path.display()))
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let current_directory = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let environment = build_environment_block_from(env::vars_os(), webview_args);
    let mut startup: STARTUPINFOW = unsafe { zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    let mut process_info: PROCESS_INFORMATION = unsafe { zeroed() };

    let created = unsafe {
        CreateProcessWithTokenW(
            token.0,
            0,
            application.as_ptr(),
            command_line.as_mut_ptr(),
            CREATE_UNICODE_ENVIRONMENT,
            environment.as_ptr().cast(),
            current_directory.as_ptr(),
            &startup,
            &mut process_info,
        )
    };
    if created == 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("无法以中等完整性启动 {}", path.display()));
    }
    unsafe {
        CloseHandle(process_info.hThread);
    }
    Ok(CopilotProcess::Token {
        handle: process_info.hProcess,
        id: process_info.dwProcessId,
    })
}

#[cfg(windows)]
fn build_environment_block_from<I>(variables: I, webview_args: &str) -> Vec<u16>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut entries = variables
        .into_iter()
        .filter(|(key, _)| {
            !key.to_string_lossy()
                .eq_ignore_ascii_case("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
        })
        .collect::<Vec<_>>();
    entries.push((
        OsString::from("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"),
        OsString::from(webview_args),
    ));
    entries.sort_by(|(left, _), (right, _)| {
        left.to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&right.to_string_lossy().to_ascii_lowercase())
    });

    let mut block = Vec::new();
    for (key, value) in entries {
        block.extend(key.encode_wide());
        block.push('=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

#[cfg(windows)]
pub fn product_version(path: &Path) -> Result<String> {
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Write-Output (Get-Item -LiteralPath $env:COPILOT_ZH_VERSION_PATH).VersionInfo.ProductVersion
"#;
    let encoded = crate::signature::encode_powershell(script);
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .env("COPILOT_ZH_VERSION_PATH", path);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.output().context("无法读取 GitHub Copilot 版本")?;
    if !output.status.success() {
        bail!("GitHub Copilot 版本查询失败")
    }
    parse_product_version(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
pub fn product_version(path: &Path) -> Result<String> {
    crate::macos::product_version(path)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn product_version(_path: &Path) -> Result<String> {
    bail!("当前系统无法读取 GitHub Copilot 版本")
}

pub fn parse_product_version(output: &str) -> Result<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
        .context("GitHub Copilot 版本信息为空")
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

#[cfg(windows)]
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
    #[cfg(windows)]
    use super::{build_environment_block_from, needs_medium_integrity_launch};
    use super::{discover_executable_from_candidates, parse_product_version, webview_arguments};
    use crate::config::AppConfig;
    #[cfg(windows)]
    use std::ffi::OsString;
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

    #[test]
    fn product_version_parser_trims_process_output() {
        assert_eq!(parse_product_version("1.1.15\r\n").unwrap(), "1.1.15");
        assert!(parse_product_version("\r\n").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn environment_block_replaces_existing_webview_arguments() {
        use std::os::windows::ffi::OsStringExt;

        let block = build_environment_block_from(
            vec![
                (OsString::from("Path"), OsString::from(r"C:\\Windows")),
                (
                    OsString::from("webview2_additional_browser_arguments"),
                    OsString::from("--old"),
                ),
            ],
            "--remote-debugging-port=43923",
        );
        assert!(block.ends_with(&[0, 0]));

        let entries = block[..block.len() - 1]
            .split(|unit| *unit == 0)
            .filter(|entry| !entry.is_empty())
            .map(|entry| OsString::from_wide(entry).to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(entries.contains(&r"Path=C:\\Windows".to_owned()));
        assert!(entries.contains(
            &"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=43923".to_owned()
        ));
        assert!(!entries.iter().any(|entry| entry.ends_with("=--old")));
    }

    #[cfg(windows)]
    #[test]
    fn elevated_processes_require_a_medium_integrity_child() {
        assert!(!needs_medium_integrity_launch(0x1000));
        assert!(!needs_medium_integrity_launch(0x2000));
        assert!(needs_medium_integrity_launch(0x3000));
        assert!(needs_medium_integrity_launch(0x4000));
    }
}
