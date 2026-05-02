use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use schemalint_conformance::{evaluate, parse_truth, ProviderTruth, TruthResult};
use tiny_http::{Header, Method, Response, Server, StatusCode};

fn json_content_type() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
}

#[derive(Parser)]
#[command(name = "schemalint-conformance")]
struct Args {
    /// Port to listen on (0 = OS-assigned).
    #[arg(long, default_value = "0")]
    port: u16,

    /// Directory containing *.truth.toml files.
    #[arg(long)]
    truth_dir: PathBuf,

    /// Maximum request body size in bytes.
    #[arg(long, default_value = "1048576")]
    max_body_size: usize,
}

fn main() {
    let args = Args::parse();
    let truth_map = load_truth_files(&args.truth_dir);
    let server_addr = format!("127.0.0.1:{}", args.port);

    let server = Server::http(&server_addr).unwrap_or_else(|e| {
        eprintln!("error: failed to bind to {server_addr}: {e}");
        std::process::exit(1);
    });

    // Print bound address for CI consumption.
    let addr = server.server_addr();
    println!("{addr}");

    let truth_map = Arc::new(truth_map);
    let max_body_size = args.max_body_size;

    // Graceful shutdown on SIGTERM/CTRL-C.
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .unwrap_or_else(|_| eprintln!("warning: unable to set SIGINT handler"));

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        match server.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Some(request)) => {
                let map = truth_map.clone();
                let mbs = max_body_size;
                std::thread::spawn(move || handle_request(request, &map, mbs));
            }
            Ok(None) => {} // timeout, continue loop
            Err(e) => {
                eprintln!("warning: server error: {e}");
                continue;
            }
        }
    }
}

fn load_truth_files(truth_dir: &std::path::Path) -> HashMap<String, ProviderTruth> {
    let entries = match std::fs::read_dir(truth_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!(
                "error: unable to read truth directory '{}': {e}",
                truth_dir.display()
            );
            std::process::exit(1);
        }
    };

    let mut map = HashMap::new();
    for entry in entries.filter_map(|r| match r {
        Ok(e) => Some(e),
        Err(e) => {
            eprintln!("warning: unable to read directory entry: {e}");
            None
        }
    }) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        // Expect filenames like "openai.truth" → provider key "openai".
        let provider_key = file_name.strip_suffix(".truth").unwrap_or(file_name);

        let truth = parse_truth(&std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("error: unable to read '{}': {e}", path.display());
            std::process::exit(1);
        }))
        .unwrap_or_else(|e| {
            eprintln!("error: unable to parse '{}': {e}", path.display());
            std::process::exit(1);
        });

        map.insert(provider_key.to_string(), truth);
    }

    if map.is_empty() {
        eprintln!("error: no truth files found in '{}'", truth_dir.display());
        std::process::exit(1);
    }

    map
}

fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().and_then(hex_val);
            let lo = chars.next().and_then(hex_val);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                result.push((hi << 4 | lo) as char);
            } else {
                result.push('%');
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn handle_request(
    mut request: tiny_http::Request,
    truth_map: &HashMap<String, ProviderTruth>,
    max_body_size: usize,
) {
    let url = request.url().to_string();

    // Route: POST /evaluate/{provider}
    let path_prefix = "/evaluate/";
    if request.method() != &Method::Post || !url.starts_with(path_prefix) {
        let resp = Response::from_string(r#"{"error": "not found"}"#)
            .with_status_code(StatusCode(404))
            .with_header(json_content_type());
        if let Err(e) = request.respond(resp) {
            eprintln!("warning: failed to send response: {e}");
        }
        return;
    }

    let provider_raw = &url[path_prefix.len()..];
    let provider = percent_decode(provider_raw);
    let Some(truth) = truth_map.get(&provider) else {
        let resp =
            Response::from_string(serde_json::json!({"error": "unknown provider"}).to_string())
                .with_status_code(StatusCode(404))
                .with_header(json_content_type());
        if let Err(e) = request.respond(resp) {
            eprintln!("warning: failed to send response: {e}");
        }
        return;
    };

    // Read body with size limit.
    let mut body = String::with_capacity(max_body_size.min(65536));
    let max_body_size_u64 = max_body_size.min(u64::MAX as usize) as u64;
    let mut reader = request
        .as_reader()
        .take(max_body_size_u64.saturating_add(1));
    match reader.read_to_string(&mut body) {
        Ok(n) if n > max_body_size => {
            let resp = Response::from_string(
                serde_json::json!({"error": "request body too large"}).to_string(),
            )
            .with_status_code(StatusCode(413))
            .with_header(json_content_type());
            if let Err(e) = request.respond(resp) {
                eprintln!("warning: failed to send response: {e}");
            }
            return;
        }
        Err(e) => {
            let resp = Response::from_string(
                serde_json::json!({"error": format!("failed to read body: {e}")}).to_string(),
            )
            .with_status_code(StatusCode(400))
            .with_header(json_content_type());
            if let Err(e) = request.respond(resp) {
                eprintln!("warning: failed to send response: {e}");
            }
            return;
        }
        _ => {}
    }

    // Parse JSON schema body.
    let schema: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            let resp = Response::from_string(
                serde_json::json!({"error": format!("invalid JSON: {e}")}).to_string(),
            )
            .with_status_code(StatusCode(400))
            .with_header(json_content_type());
            if let Err(e) = request.respond(resp) {
                eprintln!("warning: failed to send response: {e}");
            }
            return;
        }
    };

    // Evaluate.
    let result = evaluate(truth, &schema);
    let response_body = match &result {
        TruthResult::Accepted { transformed } => {
            serde_json::json!({"status": "accepted", "transformed": transformed}).to_string()
        }
        TruthResult::Rejected { errors } => {
            let errs: Vec<_> = errors
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "message": e.message,
                        "pointer": e.pointer,
                        "keyword": e.keyword,
                    })
                })
                .collect();
            serde_json::json!({"status": "rejected", "errors": errs}).to_string()
        }
    };

    let resp = Response::from_string(response_body)
        .with_status_code(StatusCode(200))
        .with_header(json_content_type());
    if let Err(e) = request.respond(resp) {
        eprintln!("warning: failed to send response: {e}");
    }
}
