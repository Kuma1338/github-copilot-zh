#!/bin/bash
set -euo pipefail

dry_run=0
for argument in "$@"; do
    case "$argument" in
        --dry-run) dry_run=1 ;;
        *) printf '未知参数：%s\n' "$argument" >&2; exit 2 ;;
    esac
done

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
source_root="$(cd -- "${script_dir}/.." && pwd -P)"
source_app="${source_root}/GitHub Copilot 中文版.app"
if [[ -n "${COPILOT_APP_PATH:-}" ]]; then
    official_app="$COPILOT_APP_PATH"
else
    official_app=""
    for candidate in "/Applications/GitHub Copilot.app" "${HOME}/Applications/GitHub Copilot.app"; do
        if [[ -d "$candidate" ]]; then
            official_app="$candidate"
            break
        fi
    done
    if [[ -z "$official_app" ]]; then
        official_app="/Applications/GitHub Copilot.app"
    fi
fi
install_root="${HOME}/Applications/GitHub Copilot 中文版.app"
resources_root="${install_root}/Contents/Resources"
config_path="${resources_root}/config.json"

if [[ "$install_root" != "${HOME}/Applications/GitHub Copilot 中文版.app" ]]; then
    printf '安装路径验证失败。\n' >&2
    exit 1
fi

if [[ ! -d "$source_app" ]]; then
    printf '缺少 macOS 应用包：%s\n请先运行 scripts/package-macos.sh。\n' "$source_app" >&2
    exit 1
fi
if [[ ! -d "$official_app" ]]; then
    printf '未找到官方 GitHub Copilot：%s\n可通过 COPILOT_APP_PATH 指定 .app 路径。\n' "$official_app" >&2
    exit 1
fi
if ! /usr/bin/codesign --verify --deep --strict "$official_app" >/dev/null 2>&1; then
    printf '官方 GitHub Copilot 未通过 codesign 校验：%s\n' "$official_app" >&2
    exit 1
fi
if ! /usr/sbin/spctl --assess --type execute "$official_app" >/dev/null 2>&1; then
    printf '官方 GitHub Copilot 未通过 macOS Gatekeeper 校验：%s\n' "$official_app" >&2
    exit 1
fi
signature_details="$(/usr/bin/codesign -dv --verbose=4 "$official_app" 2>&1 || true)"
if ! printf '%s\n' "$signature_details" | /usr/bin/grep -Fq 'Identifier=com.github.githubapp'; then
    printf '应用标识不是官方 GitHub Copilot：%s\n' "$official_app" >&2
    exit 1
fi
if ! printf '%s\n' "$signature_details" | /usr/bin/grep -Fq 'TeamIdentifier=VEKTX9H2N7'; then
    printf '应用 Team ID 不是 GitHub：%s\n' "$official_app" >&2
    exit 1
fi

printf '官方应用：%s\n安装位置：%s\n签名：codesign 有效，Team ID VEKTX9H2N7\n' "$official_app" "$install_root"
if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY_RUN=1\n'
    exit 0
fi

/bin/mkdir -p "${HOME}/Applications"
/bin/rm -rf -- "$install_root"
/usr/bin/ditto --rsrc --extattr "$source_app" "$install_root"

json_path="${official_app//\\/\\\\}"
json_path="${json_path//\"/\\\"}"
/bin/mkdir -p "$resources_root"
/usr/bin/printf '{\n  "copilot_executable": "%s"\n}\n' "$json_path" > "$config_path"
/bin/chmod 600 "$config_path"

printf '安装完成：%s\n请从“应用程序”或 Finder 打开该应用。首次启动前请先退出正在运行的官方 Copilot。\n' "$install_root"
