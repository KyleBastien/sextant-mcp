use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use tempfile::tempdir;

/// Locate the MCP binary cargo built for us. `assert_cmd` would do this but
/// we want raw stdin/stdout pipes, so we resolve the path manually.
fn mcp_binary() -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap();
    // exe is .../target/debug/deps/smoke-<hash>; the binary we want is
    // .../target/debug/sextant-mcp.
    let target_dir = exe
        .ancestors()
        .find(|p| p.ends_with("debug") || p.ends_with("release"))
        .expect("locate target dir");
    let bin = if cfg!(windows) {
        "sextant-mcp.exe"
    } else {
        "sextant-mcp"
    };
    target_dir.join(bin)
}

struct Server {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl Server {
    fn spawn(cwd: &std::path::Path) -> Self {
        let bin = mcp_binary();
        assert!(
            bin.exists(),
            "sextant-mcp binary not built; expected at {}",
            bin.display()
        );
        let mut child = Command::new(&bin)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sextant-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, msg: Value) {
        let line = serde_json::to_string(&msg).unwrap();
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).expect("read");
        assert!(n > 0, "server closed stdout before responding");
        serde_json::from_str(&line).expect("valid JSON response")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn initialize(server: &mut Server) {
    let resp = rpc(
        server,
        1,
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "smoke-test", "version": "0" }
        }),
    );
    assert_eq!(resp["id"], 1);
    let result = &resp["result"];
    assert_eq!(result["serverInfo"]["name"], "sextant-mcp");
    assert!(result["capabilities"]["tools"].is_object());

    server.send(json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    }));
}

fn rpc(server: &mut Server, id: u64, method: &str, params: Value) -> Value {
    server.send(json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    }));
    server.recv()
}

fn call_tool(server: &mut Server, id: u64, name: &str, args: Value) -> Value {
    rpc(
        server,
        id,
        "tools/call",
        json!({ "name": name, "arguments": args }),
    )
}

/// Spin up a server in a fresh tempdir, run `initialize`, hand the live
/// server to `body`, then drop. Most smoke tests just need this shape.
fn with_server<R>(body: impl FnOnce(&mut Server) -> R) -> R {
    let dir = tempdir().unwrap();
    let mut server = Server::spawn(dir.path());
    initialize(&mut server);
    body(&mut server)
}

fn extract_text(resp: &Value) -> &str {
    resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content")
}

#[test]
fn initialize_and_tools_list() {
    with_server(|server| {
        let resp = rpc(server, 2, "tools/list", json!({}));
        assert_eq!(resp["id"], 2);
        let tools = resp["result"]["tools"].as_array().expect("tools array");
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        for expected in [
            "grade_diff",
            "grade_files",
            "list_rules",
            "explain_rule",
            "get_config",
        ] {
            assert!(names.contains(&expected), "missing {expected}: {names:?}");
        }
    });
}

#[test]
fn list_rules_tool_returns_builtins() {
    with_server(|server| {
        let resp = call_tool(server, 2, "list_rules", json!({}));
        let rules: Value = serde_json::from_str(extract_text(&resp)).expect("rules JSON parses");
        let ids: Vec<&str> = rules
            .as_array()
            .expect("rules array")
            .iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"builtin.size.fn-length"), "got: {ids:?}");
    });
}

#[test]
fn explain_rule_tool_returns_markdown() {
    with_server(|server| {
        let resp = call_tool(
            server,
            3,
            "explain_rule",
            json!({ "id": "builtin.size.fn-length" }),
        );
        let text = extract_text(&resp);
        assert!(text.contains("Function length"), "got:\n{text}");
        assert!(
            text.contains("# "),
            "expected markdown heading; got:\n{text}"
        );
    });
}

#[test]
fn explain_rule_unknown_id_returns_is_error() {
    with_server(|server| {
        let resp = call_tool(server, 4, "explain_rule", json!({ "id": "no.such.rule" }));
        assert_eq!(resp["result"]["isError"], true);
    });
}

#[test]
fn grade_files_tool_returns_report() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    // Make sure the path-exclude defaults don't drop our fixture; we use .rs
    // which isn't on the exclude list.
    std::fs::write(root.join("a.rs"), "fn ok() {}\n").unwrap();

    let mut server = Server::spawn(root);
    initialize(&mut server);

    let resp = call_tool(&mut server, 5, "grade_files", json!({}));
    let text = extract_text(&resp);
    let report: Value = serde_json::from_str(text).expect("report JSON parses");
    assert!(report.get("findings").is_some(), "got:\n{text}");
    assert!(report.get("verdict").is_some());
    assert!(report.get("summary").is_some());
}

#[test]
fn unknown_method_returns_method_not_found() {
    with_server(|server| {
        let resp = rpc(server, 6, "nope/missing", json!({}));
        assert_eq!(resp["error"]["code"], -32601);
    });
}
