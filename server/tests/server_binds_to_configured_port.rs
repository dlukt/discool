use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU16, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};

fn server_exe() -> &'static str {
    option_env!("CARGO_BIN_EXE_discool-server")
        .or(option_env!("CARGO_BIN_EXE_discool_server"))
        .expect("cargo should set CARGO_BIN_EXE_<bin-name> for integration tests")
}

static NEXT_PORT: AtomicU16 = AtomicU16::new(40_000);

fn pick_free_port() -> u16 {
    loop {
        let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
}

fn new_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut dir = std::env::temp_dir();
    dir.push(format!("discool-test-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct TestServer {
    child: Child,
    dir: PathBuf,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn spawn_server(dir: &Path, configure: impl FnOnce(&mut Command)) -> TestServer {
    let mut cmd = Command::new(server_exe());
    cmd.current_dir(dir)
        // With `log.level = "warn"`, successful startup should be quiet, but failures are visible.
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    configure(&mut cmd);

    let child = cmd.spawn().unwrap();
    TestServer {
        child,
        dir: dir.to_path_buf(),
    }
}

async fn wait_for_bind(child: &mut Child, addr: &str) {
    let mut remaining = Duration::from_secs(5);
    loop {
        if let Ok(stream) = TcpStream::connect(addr).await {
            drop(stream);
            if let Some(status) = child.try_wait().unwrap() {
                panic!("server exited early with status {status}");
            }
            break;
        }

        if let Some(status) = child.try_wait().unwrap() {
            panic!("server exited early with status {status}");
        }

        if remaining.is_zero() {
            panic!("timed out waiting for server to bind to {addr}");
        }

        let step = Duration::from_millis(25);
        sleep(step).await;
        remaining = remaining.saturating_sub(step);
    }
}

async fn http_status(addr: &str, path: &str) -> u16 {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let res = String::from_utf8_lossy(&buf);

    let status_line = res.lines().next().unwrap_or("");
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0)
}

async fn http_response(addr: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_post(addr: &str, path: &str, json_body: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.as_bytes().len(),
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_post_bytes(addr: &str, path: &str, json_body: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.as_bytes().len(),
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    buf
}

fn response_header_and_body_bytes(res: &[u8]) -> (String, &[u8]) {
    let header_end = res
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(res.len());
    let header = String::from_utf8_lossy(&res[..header_end]).to_string();
    let body = res.get((header_end + 4)..).unwrap_or_default();
    (header, body)
}

async fn try_http_status(addr: &str, path: &str) -> std::io::Result<u16> {
    let mut stream = TcpStream::connect(addr).await?;

    let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    Ok(response_status(&String::from_utf8_lossy(&buf)))
}

fn response_status(res: &str) -> u16 {
    let status_line = res.lines().next().unwrap_or("");
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0)
}

fn response_header(res: &str, header_name: &str) -> Option<String> {
    let name = header_name.to_ascii_lowercase();
    for line in res.lines().skip(1) {
        if line.trim().is_empty() {
            break;
        }

        let (key, value) = line.split_once(':')?;
        if key.trim().eq_ignore_ascii_case(&name) {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn response_body(res: &str) -> &str {
    res.split("\r\n\r\n").nth(1).unwrap_or("")
}

async fn wait_for_http_status(child: &mut Child, addr: &str, path: &str, expected: u16) {
    let mut remaining = Duration::from_secs(5);
    loop {
        match try_http_status(addr, path).await {
            Ok(status) if status == expected => break,
            Ok(_) | Err(_) => {}
        }

        if let Some(status) = child.try_wait().unwrap() {
            panic!("server exited early with status {status}");
        }

        if remaining.is_zero() {
            panic!("timed out waiting for {path} to return {expected}");
        }

        let step = Duration::from_millis(25);
        sleep(step).await;
        remaining = remaining.saturating_sub(step);
    }
}

fn write_server_config(path: &Path, host: &str, port: u16, metrics_enabled: Option<bool>) {
    write_server_config_with_db_url(path, host, port, metrics_enabled, "sqlite::memory:");
}

fn write_server_config_with_db_url(
    path: &Path,
    host: &str,
    port: u16,
    metrics_enabled: Option<bool>,
    db_url: &str,
) {
    let mut cfg = format!(
        "[server]\nhost = \"{host}\"\nport = {port}\n\n[log]\nlevel = \"warn\"\nformat = \"json\"\n\n[database]\nurl = \"{db_url}\"\nmax_connections = 1\n"
    );

    if let Some(enabled) = metrics_enabled {
        cfg.push_str("\n[metrics]\n");
        cfg.push_str(&format!("enabled = {enabled}\n"));
    }

    fs::write(path, cfg).unwrap();
}

#[tokio::test]
async fn server_binds_to_port_from_config_toml() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_bind(&mut server.child, &addr).await;
}

#[tokio::test]
async fn healthz_returns_200() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_bind(&mut server.child, &addr).await;

    assert_eq!(http_status(&addr, "/healthz").await, 200);
}

#[tokio::test]
async fn readyz_returns_200_with_expected_json() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_bind(&mut server.child, &addr).await;

    let res = http_response(&addr, "/readyz").await;
    assert_eq!(response_status(&res), 200);

    let body = response_body(&res);
    let value: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        value,
        json!({
            "status": "ready",
            "checks": {
                "database": "connected",
                "migrations": "applied"
            }
        })
    );
}

#[tokio::test]
async fn metrics_returns_404_when_not_configured() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_bind(&mut server.child, &addr).await;

    assert_eq!(http_status(&addr, "/metrics").await, 404);
}

#[tokio::test]
async fn metrics_returns_200_text_plain_when_enabled() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, Some(true));

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_bind(&mut server.child, &addr).await;

    // Hit some routes first; health endpoints should NOT be tracked by metrics.
    assert_eq!(http_status(&addr, "/healthz").await, 200);
    assert_eq!(http_status(&addr, "/readyz").await, 200);
    assert_eq!(http_status(&addr, "/api/v1/ping").await, 200);

    let res = http_response(&addr, "/metrics").await;
    assert_eq!(response_status(&res), 200);

    let content_type = response_header(&res, "content-type").unwrap_or_default();
    assert!(
        content_type.starts_with("text/plain"),
        "unexpected content-type: {content_type}"
    );

    let body = response_body(&res);
    assert!(
        body.contains("axum_http_requests_total"),
        "missing axum http request counter"
    );
    assert!(
        body.contains("axum_http_requests_duration_seconds"),
        "missing axum http request duration histogram"
    );
    assert!(body.contains("discool_info"), "missing discool_info gauge");
    assert!(
        body.contains("discool_db_pool_connections"),
        "missing discool_db_pool_connections gauge"
    );
    assert!(
        body.contains("discool_uptime_seconds"),
        "missing discool_uptime_seconds gauge"
    );
    assert!(
        !body.contains("/healthz") && !body.contains("/readyz") && !body.contains("/metrics"),
        "health/metrics endpoints should not be tracked in metrics output"
    );
}

#[tokio::test]
async fn cold_start_readyz_is_under_5_seconds() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let start = Instant::now();
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let elapsed = start.elapsed();
    assert!(
        elapsed <= Duration::from_secs(5),
        "expected /readyz to return 200 within 5s; took {elapsed:?}"
    );
}

#[tokio::test]
async fn env_vars_override_config_toml() {
    let file_port = pick_free_port();
    let mut env_port = pick_free_port();
    while env_port == file_port {
        env_port = pick_free_port();
    }

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", file_port, None);

    let mut server = spawn_server(&dir, |cmd| {
        cmd.env("DISCOOL_SERVER__PORT", env_port.to_string());
    });

    let addr = format!("127.0.0.1:{env_port}");
    wait_for_bind(&mut server.child, &addr).await;
}

#[tokio::test]
async fn discool_config_overrides_config_toml() {
    let file_port = pick_free_port();
    let mut custom_port = pick_free_port();
    while custom_port == file_port {
        custom_port = pick_free_port();
    }

    let dir = new_temp_dir();
    let custom_path = dir.join("custom.toml");
    write_server_config(&dir.join("config.toml"), "127.0.0.1", file_port, None);
    write_server_config(&custom_path, "127.0.0.1", custom_port, None);

    let mut server = spawn_server(&dir, |cmd| {
        cmd.env("DISCOOL_CONFIG", custom_path.as_os_str());
    });

    let addr = format!("127.0.0.1:{custom_port}");
    wait_for_bind(&mut server.child, &addr).await;
}

#[tokio::test]
async fn env_vars_override_discool_config() {
    let file_port = pick_free_port();
    let mut custom_port = pick_free_port();
    while custom_port == file_port {
        custom_port = pick_free_port();
    }
    let mut env_port = pick_free_port();
    while env_port == file_port || env_port == custom_port {
        env_port = pick_free_port();
    }

    let dir = new_temp_dir();
    let custom_path = dir.join("custom.toml");
    write_server_config(&dir.join("config.toml"), "127.0.0.1", file_port, None);
    write_server_config(&custom_path, "127.0.0.1", custom_port, None);

    let mut server = spawn_server(&dir, |cmd| {
        cmd.env("DISCOOL_CONFIG", custom_path.as_os_str());
        cmd.env("DISCOOL_SERVER__PORT", env_port.to_string());
    });

    let addr = format!("127.0.0.1:{env_port}");
    wait_for_bind(&mut server.child, &addr).await;
}

#[tokio::test]
async fn instance_returns_uninitialized_on_fresh_server() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let res = http_response(&addr, "/api/v1/instance").await;
    assert_eq!(response_status(&res), 200);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value, json!({ "data": { "initialized": false } }));
}

#[tokio::test]
async fn instance_setup_then_get_returns_initialized() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({
        "admin_username": "tomas",
        "avatar_color": "#3399ff",
        "instance_name": "My Instance",
        "instance_description": "A cool place to hang out",
        "discovery_enabled": true
    })
    .to_string();

    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({
            "data": {
                "initialized": true,
                "name": "My Instance",
                "description": "A cool place to hang out",
                "discovery_enabled": true,
                "admin": {
                    "username": "tomas",
                    "avatar_color": "#3399ff"
                }
            }
        })
    );

    let res = http_response(&addr, "/api/v1/instance").await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({
            "data": {
                "initialized": true,
                "name": "My Instance",
                "description": "A cool place to hang out",
                "discovery_enabled": true,
                "admin": {
                    "username": "tomas",
                    "avatar_color": "#3399ff"
                }
            }
        })
    );
}

#[tokio::test]
async fn instance_setup_conflicts_on_second_call() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({
        "admin_username": "tomas",
        "instance_name": "My Instance"
    })
    .to_string();

    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 200);

    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 409);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "CONFLICT", "message": "Instance has already been initialized", "details": {} } })
    );
}

#[tokio::test]
async fn instance_setup_returns_409_when_initialized_even_with_invalid_body() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({
        "admin_username": "tomas",
        "instance_name": "My Instance"
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 200);

    let res = http_post(&addr, "/api/v1/instance/setup", "{}").await;
    assert_eq!(response_status(&res), 409);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "CONFLICT", "message": "Instance has already been initialized", "details": {} } })
    );
}

#[tokio::test]
async fn instance_setup_returns_422_for_missing_admin_username() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({ "instance_name": "My Instance" }).to_string();
    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 422);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "VALIDATION_ERROR", "message": "admin_username is required", "details": {} } })
    );
}

#[tokio::test]
async fn instance_setup_returns_422_for_missing_instance_name() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({ "admin_username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 422);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "VALIDATION_ERROR", "message": "instance_name is required", "details": {} } })
    );
}

#[tokio::test]
async fn admin_health_returns_403_before_instance_setup() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let res = http_response(&addr, "/api/v1/admin/health").await;
    assert_eq!(response_status(&res), 403);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "FORBIDDEN", "message": "Instance is not initialized", "details": {} } })
    );
}

#[tokio::test]
async fn admin_health_returns_200_after_instance_setup() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({
        "admin_username": "tomas",
        "instance_name": "My Instance"
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 200);

    let res = http_response(&addr, "/api/v1/admin/health").await;
    assert_eq!(response_status(&res), 200);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let data = value.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data.get("websocket_connections"), Some(&json!(0)));
    assert!(data.get("uptime_seconds").is_some());
    assert!(data.get("db_pool_max").is_some());
}

#[tokio::test]
async fn admin_backup_returns_403_before_instance_setup() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    let db_path = dir.join("discool.db");
    fs::write(&db_path, "").unwrap();
    write_server_config_with_db_url(
        &dir.join("config.toml"),
        "127.0.0.1",
        port,
        None,
        "sqlite://./discool.db",
    );

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let res = http_post(&addr, "/api/v1/admin/backup", "").await;
    assert_eq!(response_status(&res), 403);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "FORBIDDEN", "message": "Instance is not initialized", "details": {} } })
    );
}

#[tokio::test]
async fn admin_backup_returns_200_and_sqlite_magic_bytes_after_instance_setup() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    let db_path = dir.join("discool.db");
    fs::write(&db_path, "").unwrap();
    write_server_config_with_db_url(
        &dir.join("config.toml"),
        "127.0.0.1",
        port,
        None,
        "sqlite://./discool.db",
    );

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let setup = json!({
        "admin_username": "tomas",
        "instance_name": "My Instance"
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/instance/setup", &setup).await;
    assert_eq!(response_status(&res), 200);

    let res = http_post_bytes(&addr, "/api/v1/admin/backup", "").await;
    let (header, body) = response_header_and_body_bytes(&res);
    assert_eq!(response_status(&header), 200);

    let content_type = response_header(&header, "content-type").unwrap_or_default();
    assert!(
        content_type.starts_with("application/octet-stream"),
        "unexpected content-type: {content_type}"
    );

    let cache_control = response_header(&header, "cache-control").unwrap_or_default();
    assert_eq!(cache_control, "no-store");

    let content_disposition = response_header(&header, "content-disposition").unwrap_or_default();
    assert!(
        content_disposition.contains("attachment"),
        "unexpected content-disposition: {content_disposition}"
    );
    assert!(
        content_disposition.contains(".db"),
        "expected .db filename; content-disposition: {content_disposition}"
    );

    assert!(
        body.starts_with(b"SQLite format 3\0"),
        "expected sqlite magic bytes at start"
    );
}
