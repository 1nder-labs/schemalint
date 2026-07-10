use super::*;

#[test]
fn probe_command_returns_false_for_absent_binary() {
    assert!(!probe_command(
        "schemalint-definitely-not-a-real-binary-xyz",
        Duration::from_millis(200),
    ));
}

#[test]
fn stderr_cap_evicts_oldest_entry() {
    let mut lines = VecDeque::new();
    for index in 0..=io::STDERR_CAP {
        if lines.len() >= io::STDERR_CAP {
            lines.pop_front();
        }
        lines.push_back(format!("line-{index}"));
    }
    assert_eq!(lines.len(), io::STDERR_CAP);
    assert_ne!(lines.front().map(String::as_str), Some("line-0"));
    assert_eq!(lines.back().map(String::as_str), Some("line-1000"));
}
