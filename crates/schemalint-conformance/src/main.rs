use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use clap::Parser;
use schemalint_conformance::{evaluate, parse_truth, ProviderTruth, TruthResult};

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

    let listener = TcpListener::bind(&server_addr).unwrap_or_else(|e| {
        eprintln!("error: failed to bind to {server_addr}: {e}");
        std::process::exit(1);
    });

    // Print bound address for CI consumption.
    println!("{}", listener.local_addr().unwrap());

    let truth_map = Arc::new(truth_map);
    let max_body_size = args.max_body_size;

    // Graceful shutdown on SIGTERM/CTRL-C.
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Release);
    })
    .unwrap_or_else(|_| eprintln!("warning: unable to set SIGINT handler"));

    // Bounded channel so we don't queue an unbounded number of connections.
    let (tx, rx) = mpsc::sync_channel::<TcpStream>(32);
    let rx = Arc::new(std::sync::Mutex::new(rx));

    // Fixed-size worker thread pool.
    const NUM_WORKERS: usize = 8;
    for _ in 0..NUM_WORKERS {
        let rx = rx.clone();
        let map = truth_map.clone();
        let mbs = max_body_size;
        std::thread::spawn(move || loop {
            let stream = match rx.lock().unwrap().recv() {
                Ok(s) => s,
                Err(_) => break,
            };
            handle_connection(stream, &map, mbs);
        });
    }

    listener.set_nonblocking(true).unwrap_or_else(|e| {
        eprintln!("error: failed to set nonblocking mode: {e}");
        std::process::exit(1);
    });

    while running.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                // Apply backpressure: if the channel is full, retry until
                // shutdown or a worker becomes free.
                let mut stream = Some(stream);
                while running.load(Ordering::Acquire) {
                    match tx.try_send(stream.take().unwrap()) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(s)) => {
                            stream = Some(s);
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => return,
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("warning: accept error: {e}");
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
        // Expect filenames like "openai.truth" -> provider key "openai".
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

fn handle_connection(
    mut stream: TcpStream,
    truth_map: &HashMap<String, ProviderTruth>,
    max_body_size: usize,
) {
    // Prevent Slowloris: cap time spent waiting for bytes from the client.
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
        eprintln!("warning: failed to set read timeout: {e}");
        return;
    }
    if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(30))) {
        eprintln!("warning: failed to set write timeout: {e}");
        // Continue anyway; write timeout is best-effort.
    }

    let result = {
        let mut reader = BufReader::new(&stream);

        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            return;
        }

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() != 3 {
            let _ = send_json_response(&mut stream, 400, r#"{"error":"bad request"}"#);
            return;
        }
        let method = parts[0];
        let path = parts[1];

        // Read headers.
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some((key, value)) = trimmed.split_once(':') {
                let key = key.trim().to_lowercase();
                if key == "content-length" {
                    content_length = value.trim().parse().ok();
                }
            }
        }

        // Route: POST /evaluate/{provider}
        let path_prefix = "/evaluate/";
        if method != "POST" || !path.starts_with(path_prefix) {
            let _ = send_json_response(&mut stream, 404, r#"{"error":"not found"}"#);
            return;
        }

        let provider_raw = &path[path_prefix.len()..];
        let Some(truth) = truth_map.get(provider_raw) else {
            let _ = send_json_response(&mut stream, 404, r#"{"error":"unknown provider"}"#);
            return;
        };

        // Read body with size limit.
        let body = match content_length {
            Some(0) => String::new(),
            Some(len) => {
                if len > max_body_size {
                    let _ = send_json_response(
                        &mut stream,
                        413,
                        r#"{"error":"request body too large"}"#,
                    );
                    return;
                }
                let mut buf = vec![0u8; len];
                if reader.read_exact(&mut buf).is_err() {
                    let _ =
                        send_json_response(&mut stream, 400, r#"{"error":"failed to read body"}"#);
                    return;
                }
                String::from_utf8_lossy(&buf).into_owned()
            }
            None => {
                let _ =
                    send_json_response(&mut stream, 400, r#"{"error":"missing Content-Length"}"#);
                return;
            }
        };

        // Parse JSON schema body.
        let schema: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => {
                let msg = format!(r#"{{"error":"invalid JSON: {e}"}}"#);
                let _ = send_json_response(&mut stream, 400, &msg);
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

        send_json_response(&mut stream, 200, &response_body)
    };

    if let Err(e) = result {
        eprintln!("warning: failed to send response: {e}");
    }
}

fn send_json_response(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
