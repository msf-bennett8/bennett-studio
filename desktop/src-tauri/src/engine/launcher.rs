//! Engine launcher — spawns bennett-engine as a sidecar process
//! Desktop app bundles the engine binary and starts it on launch

use std::path::PathBuf;
use std::process::Stdio;
use tauri::api::process::{Command, CommandEvent};
use tracing::{info, warn, error};

pub struct EngineProcess {
    pub port: u16,
    pub grpc_port: u16,
}

/// Start the engine binary bundled with the Tauri app
/// Returns the HTTP port the engine is listening on
pub async fn start_engine(app_handle: tauri::AppHandle) -> anyhow::Result<EngineProcess> {
    // Resolve binary path: bundled resource or development path
    let binary_path = if cfg!(debug_assertions) {
        // Development: use cargo target directory (relative to desktop/src-tauri)
        let dev_path = PathBuf::from("../../engine/target/debug/bennett-engine");
        if dev_path.exists() {
            dev_path
        } else {
            PathBuf::from("../../engine/target/release/bennett-engine")
        }
    } else {
        // Production: bundled resource in sidecar
        let resource_path = app_handle.path_resolver()
            .resolve_resource("binaries/bennett-engine")
            .ok_or_else(|| anyhow::anyhow!("Engine binary not found in app bundle"))?;
        resource_path
    };

    if !binary_path.exists() {
        return Err(anyhow::anyhow!("Engine binary not found at {:?}", binary_path));
    }

    info!("Starting engine from {:?}", binary_path);

    // Find available ports
    let port = find_available_port(3001)?;
    let grpc_port = find_available_port(port + 100)?;

    // Set environment variables
    let env_vars = vec![
        ("BENNETT_HTTP_PORT", port.to_string()),
        ("BENNETT_GRPC_PORT", grpc_port.to_string()),
        ("BENNETT_RELAY_URL", "wss://bennett-relay.onrender.com/ws/tunnel".to_string()),
        ("BENNETT_FIREBASE_URL", "https://bennett-p2p-signaling-default-rtdb.europe-west1.firebasedatabase.app/".to_string()),
        ("RUST_LOG", "info".to_string()),
    ];

    // Spawn engine process
    let mut command = Command::new(binary_path);
    for (key, value) in &env_vars {
        command = command.env(key, value);
    }

    let (mut rx, _child) = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn engine: {}", e))?;

    // Log engine output
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => info!("[engine] {}", line),
                CommandEvent::Stderr(line) => warn!("[engine] {}", line),
                CommandEvent::Error(e) => error!("[engine] process error: {}", e),
                CommandEvent::Terminated(payload) => {
                    warn!("[engine] process exited: code={:?}, signal={:?}", payload.code, payload.signal);
                }
                _ => {}
            }
        }
    });

    // Wait for engine to be ready (poll health endpoint)
    let health_url = format!("http://127.0.0.1:{}/api/health", port);
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    let client = reqwest::Client::new();
    loop {
        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!("Engine failed to start within 30 seconds"));
        }

        match client.get(&health_url).timeout(std::time::Duration::from_secs(2)).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Engine ready on port {}", port);
                break;
            }
            _ => {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }

    Ok(EngineProcess { port, grpc_port })
}

/// Find an available TCP port
fn find_available_port(start: u16) -> anyhow::Result<u16> {
    for port in start..=start + 100 {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err(anyhow::anyhow!("No available port found in range {}-{}", start, start + 100))
}

/// Stop the engine process (called on app exit)
pub async fn stop_engine() {
    // The Command child is dropped when the app exits, which kills the process
    info!("Engine process will be stopped on app exit");
}
