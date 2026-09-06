use crate::config::Dictionary;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
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

pub fn endpoint_paths(port: u16) -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::inspector_endpoint_paths(port)
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec![format!("http://127.0.0.1:{port}/json/list")]
    }
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
    let value: Value = serde_json::from_str(body).context("DevTools 目标列表不是有效 JSON")?;
    let values = match value {
        Value::Array(values) => values,
        Value::Object(mut object) => match object.remove("targets") {
            Some(Value::Array(values)) => values,
            Some(_) => bail!("DevTools targets 字段不是数组"),
            None => vec![Value::Object(object)],
        },
        _ => bail!("DevTools 目标列表不是数组或对象"),
    };
    let targets = values
        .into_iter()
        .map(|value| serde_json::from_value(value).context("DevTools 目标格式无效"))
        .collect::<Result<Vec<DevToolsTarget>>>()?;
    Ok(targets
        .into_iter()
        .filter(|target| matches!(target.target_type.as_str(), "page" | "webview" | "webpage"))
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
    let endpoints = endpoint_paths(port);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(1))
        .timeout_read(Duration::from_secs(2))
        .build();
    let startup_deadline = Instant::now() + Duration::from_secs(15);
    let mut stats = InjectionStats::default();
    let mut injected = HashSet::new();
    let mut attempts: HashMap<String, u8> = HashMap::new();
    let mut devtools_seen = false;
    let mut injectable_target_seen = false;
    let mut active_endpoint = None;

    while process_exists(app_pid) {
        match fetch_target_list(&agent, &endpoints, &mut active_endpoint)? {
            Some(body) => {
                devtools_seen = true;
                let targets = parse_targets(&body)?;
                injectable_target_seen |= !targets.is_empty();
                for target in targets {
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
            None if (!devtools_seen || !injectable_target_seen)
                && Instant::now() >= startup_deadline =>
            {
                bail!(inspector_timeout_message())
            }
            None => {}
        }
        thread::sleep(Duration::from_millis(750));
    }

    validate_injection_result(stats, devtools_seen, injectable_target_seen)?;
    Ok(stats)
}

fn validate_injection_result(
    stats: InjectionStats,
    devtools_seen: bool,
    injectable_target_seen: bool,
) -> Result<()> {
    if !devtools_seen || !injectable_target_seen || stats.targets_injected == 0 {
        bail!(injection_failure_message())
    }
    Ok(())
}

fn injection_failure_message() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "当前官方 macOS 版没有返回可用的 Inspector/CDP 页面目标，未注入汉化层。请使用官方应用，或等待官方开放可调试接口。"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "应用已启动，但 WebView2 调试接口没有返回可注入的页面目标"
    }
}

fn fetch_target_list(
    agent: &ureq::Agent,
    endpoints: &[String],
    active_endpoint: &mut Option<String>,
) -> Result<Option<String>> {
    if let Some(endpoint) = active_endpoint.as_deref() {
        match agent.get(endpoint).call() {
            Ok(response) => {
                return Ok(Some(
                    response
                        .into_string()
                        .context("无法读取 DevTools 目标列表")?,
                ))
            }
            Err(_) => *active_endpoint = None,
        }
    }

    for endpoint in endpoints {
        if let Ok(response) = agent.get(endpoint).call() {
            *active_endpoint = Some(endpoint.clone());
            return Ok(Some(
                response
                    .into_string()
                    .context("无法读取 DevTools 目标列表")?,
            ));
        }
    }
    Ok(None)
}

fn inspector_timeout_message() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        crate::macos::unsupported_inspector_message()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "应用已启动，但 WebView2 调试接口在 15 秒内没有响应"
    }
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
    match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .context("无法设置 DevTools WebSocket 读取超时")?,
        _ => bail!("DevTools WebSocket 不是本机明文连接"),
    }
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

    let mut pending = HashSet::from([1, 2, 3]);
    while !pending.is_empty() {
        let message = socket.read().context("等待 DevTools 注入响应失败或超时")?;
        if let Message::Text(text) = message {
            if let Some(id) = parse_cdp_response(text.as_ref())? {
                pending.remove(&id);
            }
        }
    }

    let _ = socket.close(None);
    Ok(())
}

fn parse_cdp_response(message: &str) -> Result<Option<u64>> {
    let value: Value = serde_json::from_str(message).context("DevTools 响应不是有效 JSON")?;
    let Some(id) = value.get("id").and_then(Value::as_u64) else {
        return Ok(None);
    };

    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("未知协议错误");
        bail!("DevTools 命令 {id} 失败：{message}");
    }

    if let Some(exception) = value
        .pointer("/result/exceptionDetails/text")
        .and_then(Value::as_str)
    {
        bail!("DevTools 脚本 {id} 执行失败：{exception}");
    }

    if value.get("result").is_none() {
        bail!("DevTools 命令 {id} 的响应缺少 result");
    }

    Ok(Some(id))
}

#[cfg(test)]
mod tests {
    use super::{
        allocate_loopback_port, build_injection_source, parse_cdp_response, parse_targets,
        validate_injection_result,
    };
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
    fn parses_webkit_single_target_and_webpage_target_types() {
        let json = r#"{
          "id":"webkit",
          "type":"webpage",
          "url":"tauri://localhost",
          "webSocketDebuggerUrl":"ws://127.0.0.1:43123/devtools/page/webkit"
        }"#;
        let targets = parse_targets(json).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "webkit");
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
                parameter_rules: BTreeMap::new(),
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

    #[test]
    fn parses_successful_cdp_command_responses() {
        assert_eq!(
            parse_cdp_response(r#"{"id":2,"result":{"identifier":"1"}}"#).unwrap(),
            Some(2)
        );
        assert_eq!(
            parse_cdp_response(r#"{"method":"Page.loadEventFired"}"#).unwrap(),
            None
        );
    }

    #[test]
    fn rejects_cdp_command_and_script_errors() {
        let protocol_error =
            parse_cdp_response(r#"{"id":2,"error":{"code":-32601,"message":"Method not found"}}"#)
                .unwrap_err()
                .to_string();
        assert!(protocol_error.contains("Method not found"));

        let script_error = parse_cdp_response(
            r#"{"id":3,"result":{"result":{"type":"object"},"exceptionDetails":{"text":"Uncaught"}}}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(script_error.contains("Uncaught"));
    }

    #[test]
    fn rejects_a_devtools_endpoint_without_an_injectable_target() {
        let error = validate_injection_result(Default::default(), true, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("可注入"));

        validate_injection_result(
            super::InjectionStats {
                targets_seen: 1,
                targets_injected: 1,
                errors: 0,
            },
            true,
            true,
        )
        .unwrap();
    }
}
