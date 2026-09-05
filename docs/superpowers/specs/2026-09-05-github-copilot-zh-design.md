# GitHub Copilot App Chinese Launcher Design

## Goal

Provide a reversible Windows localization layer for the official GitHub Copilot App. The tool translates application chrome into Simplified Chinese without modifying `github.exe`, reading GitHub credentials, or translating repository and conversation content.

The first supported target is GitHub Copilot App 1.1.15 installed at a user-selected path. The launcher must also discover common future install locations and report a clear error when no supported executable is found.

## Deliverables

- A Windows launcher named `GitHub Copilot 中文版`.
- An offline Simplified Chinese translation dictionary.
- A runtime DOM localization injector.
- Installer and uninstaller scripts.
- A desktop shortcut created by the installer.
- A ZIP package with all runtime files and Chinese documentation.
- A compatibility report for the detected Copilot version.

## Architecture

The solution has four units:

1. **Installer** copies the package into `%LOCALAPPDATA%\GitHubCopilotZh`, creates a desktop shortcut, and records only files owned by this tool.
2. **Launcher** finds the official executable, verifies that it is signed by GitHub, selects an unused loopback port, and starts the app with WebView2 remote debugging enabled on that port.
3. **Injector** connects to the WebView2 DevTools endpoint on `127.0.0.1`, installs the localization script into each application page, and remains active while the Copilot process is running.
4. **Localization runtime** translates allowlisted UI strings and selected accessibility attributes, then watches newly inserted DOM nodes for matching strings.

The launcher never patches or replaces the official executable. The original desktop shortcut remains usable and starts the untouched English application.

## Startup Flow

1. Detect whether GitHub Copilot is already running.
2. If it is running without the localization injector, show a Chinese message asking the user to close it normally, then exit without terminating the process.
3. Resolve the executable from the saved configuration, the current observed path, uninstall metadata, and common install paths.
4. Validate the executable's Authenticode signature and signer subject.
5. Allocate an available loopback TCP port and start the injector.
6. Start the official app with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>` scoped to the child process.
7. Poll the local DevTools endpoint with a bounded timeout and attach to every relevant WebView target.
8. Inject the localization runtime immediately and register it for future document loads.
9. Stop the injector when the application exits.

A second launcher invocation while Copilot is running shows the same close-and-retry message and exits cleanly.

## Translation Rules

Localization is dictionary driven and offline. Exact source strings are preferred because they minimize accidental translation.

The runtime may translate:

- Navigation labels, page headings, buttons, menus, tabs, empty states, tooltips, placeholders, and status labels.
- `aria-label`, `title`, and `placeholder` attributes when the full value is allowlisted.
- Controlled parameterized messages whose variable positions are explicitly defined in the dictionary.

The runtime must skip:

- `code`, `pre`, `textarea`, terminal surfaces, editors, diffs, and content-editable elements.
- Conversation messages, prompts, generated answers, issue and pull request bodies, filenames, repository names, branch names, command output, and user-provided content.
- Unknown strings. They remain in English until explicitly added to the dictionary.

The initial dictionary covers the visible Home, My work, Automations, Customize, Chats, Settings, and common dialog flows in version 1.1.15. Dictionary entries include context notes where the same English word has multiple possible translations.

## Security And Privacy

- All translation occurs locally. The tool makes no external network requests.
- The DevTools endpoint binds through the browser's loopback behavior and uses a newly selected port per launch.
- The injector connects only to `127.0.0.1` and only while the launched Copilot process is active.
- The tool does not inspect cookies, local storage, authentication data, clipboard contents, repositories, or chat payloads.
- Signature validation fails closed when the selected executable is unsigned or is not signed by GitHub, Inc.
- Logs contain version, target attachment, translation counts, and errors; they exclude DOM text and account data.

## Installation And Removal

Installation copies files into `%LOCALAPPDATA%\GitHubCopilotZh`. The desktop shortcut targets the launcher and uses the official Copilot icon when available.

Uninstallation removes the shortcut and the tool-owned installation directory. It does not touch the official application, GitHub configuration, WebView2 data, projects, or credentials. Removing the localization tool therefore restores normal English startup immediately.

## Updates And Compatibility

Each launch records the detected Copilot product version. A known-version manifest marks versions that have been tested. Unknown versions are allowed to start in compatibility mode because exact dictionary matching is low risk, but the user receives a concise notice and a compatibility report is written locally.

When GitHub changes source labels, unmatched text stays in English. Translation rules never fall back to machine translation or broad substring replacement.

## Error Handling

- Missing app: show the searched locations and explain how to set an explicit executable path in `config.json`.
- Invalid signature: refuse to launch through the Chinese shortcut and explain how to use the original app.
- Occupied or unavailable port: retry with a different ephemeral port up to a fixed limit.
- DevTools unavailable: start failure is reported and the injector exits; the user can still launch the original shortcut.
- Injection failure in one target: log the target type and continue attaching to other targets.
- Dictionary parse failure: refuse localization and report the exact file and JSON error.

## Packaging Choice

The launcher and injector will be implemented as one native Rust Windows GUI executable so normal use does not show a console window or require a separate runtime. The package keeps the dictionary external so translations can be updated without rebuilding the launcher. The installer and uninstaller are signed-readable local scripts invoked only during setup and removal; normal launches do not depend on PowerShell execution policy.

## Verification

Automated checks cover:

- Exact and parameterized dictionary matching.
- Exclusion of code, terminal, editable, and conversation containers.
- Executable discovery and GitHub signature validation.
- Configuration and malformed dictionary handling.
- Installer ownership and uninstall manifest behavior.

End-to-end verification on this computer covers:

- Installation into the per-user directory and desktop shortcut creation.
- Launching the signed GitHub Copilot App 1.1.15 through the Chinese shortcut.
- Chinese labels on Home, My work, Automations, Customize, Chats, and Settings.
- Dynamic dialogs translated after navigation.
- Code, repository names, prompts, and chat responses left unchanged.
- Original shortcut still launching the unmodified app.
- Uninstall removing only localization files and its shortcut.

## Acceptance Criteria

The work is accepted when the ZIP installs without administrator rights, the desktop shortcut starts the official signed app with the localization layer, the specified primary screens are translated, protected content is unchanged, the original executable hash remains unchanged, and uninstall removes all localization files and restores the pre-install state.
