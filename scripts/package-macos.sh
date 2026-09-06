#!/bin/bash
set -euo pipefail

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

version="0.2.0"
target="$(rustc -vV | /usr/bin/awk '/^host:/{print $2}')"
output_directory="dist"

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --target) target="$2"; shift 2 ;;
        --output) output_directory="$2"; shift 2 ;;
        *) printf '未知参数：%s\n' "$1" >&2; exit 2 ;;
    esac
done

case "$target" in
    aarch64-apple-darwin) architecture="arm64" ;;
    x86_64-apple-darwin) architecture="x86_64" ;;
    *) printf '不支持的 macOS target：%s\n' "$target" >&2; exit 2 ;;
esac
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]]; then
    printf '版本号格式无效：%s\n' "$version" >&2
    exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
project_root="$(cd -- "${script_dir}/.." && pwd -P)"
cd "$project_root"

for required in macos/Info.plist localization/runtime.js localization/zh-CN.json scripts/install-macos.sh scripts/uninstall-macos.sh README.md; do
    if [[ ! -f "$required" ]]; then
        printf '缺少打包文件：%s\n' "$required" >&2
        exit 1
    fi
done

if [[ -n "$(git status --porcelain)" ]]; then
    printf '打包前工作树必须干净。\n' >&2
    exit 1
fi
npm test
cargo test --target "$target"
cargo build --release --target "$target"

artifact_name="GitHubCopilotZh-${version}-macos-${architecture}"
artifact_root="${project_root}/artifacts/${artifact_name}"
app_root="${artifact_root}/GitHub Copilot 中文版.app"
output_root="${project_root}/${output_directory}"
zip_path="${output_root}/${artifact_name}.zip"

/bin/rm -rf -- "$artifact_root"
/bin/mkdir -p "${app_root}/Contents/MacOS" "${app_root}/Contents/Resources/localization" "${app_root}/Contents/Resources/scripts" "${artifact_root}/scripts" "$output_root"
/usr/bin/sed "s/__VERSION__/${version}/g" macos/Info.plist > "${app_root}/Contents/Info.plist"
/usr/bin/plutil -lint "${app_root}/Contents/Info.plist" >/dev/null
/bin/cp "target/${target}/release/copilot-zh" "${app_root}/Contents/MacOS/GitHubCopilotZh"
/bin/chmod 755 "${app_root}/Contents/MacOS/GitHubCopilotZh"
/bin/cp localization/runtime.js localization/zh-CN.json "${app_root}/Contents/Resources/localization/"
/bin/cp scripts/uninstall-macos.sh "${app_root}/Contents/Resources/scripts/"
/bin/cp "scripts/install-macos.sh" "scripts/uninstall-macos.sh" "${artifact_root}/scripts/"
/bin/cp README.md "${artifact_root}/README.md"
/bin/cp scripts/install-macos.sh "${artifact_root}/安装.command"
/bin/cp scripts/uninstall-macos.sh "${artifact_root}/卸载.command"
/bin/chmod 755 "${artifact_root}/安装.command" "${artifact_root}/卸载.command"

if /usr/bin/codesign --force --deep --sign - "$app_root" >/dev/null 2>&1; then
    printf '已对本地启动器执行 ad-hoc 签名。\n'
fi

/bin/rm -f -- "${artifact_root}/SHA256SUMS.txt"
(
    cd "$artifact_root"
    /usr/bin/find . -type f -not -path './SHA256SUMS.txt' -print | LC_ALL=C /usr/bin/sort | while IFS= read -r file; do
        relative="${file#./}"
        hash="$(/usr/bin/shasum -a 256 "$file" | /usr/bin/awk '{print $1}')"
        printf '%s  %s\n' "$hash" "$relative"
    done
) > "${artifact_root}/SHA256SUMS.txt"

/bin/rm -f -- "$zip_path"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$artifact_root" "$zip_path"
printf '%s\n' "$zip_path"
