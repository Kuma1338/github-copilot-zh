#!/bin/bash
set -euo pipefail

dry_run=0
for argument in "$@"; do
    case "$argument" in
        --dry-run) dry_run=1 ;;
        *) printf '未知参数：%s\n' "$argument" >&2; exit 2 ;;
    esac
done

install_root="${HOME}/Applications/GitHub Copilot 中文版.app"
expected_root="${HOME}/Applications/GitHub Copilot 中文版.app"
if [[ "$install_root" != "$expected_root" ]]; then
    printf '卸载路径验证失败。\n' >&2
    exit 1
fi

if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY_RUN=1\nREMOVE_ROOT=%s\n' "$install_root"
    exit 0
fi

if /usr/bin/pgrep -f "$install_root/Contents/MacOS/GitHubCopilotZh" >/dev/null 2>&1; then
    printf '汉化启动器仍在运行。请先退出 GitHub Copilot，再执行卸载。\n' >&2
    exit 1
fi
if [[ -e "$install_root" ]]; then
    /bin/rm -rf -- "$install_root"
fi
printf 'GitHub Copilot 中文版已卸载。官方应用和用户数据未被修改。\n'
