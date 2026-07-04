//! Verifies that the binary terminates promptly on SIGINT (Ctrl+C).
#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn assert_sigint_exits(args: &[&str]) -> Result<(), String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_zz"))
        .args(args)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn zz: {e}"))?;

    // Give the process time to start and install its signal handler.
    std::thread::sleep(Duration::from_millis(500));

    let status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .map_err(|e| format!("failed to run kill: {e}"))?;
    if !status.success() {
        let _ = child.kill();
        return Err("kill -INT failed".to_string());
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(exit) = child
            .try_wait()
            .map_err(|e| format!("try_wait failed: {e}"))?
        {
            // 128 + SIGINT(2): the conventional exit code for Ctrl+C
            if exit.code() == Some(130) {
                return Ok(());
            }
            return Err(format!("unexpected exit status: {exit}"));
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            return Err("process did not exit within 5s of SIGINT".to_string());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn sigint_terminates_progress_mode() -> Result<(), String> {
    assert_sigint_exits(&["30"])
}

#[test]
fn sigint_terminates_quiet_mode() -> Result<(), String> {
    assert_sigint_exits(&["-q", "30"])
}
