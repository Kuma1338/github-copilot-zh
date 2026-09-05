use crate::config::Dictionary;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Pid, System};
use tungstenite::{connect, Message};
use url::Url;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DevToolsTarget {
    pub id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    #[serde(default)]
    pub url: String,
    pub web_socket_debugger_url: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InjectionStats {
    pub targets_seen: usize,
    pub targets_injected: usize,
    pub errors: usize,
}

pub fn allocate_loopback_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).context("无法分配本机调试端口")?;
    Ok(listener.local_addr()?.port())
}

pub fn build_injection_source(runtime: &str, dictionary: &Dictionary) -> Result<String> {
    let dictionary_json = serde_json::to_string(dictionary)?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let json_literal = serde_json::to_string(&dictionary_json)?;
    Ok(format!(
        "globalThis.__COPILOT_ZH_DICTIONARY__ = JSON.parse({json_literal});\n{runtime}\nglobalThis.__COPILOT_ZH__.start();\n"
    ))
}

pub fn parse_targets(body: &str) -> Result<Vec<DevToolsTarget>> {
    let targets: Vec<DevToolsTarget> =
        serde_json::from_str(body).context("DevTools 目标列表不是有效 JSON")?;
    Ok(targets
        .into_iter()
        .filter(|target| matches!(target.target_type.as_str(), "page" | "webview"))
        .filter(|target| {
            target
                .web_socket_debugger_url
                .as_deref()
                .and_then(|value| Url::parse(value).ok())
                .is_some_and(|url| url.scheme() == "ws" && url.host_str() == Some("127.0.0.1"))
        })
        .collect())
}

pub fn run_injector(port: u16, source: &str, app_pid: u32) -> Result<InjectionStats> {
    let endpoint = format!("http://127.0.0.1:{port}/json/list");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(1))
        .timeout_read(Duration::from_secs(2))
        .build();
    let startup_deadline = Instant::now() + Duration::from_secs(15);
    let mut stats = InjectionStats::default();
    let mut injected = HashSet::new();
    let mut attempts: HashMap<String, u8> = HashMap::new();
    let mut devtools_seen = false;

    while process_exists(app_pid) {
        match agent.get(&endpoint).call() {
            Ok(response) => {
                devtools_seen = true;
                let body = response
                    .into_string()
                    .context("无法读取 DevTools 目标列表")?;
                for target in parse_targets(&body)? {
                    stats.targets_seen += usize::from(!attempts.contains_key(&target.id));
                    if injected.contains(&target.id) {
                        continue;
                    }
                    let count = attempts.entry(target.id.clone()).or_default();
                    if *count >= 3 {
                        continue;
                    }
                    *count += 1;
                    match inject_target(&target, source) {
                        Ok(()) => {
                            injected.insert(target.id);
                            stats.targets_injected += 1;
                        }
                        Err(_) => stats.errors += 1,
                    }
                }
            }
            Err(_) if !devtools_seen && Instant::now() >= startup_deadline => {
                bail!("应用已启动，但 WebView2 调试接口在 15 秒内没有响应")
            }
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(750));
    }

    if !devtools_seen {
        bail!("GitHub Copilot 在 WebView2 调试接口就绪前退出")
    }
    Ok(stats)
}

fn process_exists(pid: u32) -> bool {
    System::new_all().process(Pid::from_u32(pid)).is_some()
}

fn inject_target(target: &DevToolsTarget, source: &str) -> Result<()> {
    let socket_url = target
        .web_socket_debugger_url
        .as_deref()
        .context("DevTools 目标缺少 WebSocket 地址")?;
    let (mut socket, _) = connect(socket_url).context("无法连接 WebView2 DevTools WebSocket")?;
    let commands = [
        json!({"id": 1, "method": "Page.enable"}),
        json!({"id": 2, "method": "Page.addScriptToEvaluateOnNewDocument", "params": {"source": source}}),
        json!({"id": 3, "method": "Runtime.evaluate", "params": {"expression": source, "returnByValue": false}}),
    ];
    for command in commands {
        socket
            .send(Message::Text(command.to_string().into()))
            .context("无法发送 DevTools 注入命令")?;
    }
    let _ = socket.close(None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{allocate_loopback_port, build_injection_source, parse_targets};
    use crate::config::{Dictionary, Pattern};
    use std::collections::BTreeMap;

    #[test]
    fn parses_only_loopback_injectable_targets() {
        let json = r#"[
          {"id":"a","type":"page","url":"tauri://localhost","webSocketDebuggerUrl":"ws://127.0.0.1:32123/devtools/page/a"},
          {"id":"b","type":"other","url":"","webSocketDebuggerUrl":"ws://127.0.0.1:32123/devtools/page/b"},
          {"id":"c","type":"webview","url":"https://example.com","webSocketDebuggerUrl":"ws://example.com/devtools/page/c"}
        ]"#;
        let targets = parse_targets(json).unwrap();
        assert_eq!(
            targets
                .iter()
                .map(|target| target.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a"]
        );
    }

    #[test]
    fn builds_safe_dictionary_assignment_before_runtime() {
        let dictionary = Dictionary {
            version: 1,
            tested_app_versions: vec!["1.1.15".into()],
            exact: BTreeMap::from([("Home".into(), "主页 </script>".into())]),
            patterns: vec![Pattern {
                source: "{count} files".into(),
                target: "{count} 个文件".into(),
            }],
            excluded_selectors: vec!["code".into()],
        };
        let source =
            build_injection_source("globalThis.runtimeLoaded = true;", &dictionary).unwrap();
        assert!(source.starts_with("globalThis.__COPILOT_ZH_DICTIONARY__ = JSON.parse("));
        assert!(source.contains("\\\\u003c/script\\\\u003e"));
        let runtime_position = source.find("globalThis.runtimeLoaded = true;").unwrap();
        let start_position = source.find("globalThis.__COPILOT_ZH__.start();").unwrap();
        assert!(runtime_position < start_position);
    }

    #[test]
    fn allocates_a_bindable_loopback_port() {
        let port = allocate_loopback_port().unwrap();
        assert_ne!(port, 0);
        let listener = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        drop(listener);
    }
}
