use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
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

fn pick_free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
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
    let mut cfg = format!(
        "[server]\nhost = \"{host}\"\nport = {port}\n\n[log]\nlevel = \"warn\"\nformat = \"json\"\n\n[database]\nurl = \"sqlite::memory:\"\nmax_connections = 1\n"
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
