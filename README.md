# GitHub Copilot 中文版启动器

这是一个适用于 Windows 和 macOS 的非官方、本地汉化层。它通过独立入口启动官方 GitHub Copilot App，并在应用网页容器提供本地调试接口时注入离线简体中文词典。

本项目与 GitHub, Inc. 没有隶属、赞助或授权关系。项目不包含、不修改、不替换、不分发官方 GitHub Copilot 二进制文件、图标或应用资源；使用者必须自行安装官方应用，并自行确认其服务条款和软件许可。GitHub 和 GitHub Copilot 是其各自所有者的商标。

## 特点

- 不修改、不替换官方 `github.exe` 或 `GitHub Copilot.app`，官方数字签名保持有效。
- 不读取 GitHub 登录信息、令牌、Cookie、剪贴板、仓库或聊天内容。
- 不调用在线翻译服务；运行时没有外部网络请求。
- 只翻译词典中明确列出的界面文字。
- 代码、终端、编辑器、聊天正文、提示词、文件名、仓库名和分支名保持原样。
- 原来的 GitHub Copilot 入口仍然启动英文原版。

## 系统要求

- Windows 10 或 Windows 11 x64，或 macOS 12 及更高版本。
- 已安装官方 GitHub Copilot App。
- 当前词典已针对官方版本 `1.1.15` 扫描；未知版本会进入兼容模式。

## 平台支持

Windows x64 是已在本机端到端验证的平台：启动器校验 Authenticode，设置 WebView2 本地 DevTools 端口，并附着到每个页面目标。

macOS 提供 arm64 和 x86_64 实验包。启动器会发现 `GitHub Copilot.app`，读取 `Contents/Info.plist`，用 `codesign` 和 `spctl` 校验 Bundle ID `com.github.githubapp` 及 GitHub Team ID `VEKTX9H2N7`，然后设置 `WEBKIT_INSPECTOR_SERVER` 与 `WEBKIT_INSPECTOR_HTTP_SERVER` 并探测本地 Inspector 的 `/json/list`、`/json` 入口。

官方 macOS 生产版使用 WKWebView，目前没有公开稳定的 Inspector/CDP 接口。没有真实可用接口时，启动器会显示明确提示并关闭它刚启动的进程；这意味着 macOS 实验包可以验证发现、签名、版本、启动和能力探测，但不能承诺在当前官方生产版中显示中文。项目不会修改、重新签名或替换官方 App，也不会通过不稳定的私有注入方式绕过这一限制。

## 安装

1. 解压整个 Windows ZIP，不能只取出单个 EXE。
2. 双击 `安装.cmd`。
3. 安装器会验证官方程序的 Authenticode 签名。
4. 桌面出现 `GitHub Copilot 中文版` 快捷方式后，先从系统托盘退出正在运行的 Copilot，再双击该快捷方式。

安装位置为 `%LOCALAPPDATA%\GitHubCopilotZh`，不需要管理员权限。

### macOS

从 Release 下载与你的 Mac 对应的 `GitHubCopilotZh-<版本>-macos-arm64.zip` 或 `GitHubCopilotZh-<版本>-macos-x86_64.zip`，解压后双击 `安装.command`。如果 Finder 阻止执行，请右键选择“打开”，或在终端运行：

```sh
bash ./安装.command
```

安装器默认查找 `/Applications/GitHub Copilot.app`。官方 App 在其他位置时，可以用环境变量指定：

```sh
COPILOT_APP_PATH="$HOME/Applications/GitHub Copilot.app" bash ./安装.command
```

安装器会把 `GitHub Copilot 中文版.app` 复制到 `~/Applications`。首次启动前先完全退出官方 Copilot；然后从 Finder 打开中文 App。macOS 启动器是本项目的本地 ad-hoc 签名包，未使用 Apple Developer ID 签名或公证，首次打开可能需要在“系统设置 > 隐私与安全性”中允许。

## 使用与更新

每次需要中文界面时，从 `GitHub Copilot 中文版` 快捷方式启动。若直接使用官方快捷方式，应用仍显示英文。

macOS 使用 `GitHub Copilot 中文版.app` 启动；官方 App 仍可直接打开。macOS 的日志位于 `~/Library/Application Support/GitHubCopilotZh/logs`。

GitHub Copilot 更新到未验证版本时，启动器会提示进入兼容模式。精确匹配仍会工作，新的或变更过的英文标签保持英文。更新汉化包时，重新运行新版 `安装.cmd` 即可覆盖工具自身文件。

词典位于 `%LOCALAPPDATA%\GitHubCopilotZh\localization\zh-CN.json`。可以添加明确的完整英文文本与中文译文；请勿使用宽泛的片段替换，以免影响用户内容。

## 常见问题

**提示 Copilot 正在运行**

先从系统托盘菜单退出 GitHub Copilot，再重新双击中文快捷方式。只关闭窗口时，官方应用可能仍在后台运行；启动器不会强制结束官方应用。

**仍有部分英文**

这是允许列表策略的预期行为。未知文字会保持英文，避免误改代码、仓库数据或聊天内容。

**提示 WebView2 调试接口没有响应（Windows）**

确认所有 GitHub Copilot 进程已关闭后重试。若组织策略禁用了 WebView2 远程调试，该汉化方式无法工作，可继续使用官方英文快捷方式。

**macOS 提示没有可用的 Inspector/CDP 接口**

这是当前官方生产版 WKWebView 的已知兼容性限制，不代表官方 App 损坏。启动器只会在本地接口真实响应并返回可连接的页面目标时注入。可以继续使用官方英文 App，或在官方开放稳定调试接口后重新运行新版汉化包。

**macOS 提示签名或 Team ID 不匹配**

请确认指定的是官方 `GitHub Copilot.app`，而不是复制、破解或重新签名的版本。可用下面的命令查看实际 Bundle ID 和 Team ID：

```sh
codesign -dv --verbose=4 "/Applications/GitHub Copilot.app" 2>&1 | grep -E 'Identifier|TeamIdentifier'
```

**查看诊断信息**

Windows 日志位于 `%LOCALAPPDATA%\GitHubCopilotZh\logs`，macOS 日志位于 `~/Library/Application Support/GitHubCopilotZh/logs`。日志只包含应用版本、是否已验证、附着目标数和错误计数，不包含页面文字或账号信息。

## 卸载

Windows：先从系统托盘退出 GitHub Copilot，然后双击安装包中的 `卸载.cmd`，或运行 `%LOCALAPPDATA%\GitHubCopilotZh\卸载.cmd`。卸载只删除汉化目录和 `GitHub Copilot 中文版` 桌面快捷方式。

macOS：退出中文 App 后，在解压目录双击 `卸载.command`，或运行 `bash ./卸载.command`。卸载只删除 `~/Applications/GitHub Copilot 中文版.app`。两个平台都不会修改官方应用、项目、登录信息或用户数据。

## 发布包

GitHub Actions 在推送 `v*` 标签后构建并发布三个 ZIP：Windows x64、macOS arm64 和 macOS x86_64。每个包都包含 `SHA256SUMS.txt`。macOS 包是实验性支持包；它不包含官方 Copilot App，也不包含任何 GitHub 专有资源。

## 许可证

本项目代码使用 MIT License，详见 [`LICENSE`](LICENSE)。第三方依赖仍受其各自许可证约束。
