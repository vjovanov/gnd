use serde_json::{Value, json};
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
