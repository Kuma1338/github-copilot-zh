[CmdletBinding()]
param([switch]$DryRun)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$localRoot = [IO.Path]::GetFullPath($env:LOCALAPPDATA).TrimEnd('\')
$installRoot = [IO.Path]::GetFullPath((Join-Path $localRoot 'GitHubCopilotZh')).TrimEnd('\')
$expectedRoot = "$localRoot\GitHubCopilotZh"
if (-not $installRoot.Equals($expectedRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "卸载路径验证失败：$installRoot"
}
$shortcutPath = [IO.Path]::GetFullPath((Join-Path ([Environment]::GetFolderPath('Desktop')) 'GitHub Copilot 中文版.lnk'))

if ($DryRun) {
    Write-Output 'DRY_RUN=1'
    Write-Output "REMOVE_ROOT=$installRoot"
    Write-Output "REMOVE_SHORTCUT=$shortcutPath"
    return
}

$running = Get-Process -Name 'copilot-zh' -ErrorAction SilentlyContinue |
    Where-Object { $_.Id -ne $PID }
if ($running) {
    throw '汉化启动器仍在运行。请先关闭 GitHub Copilot，再执行卸载。'
}

if (Test-Path -LiteralPath $shortcutPath) {
    Remove-Item -LiteralPath $shortcutPath -Force
}

if (Test-Path -LiteralPath $installRoot) {
    $escapedRoot = $installRoot.Replace("'", "''")
    $cleanup = @"
`$ErrorActionPreference = 'Stop'
Start-Sleep -Milliseconds 1000
`$root = [IO.Path]::GetFullPath('$escapedRoot').TrimEnd('\')
`$local = [IO.Path]::GetFullPath(`$env:LOCALAPPDATA).TrimEnd('\')
`$expected = "`$local\GitHubCopilotZh"
if (-not `$root.Equals(`$expected, [StringComparison]::OrdinalIgnoreCase)) { exit 2 }
if (Test-Path -LiteralPath `$root) { Remove-Item -LiteralPath `$root -Recurse -Force }
"@
    $bytes = [Text.Encoding]::Unicode.GetBytes($cleanup)
    $encoded = [Convert]::ToBase64String($bytes)
    Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoLogo', '-NoProfile', '-NonInteractive', '-WindowStyle', 'Hidden', '-EncodedCommand', $encoded) -WindowStyle Hidden
}

Write-Output 'GitHub Copilot 中文版已卸载。官方应用和用户数据未被修改。'
