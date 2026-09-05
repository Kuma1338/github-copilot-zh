# GitHub Copilot 中文版启动器

这是一个适用于 Windows 的非官方、本地汉化层。它通过独立快捷方式启动官方 GitHub Copilot App，并在 WebView2 页面加载后注入离线简体中文词典。

本项目与 GitHub, Inc. 没有隶属、赞助或授权关系。项目不包含、不修改、不替换、不分发官方 GitHub Copilot 二进制文件、图标或应用资源；使用者必须自行安装官方应用，并自行确认其服务条款和软件许可。GitHub 和 GitHub Copilot 是其各自所有者的商标。

## 特点

- 不修改、不替换官方 `github.exe`，GitHub 数字签名保持有效。
- 不读取 GitHub 登录信息、令牌、Cookie、剪贴板、仓库或聊天内容。
- 不调用在线翻译服务；运行时没有外部网络请求。
- 只翻译词典中明确列出的界面文字。
- 代码、终端、编辑器、聊天正文、提示词、文件名、仓库名和分支名保持原样。
- 原来的 GitHub Copilot 快捷方式仍然启动英文原版。

## 系统要求

- Windows 10 或 Windows 11 x64。
- 已安装官方 GitHub Copilot App。
- 当前已验证版本：`1.1.15`。

## 平台支持

当前版本仅支持 Windows x64。macOS 不能直接运行此版本，因为启动器、安装器、快捷方式、签名校验和进程启动逻辑均使用 Windows API、PowerShell、Authenticode 和 WebView2。中文词典与页面注入脚本可以复用；若官方 Copilot App 提供 macOS 版本，仍需为 macOS 重新实现应用发现、签名校验、启动参数、调试接口连接和安装入口，并针对 WKWebView 或官方实际使用的网页容器重新验证。

## 安装

1. 解压整个 ZIP，不能只取出单个 EXE。
2. 双击 `安装.cmd`。
3. 安装器会验证官方程序的 Authenticode 签名。
4. 桌面出现 `GitHub Copilot 中文版` 快捷方式后，先从系统托盘退出正在运行的 Copilot，再双击该快捷方式。

安装位置为 `%LOCALAPPDATA%\GitHubCopilotZh`，不需要管理员权限。

## 使用与更新

每次需要中文界面时，从 `GitHub Copilot 中文版` 快捷方式启动。若直接使用官方快捷方式，应用仍显示英文。

GitHub Copilot 更新到未验证版本时，启动器会提示进入兼容模式。精确匹配仍会工作，新的或变更过的英文标签保持英文。更新汉化包时，重新运行新版 `安装.cmd` 即可覆盖工具自身文件。

词典位于 `%LOCALAPPDATA%\GitHubCopilotZh\localization\zh-CN.json`。可以添加明确的完整英文文本与中文译文；请勿使用宽泛的片段替换，以免影响用户内容。

## 常见问题

**提示 Copilot 正在运行**

先从系统托盘菜单退出 GitHub Copilot，再重新双击中文快捷方式。只关闭窗口时，官方应用可能仍在后台运行；启动器不会强制结束官方应用。

**仍有部分英文**

这是允许列表策略的预期行为。未知文字会保持英文，避免误改代码、仓库数据或聊天内容。

**提示 WebView2 调试接口没有响应**

确认所有 GitHub Copilot 进程已关闭后重试。若组织策略禁用了 WebView2 远程调试，该汉化方式无法工作，可继续使用官方英文快捷方式。

**查看诊断信息**

日志位于 `%LOCALAPPDATA%\GitHubCopilotZh\logs`，只包含应用版本、是否已验证、附着目标数和错误计数，不包含页面文字或账号信息。

## 卸载

先从系统托盘退出 GitHub Copilot，然后双击安装包中的 `卸载.cmd`，或运行 `%LOCALAPPDATA%\GitHubCopilotZh\卸载.cmd`。卸载只删除汉化目录和 `GitHub Copilot 中文版` 桌面快捷方式，不修改官方应用或用户数据。

## 许可证

本项目代码使用 MIT License，详见 [`LICENSE`](LICENSE)。第三方依赖仍受其各自许可证约束。
