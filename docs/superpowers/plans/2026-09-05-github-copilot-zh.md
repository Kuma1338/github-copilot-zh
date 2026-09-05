# GitHub Copilot App Chinese Launcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build, install, and verify a reversible Simplified Chinese localization launcher for the signed GitHub Copilot App on Windows.

**Architecture:** A native Rust GUI launcher validates and starts the official executable with a loopback WebView2 DevTools port, then injects an offline allowlist-based DOM localization runtime into each app WebView. External JSON translations remain editable, while setup scripts install the tool per user and create an independent Chinese desktop shortcut.

**Tech Stack:** Rust stable, Cargo, `serde`, `serde_json`, `sysinfo`, `ureq`, `tungstenite`, `windows-sys`, browser JavaScript, Windows PowerShell 5.1, Node.js test runner with `jsdom`.

**Spec:** `docs/superpowers/specs/2026-09-05-github-copilot-zh-design.md`

## Global Constraints

- Support GitHub Copilot App 1.1.15 at a discovered or configured path.
- Never modify or replace the official `github.exe`.
- Refuse unsigned executables and executables not signed by GitHub, Inc.
- Bind DevTools through WebView2 loopback behavior and connect only to `127.0.0.1`.
- Make no external network requests at runtime.
- Translate only allowlisted application chrome; skip code, editors, terminals, conversations, prompts, repository data, filenames, branches, and user content.
- Install per user without administrator rights and remove only localization-owned files.
- Keep code comments and Git commit messages in English.

## File Map

- `Cargo.toml`: Rust package metadata and runtime dependencies.
- `src/main.rs`: process orchestration and Windows GUI entry point.
- `src/app.rs`: executable discovery, running-process checks, version lookup, and launch configuration.
- `src/signature.rs`: Authenticode verification through a noninteractive encoded PowerShell command.
- `src/cdp.rs`: loopback DevTools discovery and per-target script injection.
- `src/config.rs`: `config.json`, translation dictionary, and compatibility manifest loading.
- `src/ui.rs`: native Windows message boxes and user-facing errors.
- `localization/runtime.js`: allowlist DOM translation and mutation handling.
- `localization/zh-CN.json`: exact strings, parameterized strings, version metadata, and exclusion selectors.
- `tests/runtime.test.mjs`: DOM localization tests with `jsdom`.
- `package.json`: JavaScript test-only dependency and command.
- `scripts/install.ps1`: per-user installation and shortcut creation.
- `scripts/uninstall.ps1`: owned-file and shortcut removal.
- `安装.cmd`: clickable installer entry point.
- `卸载.cmd`: clickable uninstaller entry point.
- `README.md`: Chinese installation, usage, update, troubleshooting, privacy, and removal guide.
- `scripts/package.ps1`: release build, file staging, hashes, and ZIP creation.

---

### Task 1: Native Launcher Foundation

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/app.rs`
- Create: `src/config.rs`
- Create: `src/signature.rs`
- Create: `src/ui.rs`
- Test: unit tests inside `src/app.rs`, `src/config.rs`, and `src/signature.rs`

**Interfaces:**
- Produces: `config::AppConfig::load(base_dir: &Path) -> Result<AppConfig>`
- Produces: `app::discover_executable(config: &AppConfig) -> Result<PathBuf>`
- Produces: `app::is_copilot_running(executable: &Path) -> bool`
- Produces: `signature::verify_github_signature(path: &Path) -> Result<()>`
- Produces: `app::launch_copilot(path: &Path, port: u16) -> Result<Child>`
- Produces: `ui::show_error(title: &str, message: &str)` and `ui::show_info(title: &str, message: &str)`

- [ ] **Step 1: Scaffold the crate and write failing configuration tests**

Add tests that create temporary `config.json` files and assert UTF-8 path loading, missing-file defaults, malformed JSON errors, and explicit executable precedence:

```rust
#[test]
fn explicit_executable_path_is_loaded() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("config.json"), r#"{"copilot_executable":"C:\\Path\\To\\github.exe"}"#).unwrap();
    let config = AppConfig::load(temp.path()).unwrap();
    assert_eq!(config.copilot_executable.unwrap(), PathBuf::from(r"C:\Path\To\github.exe"));
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test config::tests app::tests signature::tests`

Expected: FAIL because the modules and interfaces do not exist.

- [ ] **Step 3: Implement configuration, discovery, signature verification, and GUI messages**

Use `sysinfo` for running-process detection. Discover the executable in this order: configured path, matching running process path, uninstall registry data queried by PowerShell, and common per-user/program-files paths. Encode the signature PowerShell script as UTF-16LE Base64 before invoking `powershell.exe -EncodedCommand`; require `Status == Valid` and a signer subject containing `O="GitHub, Inc."` or `CN="GitHub, Inc."`.

The GUI entry point must use:

```rust
#![cfg_attr(not(test), windows_subsystem = "windows")]
```

Merge any pre-existing `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` value with:

```text
--remote-debugging-port=<port> --remote-allow-origins=http://127.0.0.1:<port>
```

- [ ] **Step 4: Run launcher unit tests**

Run: `cargo test config::tests app::tests signature::tests`

Expected: PASS, including a live signature check against the locally installed `github.exe` when present.

- [ ] **Step 5: Commit the launcher foundation**

```powershell
git add Cargo.toml Cargo.lock src
git commit -m "feat: add native Copilot launcher foundation"
```

### Task 2: Allowlist Localization Runtime

**Files:**
- Create: `localization/runtime.js`
- Create: `localization/zh-CN.json`
- Create: `package.json`
- Create: `tests/runtime.test.mjs`

**Interfaces:**
- Consumes: JSON injected into `globalThis.__COPILOT_ZH_DICTIONARY__`
- Produces: `globalThis.__COPILOT_ZH__.translateRoot(root: Node): number`
- Produces: `globalThis.__COPILOT_ZH__.start(): void`
- Dictionary shape: `{ version, testedAppVersions, exact, patterns, excludedSelectors }`

- [ ] **Step 1: Write failing DOM translation tests**

Cover exact text, whitespace preservation, translated attributes, dynamically appended nodes, parameterized labels, and protected content:

```javascript
test('skips code and conversation content', () => {
  document.body.innerHTML = '<nav>Home</nav><pre>Home</pre><section data-copilot-zh-skip>Home</section>';
  api.translateRoot(document.body);
  assert.equal(document.querySelector('nav').textContent, '主页');
  assert.equal(document.querySelector('pre').textContent, 'Home');
  assert.equal(document.querySelector('section').textContent, 'Home');
});
```

- [ ] **Step 2: Run JavaScript tests and confirm they fail**

Run: `npm install` then `npm test`

Expected: FAIL because `localization/runtime.js` does not exist.

- [ ] **Step 3: Implement the DOM runtime and initial dictionary**

Implement exact full-string replacement with original leading/trailing whitespace preserved. Translate `aria-label`, `title`, and `placeholder` only on full allowlist matches. Use `MutationObserver` for added nodes and changed allowlisted attributes. Reject ancestors matching selectors such as:

```text
code, pre, textarea, [contenteditable="true"], [role="textbox"], .monaco-editor, .xterm, [data-copilot-zh-skip]
```

Add initial translations for Home, My work, Automations, Customize, Chats, Settings, automation cards, common actions, status words, dialogs, search placeholders, model controls, project controls, and update/error messages visible in 1.1.15.

- [ ] **Step 4: Run JavaScript tests**

Run: `npm test`

Expected: PASS with all protected-content assertions unchanged.

- [ ] **Step 5: Commit localization runtime**

```powershell
git add localization package.json package-lock.json tests
git commit -m "feat: add offline Chinese localization runtime"
```

### Task 3: DevTools Injection And Lifecycle

**Files:**
- Create: `src/cdp.rs`
- Modify: `src/main.rs`
- Modify: `src/config.rs`
- Test: unit tests inside `src/cdp.rs`

**Interfaces:**
- Consumes: `localization/runtime.js` and `localization/zh-CN.json` beside the installed executable.
- Produces: `cdp::allocate_loopback_port() -> Result<u16>`
- Produces: `cdp::build_injection_source(runtime: &str, dictionary: &Dictionary) -> Result<String>`
- Produces: `cdp::run_injector(port: u16, source: &str, app_pid: u32) -> Result<InjectionStats>`
- Produces: `InjectionStats { targets_seen: usize, targets_injected: usize, errors: usize }`

- [ ] **Step 1: Write failing CDP tests**

Test endpoint URL construction, target JSON parsing, filtering to `page` and `webview` targets with WebSocket URLs, injection source escaping, and unique port allocation.

```rust
#[test]
fn parses_only_injectable_targets() {
    let json = r#"[{"id":"a","type":"page","webSocketDebuggerUrl":"ws://127.0.0.1:1/a"},{"id":"b","type":"other"}]"#;
    let targets = parse_targets(json).unwrap();
    assert_eq!(targets.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["a"]);
}
```

- [ ] **Step 2: Run focused CDP tests and confirm they fail**

Run: `cargo test cdp::tests`

Expected: FAIL because `src/cdp.rs` does not exist.

- [ ] **Step 3: Implement blocking loopback CDP injection**

Poll `http://127.0.0.1:<port>/json/list` with a 15-second startup deadline and 750-millisecond steady interval. For each new target, send `Page.enable`, `Page.addScriptToEvaluateOnNewDocument`, and `Runtime.evaluate` with monotonically increasing request IDs. Never fetch non-loopback URLs returned by page content. Keep a set of target IDs and retry failed targets up to three times.

- [ ] **Step 4: Connect orchestration in `main.rs`**

Load all local resources before app launch, reject malformed dictionaries, detect already-running Copilot, validate signature, allocate the port, launch the child, inject until the child exits, and write metadata-only logs under `%LOCALAPPDATA%\GitHubCopilotZh\logs`.

- [ ] **Step 5: Run Rust tests and release build**

Run: `cargo test` then `cargo build --release`

Expected: all tests PASS and `target\release\copilot-zh.exe` exists with no console subsystem.

- [ ] **Step 6: Commit injection lifecycle**

```powershell
git add src
git commit -m "feat: inject localization through WebView2 DevTools"
```

### Task 4: Installation, Removal, And Documentation

**Files:**
- Create: `scripts/install.ps1`
- Create: `scripts/uninstall.ps1`
- Create: `scripts/package.ps1`
- Create: `安装.cmd`
- Create: `卸载.cmd`
- Create: `README.md`
- Create: `assets/README.txt`

**Interfaces:**
- Consumes: release executable, localization resources, and detected official executable.
- Produces: `%LOCALAPPDATA%\GitHubCopilotZh\copilot-zh.exe`
- Produces: `%USERPROFILE%\Desktop\GitHub Copilot 中文版.lnk`
- Produces: `dist\GitHubCopilotZh-<version>-win-x64.zip`

- [ ] **Step 1: Write installer dry-run and ownership checks**

Add `-DryRun` and `-SourceRoot` parameters to `install.ps1`. Dry-run output must enumerate source, destination, executable path, signature status, and shortcut path without writing files. Add `-DryRun` to uninstall and ensure every removal target resolves under `%LOCALAPPDATA%\GitHubCopilotZh` or equals the exact shortcut path.

- [ ] **Step 2: Implement installer and uninstaller**

The installer validates the official signature, copies only the manifest-listed files, writes `config.json` with the detected path, and creates the desktop shortcut through `WScript.Shell`. The uninstaller validates resolved paths before removing the exact shortcut and owned installation directory.

- [ ] **Step 3: Implement deterministic packaging**

`package.ps1` runs tests, builds release, stages only runtime files, writes `SHA256SUMS.txt`, and creates the ZIP with `Compress-Archive`. It fails when required files are missing or the working tree contains unstaged implementation changes.

- [ ] **Step 4: Write the Chinese README**

Document installation, the need to close an already-running app, use of original and Chinese shortcuts, translation boundaries, dictionary updates, unknown-version behavior, privacy, troubleshooting, and uninstall. State explicitly that this is an unofficial local localization layer and does not redistribute GitHub's executable.

- [ ] **Step 5: Run script syntax and dry-run checks**

Run:

```powershell
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile('scripts/install.ps1', [ref]$null, [ref]$errors) | Out-Null
if ($errors) { throw $errors }
& scripts/install.ps1 -DryRun
& scripts/uninstall.ps1 -DryRun
```

Expected: no parser errors; dry runs report validated paths and make no changes.

- [ ] **Step 6: Commit packaging and documentation**

```powershell
git add scripts assets README.md 安装.cmd 卸载.cmd
git commit -m "feat: package Windows localization installer"
```

### Task 5: Install And End-To-End Verification

**Files:**
- Modify: `localization/zh-CN.json` for labels observed during verification.
- Create: `verification/1.1.15-report.md`
- Create: `dist/GitHubCopilotZh-0.1.0-win-x64.zip`

**Interfaces:**
- Consumes: all previous deliverables.
- Produces: installed local tool, desktop shortcut, verification report, hashes, and final ZIP.

- [ ] **Step 1: Record the original executable evidence**

Run `Get-FileHash '<installed-copilot-path>\github.exe' -Algorithm SHA256`, version inspection, and `Get-AuthenticodeSignature`; save hash, version, signature status, signer, and timestamp in the report.

- [ ] **Step 2: Install the localization tool**

Close the running Copilot window normally, run `scripts/install.ps1`, and verify the installation manifest and desktop shortcut targets. Do not terminate the app if it presents an unsaved-work warning.

- [ ] **Step 3: Launch through the Chinese shortcut and inspect the live UI**

Verify visible Chinese labels on Home, My work, Automations, Customize, Chats, and Settings. Navigate through common dialogs and capture a screenshot of each primary screen. Inspect application logs for successful target attachment and zero repeated injection errors.

- [ ] **Step 4: Verify protected content and dynamic translation**

Open a project/chat surface without sending messages. Confirm repository names, filenames, code, existing prompts, and responses remain unchanged. Open a dynamically rendered dialog and verify its allowlisted chrome appears in Chinese.

- [ ] **Step 5: Verify reversibility**

Recompute the official executable SHA-256 and require an exact match. Launch the original shortcut and confirm English chrome remains available. Run uninstall, verify only localization files and shortcut are removed, then reinstall for final delivery.

- [ ] **Step 6: Final regression and package**

Run `npm test`, `cargo test`, `cargo build --release`, PowerShell parser checks, installer dry-run, uninstaller dry-run, and `scripts/package.ps1`. Record exact results and any untranslated known labels in `verification/1.1.15-report.md`.

- [ ] **Step 7: Commit verification evidence**

```powershell
git add localization/zh-CN.json verification
git commit -m "test: verify Copilot Chinese launcher on Windows"
```

- [ ] **Step 8: Copy the final ZIP to the user output directory**

Copy the verified ZIP to the project output directory and verify its SHA-256 matches the staged artifact.
