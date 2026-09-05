# GitHub Copilot 中文版启动器

这是一个适用于 Windows 的非官方、本地汉化层。它通过独立快捷方式启动官方 GitHub Copilot App，并在 WebView2 页面加载后注入离线简体中文词典。

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

## 说明

本项目与 GitHub, Inc. 无隶属或授权关系，不分发 GitHub Copilot 的可执行文件、图标或应用资源。它只在本机调用用户已经安装并通过签名验证的官方应用。
