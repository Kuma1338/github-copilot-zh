$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$installRoot = Join-Path $env:LOCALAPPDATA 'GitHubCopilotZh'
$shortcut = Join-Path ([Environment]::GetFolderPath('Desktop')) 'GitHub Copilot 中文版.lnk'
$installRootBefore = Test-Path -LiteralPath $installRoot
$shortcutBefore = Test-Path -LiteralPath $shortcut

$installOutput = & (Join-Path $projectRoot 'scripts\install.ps1') -DryRun -SourceRoot $projectRoot | Out-String
if ($installOutput -notmatch 'DRY_RUN=1') { throw 'Install dry-run marker is missing.' }
if ($installOutput -notmatch 'SIGNATURE=Valid') { throw 'Install dry-run did not validate the GitHub signature.' }
if ($installOutput -notmatch 'COPILOT=.*github\.exe') { throw 'Install dry-run did not report the Copilot executable.' }
if ($installOutput -notmatch 'DESTINATION=.*GitHubCopilotZh') { throw 'Install dry-run did not report the destination.' }
if ($installOutput -notmatch 'SHORTCUT=.*GitHub Copilot 中文版\.lnk') { throw 'Install dry-run did not report the shortcut.' }

$uninstallOutput = & (Join-Path $projectRoot 'scripts\uninstall.ps1') -DryRun | Out-String
if ($uninstallOutput -notmatch 'DRY_RUN=1') { throw 'Uninstall dry-run marker is missing.' }
if ($uninstallOutput -notmatch 'REMOVE_ROOT=.*GitHubCopilotZh') { throw 'Uninstall dry-run root is missing.' }
if ($uninstallOutput -notmatch 'REMOVE_SHORTCUT=.*GitHub Copilot 中文版\.lnk') { throw 'Uninstall dry-run shortcut is missing.' }

if ((Test-Path -LiteralPath $installRoot) -ne $installRootBefore) { throw 'Dry-run changed the installation directory.' }
if ((Test-Path -LiteralPath $shortcut) -ne $shortcutBefore) { throw 'Dry-run changed the shortcut.' }

Write-Output 'Installer dry-run checks passed.'
