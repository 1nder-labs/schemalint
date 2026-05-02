use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command};
use std::time::Duration;

fn start_server(truth_dir: &std::path::Path) -> (Child, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_schemalint-conformance"));
    cmd.arg("--truth-dir")
        .arg(truth_dir)
        .arg("--port")
        .arg("0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().expect("failed to start conformance server");

    // Read the bound address from stdout.
    let mut addr = String::new();
    let stdout = child.stdout.take().expect("stdout not captured");
    let mut reader = std::io::BufReader::new(stdout);
    use std::io::BufRead;
    reader.read_line(&mut addr).expect("failed to read address");
    let addr = addr.trim().to_string();

    // Give the server time to start.
    std::thread::sleep(Duration::from_millis(500));

    (child, addr)
}

fn post_json(addr: &str, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("failed to connect");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    // Parse status code and body.
    let status_line = response.lines().next().unwrap_or("");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Extract body (after double CRLF).
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

    (status_code, body)
}

#[test]
fn server_accepts_clean_schema() {
    let dir = tempfile::tempdir().unwrap();
    let truth_path = dir.path().join("test.truth.toml");
    std::fs::write(
        &truth_path,
        r#"
[provider]
name = "test"
version = "1.0"
behavior = "strict"

[[keywords]]
name = "type"
behavior = "accept"
test_schema = '''
{ "type": "object" }
'''

[[keywords]]
name = "allOf"
behavior = "reject"
test_schema = '''
{ "allOf": [] }
'''
expected_error = "allOf rejected"
expected_error_path = "/allOf"
"#,
    )
    .unwrap();

    let (mut child, addr) = start_server(dir.path());

    // Test accept.
    let (status, body) = post_json(
        &addr,
        "/evaluate/test",
        r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
    );
    assert_eq!(status, 200);
    assert!(body.contains(r#""status":"accepted""#));

    // Test reject.
    let (status, body) = post_json(
        &addr,
        "/evaluate/test",
        r#"{"type": "object", "allOf": [{"properties": {"x": {"type": "string"}}}], "properties": {}}"#,
    );
    assert_eq!(status, 200);
    assert!(body.contains(r#""status":"rejected""#));
    assert!(body.contains("allOf rejected"));

    // Test unknown provider.
    let (status, body) = post_json(&addr, "/evaluate/unknown", r#"{"type": "object"}"#);
    assert_eq!(status, 404);
    assert!(body.contains("unknown provider"));

    // Test invalid JSON body.
    let (status, body) = post_json(&addr, "/evaluate/test", "not json");
    assert_eq!(status, 400);
    assert!(body.contains("invalid JSON"));

    // Test wrong route.
    let (status, _body) = post_json(&addr, "/nonexistent", r#"{"type": "object"}"#);
    assert_eq!(status, 404);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn server_rejects_empty_truth_dir() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_schemalint-conformance"))
        .arg("--truth-dir")
        .arg(dir.path())
        .arg("--port")
        .arg("0")
        .output()
        .expect("failed to run server");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no truth files found"));
}
