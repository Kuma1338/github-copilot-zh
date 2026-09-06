[CmdletBinding()]
param(
    [switch]$DryRun,
    [string]$SourceRoot = (Split-Path -Parent $PSScriptRoot)
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-CopilotExecutable {
    param([string]$Root)

    $candidates = [System.Collections.Generic.List[string]]::new()
    $sourceConfig = Join-Path $Root 'config.json'
    if (Test-Path -LiteralPath $sourceConfig) {
        try {
            $configured = (Get-Content -LiteralPath $sourceConfig -Raw | ConvertFrom-Json).copilot_executable
            if ($configured) { $candidates.Add([string]$configured) }
        } catch {
            throw "无法解析 $sourceConfig：$($_.Exception.Message)"
        }
    }

    Get-Process -Name github -ErrorAction SilentlyContinue | ForEach-Object {
        if ($_.Path) { $candidates.Add($_.Path) }
    }

    $shell = New-Object -ComObject WScript.Shell
    $desktopDirectories = @(
        [Environment]::GetFolderPath('Desktop'),
        [Environment]::GetFolderPath('CommonDesktopDirectory')
    ) | Where-Object { $_ } | Select-Object -Unique
    foreach ($desktopDirectory in $desktopDirectories) {
        $officialShortcut = Join-Path $desktopDirectory 'GitHub Copilot.lnk'
        if (-not (Test-Path -LiteralPath $officialShortcut -PathType Leaf)) { continue }
        try {
            $target = $shell.CreateShortcut($officialShortcut).TargetPath
            if ($target) { $candidates.Add($target) }
        } catch {
            continue
        }
    }

    $uninstallRoots = @(
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    Get-ItemProperty $uninstallRoots -ErrorAction SilentlyContinue |
        Where-Object {
            $_.PSObject.Properties['DisplayName'] -and $_.DisplayName -eq 'GitHub Copilot App'
        } |
        ForEach-Object {
            if ($_.PSObject.Properties['InstallLocation'] -and $_.InstallLocation) {
                $candidates.Add((Join-Path $_.InstallLocation 'github.exe'))
            } elseif ($_.PSObject.Properties['DisplayIcon'] -and $_.DisplayIcon) {
                $candidates.Add(($_.DisplayIcon -replace ',\d+$', ''))
            }
        }

    if ($env:LOCALAPPDATA) {
        $candidates.Add((Join-Path $env:LOCALAPPDATA 'Programs\GitHub Copilot\github.exe'))
        $candidates.Add((Join-Path $env:LOCALAPPDATA 'GitHub Copilot\github.exe'))
    }
    if ($env:ProgramFiles) {
        $candidates.Add((Join-Path $env:ProgramFiles 'GitHub Copilot\github.exe'))
    }

    foreach ($candidate in ($candidates | Select-Object -Unique)) {
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { continue }
        $item = Get-Item -LiteralPath $candidate
        if ($item.VersionInfo.ProductName -eq 'GitHub Copilot') { return $item.FullName }
    }
    throw '未找到 GitHub Copilot App。请先安装官方应用，或在安装包根目录的 config.json 中指定 copilot_executable。'
}

function Assert-GitHubSignature {
    param([string]$Path)
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid') {
        throw "官方程序签名状态无效：$($signature.Status)"
    }
    $subject = $signature.SignerCertificate.Subject
    if ($subject -notmatch '(CN|O)="?GitHub, Inc\.') {
        throw "程序签名者不是 GitHub, Inc.：$subject"
    }
    return $signature
}

$SourceRoot = [IO.Path]::GetFullPath($SourceRoot)
$launcherCandidates = @(
    (Join-Path $SourceRoot 'copilot-zh.exe'),
    (Join-Path $SourceRoot 'target\release\copilot-zh.exe')
)
$sourceLauncher = $launcherCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $sourceLauncher) { throw '缺少 copilot-zh.exe。请使用完整发布包，或先运行 cargo build --release。' }

$sourceRuntime = Join-Path $SourceRoot 'localization\runtime.js'
$sourceDictionary = Join-Path $SourceRoot 'localization\zh-CN.json'
$sourceUninstaller = Join-Path $SourceRoot 'scripts\uninstall.ps1'
$sourceUninstallCmd = Join-Path $SourceRoot '卸载.cmd'
foreach ($required in @($sourceRuntime, $sourceDictionary, $sourceUninstaller, $sourceUninstallCmd)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "安装包缺少文件：$required" }
}

$sourceVersion = Join-Path $SourceRoot 'VERSION'
$packageVersion = '0.2.0'
if (Test-Path -LiteralPath $sourceVersion -PathType Leaf) {
    $packageVersion = (Get-Content -LiteralPath $sourceVersion -Raw).Trim()
}
if ($packageVersion -notmatch '^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$') {
    throw "安装包版本号格式无效：$packageVersion"
}

$copilot = Get-CopilotExecutable -Root $SourceRoot
$signature = Assert-GitHubSignature -Path $copilot
$installRoot = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'GitHubCopilotZh'))
$shortcutPath = [IO.Path]::GetFullPath((Join-Path ([Environment]::GetFolderPath('Desktop')) 'GitHub Copilot 中文版.lnk'))

if ($DryRun) {
    Write-Output 'DRY_RUN=1'
    Write-Output "SIGNATURE=$($signature.Status)"
    Write-Output "COPILOT=$copilot"
    Write-Output "DESTINATION=$installRoot"
    Write-Output "SHORTCUT=$shortcutPath"
    return
}

$localizationRoot = Join-Path $installRoot 'localization'
$scriptsRoot = Join-Path $installRoot 'scripts'
New-Item -ItemType Directory -Path $localizationRoot -Force | Out-Null
New-Item -ItemType Directory -Path $scriptsRoot -Force | Out-Null
Copy-Item -LiteralPath $sourceLauncher -Destination (Join-Path $installRoot 'copilot-zh.exe') -Force
Copy-Item -LiteralPath $sourceRuntime -Destination (Join-Path $localizationRoot 'runtime.js') -Force
Copy-Item -LiteralPath $sourceDictionary -Destination (Join-Path $localizationRoot 'zh-CN.json') -Force
Copy-Item -LiteralPath $sourceUninstaller -Destination (Join-Path $scriptsRoot 'uninstall.ps1') -Force
Copy-Item -LiteralPath $sourceUninstallCmd -Destination (Join-Path $installRoot '卸载.cmd') -Force

$utf8NoBom = [Text.UTF8Encoding]::new($false)
$config = @{ copilot_executable = $copilot } | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $installRoot 'config.json'), $config, $utf8NoBom)
[IO.File]::WriteAllText((Join-Path $installRoot 'VERSION'), "$packageVersion`n", $utf8NoBom)
$manifest = @{
    version = $packageVersion
    installedAt = [DateTime]::UtcNow.ToString('o')
    files = @(
        'copilot-zh.exe',
        'config.json',
        'VERSION',
        'install-manifest.json',
        'localization\runtime.js',
        'localization\zh-CN.json',
        'scripts\uninstall.ps1',
        '卸载.cmd'
    )
} | ConvertTo-Json -Depth 4
[IO.File]::WriteAllText((Join-Path $installRoot 'install-manifest.json'), $manifest, $utf8NoBom)

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = Join-Path $installRoot 'copilot-zh.exe'
$shortcut.WorkingDirectory = $installRoot
$shortcut.IconLocation = "$copilot,0"
$shortcut.Description = '使用简体中文界面启动 GitHub Copilot App'
$shortcut.Save()

Write-Output "安装完成：$shortcutPath"
Write-Output '首次使用前，请先从系统托盘退出正在运行的 GitHub Copilot。'
