//! Daemon management commands.

use anyhow::Result;
use std::process::Command;

pub async fn run_start() -> Result<()> {
    // Check if daemon is already running
    match is_daemon_running() {
        Ok(true) => {
            println!("\u{2139}  Daemon is already running.");
            return Ok(());
        }
        Ok(false) => {}
        Err(e) => {
            println!("\u{26a0}  Could not check daemon status: {}", e);
            println!("  Attempting to start anyway...");
        }
    }

    // Start daemon in background
    let daemon_path = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not determine executable directory"))?
        .join("todoee-daemon");

    if !daemon_path.exists() {
        println!("\u{2717} Daemon binary not found at {:?}", daemon_path);
        println!("  Run `cargo build -p todoee-daemon` first.");
        return Ok(());
    }

    Command::new(&daemon_path)
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start daemon: {}", e))?;

    println!("\u{2713} Daemon started.");
    Ok(())
}

pub async fn run_stop() -> Result<()> {
    #[cfg(unix)]
    {
        let output = Command::new("pkill")
            .arg("-f")
            .arg("todoee-daemon")
            .output();

        match output {
            Ok(o) if o.status.success() => println!("\u{2713} Daemon stopped."),
            Ok(o) => {
                // pkill returns exit code 1 if no processes matched
                if o.status.code() == Some(1) {
                    println!("\u{2139}  Daemon was not running.");
                } else {
                    println!(
                        "\u{26a0}  pkill exited with unexpected status: {:?}",
                        o.status.code()
                    );
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to execute pkill: {}", e));
            }
        }
    }

    #[cfg(windows)]
    {
        let output = Command::new("taskkill")
            .args(["/F", "/IM", "todoee-daemon.exe"])
            .output();

        match output {
            Ok(o) if o.status.success() => println!("\u{2713} Daemon stopped."),
            Ok(_) => println!("\u{2139}  Daemon was not running."),
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to execute taskkill: {}", e));
            }
        }
    }

    Ok(())
}

pub async fn run_status() -> Result<()> {
    match is_daemon_running() {
        Ok(true) => {
            println!("\u{2713} Daemon is running.");
        }
        Ok(false) => {
            println!("\u{2717} Daemon is not running.");
            println!("  Run `todoee daemon start` to start it.");
        }
        Err(e) => {
            println!("\u{26a0}  Could not determine daemon status: {}", e);
        }
    }
    Ok(())
}

fn is_daemon_running() -> Result<bool> {
    #[cfg(unix)]
    {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg("todoee-daemon")
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute pgrep: {}", e))?;

        Ok(output.status.success())
    }

    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq todoee-daemon.exe"])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute tasklist: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).contains("todoee-daemon"))
    }
}
