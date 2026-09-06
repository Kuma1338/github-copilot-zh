[CmdletBinding()]
param(
    [string]$Version = '0.2.0',
    [string]$OutputDirectory = 'dist'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Push-Location $projectRoot
try {
    $status = git status --porcelain
    if ($LASTEXITCODE -ne 0) { throw '无法读取 Git 工作树状态。' }
    if ($status) { throw "打包前工作树必须干净：`n$status" }

    npm test
    if ($LASTEXITCODE -ne 0) { throw 'JavaScript 测试失败。' }
    cargo test
    if ($LASTEXITCODE -ne 0) { throw 'Rust 测试失败。' }
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust 发布构建失败。' }

    $artifactRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot "artifacts\GitHubCopilotZh-$Version-win-x64"))
    $artifactsParent = [IO.Path]::GetFullPath((Join-Path $projectRoot 'artifacts')).TrimEnd('\')
    if (-not $artifactRoot.StartsWith("$artifactsParent\", [StringComparison]::OrdinalIgnoreCase)) {
        throw "打包暂存路径验证失败：$artifactRoot"
    }
    if (Test-Path -LiteralPath $artifactRoot) { Remove-Item -LiteralPath $artifactRoot -Recurse -Force }
    New-Item -ItemType Directory -Path (Join-Path $artifactRoot 'localization') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $artifactRoot 'scripts') -Force | Out-Null

    Copy-Item -LiteralPath 'target\release\copilot-zh.exe' -Destination $artifactRoot
    Copy-Item -LiteralPath 'localization\runtime.js' -Destination (Join-Path $artifactRoot 'localization')
    Copy-Item -LiteralPath 'localization\zh-CN.json' -Destination (Join-Path $artifactRoot 'localization')
    Copy-Item -LiteralPath 'scripts\install.ps1' -Destination (Join-Path $artifactRoot 'scripts')
    Copy-Item -LiteralPath 'scripts\uninstall.ps1' -Destination (Join-Path $artifactRoot 'scripts')
    Copy-Item -LiteralPath '安装.cmd' -Destination $artifactRoot
    Copy-Item -LiteralPath '卸载.cmd' -Destination $artifactRoot
    Copy-Item -LiteralPath 'README.md' -Destination $artifactRoot

    $hashLines = Get-ChildItem -LiteralPath $artifactRoot -Recurse -File |
        Sort-Object FullName |
        ForEach-Object {
            $relative = $_.FullName.Substring($artifactRoot.Length + 1).Replace('\', '/')
            $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$hash  $relative"
        }
    [IO.File]::WriteAllLines((Join-Path $artifactRoot 'SHA256SUMS.txt'), $hashLines, [Text.UTF8Encoding]::new($false))

    $outputRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot $OutputDirectory))
    New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
    $zipPath = Join-Path $outputRoot "GitHubCopilotZh-$Version-win-x64.zip"
    if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
    Compress-Archive -Path (Join-Path $artifactRoot '*') -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Output $zipPath
} finally {
    Pop-Location
}
