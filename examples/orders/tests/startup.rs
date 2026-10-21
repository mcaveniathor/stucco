use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// The printed address must be the CSRF origin, or every form posted from it
/// is rejected.
#[test]
fn printed_url_matches_the_trusted_origin() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_orders"))
        .env("PORT", "4199")
        .env("ORDERS_DB", dir.path().join("orders.redb"))
        .env_remove("ORDERS_ORIGIN")
        .env("STUCCO_SECRET_KEY", "0".repeat(63) + "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(
        line.trim(),
        "orders listening on http://localhost:4199/orders"
    );
}
