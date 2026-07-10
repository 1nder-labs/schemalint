use std::collections::VecDeque;
use std::io::{BufReader, Read};
use std::process::{ChildStderr, ChildStdout};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub(super) const STDERR_CAP: usize = 1_000;
const STDERR_MAX_LINE_BYTES: usize = 1 << 20;
const STDOUT_MAX_LINE_BYTES: usize = 1 << 20;

fn push_stderr(lines: &Mutex<VecDeque<String>>, line: String) {
    let mut lines = lines.lock().unwrap_or_else(|error| error.into_inner());
    if lines.len() >= STDERR_CAP {
        lines.pop_front();
    }
    lines.push_back(line);
}

pub(super) fn spawn_stderr_reader(
    stderr: ChildStderr,
    echo_prefix: Option<&'static str>,
) -> Arc<Mutex<VecDeque<String>>> {
    let captured = Arc::new(Mutex::new(VecDeque::new()));
    let writer = Arc::clone(&captured);
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        loop {
            let mut bytes = Vec::with_capacity(256);
            let mut truncated = false;
            let mut byte = [0_u8; 1];
            loop {
                match reader.read(&mut byte) {
                    Ok(0) => {
                        if !bytes.is_empty() {
                            let line = String::from_utf8_lossy(&bytes).into_owned();
                            if let Some(prefix) = echo_prefix {
                                eprintln!("[{prefix}] {line}");
                            }
                            push_stderr(&writer, line);
                        }
                        return;
                    }
                    Ok(_) if byte[0] == b'\n' => break,
                    Ok(_) if bytes.len() < STDERR_MAX_LINE_BYTES => bytes.push(byte[0]),
                    Ok(_) => truncated = true,
                    Err(_) => return,
                }
            }
            let mut line = String::from_utf8_lossy(&bytes).into_owned();
            if truncated {
                line.push_str("\u{2026}[truncated]");
            }
            if let Some(prefix) = echo_prefix {
                eprintln!("[{prefix}] {line}");
            }
            push_stderr(&writer, line);
        }
    });
    captured
}

pub(super) fn spawn_stdout_reader(stdout: ChildStdout) -> mpsc::Receiver<Option<String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut bytes = Vec::with_capacity(512);
            let mut over_limit = false;
            let mut byte = [0_u8; 1];
            loop {
                match reader.read(&mut byte) {
                    Ok(0) => {
                        let _ = sender.send(None);
                        return;
                    }
                    Ok(_) if byte[0] == b'\n' => break,
                    Ok(_) if bytes.len() < STDOUT_MAX_LINE_BYTES => bytes.push(byte[0]),
                    Ok(_) => over_limit = true,
                    Err(_) => {
                        let _ = sender.send(None);
                        return;
                    }
                }
            }
            if over_limit {
                return;
            }
            if sender
                .send(Some(String::from_utf8_lossy(&bytes).into_owned()))
                .is_err()
            {
                return;
            }
        }
    });
    receiver
}
