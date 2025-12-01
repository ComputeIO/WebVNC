// Integration test scaffold for Xvfb-based tests.
//
// This test is intentionally conservative: it will only run when the
// environment variable `RUN_XVFB_TESTS=1` is set. This avoids CI flakiness
// on runners that don't have Xvfb installed. When enabled the test will
// attempt to spawn `Xvfb` and verify it starts.

use std::process::{Command, Child};
use std::time::Duration;
use std::thread::sleep;

fn start_xvfb() -> Option<Child> {
    if std::env::var("RUN_XVFB_TESTS").unwrap_or_default() != "1" {
        return None;
    }

    // Try to start Xvfb on :99 with a default screen configuration.
    match Command::new("Xvfb")
        .arg(":99")
        .arg("-screen")
        .arg("0")
        .arg("1024x768x24")
        .spawn()
    {
        Ok(child) => Some(child),
        Err(_) => None,
    }
}

#[test]
fn xvfb_integration_smoke() {
    let mut child = match start_xvfb() {
        Some(c) => c,
        None => {
            eprintln!("Skipping Xvfb integration test (set RUN_XVFB_TESTS=1 and ensure Xvfb is available)");
            return;
        }
    };

    // Give Xvfb a short moment to start
    sleep(Duration::from_millis(200));

    // Basic check: child is still running
    match child.try_wait() {
        Ok(Some(status)) => panic!("Xvfb exited prematurely: {:?}", status),
        Ok(None) => (),
        Err(e) => panic!("failed to poll Xvfb: {}", e),
    }

    // Teardown
    let _ = child.kill();
    let _ = child.wait();
}
