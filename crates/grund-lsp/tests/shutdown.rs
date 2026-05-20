use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn test_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "grund-lsp-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".agents")).expect("create config dir");
    fs::create_dir_all(dir.join("docs")).expect("create docs dir");
    fs::write(
        dir.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\"]\nextensions = [\"md\"]\n",
    )
    .expect("write config");
    dir
}

fn file_uri(path: &Path) -> String {
    url::Url::from_file_path(path)
        .expect("file uri")
        .to_string()
}

fn send_message(stdin: &mut impl Write, message: Value) {
    let body = serde_json::to_vec(&message).expect("serialize message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    stdin.write_all(&body).expect("write body");
    stdin.flush().expect("flush message");
}

fn read_messages(stdout: impl Read + Send + 'static) -> mpsc::Receiver<Value> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let mut content_length = None;
            loop {
                let mut line = String::new();
                let Ok(bytes) = stdout.read_line(&mut line) else {
                    return;
                };
                if bytes == 0 {
                    return;
                }
                let line = line.trim_end_matches(['\r', '\n']);
                if line.is_empty() {
                    break;
                }
                if let Some(length) = line.strip_prefix("Content-Length: ") {
                    content_length = length.parse::<usize>().ok();
                }
            }
            let Some(content_length) = content_length else {
                return;
            };
            let mut body = vec![0; content_length];
            if stdout.read_exact(&mut body).is_err() {
                return;
            }
            let Ok(message) = serde_json::from_slice(&body) else {
                return;
            };
            let _ = sender.send(message);
        }
    });
    receiver
}

fn recv_response(receiver: &mpsc::Receiver<Value>, id: i64) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("timed out waiting for LSP response {id}"));
        }
        let message = receiver
            .recv_timeout(remaining)
            .map_err(|err| format!("receive LSP message: {err}"))?;
        if message.get("id").and_then(Value::as_i64) == Some(id) {
            return Ok(message);
        }
    }
}

fn recv_response_or_panic(receiver: &mpsc::Receiver<Value>, child: &mut Child, id: i64) -> Value {
    match recv_response(receiver, id) {
        Ok(message) => message,
        Err(err) => {
            if child.try_wait().expect("poll child").is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
            let mut stderr = String::new();
            if let Some(child_stderr) = child.stderr.as_mut() {
                let _ = child_stderr.read_to_string(&mut stderr);
            }
            panic!("{err}; server stderr: {stderr}");
        }
    }
}

fn wait_for_exit(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            assert!(status.success(), "grund-lsp exited with {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("grund-lsp did not exit after shutdown/exit");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn shutdown_exit_terminates_stdio_server() {
    // The editor owns the lifecycle and talks to grund-lsp over stdio only.
    // §FS-lsp.2.2 §AR-lsp.4
    let root = test_root("shutdown");
    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": null
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 2);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }),
    );
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn navigation_covers_source_comment_citations_and_stub_titles() {
    // Stub-title definition follows the inline source home (§FS-lsp.1.3), and
    // source-comment citations expose both document links and references
    // (§FS-lsp.1.3.1 §FS-lsp.1.3.2).
    let root = test_root("navigation");
    fs::write(
        root.join(".agents/grund.toml"),
        "grund_config_version = 1\n[scan]\ninclude = [\"docs\", \"src\"]\n\
         extensions = [\"md\", \"rs\"]\n",
    )
    .expect("write config");
    fs::create_dir_all(root.join("docs/functional-spec")).expect("create specs");
    fs::create_dir_all(root.join("docs/architecture")).expect("create architecture");
    fs::create_dir_all(root.join("src")).expect("create source");
    let spec = root.join("docs/functional-spec/FS-001-alpha.md");
    let stub = root.join("docs/architecture/AR-001-router.md");
    let source = root.join("src/router.rs");
    let spec_heading = "# FS-001-alpha: Alpha";
    fs::write(
        &spec,
        format!("{spec_heading}\n\nLead.\n\n## 1. Detail\nMore.\n"),
    )
    .expect("write spec");
    let stub_heading = "# AR-001-router: [src/router.rs](../../src/router.rs)";
    fs::write(&stub, format!("{stub_heading}\n")).expect("write stub");
    fs::write(
        &source,
        "/// AR-001-router: Router\n/// Uses §FS-001-alpha.1.\npub fn router() {}\n",
    )
    .expect("write source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_grund-lsp"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn grund-lsp");
    let mut stdin = child.stdin.take().expect("child stdin");
    let receiver = read_messages(child.stdout.take().expect("child stdout"));

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 1);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&stub) },
                "position": { "line": 0, "character": 22 }
            }
        }),
    );
    let stub_definition = recv_response_or_panic(&receiver, &mut child, 2);
    assert_eq!(
        stub_definition["result"]["uri"].as_str(),
        Some(file_uri(&source).as_str())
    );
    assert_eq!(
        stub_definition["result"]["range"]["start"]["line"].as_i64(),
        Some(0)
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&stub) }
            }
        }),
    );
    let stub_links = recv_response_or_panic(&receiver, &mut child, 3);
    let stub_links = stub_links["result"]
        .as_array()
        .expect("stub document links");
    assert!(
        stub_links.iter().any(|link| {
            link["range"]["start"]["character"].as_i64() == Some(2)
                && link["range"]["end"]["character"].as_i64() == Some(stub_heading.len() as i64)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("src/router.rs#L1"))
        }),
        "stub title should be one document link: {stub_links:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&spec) }
            }
        }),
    );
    let spec_links = recv_response_or_panic(&receiver, &mut child, 4);
    let spec_links = spec_links["result"]
        .as_array()
        .expect("spec document links");
    assert!(
        spec_links.iter().any(|link| {
            link["range"]["start"]["character"].as_i64() == Some(2)
                && link["range"]["end"]["character"].as_i64() == Some(spec_heading.len() as i64)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md#L1"))
        }),
        "markdown declaration title should be one document link: {spec_links:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": { "uri": file_uri(&source) }
            }
        }),
    );
    let links = recv_response_or_panic(&receiver, &mut child, 5);
    let links = links["result"].as_array().expect("document links");
    assert!(
        links.iter().any(|link| {
            link["range"]["start"]["line"].as_i64() == Some(1)
                && link["range"]["start"]["character"].as_i64() == Some(9)
                && link["target"]
                    .as_str()
                    .is_some_and(|target| target.contains("FS-001-alpha.md#L5"))
        }),
        "source-comment citation should be a document link: {links:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&source) },
                "position": { "line": 1, "character": 10 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let references = recv_response_or_panic(&receiver, &mut child, 6);
    let references = references["result"].as_array().expect("references");
    assert!(
        references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&spec).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(0)
        }),
        "references should include the declaration: {references:?}"
    );
    assert!(
        references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&source).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(1)
        }),
        "references should include the source-comment citation: {references:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 0, "character": 17 },
                "context": { "includeDeclaration": true }
            }
        }),
    );
    let markdown_references = recv_response_or_panic(&receiver, &mut child, 7);
    let markdown_references = markdown_references["result"]
        .as_array()
        .expect("markdown references");
    assert!(
        markdown_references.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&source).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(1)
        }),
        "markdown title references should include source-comment citations: {markdown_references:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": file_uri(&spec) },
                "position": { "line": 0, "character": 17 }
            }
        }),
    );
    let markdown_definition = recv_response_or_panic(&receiver, &mut child, 8);
    let markdown_definition = markdown_definition["result"]
        .as_array()
        .expect("markdown definition locations");
    assert!(
        markdown_definition.iter().any(|location| {
            location["uri"].as_str() == Some(file_uri(&source).as_str())
                && location["range"]["start"]["line"].as_i64() == Some(1)
        }),
        "markdown title definition navigation should include source-comment citations: {markdown_definition:?}"
    );

    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "shutdown",
            "params": null
        }),
    );
    recv_response_or_panic(&receiver, &mut child, 9);
    send_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }),
    );
    drop(stdin);

    wait_for_exit(&mut child);
    let _ = fs::remove_dir_all(root);
}
