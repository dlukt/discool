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

// Avoid the default Linux ephemeral port range (often starts at 32768), since it can cause
// rare test flakes when an outgoing connection grabs the port between "pick" and "bind".
static NEXT_PORT: AtomicU16 = AtomicU16::new(20_000);

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

async fn http_response_with_bearer(addr: &str, path: &str, token: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
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

async fn http_post_with_bearer(addr: &str, path: &str, json_body: &str, token: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.as_bytes().len(),
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_patch_with_bearer(addr: &str, path: &str, json_body: &str, token: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "PATCH {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.as_bytes().len(),
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_put_with_bearer(addr: &str, path: &str, json_body: &str, token: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "PUT {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.as_bytes().len(),
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_delete_with_bearer(addr: &str, path: &str, token: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "DELETE {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_post_multipart_with_bearer(
    addr: &str,
    path: &str,
    boundary: &str,
    body: &[u8],
    token: &str,
) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let headers = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: multipart/form-data; boundary={boundary}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len(),
    );
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

async fn http_post_bytes_with_bearer(
    addr: &str,
    path: &str,
    json_body: &str,
    token: &str,
) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
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

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
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

fn did_for_signing_key(secret: [u8; 32]) -> String {
    let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
    let public = signing.verifying_key().to_bytes();

    let mut bytes = Vec::with_capacity(34);
    bytes.extend_from_slice(&[0xed, 0x01]);
    bytes.extend_from_slice(&public);
    format!("did:key:z{}", bs58::encode(bytes).into_string())
}

async fn register_and_authenticate(addr: &str, username: &str, secret: [u8; 32]) -> String {
    use ed25519_dalek::Signer;
    use serde_json::json;

    let did_key = did_for_signing_key(secret);

    let register = json!({ "did_key": did_key, "username": username }).to_string();
    let res = http_post(addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    value["data"]["token"].as_str().unwrap().to_string()
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

fn write_p2p_identity(path: &Path) -> String {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id().to_string();
    let bytes = keypair.to_protobuf_encoding().unwrap();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
    peer_id
}

fn write_server_config_with_p2p(
    path: &Path,
    host: &str,
    port: u16,
    db_url: &str,
    p2p_host: &str,
    p2p_port: u16,
    identity_key_path: &str,
    bootstrap_peers: &[String],
) {
    let mut cfg = format!(
        "[server]\nhost = \"{host}\"\nport = {port}\n\n[log]\nlevel = \"warn\"\nformat = \"json\"\n\n[database]\nurl = \"{db_url}\"\nmax_connections = 1\n\n[p2p]\nenabled = true\nlisten_host = \"{p2p_host}\"\nlisten_port = {p2p_port}\nidentity_key_path = \"{identity_key_path}\"\ndiscovery_retry_initial_secs = 1\ndiscovery_retry_max_secs = 4\ndiscovery_retry_jitter_millis = 0\ndiscovery_refresh_interval_secs = 1\n"
    );
    if bootstrap_peers.is_empty() {
        cfg.push_str("bootstrap_peers = []\n");
    } else {
        let joined = bootstrap_peers
            .iter()
            .map(|peer| format!("\"{peer}\""))
            .collect::<Vec<_>>()
            .join(", ");
        cfg.push_str(&format!("bootstrap_peers = [{joined}]\n"));
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
async fn server_stays_up_when_p2p_startup_fails() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    let cfg_path = dir.join("config.toml");
    write_server_config(&cfg_path, "127.0.0.1", port, None);

    let mut cfg = fs::read_to_string(&cfg_path).unwrap();
    cfg.push_str("\n[p2p]\nlisten_host = \"not-an-ip\"\n");
    fs::write(&cfg_path, cfg).unwrap();

    let mut server = spawn_server(&dir, |_| {});
    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    assert_eq!(http_status(&addr, "/healthz").await, 200);
    assert_eq!(http_status(&addr, "/readyz").await, 200);
}

#[tokio::test]
async fn server_stays_up_with_unreachable_bootstrap_peer() {
    use serde_json::json;

    let port = pick_free_port();
    let p2p_port = pick_free_port();

    let dir = new_temp_dir();
    let bootstrap_identity_path = dir.join("bootstrap-peer.key");
    let bootstrap_peer_id = write_p2p_identity(&bootstrap_identity_path);
    let bootstrap_addr = format!(
        "/ip4/127.0.0.1/tcp/{}/p2p/{}",
        pick_free_port(),
        bootstrap_peer_id
    );

    write_server_config_with_p2p(
        &dir.join("config.toml"),
        "127.0.0.1",
        port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_port,
        "./data/p2p/identity.key",
        &[bootstrap_addr],
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

    let token = register_and_authenticate(&addr, "tomas", [3u8; 32]).await;

    sleep(Duration::from_secs(3)).await;

    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let data = value.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data.get("p2p_connection_count"), Some(&json!(0)));
    assert_eq!(data.get("p2p_discovered_instances"), Some(&json!(0)));
    assert_eq!(data.get("p2p_message_rate_per_minute"), Some(&json!(0.0)));
    assert_eq!(data.get("p2p_rejected_total"), Some(&json!(0)));
    assert_eq!(data.get("p2p_throttled_total"), Some(&json!(0)));
    assert_eq!(data.get("p2p_discovery_enabled"), Some(&json!(true)));
    assert_eq!(data.get("p2p_discovery_label"), Some(&json!("Enabled")));

    assert_eq!(http_status(&addr, "/healthz").await, 200);
    assert_eq!(http_status(&addr, "/readyz").await, 200);
}

#[tokio::test]
async fn p2p_bootstrap_discovers_other_instance_within_startup_window() {
    use serde_json::json;

    let server_a_port = pick_free_port();
    let server_b_port = pick_free_port();
    let p2p_a_port = pick_free_port();
    let p2p_b_port = pick_free_port();

    let dir_a = new_temp_dir();
    let dir_b = new_temp_dir();

    let peer_id_a = write_p2p_identity(&dir_a.join("data/p2p/identity.key"));
    let _peer_id_b = write_p2p_identity(&dir_b.join("data/p2p/identity.key"));

    write_server_config_with_p2p(
        &dir_a.join("config.toml"),
        "127.0.0.1",
        server_a_port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_a_port,
        "./data/p2p/identity.key",
        &[],
    );

    let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{p2p_a_port}/p2p/{peer_id_a}");
    write_server_config_with_p2p(
        &dir_b.join("config.toml"),
        "127.0.0.1",
        server_b_port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_b_port,
        "./data/p2p/identity.key",
        &[bootstrap_addr],
    );

    let mut server_a = spawn_server(&dir_a, |_| {});
    let addr_a = format!("127.0.0.1:{server_a_port}");
    wait_for_http_status(&mut server_a.child, &addr_a, "/readyz", 200).await;

    let mut server_b = spawn_server(&dir_b, |_| {});
    let addr_b = format!("127.0.0.1:{server_b_port}");
    wait_for_http_status(&mut server_b.child, &addr_b, "/readyz", 200).await;

    let setup_a = json!({
        "admin_username": "bootstrap-admin",
        "instance_name": "Bootstrap Instance"
    })
    .to_string();
    let res = http_post(&addr_a, "/api/v1/instance/setup", &setup_a).await;
    assert_eq!(response_status(&res), 200);

    let setup_b = json!({
        "admin_username": "tomas",
        "instance_name": "Joining Instance"
    })
    .to_string();
    let res = http_post(&addr_b, "/api/v1/instance/setup", &setup_b).await;
    assert_eq!(response_status(&res), 200);

    let token = register_and_authenticate(&addr_b, "tomas", [4u8; 32]).await;

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut discovered = false;
    while Instant::now() < deadline {
        let res = http_response_with_bearer(&addr_b, "/api/v1/admin/health", &token).await;
        if response_status(&res) == 200 {
            let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
            let data = value.get("data").and_then(|v| v.as_object()).unwrap();
            let connection_count = data
                .get("p2p_connection_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let discovered_instances = data
                .get("p2p_discovered_instances")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if connection_count >= 1 && discovered_instances >= 1 {
                discovered = true;
                break;
            }
        }
        sleep(Duration::from_millis(200)).await;
    }

    assert!(
        discovered,
        "expected discovery to report at least one connection and one discovered instance within 20 seconds"
    );
}

#[tokio::test]
async fn p2p_discovery_disabled_runs_unlisted_without_bootstrap_advertisement() {
    use serde_json::json;

    let server_a_port = pick_free_port();
    let server_b_port = pick_free_port();
    let p2p_a_port = pick_free_port();
    let p2p_b_port = pick_free_port();

    let dir_a = new_temp_dir();
    let dir_b = new_temp_dir();

    let peer_id_a = write_p2p_identity(&dir_a.join("data/p2p/identity.key"));
    let _peer_id_b = write_p2p_identity(&dir_b.join("data/p2p/identity.key"));

    write_server_config_with_p2p(
        &dir_a.join("config.toml"),
        "127.0.0.1",
        server_a_port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_a_port,
        "./data/p2p/identity.key",
        &[],
    );

    let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{p2p_a_port}/p2p/{peer_id_a}");
    let config_b_path = dir_b.join("config.toml");
    write_server_config_with_p2p(
        &config_b_path,
        "127.0.0.1",
        server_b_port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_b_port,
        "./data/p2p/identity.key",
        &[bootstrap_addr],
    );
    let mut cfg_b = fs::read_to_string(&config_b_path).unwrap();
    cfg_b.push_str("discovery.enabled = false\n");
    fs::write(&config_b_path, cfg_b).unwrap();

    let mut server_a = spawn_server(&dir_a, |_| {});
    let addr_a = format!("127.0.0.1:{server_a_port}");
    wait_for_http_status(&mut server_a.child, &addr_a, "/readyz", 200).await;

    let mut server_b = spawn_server(&dir_b, |_| {});
    let addr_b = format!("127.0.0.1:{server_b_port}");
    wait_for_http_status(&mut server_b.child, &addr_b, "/readyz", 200).await;

    let setup_a = json!({
        "admin_username": "bootstrap-admin",
        "instance_name": "Bootstrap Instance"
    })
    .to_string();
    let res = http_post(&addr_a, "/api/v1/instance/setup", &setup_a).await;
    assert_eq!(response_status(&res), 200);

    let setup_b = json!({
        "admin_username": "private-admin",
        "instance_name": "Private Instance",
        "discovery_enabled": false
    })
    .to_string();
    let res = http_post(&addr_b, "/api/v1/instance/setup", &setup_b).await;
    assert_eq!(response_status(&res), 200);

    let token_a = register_and_authenticate(&addr_a, "bootstrap-admin", [5u8; 32]).await;
    let token_b = register_and_authenticate(&addr_b, "private-admin", [6u8; 32]).await;

    sleep(Duration::from_secs(3)).await;

    let instance_res = http_response(&addr_b, "/api/v1/instance").await;
    assert_eq!(response_status(&instance_res), 200);
    let instance_value: serde_json::Value =
        serde_json::from_str(response_body(&instance_res)).unwrap();
    assert_eq!(instance_value["data"]["discovery_enabled"], json!(false));

    let health_b = http_response_with_bearer(&addr_b, "/api/v1/admin/health", &token_b).await;
    assert_eq!(response_status(&health_b), 200);
    let value_b: serde_json::Value = serde_json::from_str(response_body(&health_b)).unwrap();
    let data_b = value_b.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data_b.get("p2p_discovery_enabled"), Some(&json!(false)));
    assert_eq!(
        data_b.get("p2p_discovery_label"),
        Some(&json!("Disabled (Unlisted)"))
    );
    assert_eq!(data_b.get("p2p_connection_count"), Some(&json!(0)));
    assert_eq!(data_b.get("p2p_discovered_instances"), Some(&json!(0)));

    let health_a = http_response_with_bearer(&addr_a, "/api/v1/admin/health", &token_a).await;
    assert_eq!(response_status(&health_a), 200);
    let value_a: serde_json::Value = serde_json::from_str(response_body(&health_a)).unwrap();
    let data_a = value_a.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data_a.get("p2p_discovered_instances"), Some(&json!(0)));
}

#[tokio::test]
async fn p2p_discovery_reenabled_after_restart_resumes_discovery() {
    use serde_json::json;

    let server_a_port = pick_free_port();
    let server_b_port = pick_free_port();
    let p2p_a_port = pick_free_port();
    let p2p_b_port = pick_free_port();

    let dir_a = new_temp_dir();
    let dir_b = new_temp_dir();

    let peer_id_a = write_p2p_identity(&dir_a.join("data/p2p/identity.key"));
    let _peer_id_b = write_p2p_identity(&dir_b.join("data/p2p/identity.key"));

    write_server_config_with_p2p(
        &dir_a.join("config.toml"),
        "127.0.0.1",
        server_a_port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_a_port,
        "./data/p2p/identity.key",
        &[],
    );

    let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{p2p_a_port}/p2p/{peer_id_a}");
    let config_b_path = dir_b.join("config.toml");
    write_server_config_with_p2p(
        &config_b_path,
        "127.0.0.1",
        server_b_port,
        "sqlite::memory:",
        "127.0.0.1",
        p2p_b_port,
        "./data/p2p/identity.key",
        &[bootstrap_addr],
    );
    let mut cfg_b = fs::read_to_string(&config_b_path).unwrap();
    cfg_b.push_str("discovery.enabled = false\n");
    fs::write(&config_b_path, cfg_b).unwrap();

    let mut server_a = spawn_server(&dir_a, |_| {});
    let addr_a = format!("127.0.0.1:{server_a_port}");
    wait_for_http_status(&mut server_a.child, &addr_a, "/readyz", 200).await;

    let mut server_b = spawn_server(&dir_b, |_| {});
    let addr_b = format!("127.0.0.1:{server_b_port}");
    wait_for_http_status(&mut server_b.child, &addr_b, "/readyz", 200).await;

    let setup_a = json!({
        "admin_username": "bootstrap-admin",
        "instance_name": "Bootstrap Instance"
    })
    .to_string();
    let res = http_post(&addr_a, "/api/v1/instance/setup", &setup_a).await;
    assert_eq!(response_status(&res), 200);

    let setup_b = json!({
        "admin_username": "private-admin",
        "instance_name": "Private Instance",
        "discovery_enabled": false
    })
    .to_string();
    let res = http_post(&addr_b, "/api/v1/instance/setup", &setup_b).await;
    assert_eq!(response_status(&res), 200);

    let token_b = register_and_authenticate(&addr_b, "private-admin", [7u8; 32]).await;

    sleep(Duration::from_secs(3)).await;

    let health_b = http_response_with_bearer(&addr_b, "/api/v1/admin/health", &token_b).await;
    assert_eq!(response_status(&health_b), 200);
    let value_b: serde_json::Value = serde_json::from_str(response_body(&health_b)).unwrap();
    let data_b = value_b.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data_b.get("p2p_discovery_enabled"), Some(&json!(false)));
    assert_eq!(data_b.get("p2p_connection_count"), Some(&json!(0)));
    assert_eq!(data_b.get("p2p_discovered_instances"), Some(&json!(0)));

    server_b.child.kill().unwrap();
    let _ = server_b.child.wait();

    let cfg_b = fs::read_to_string(&config_b_path).unwrap();
    let cfg_b = cfg_b.replace("discovery.enabled = false\n", "discovery.enabled = true\n");
    fs::write(&config_b_path, cfg_b).unwrap();

    let mut server_b_restarted = spawn_server(&dir_b, |_| {});
    wait_for_http_status(&mut server_b_restarted.child, &addr_b, "/readyz", 200).await;

    let setup_b_restarted = json!({
        "admin_username": "private-admin-2",
        "instance_name": "Private Instance Reenabled"
    })
    .to_string();
    let res = http_post(&addr_b, "/api/v1/instance/setup", &setup_b_restarted).await;
    assert_eq!(response_status(&res), 200);

    let token_b_restarted = register_and_authenticate(&addr_b, "private-admin-2", [8u8; 32]).await;

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut discovered = false;
    while Instant::now() < deadline {
        let res =
            http_response_with_bearer(&addr_b, "/api/v1/admin/health", &token_b_restarted).await;
        if response_status(&res) == 200 {
            let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
            let data = value.get("data").and_then(|v| v.as_object()).unwrap();
            let discovery_enabled = data
                .get("p2p_discovery_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let connection_count = data
                .get("p2p_connection_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let discovered_instances = data
                .get("p2p_discovered_instances")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if discovery_enabled && connection_count >= 1 && discovered_instances >= 1 {
                discovered = true;
                break;
            }
        }
        sleep(Duration::from_millis(200)).await;
    }

    assert!(
        discovered,
        "expected discovery to resume after re-enabling config and restarting instance"
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
async fn admin_health_returns_401_before_instance_setup() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let res = http_response(&addr, "/api/v1/admin/health").await;
    assert_eq!(response_status(&res), 401);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "UNAUTHORIZED", "message": "Missing Authorization header", "details": {} } })
    );
}

#[tokio::test]
async fn admin_health_returns_200_after_instance_setup() {
    use ed25519_dalek::Signer;
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

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let token = value["data"]["token"].as_str().unwrap();

    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", token).await;
    assert_eq!(response_status(&res), 200);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let data = value.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data.get("websocket_connections"), Some(&json!(0)));
    assert_eq!(data.get("p2p_discovered_instances"), Some(&json!(0)));
    assert_eq!(data.get("p2p_connection_count"), Some(&json!(0)));
    assert_eq!(data.get("p2p_message_rate_per_minute"), Some(&json!(0.0)));
    assert_eq!(data.get("p2p_rejected_total"), Some(&json!(0)));
    assert_eq!(data.get("p2p_throttled_total"), Some(&json!(0)));
    assert_eq!(data.get("p2p_discovery_enabled"), Some(&json!(true)));
    assert_eq!(data.get("p2p_discovery_label"), Some(&json!("Enabled")));
    assert!(data.get("uptime_seconds").is_some());
    assert!(data.get("db_pool_max").is_some());
}

#[tokio::test]
async fn admin_health_reports_discovery_disabled_when_p2p_runtime_disabled() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    let cfg_path = dir.join("config.toml");
    write_server_config(&cfg_path, "127.0.0.1", port, None);

    let mut cfg = fs::read_to_string(&cfg_path).unwrap();
    cfg.push_str("\n[p2p]\nenabled = false\ndiscovery.enabled = true\n");
    fs::write(&cfg_path, cfg).unwrap();

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

    let token = register_and_authenticate(&addr, "tomas", [11u8; 32]).await;
    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", &token).await;
    assert_eq!(response_status(&res), 200);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let data = value.get("data").and_then(|v| v.as_object()).unwrap();
    assert_eq!(data.get("p2p_discovery_enabled"), Some(&json!(false)));
    assert_eq!(
        data.get("p2p_discovery_label"),
        Some(&json!("Disabled (Unlisted)"))
    );
}

#[tokio::test]
async fn admin_health_returns_403_for_non_admin_user() {
    use ed25519_dalek::Signer;
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

    // Register admin user.
    let admin_did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": admin_did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    // Register non-admin user (same instance).
    let did_key = did_for_signing_key([2u8; 32]);
    let register = json!({ "did_key": did_key, "username": "liam" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    // Authenticate non-admin user.
    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let token = value["data"]["token"].as_str().unwrap();

    // Non-admin should get 403 Forbidden.
    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", token).await;
    assert_eq!(response_status(&res), 403);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "FORBIDDEN", "message": "Admin access required", "details": {} } })
    );
}

#[tokio::test]
async fn admin_backup_returns_401_when_missing_auth() {
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
    assert_eq!(response_status(&res), 401);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "UNAUTHORIZED", "message": "Missing Authorization header", "details": {} } })
    );
}

#[tokio::test]
async fn admin_backup_returns_200_and_sqlite_magic_bytes_after_instance_setup() {
    use ed25519_dalek::Signer;
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

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap().to_string();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let token = value["data"]["token"].as_str().unwrap();

    let res = http_post_bytes_with_bearer(&addr, "/api/v1/admin/backup", "", token).await;
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

#[tokio::test]
async fn auth_register_returns_201_with_expected_json() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let req = json!({
        "did_key": did_key,
        "username": "liam",
        "avatar_color": "#3B82F6"
    })
    .to_string();

    let res = http_post(&addr, "/api/v1/auth/register", &req).await;
    assert_eq!(response_status(&res), 201);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"]["username"], json!("liam"));
    assert_eq!(value["data"]["avatar_color"], json!("#3B82F6"));
    assert!(value["data"]["id"].as_str().is_some());
    assert!(value["data"]["created_at"].as_str().is_some());
}

#[tokio::test]
async fn auth_register_returns_409_for_duplicate_did() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let req1 = json!({ "did_key": did_key, "username": "liam" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &req1).await;
    assert_eq!(response_status(&res), 201);

    let req2 =
        json!({ "did_key": did_for_signing_key([1u8; 32]), "username": "other" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &req2).await;
    assert_eq!(response_status(&res), 409);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "CONFLICT", "message": "Identity already registered on this instance", "details": {} } })
    );
}

#[tokio::test]
async fn auth_register_returns_409_for_duplicate_username() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let req1 = json!({
        "did_key": did_for_signing_key([1u8; 32]),
        "username": "liam"
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &req1).await;
    assert_eq!(response_status(&res), 201);

    let req2 = json!({
        "did_key": did_for_signing_key([2u8; 32]),
        "username": "liam"
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &req2).await;
    assert_eq!(response_status(&res), 409);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "CONFLICT", "message": "Username already taken", "details": {} } })
    );
}

#[tokio::test]
async fn auth_register_returns_422_for_invalid_did_format() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let req = json!({ "did_key": "nope", "username": "liam" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &req).await;
    assert_eq!(response_status(&res), 422);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "VALIDATION_ERROR", "message": "Invalid DID format: must start with did:key:z6Mk", "details": {} } })
    );
}

#[tokio::test]
async fn auth_register_returns_422_for_empty_username() {
    use serde_json::json;

    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let req = json!({
        "did_key": did_for_signing_key([1u8; 32]),
        "username": "   "
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &req).await;
    assert_eq!(response_status(&res), 422);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "VALIDATION_ERROR", "message": "username is required", "details": {} } })
    );
}

#[tokio::test]
async fn auth_challenge_returns_200_for_registered_did() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap_or("");
    assert_eq!(challenge.len(), 64);
    assert!(challenge.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn auth_challenge_returns_404_for_unregistered_did() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let challenge_req = json!({ "did_key": did_for_signing_key([1u8; 32]) }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 404);
}

#[tokio::test]
async fn auth_verify_returns_200_for_valid_signature() {
    use ed25519_dalek::Signer;
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();

    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert!(value["data"]["token"].as_str().is_some());
    assert!(value["data"]["expires_at"].as_str().is_some());
    assert!(value["data"]["user"]["id"].as_str().is_some());
}

#[tokio::test]
async fn auth_verify_returns_401_for_invalid_signature() {
    use ed25519_dalek::Signer;
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "liam" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]); // wrong key
    let sig = signing.sign(challenge.as_bytes()).to_bytes();

    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn auth_verify_returns_401_for_expired_challenge() {
    use ed25519_dalek::Signer;
    use serde_json::json;
    use std::io::Write;

    let port = pick_free_port();
    let dir = new_temp_dir();

    // Override the challenge TTL to make expiry test fast.
    let cfg_path = dir.join("config.toml");
    write_server_config(&cfg_path, "127.0.0.1", port, None);
    let mut f = fs::OpenOptions::new().append(true).open(&cfg_path).unwrap();
    f.write_all(b"\n[auth]\nchallenge_ttl_seconds = 1\n")
        .unwrap();

    let mut server = spawn_server(&dir, |_| {});
    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "liam" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    sleep(Duration::from_secs(2)).await;

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();

    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn auth_verify_rejects_replay() {
    use ed25519_dalek::Signer;
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "liam" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();

    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);

    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn auth_logout_invalidates_token_for_protected_routes() {
    use ed25519_dalek::Signer;
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

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let token = value["data"]["token"].as_str().unwrap();

    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", token).await;
    assert_eq!(response_status(&res), 200);

    let res = http_delete_with_bearer(&addr, "/api/v1/auth/logout", token).await;
    assert_eq!(response_status(&res), 204);

    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", token).await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn expired_session_returns_401() {
    use ed25519_dalek::Signer;
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

    let did_key = did_for_signing_key([1u8; 32]);
    let register = json!({ "did_key": did_key, "username": "tomas" }).to_string();
    let res = http_post(&addr, "/api/v1/auth/register", &register).await;
    assert_eq!(response_status(&res), 201);

    let challenge_req = json!({ "did_key": did_key }).to_string();
    let res = http_post(&addr, "/api/v1/auth/challenge", &challenge_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let challenge = value["data"]["challenge"].as_str().unwrap();

    let signing = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let sig = signing.sign(challenge.as_bytes()).to_bytes();
    let verify_req = json!({
        "did_key": did_key,
        "challenge": challenge,
        "signature": bytes_to_hex(&sig),
    })
    .to_string();
    let res = http_post(&addr, "/api/v1/auth/verify", &verify_req).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let token = value["data"]["token"].as_str().unwrap();

    let url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("UPDATE sessions SET expires_at = ?1 WHERE token = ?2")
        .bind("2000-01-01T00:00:00Z")
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();
    drop(pool);

    let res = http_response_with_bearer(&addr, "/api/v1/admin/health", token).await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn guilds_require_authentication() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let create_body = json!({ "name": "Makers Hub" }).to_string();
    let res = http_post(&addr, "/api/v1/guilds", &create_body).await;
    assert_eq!(response_status(&res), 401);

    let res = http_response(&addr, "/api/v1/guilds").await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn guilds_create_sets_owner_and_default_general_channel() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "liam", [21u8; 32]).await;
    let create_body = json!({
        "name": "Makers Hub",
        "description": "Build cool things",
    })
    .to_string();

    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &create_body, &token).await;
    assert_eq!(response_status(&res), 201);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"]["name"], json!("Makers Hub"));
    assert_eq!(value["data"]["slug"], json!("makers-hub"));
    assert_eq!(value["data"]["default_channel_slug"], json!("general"));
    assert_eq!(value["data"]["is_owner"], json!(true));

    let res = http_response_with_bearer(&addr, "/api/v1/guilds", &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"].as_array().unwrap().len(), 1);
    assert_eq!(value["data"][0]["slug"], json!("makers-hub"));
}

#[tokio::test]
async fn guilds_update_rejects_non_owner_and_allows_owner() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner", [31u8; 32]).await;
    let create_body = json!({ "name": "Owner Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &create_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let created: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let slug = created["data"]["slug"].as_str().unwrap();

    let other_token = register_and_authenticate(&addr, "other", [32u8; 32]).await;
    let patch_body = json!({ "name": "Attempted Takeover" }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{slug}"),
        &patch_body,
        &other_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["error"]["code"], json!("FORBIDDEN"));

    let owner_patch = json!({
        "name": "Owner Guild Updated",
        "description": "Updated by owner",
    })
    .to_string();
    let res = http_patch_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{slug}"),
        &owner_patch,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);
    let updated: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(updated["data"]["name"], json!("Owner Guild Updated"));
    assert_eq!(updated["data"]["description"], json!("Updated by owner"));
}

#[tokio::test]
async fn channels_mutations_require_authentication() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let create_body = json!({ "name": "Ops", "channel_type": "text" }).to_string();
    let res = http_post(&addr, "/api/v1/guilds/lobby/channels", &create_body).await;
    assert_eq!(response_status(&res), 401);

    let reorder_body = json!({ "channel_slugs": ["general"] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        "/api/v1/guilds/lobby/channels/reorder",
        &reorder_body,
        "bad-token",
    )
    .await;
    assert_eq!(response_status(&res), 401);

    let res =
        http_delete_with_bearer(&addr, "/api/v1/guilds/lobby/channels/general", "bad-token").await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn channels_owner_crud_reorder_and_default_fallback_work() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "owner", [41u8; 32]).await;
    let guild_body = json!({ "name": "Owner Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let list_path = format!("/api/v1/guilds/{guild_slug}/channels");
    let res = http_response_with_bearer(&addr, &list_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"].as_array().unwrap().len(), 1);
    assert_eq!(value["data"][0]["slug"], json!("general"));
    assert_eq!(value["data"][0]["channel_type"], json!("text"));

    let create_voice = json!({
        "name": "Standup Voice",
        "channel_type": "voice",
    })
    .to_string();
    let res = http_post_with_bearer(&addr, &list_path, &create_voice, &token).await;
    assert_eq!(response_status(&res), 201);
    let created_voice: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let voice_slug = created_voice["data"]["slug"].as_str().unwrap().to_string();

    let create_text = json!({
        "name": "Announcements",
        "channel_type": "text",
    })
    .to_string();
    let res = http_post_with_bearer(&addr, &list_path, &create_text, &token).await;
    assert_eq!(response_status(&res), 201);
    let created_text: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let text_slug = created_text["data"]["slug"].as_str().unwrap().to_string();

    let rename_body = json!({ "name": "Release Notes" }).to_string();
    let rename_path = format!("/api/v1/guilds/{guild_slug}/channels/{text_slug}");
    let res = http_patch_with_bearer(&addr, &rename_path, &rename_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let renamed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let renamed_slug = renamed["data"]["slug"].as_str().unwrap().to_string();

    let reorder_body = json!({
        "channel_slugs": [voice_slug, renamed_slug, "general"],
    })
    .to_string();
    let reorder_path = format!("/api/v1/guilds/{guild_slug}/channels/reorder");
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let reordered: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(reordered["data"][0]["channel_type"], json!("voice"));
    assert_eq!(reordered["data"][0]["position"], json!(0));
    assert_eq!(reordered["data"][1]["slug"], json!(renamed_slug));
    assert_eq!(reordered["data"][2]["slug"], json!("general"));

    let delete_voice_path = format!("/api/v1/guilds/{guild_slug}/channels/{voice_slug}");
    let res = http_delete_with_bearer(&addr, &delete_voice_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let deleted_voice: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(deleted_voice["data"]["deleted_slug"], json!(voice_slug));
    assert_eq!(
        deleted_voice["data"]["fallback_channel_slug"],
        json!("general")
    );

    let delete_general_path = format!("/api/v1/guilds/{guild_slug}/channels/general");
    let res = http_delete_with_bearer(&addr, &delete_general_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let deleted_general: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(deleted_general["data"]["deleted_slug"], json!("general"));
    assert_eq!(
        deleted_general["data"]["fallback_channel_slug"],
        json!(renamed_slug)
    );

    let res = http_response_with_bearer(&addr, &list_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let listed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(listed["data"].as_array().unwrap().len(), 1);
    assert_eq!(listed["data"][0]["slug"], json!(renamed_slug));
    assert_eq!(listed["data"][0]["is_default"], json!(true));

    let delete_last_path = format!("/api/v1/guilds/{guild_slug}/channels/{renamed_slug}");
    let res = http_delete_with_bearer(&addr, &delete_last_path, &token).await;
    assert_eq!(response_status(&res), 422);
}

#[tokio::test]
async fn channels_mutations_reject_non_owner() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner", [51u8; 32]).await;
    let other_token = register_and_authenticate(&addr, "other", [52u8; 32]).await;
    let guild_body = json!({ "name": "Owner Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let create_path = format!("/api/v1/guilds/{guild_slug}/channels");
    let create_body = json!({ "name": "Ops", "channel_type": "text" }).to_string();
    let res = http_post_with_bearer(&addr, &create_path, &create_body, &other_token).await;
    assert_eq!(response_status(&res), 403);
    let forbidden: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(forbidden["error"]["code"], json!("FORBIDDEN"));

    let update_path = format!("/api/v1/guilds/{guild_slug}/channels/general");
    let update_body = json!({ "name": "General Updated" }).to_string();
    let res = http_patch_with_bearer(&addr, &update_path, &update_body, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let reorder_path = format!("/api/v1/guilds/{guild_slug}/channels/reorder");
    let reorder_body = json!({ "channel_slugs": ["general"] }).to_string();
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_body, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let delete_path = format!("/api/v1/guilds/{guild_slug}/channels/general");
    let res = http_delete_with_bearer(&addr, &delete_path, &other_token).await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn channel_permission_overrides_private_visibility_and_cache_invalidation() {
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

    let owner_token = register_and_authenticate(&addr, "owner-overrides", [81u8; 32]).await;
    let manager_token = register_and_authenticate(&addr, "manager-overrides", [82u8; 32]).await;
    let viewer_token = register_and_authenticate(&addr, "viewer-overrides", [83u8; 32]).await;

    let guild_body = json!({ "name": "Overrides Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap().to_string();
    let guild_id = guild["data"]["id"].as_str().unwrap().to_string();
    let channels_path = format!("/api/v1/guilds/{guild_slug}/channels");
    let roles_path = format!("/api/v1/guilds/{guild_slug}/roles");

    let create_role_body = json!({ "name": "Channel Manager", "color": "#3366ff" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_role_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let manager_role: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let manager_role_id = manager_role["data"]["id"].as_str().unwrap().to_string();

    let update_role_path = format!("/api/v1/guilds/{guild_slug}/roles/{manager_role_id}");
    let update_role_body = json!({ "permissions_bitflag": 2 }).to_string();
    let res =
        http_patch_with_bearer(&addr, &update_role_path, &update_role_body, &owner_token).await;
    assert_eq!(response_status(&res), 200);

    let url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let manager_id = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?1")
        .bind("manager-overrides")
        .fetch_one(&pool)
        .await
        .unwrap();
    let viewer_id = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?1")
        .bind("viewer-overrides")
        .fetch_one(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)",
    )
    .bind(&guild_id)
    .bind(&manager_id)
    .bind("2026-02-28T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)",
    )
    .bind(&guild_id)
    .bind(&viewer_id)
    .bind("2026-02-28T00:00:01Z")
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&guild_id)
    .bind(&manager_id)
    .bind(&manager_role_id)
    .bind("2026-02-28T00:00:02Z")
    .execute(&pool)
    .await
    .unwrap();
    drop(pool);

    let create_secret_body = json!({ "name": "Secret", "channel_type": "text" }).to_string();
    let res =
        http_post_with_bearer(&addr, &channels_path, &create_secret_body, &manager_token).await;
    assert_eq!(response_status(&res), 201);
    let secret_channel: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let secret_slug = secret_channel["data"]["slug"].as_str().unwrap().to_string();

    let overrides_path =
        format!("/api/v1/guilds/{guild_slug}/channels/{secret_slug}/permission-overrides");
    let manager_override_path = format!("{overrides_path}/{manager_role_id}");

    let res = http_response(&addr, &overrides_path).await;
    assert_eq!(response_status(&res), 401);

    let bad_token_body = json!({ "allow_bitflag": 0, "deny_bitflag": 0 }).to_string();
    let res =
        http_put_with_bearer(&addr, &manager_override_path, &bad_token_body, "bad-token").await;
    assert_eq!(response_status(&res), 401);

    let res = http_response_with_bearer(&addr, &overrides_path, &viewer_token).await;
    assert_eq!(response_status(&res), 403);

    let res = http_response_with_bearer(&addr, &roles_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let roles_value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let everyone_role_id = roles_value["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["is_default"] == json!(true))
        .and_then(|item| item["id"].as_str())
        .unwrap()
        .to_string();

    let overlap_body = json!({ "allow_bitflag": 4096, "deny_bitflag": 4096 }).to_string();
    let res =
        http_put_with_bearer(&addr, &manager_override_path, &overlap_body, &owner_token).await;
    assert_eq!(response_status(&res), 422);

    let unknown_role_path = format!("{overrides_path}/unknown-role");
    let valid_body = json!({ "allow_bitflag": 0, "deny_bitflag": 4096 }).to_string();
    let res = http_put_with_bearer(&addr, &unknown_role_path, &valid_body, &owner_token).await;
    assert_eq!(response_status(&res), 422);

    let everyone_override_path = format!("{overrides_path}/{everyone_role_id}");
    let deny_view_body = json!({ "allow_bitflag": 0, "deny_bitflag": 4096 }).to_string();
    let res = http_put_with_bearer(
        &addr,
        &everyone_override_path,
        &deny_view_body,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let res = http_response_with_bearer(&addr, &overrides_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let listed_overrides: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let everyone_override = listed_overrides["data"]["overrides"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["role_id"] == json!(everyone_role_id))
        .unwrap();
    assert_eq!(everyone_override["deny_bitflag"], json!(4096));

    let res = http_response_with_bearer(&addr, &channels_path, &viewer_token).await;
    assert_eq!(response_status(&res), 200);
    let viewer_channels: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let viewer_slugs: Vec<&str> = viewer_channels["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["slug"].as_str())
        .collect();
    assert!(!viewer_slugs.contains(&secret_slug.as_str()));

    let allow_manager_view_body = json!({ "allow_bitflag": 4096, "deny_bitflag": 0 }).to_string();
    let res = http_put_with_bearer(
        &addr,
        &manager_override_path,
        &allow_manager_view_body,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let res = http_response_with_bearer(&addr, &channels_path, &manager_token).await;
    assert_eq!(response_status(&res), 200);
    let manager_channels: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let manager_slugs: Vec<&str> = manager_channels["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["slug"].as_str())
        .collect();
    assert!(manager_slugs.contains(&secret_slug.as_str()));

    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("UPDATE roles SET permissions_bitflag = 0 WHERE id = ?1")
        .bind(&manager_role_id)
        .execute(&pool)
        .await
        .unwrap();
    drop(pool);

    let stale_create_body =
        json!({ "name": "Cache Before Flush", "channel_type": "text" }).to_string();
    let res =
        http_post_with_bearer(&addr, &channels_path, &stale_create_body, &manager_token).await;
    assert_eq!(response_status(&res), 201);

    let res = http_delete_with_bearer(&addr, &manager_override_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);

    let after_flush_body =
        json!({ "name": "Cache After Flush", "channel_type": "text" }).to_string();
    let res = http_post_with_bearer(&addr, &channels_path, &after_flush_body, &manager_token).await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn categories_mutations_require_authentication() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let create_body = json!({ "name": "Ops" }).to_string();
    let res = http_post(&addr, "/api/v1/guilds/lobby/categories", &create_body).await;
    assert_eq!(response_status(&res), 401);

    let collapse_body = json!({ "collapsed": true }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        "/api/v1/guilds/lobby/categories/ops/collapse",
        &collapse_body,
        "bad-token",
    )
    .await;
    assert_eq!(response_status(&res), 401);

    let res =
        http_delete_with_bearer(&addr, "/api/v1/guilds/lobby/categories/ops", "bad-token").await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn categories_owner_crud_collapse_and_delete_move_channels_to_uncategorized() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "owner-categories", [53u8; 32]).await;
    let guild_body = json!({ "name": "Category Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let categories_path = format!("/api/v1/guilds/{guild_slug}/categories");
    let create_category = json!({ "name": "Ops" }).to_string();
    let res = http_post_with_bearer(&addr, &categories_path, &create_category, &token).await;
    assert_eq!(response_status(&res), 201);
    let category: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(category["data"]["slug"], json!("ops"));
    assert_eq!(category["data"]["collapsed"], json!(false));

    let list_channels_path = format!("/api/v1/guilds/{guild_slug}/channels");
    let create_channel = json!({ "name": "Incidents", "channel_type": "text" }).to_string();
    let res = http_post_with_bearer(&addr, &list_channels_path, &create_channel, &token).await;
    assert_eq!(response_status(&res), 201);
    let created_channel: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let incidents_slug = created_channel["data"]["slug"].as_str().unwrap();

    let reorder_body = json!({
        "channel_positions": [
            { "channel_slug": "general", "category_slug": null, "position": 0 },
            { "channel_slug": incidents_slug, "category_slug": "ops", "position": 0 }
        ]
    })
    .to_string();
    let reorder_path = format!("/api/v1/guilds/{guild_slug}/channels/reorder");
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let reordered: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let incidents = reordered["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["slug"] == json!(incidents_slug))
        .unwrap();
    assert_eq!(incidents["category_slug"], json!("ops"));
    assert_eq!(incidents["position"], json!(0));

    let reorder_by_slug_body = json!({
        "channel_slugs": [incidents_slug, "general"]
    })
    .to_string();
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_by_slug_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let reordered_by_slug: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let incidents_after_slug_reorder = reordered_by_slug["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["slug"] == json!(incidents_slug))
        .unwrap();
    assert_eq!(incidents_after_slug_reorder["category_slug"], json!("ops"));
    assert_eq!(incidents_after_slug_reorder["position"], json!(0));
    let general_after_slug_reorder = reordered_by_slug["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["slug"] == json!("general"))
        .unwrap();
    assert_eq!(
        general_after_slug_reorder["category_slug"],
        serde_json::Value::Null
    );
    assert_eq!(general_after_slug_reorder["position"], json!(0));

    let collapse_path = format!("/api/v1/guilds/{guild_slug}/categories/ops/collapse");
    let collapse_body = json!({ "collapsed": true }).to_string();
    let res = http_patch_with_bearer(&addr, &collapse_path, &collapse_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let collapsed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(collapsed["data"]["collapsed"], json!(true));

    let res = http_response_with_bearer(&addr, &categories_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let listed_categories: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(listed_categories["data"][0]["collapsed"], json!(true));

    let rename_path = format!("/api/v1/guilds/{guild_slug}/categories/ops");
    let rename_body = json!({ "name": "Operations" }).to_string();
    let res = http_patch_with_bearer(&addr, &rename_path, &rename_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let renamed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(renamed["data"]["slug"], json!("operations"));

    let delete_path = format!("/api/v1/guilds/{guild_slug}/categories/operations");
    let res = http_delete_with_bearer(&addr, &delete_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let deleted: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(deleted["data"]["deleted_slug"], json!("operations"));
    assert_eq!(deleted["data"]["reassigned_channel_count"], json!(1));

    let res = http_response_with_bearer(&addr, &list_channels_path, &token).await;
    assert_eq!(response_status(&res), 200);
    let listed_channels: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let incidents = listed_channels["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["slug"] == json!(incidents_slug))
        .unwrap();
    assert_eq!(incidents["category_slug"], serde_json::Value::Null);
}

#[tokio::test]
async fn categories_mutations_reject_non_owner() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner-categories", [54u8; 32]).await;
    let other_token = register_and_authenticate(&addr, "other-categories", [55u8; 32]).await;
    let guild_body = json!({ "name": "Category Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let categories_path = format!("/api/v1/guilds/{guild_slug}/categories");
    let create_category = json!({ "name": "Ops" }).to_string();
    let res = http_post_with_bearer(&addr, &categories_path, &create_category, &owner_token).await;
    assert_eq!(response_status(&res), 201);

    let create_other = json!({ "name": "Product" }).to_string();
    let res = http_post_with_bearer(&addr, &categories_path, &create_other, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let rename_path = format!("/api/v1/guilds/{guild_slug}/categories/ops");
    let rename_body = json!({ "name": "Operations" }).to_string();
    let res = http_patch_with_bearer(&addr, &rename_path, &rename_body, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let reorder_path = format!("/api/v1/guilds/{guild_slug}/categories/reorder");
    let reorder_body = json!({ "category_slugs": ["ops"] }).to_string();
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_body, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let collapse_path = format!("/api/v1/guilds/{guild_slug}/categories/ops/collapse");
    let collapse_body = json!({ "collapsed": true }).to_string();
    let res = http_patch_with_bearer(&addr, &collapse_path, &collapse_body, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let delete_path = format!("/api/v1/guilds/{guild_slug}/categories/ops");
    let res = http_delete_with_bearer(&addr, &delete_path, &other_token).await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn roles_mutations_require_authentication() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let create_body = json!({ "name": "Moderators", "color": "#3366ff" }).to_string();
    let res = http_post(&addr, "/api/v1/guilds/lobby/roles", &create_body).await;
    assert_eq!(response_status(&res), 401);

    let update_body = json!({ "name": "Moderation", "color": "#6633ff" }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        "/api/v1/guilds/lobby/roles/example-role",
        &update_body,
        "bad-token",
    )
    .await;
    assert_eq!(response_status(&res), 401);

    let res = http_delete_with_bearer(
        &addr,
        "/api/v1/guilds/lobby/roles/example-role",
        "bad-token",
    )
    .await;
    assert_eq!(response_status(&res), 401);

    let reorder_body = json!({ "role_ids": ["example-role"] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        "/api/v1/guilds/lobby/roles/reorder",
        &reorder_body,
        "bad-token",
    )
    .await;
    assert_eq!(response_status(&res), 401);

    let res = http_response(&addr, "/api/v1/guilds/lobby/members").await;
    assert_eq!(response_status(&res), 401);

    let update_member_roles_body = json!({ "role_ids": ["example-role"] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        "/api/v1/guilds/lobby/members/example-user/roles",
        &update_member_roles_body,
        "bad-token",
    )
    .await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn roles_mutations_reject_non_owner() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner-roles", [56u8; 32]).await;
    let other_token = register_and_authenticate(&addr, "other-roles", [57u8; 32]).await;
    let guild_body = json!({ "name": "Role Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let roles_path = format!("/api/v1/guilds/{guild_slug}/roles");
    let create_role = json!({ "name": "Moderators", "color": "#3366ff" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_role, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let created_role: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let role_id = created_role["data"]["id"].as_str().unwrap();

    let create_other = json!({ "name": "Ops", "color": "#112233" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_other, &other_token).await;
    assert_eq!(response_status(&res), 403);
    let forbidden: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(forbidden["error"]["code"], json!("FORBIDDEN"));

    let update_path = format!("/api/v1/guilds/{guild_slug}/roles/{role_id}");
    let update_role = json!({ "name": "Ops Team", "color": "#445566" }).to_string();
    let res = http_patch_with_bearer(&addr, &update_path, &update_role, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let delete_path = format!("/api/v1/guilds/{guild_slug}/roles/{role_id}");
    let res = http_delete_with_bearer(&addr, &delete_path, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let reorder_body = json!({ "role_ids": [role_id] }).to_string();
    let reorder_path = format!("/api/v1/guilds/{guild_slug}/roles/reorder");
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_body, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let members_path = format!("/api/v1/guilds/{guild_slug}/members");
    let res = http_response_with_bearer(&addr, &members_path, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let update_member_roles_path = format!("/api/v1/guilds/{guild_slug}/members/{role_id}/roles");
    let update_member_roles_body = json!({ "role_ids": [role_id] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_member_roles_path,
        &update_member_roles_body,
        &other_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn roles_owner_crud_hierarchy_and_delete_cleanup_work() {
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

    let owner_token = register_and_authenticate(&addr, "owner-role-crud", [58u8; 32]).await;
    let _member_token = register_and_authenticate(&addr, "member-role-crud", [59u8; 32]).await;
    let guild_body = json!({ "name": "Owner Role Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();
    let guild_id = guild["data"]["id"].as_str().unwrap();

    let roles_path = format!("/api/v1/guilds/{guild_slug}/roles");
    let res = http_response_with_bearer(&addr, &roles_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let listed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let listed_roles = listed["data"].as_array().unwrap();
    assert_eq!(listed_roles[0]["name"], json!("Owner"));
    assert_eq!(listed_roles[0]["is_system"], json!(true));
    assert_eq!(listed_roles[0]["permissions_bitflag"], json!(8191));
    let owner_role_id = listed_roles[0]["id"].as_str().unwrap().to_string();
    assert_eq!(listed_roles.last().unwrap()["name"], json!("@everyone"));
    assert_eq!(listed_roles.last().unwrap()["is_default"], json!(true));
    assert_eq!(
        listed_roles.last().unwrap()["permissions_bitflag"],
        json!(5633)
    );
    let everyone_role_id = listed_roles.last().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let create_body = json!({ "name": "Moderators", "color": "#3366ff" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let created: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let custom_role_id = created["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(created["data"]["name"], json!("Moderators"));

    let create_second_body = json!({ "name": "Helpers", "color": "#22aa88" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_second_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let created_second: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let second_custom_role_id = created_second["data"]["id"].as_str().unwrap().to_string();

    let reorder_path = format!("/api/v1/guilds/{guild_slug}/roles/reorder");
    let missing_role_body = json!({ "role_ids": [custom_role_id.clone()] }).to_string();
    let res = http_patch_with_bearer(&addr, &reorder_path, &missing_role_body, &owner_token).await;
    assert_eq!(response_status(&res), 422);

    let duplicate_roles_body =
        json!({ "role_ids": [custom_role_id.clone(), custom_role_id.clone()] }).to_string();
    let res =
        http_patch_with_bearer(&addr, &reorder_path, &duplicate_roles_body, &owner_token).await;
    assert_eq!(response_status(&res), 422);

    let unknown_role_body =
        json!({ "role_ids": [custom_role_id.clone(), "unknown-role-id"] }).to_string();
    let res = http_patch_with_bearer(&addr, &reorder_path, &unknown_role_body, &owner_token).await;
    assert_eq!(response_status(&res), 422);

    let reorder_body = json!({
        "role_ids": [second_custom_role_id.clone(), custom_role_id.clone()],
    })
    .to_string();
    let res = http_patch_with_bearer(&addr, &reorder_path, &reorder_body, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let reordered: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let reordered_roles = reordered["data"].as_array().unwrap();
    assert_eq!(reordered_roles[0]["name"], json!("Owner"));
    assert_eq!(reordered_roles[0]["position"], json!(-1));
    assert_eq!(reordered_roles[1]["id"], json!(second_custom_role_id));
    assert_eq!(reordered_roles[1]["position"], json!(0));
    assert_eq!(reordered_roles[2]["id"], json!(custom_role_id));
    assert_eq!(reordered_roles[2]["position"], json!(1));
    assert_eq!(reordered_roles.last().unwrap()["name"], json!("@everyone"));
    assert_eq!(
        reordered_roles.last().unwrap()["position"],
        json!(2147483647)
    );

    let update_owner_path = format!("/api/v1/guilds/{guild_slug}/roles/{owner_role_id}");
    let update_owner_body = json!({ "permissions_bitflag": 0 }).to_string();
    let res =
        http_patch_with_bearer(&addr, &update_owner_path, &update_owner_body, &owner_token).await;
    assert_eq!(response_status(&res), 422);

    let update_path = format!("/api/v1/guilds/{guild_slug}/roles/{custom_role_id}");
    let update_body =
        json!({ "name": "Moderation Team", "color": "#6633ff", "permissions_bitflag": 82 })
            .to_string();
    let res = http_patch_with_bearer(&addr, &update_path, &update_body, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let updated: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(updated["data"]["name"], json!("Moderation Team"));
    assert_eq!(updated["data"]["color"], json!("#6633ff"));
    assert_eq!(updated["data"]["permissions_bitflag"], json!(82));

    let url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let member_id = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?1")
        .bind("member-role-crud")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(guild_id)
    .bind(member_id)
    .bind(&custom_role_id)
    .bind("2026-02-28T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();
    let assigned_before =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM role_assignments WHERE role_id = ?1")
            .bind(&custom_role_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(assigned_before, 1);

    let delete_path = format!("/api/v1/guilds/{guild_slug}/roles/{custom_role_id}");
    let res = http_delete_with_bearer(&addr, &delete_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let deleted: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(deleted["data"]["deleted_id"], json!(custom_role_id));
    assert_eq!(deleted["data"]["removed_assignment_count"], json!(1));

    let assigned_after =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM role_assignments WHERE role_id = ?1")
            .bind(&custom_role_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(assigned_after, 0);
    drop(pool);

    let delete_everyone_path = format!("/api/v1/guilds/{guild_slug}/roles/{everyone_role_id}");
    let res = http_delete_with_bearer(&addr, &delete_everyone_path, &owner_token).await;
    assert_eq!(response_status(&res), 422);
}

#[tokio::test]
async fn member_role_assignment_enforces_hierarchy_and_invalidates_permission_cache() {
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

    let owner_token = register_and_authenticate(&addr, "owner-member-roles", [71u8; 32]).await;
    let manager_token = register_and_authenticate(&addr, "manager-member-roles", [72u8; 32]).await;
    let target_token = register_and_authenticate(&addr, "target-member-roles", [73u8; 32]).await;

    let guild_body = json!({ "name": "Member Roles Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap().to_string();
    let guild_id = guild["data"]["id"].as_str().unwrap().to_string();

    let roles_path = format!("/api/v1/guilds/{guild_slug}/roles");
    let create_high_body = json!({ "name": "High Guard", "color": "#a855f7" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_high_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let high_role: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let high_role_id = high_role["data"]["id"].as_str().unwrap().to_string();

    let create_manager_body = json!({ "name": "Role Manager", "color": "#3366ff" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_manager_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let manager_role: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let manager_role_id = manager_role["data"]["id"].as_str().unwrap().to_string();

    let create_helper_body = json!({ "name": "Invite Helper", "color": "#22aa88" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_helper_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let helper_role: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let helper_role_id = helper_role["data"]["id"].as_str().unwrap().to_string();

    let update_manager_path = format!("/api/v1/guilds/{guild_slug}/roles/{manager_role_id}");
    let update_manager_permissions = json!({ "permissions_bitflag": (16 + 64) }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_manager_path,
        &update_manager_permissions,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let update_helper_path = format!("/api/v1/guilds/{guild_slug}/roles/{helper_role_id}");
    let update_helper_permissions = json!({ "permissions_bitflag": 64 }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_helper_path,
        &update_helper_permissions,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let manager_id = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?1")
        .bind("manager-member-roles")
        .fetch_one(&pool)
        .await
        .unwrap();
    let target_id = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?1")
        .bind("target-member-roles")
        .fetch_one(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)",
    )
    .bind(&guild_id)
    .bind(&manager_id)
    .bind("2026-02-28T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)",
    )
    .bind(&guild_id)
    .bind(&target_id)
    .bind("2026-02-28T00:00:01Z")
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&guild_id)
    .bind(&manager_id)
    .bind(&manager_role_id)
    .bind("2026-02-28T00:00:02Z")
    .execute(&pool)
    .await
    .unwrap();
    drop(pool);

    let delegated_create_role =
        json!({ "name": "Delegated Create", "color": "#445566" }).to_string();
    let res =
        http_post_with_bearer(&addr, &roles_path, &delegated_create_role, &manager_token).await;
    assert_eq!(response_status(&res), 403);

    let delegated_update_role = json!({ "name": "Delegated Update" }).to_string();
    let delegated_update_path = format!("/api/v1/guilds/{guild_slug}/roles/{helper_role_id}");
    let res = http_patch_with_bearer(
        &addr,
        &delegated_update_path,
        &delegated_update_role,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);

    let res = http_delete_with_bearer(&addr, &delegated_update_path, &manager_token).await;
    assert_eq!(response_status(&res), 403);

    let delegated_reorder_roles = json!({
        "role_ids": [
            manager_role_id.clone(),
            helper_role_id.clone(),
            high_role_id.clone(),
        ],
    })
    .to_string();
    let delegated_reorder_path = format!("/api/v1/guilds/{guild_slug}/roles/reorder");
    let res = http_patch_with_bearer(
        &addr,
        &delegated_reorder_path,
        &delegated_reorder_roles,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);

    let members_path = format!("/api/v1/guilds/{guild_slug}/members");
    let res = http_response_with_bearer(&addr, &members_path, &manager_token).await;
    assert_eq!(response_status(&res), 200);
    let listed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let assignable_role_ids = listed["data"]["assignable_role_ids"].as_array().unwrap();
    assert!(assignable_role_ids.contains(&json!(helper_role_id)));
    assert!(!assignable_role_ids.contains(&json!(high_role_id)));
    assert!(!assignable_role_ids.contains(&json!(manager_role_id)));

    let update_target_path = format!("/api/v1/guilds/{guild_slug}/members/{target_id}/roles");
    let high_role_assignment = json!({ "role_ids": [high_role_id.clone()] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_target_path,
        &high_role_assignment,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);

    let manager_role_assignment = json!({ "role_ids": [manager_role_id.clone()] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_target_path,
        &manager_role_assignment,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);

    let unknown_role_assignment = json!({ "role_ids": ["unknown-role-id"] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_target_path,
        &unknown_role_assignment,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 422);

    let helper_assignment = json!({ "role_ids": [helper_role_id.clone()] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_target_path,
        &helper_assignment,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);
    let assigned: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        assigned["data"]["role_ids"],
        json!([helper_role_id.clone()])
    );

    let res = http_patch_with_bearer(
        &addr,
        &update_target_path,
        &helper_assignment,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let assigned_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM role_assignments WHERE guild_id = ?1 AND user_id = ?2 AND role_id = ?3",
    )
    .bind(&guild_id)
    .bind(&target_id)
    .bind(&helper_role_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(assigned_count, 1);
    drop(pool);

    let invite_create_body = json!({ "type": "single_use" }).to_string();
    let invites_path = format!("/api/v1/guilds/{guild_slug}/invites");
    let res = http_post_with_bearer(&addr, &invites_path, &invite_create_body, &target_token).await;
    assert_eq!(response_status(&res), 201);

    let remove_helper_assignment = json!({ "role_ids": [] }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &update_target_path,
        &remove_helper_assignment,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);
    let removed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(removed["data"]["role_ids"], json!([]));

    let res = http_post_with_bearer(&addr, &invites_path, &invite_create_body, &target_token).await;
    assert_eq!(response_status(&res), 403);

    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let assigned_after_removal = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM role_assignments WHERE guild_id = ?1 AND user_id = ?2 AND role_id = ?3",
    )
    .bind(&guild_id)
    .bind(&target_id)
    .bind(&helper_role_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(assigned_after_removal, 0);
}

#[tokio::test]
async fn guild_permission_bitflags_authorize_member_mutations_and_invalidate_cache() {
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

    let owner_token = register_and_authenticate(&addr, "owner-perms", [61u8; 32]).await;
    let manager_token = register_and_authenticate(&addr, "manager-perms", [62u8; 32]).await;

    let guild_body = json!({ "name": "Permissions Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap().to_string();
    let guild_id = guild["data"]["id"].as_str().unwrap().to_string();

    let roles_path = format!("/api/v1/guilds/{guild_slug}/roles");
    let create_role_body = json!({ "name": "Guild Manager", "color": "#3366ff" }).to_string();
    let res = http_post_with_bearer(&addr, &roles_path, &create_role_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let created_role: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let manager_role_id = created_role["data"]["id"].as_str().unwrap().to_string();

    let manager_permission_mask = 2 + 32 + 64;
    let role_update_path = format!("/api/v1/guilds/{guild_slug}/roles/{manager_role_id}");
    let role_update_body = json!({ "permissions_bitflag": manager_permission_mask }).to_string();
    let res =
        http_patch_with_bearer(&addr, &role_update_path, &role_update_body, &owner_token).await;
    assert_eq!(response_status(&res), 200);

    let url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let manager_id = sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE username = ?1")
        .bind("manager-perms")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)",
    )
    .bind(&guild_id)
    .bind(&manager_id)
    .bind("2026-02-28T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&guild_id)
    .bind(&manager_id)
    .bind(&manager_role_id)
    .bind("2026-02-28T00:00:01Z")
    .execute(&pool)
    .await
    .unwrap();
    drop(pool);

    let update_guild_body = json!({ "name": "Permissions Guild Updated" }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{guild_slug}"),
        &update_guild_body,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let create_channel_body = json!({ "name": "Operations", "channel_type": "text" }).to_string();
    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{guild_slug}/channels"),
        &create_channel_body,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 201);

    let create_category_body = json!({ "name": "Ops" }).to_string();
    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{guild_slug}/categories"),
        &create_category_body,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 201);

    let create_invite_body = json!({ "type": "reusable" }).to_string();
    let invites_path = format!("/api/v1/guilds/{guild_slug}/invites");
    let res =
        http_post_with_bearer(&addr, &invites_path, &create_invite_body, &manager_token).await;
    assert_eq!(response_status(&res), 201);

    let denied_role_update_body = json!({ "name": "Cannot Edit Roles" }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &role_update_path,
        &denied_role_update_body,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);

    let reduced_permission_mask = 32 + 64;
    let remove_manage_channels_body =
        json!({ "permissions_bitflag": reduced_permission_mask }).to_string();
    let res = http_patch_with_bearer(
        &addr,
        &role_update_path,
        &remove_manage_channels_body,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);

    let blocked_channel_body = json!({ "name": "Incidents", "channel_type": "text" }).to_string();
    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{guild_slug}/channels"),
        &blocked_channel_body,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);

    let another_invite_body = json!({ "type": "single_use" }).to_string();
    let res =
        http_post_with_bearer(&addr, &invites_path, &another_invite_body, &manager_token).await;
    assert_eq!(response_status(&res), 201);

    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("DELETE FROM guild_members WHERE guild_id = ?1 AND user_id = ?2")
        .bind(&guild_id)
        .bind(&manager_id)
        .execute(&pool)
        .await
        .unwrap();
    drop(pool);

    let post_removal_invite_body = json!({ "type": "single_use" }).to_string();
    let res = http_post_with_bearer(
        &addr,
        &invites_path,
        &post_removal_invite_body,
        &manager_token,
    )
    .await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn invites_mutations_require_authentication() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let create_body = json!({ "type": "reusable" }).to_string();
    let res = http_post(&addr, "/api/v1/guilds/lobby/invites", &create_body).await;
    assert_eq!(response_status(&res), 401);

    let res = http_response(&addr, "/api/v1/guilds/lobby/invites").await;
    assert_eq!(response_status(&res), 401);

    let res =
        http_delete_with_bearer(&addr, "/api/v1/guilds/lobby/invites/test", "bad-token").await;
    assert_eq!(response_status(&res), 401);
}

#[tokio::test]
async fn invites_owner_can_create_list_and_revoke_with_single_use_metadata() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner-invites", [56u8; 32]).await;
    let guild_body = json!({ "name": "Invite Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let invites_path = format!("/api/v1/guilds/{guild_slug}/invites");

    let create_reusable = json!({ "type": "reusable" }).to_string();
    let res = http_post_with_bearer(&addr, &invites_path, &create_reusable, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let reusable: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let reusable_code = reusable["data"]["code"].as_str().unwrap().to_string();
    assert_eq!(reusable["data"]["type"], json!("reusable"));
    assert_eq!(reusable["data"]["uses_remaining"], json!(0));
    assert_eq!(reusable["data"]["creator_username"], json!("owner-invites"));
    assert_eq!(
        reusable["data"]["invite_url"],
        json!(format!("/invite/{reusable_code}"))
    );

    let create_single_use = json!({ "type": "single_use" }).to_string();
    let res = http_post_with_bearer(&addr, &invites_path, &create_single_use, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let single_use: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let single_use_code = single_use["data"]["code"].as_str().unwrap().to_string();
    assert_eq!(single_use["data"]["type"], json!("single_use"));
    assert_eq!(single_use["data"]["uses_remaining"], json!(1));

    let res = http_response_with_bearer(&addr, &invites_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let listed: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let invites = listed["data"].as_array().unwrap();
    assert_eq!(invites.len(), 2);
    let listed_single_use = invites
        .iter()
        .find(|item| item["code"] == json!(single_use_code))
        .unwrap();
    assert_eq!(listed_single_use["type"], json!("single_use"));
    assert_eq!(listed_single_use["uses_remaining"], json!(1));
    assert_eq!(listed_single_use["revoked"], json!(false));

    let revoke_path = format!("{invites_path}/{single_use_code}");
    let res = http_delete_with_bearer(&addr, &revoke_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let revoked: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(revoked["data"]["code"], json!(single_use_code));
    assert_eq!(revoked["data"]["revoked"], json!(true));

    let res = http_response_with_bearer(&addr, &invites_path, &owner_token).await;
    assert_eq!(response_status(&res), 200);
    let listed_after_revoke: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let invites_after_revoke = listed_after_revoke["data"].as_array().unwrap();
    assert_eq!(invites_after_revoke.len(), 1);
    assert_eq!(invites_after_revoke[0]["code"], json!(reusable_code));
}

#[tokio::test]
async fn invites_mutations_reject_non_owner() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner-invites", [57u8; 32]).await;
    let other_token = register_and_authenticate(&addr, "other-invites", [58u8; 32]).await;
    let guild_body = json!({ "name": "Invite Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let invites_path = format!("/api/v1/guilds/{guild_slug}/invites");
    let create_reusable = json!({ "type": "reusable" }).to_string();
    let res = http_post_with_bearer(&addr, &invites_path, &create_reusable, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let created: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let invite_code = created["data"]["code"].as_str().unwrap();

    let create_other = json!({ "type": "single_use" }).to_string();
    let res = http_post_with_bearer(&addr, &invites_path, &create_other, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let res = http_response_with_bearer(&addr, &invites_path, &other_token).await;
    assert_eq!(response_status(&res), 403);

    let delete_path = format!("{invites_path}/{invite_code}");
    let res = http_delete_with_bearer(&addr, &delete_path, &other_token).await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn invite_resolution_and_join_flow_supports_membership_reads_and_single_use_semantics() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner-join", [59u8; 32]).await;
    let member_token = register_and_authenticate(&addr, "member-join", [60u8; 32]).await;
    let third_token = register_and_authenticate(&addr, "third-join", [61u8; 32]).await;

    let guild_body = json!({ "name": "Join Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap().to_string();

    let create_invite_body = json!({ "type": "single_use" }).to_string();
    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{guild_slug}/invites"),
        &create_invite_body,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 201);
    let invite: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let invite_code = invite["data"]["code"].as_str().unwrap().to_string();

    let res = http_response(&addr, &format!("/api/v1/invites/{invite_code}")).await;
    assert_eq!(response_status(&res), 200);
    let metadata: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(metadata["data"]["guild_slug"], json!(guild_slug));
    assert_eq!(metadata["data"]["guild_name"], json!("Join Guild"));
    assert_eq!(metadata["data"]["default_channel_slug"], json!("general"));
    assert_eq!(metadata["data"]["welcome_screen"]["enabled"], json!(false));

    let res = http_response_with_bearer(&addr, "/api/v1/guilds", &member_token).await;
    assert_eq!(response_status(&res), 200);
    let before_join: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(before_join["data"].as_array().unwrap().len(), 0);

    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/invites/{invite_code}/join"),
        "{}",
        &member_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);
    let joined: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(joined["data"]["guild_slug"], json!(guild_slug));
    assert_eq!(joined["data"]["default_channel_slug"], json!("general"));
    assert_eq!(joined["data"]["already_member"], json!(false));

    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/invites/{invite_code}/join"),
        "{}",
        &member_token,
    )
    .await;
    assert_eq!(response_status(&res), 200);
    let joined_again: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(joined_again["data"]["already_member"], json!(true));

    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/invites/{invite_code}/join"),
        "{}",
        &third_token,
    )
    .await;
    assert_eq!(response_status(&res), 422);
    let exhausted: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        exhausted["error"]["message"],
        json!("This invite link is invalid or has expired")
    );

    let res = http_response_with_bearer(&addr, "/api/v1/guilds", &member_token).await;
    assert_eq!(response_status(&res), 200);
    let after_join: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(after_join["data"].as_array().unwrap().len(), 1);
    assert_eq!(after_join["data"][0]["slug"], json!(guild_slug));
    assert_eq!(after_join["data"][0]["is_owner"], json!(false));

    let channels_path = format!("/api/v1/guilds/{guild_slug}/channels");
    let res = http_response_with_bearer(&addr, &channels_path, &member_token).await;
    assert_eq!(response_status(&res), 200);

    let categories_path = format!("/api/v1/guilds/{guild_slug}/categories");
    let res = http_response_with_bearer(&addr, &categories_path, &member_token).await;
    assert_eq!(response_status(&res), 200);

    let create_channel_body = json!({ "name": "ops", "channel_type": "text" }).to_string();
    let res =
        http_post_with_bearer(&addr, &channels_path, &create_channel_body, &member_token).await;
    assert_eq!(response_status(&res), 403);
}

#[tokio::test]
async fn invite_endpoints_return_exact_invalid_message_for_unknown_codes() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "invalid-invite", [62u8; 32]).await;
    let res = http_response(&addr, "/api/v1/invites/does-not-exist").await;
    assert_eq!(response_status(&res), 422);
    let invalid_resolve: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        invalid_resolve["error"]["message"],
        json!("This invite link is invalid or has expired")
    );

    let res =
        http_post_with_bearer(&addr, "/api/v1/invites/does-not-exist/join", "{}", &token).await;
    assert_eq!(response_status(&res), 422);
    let invalid_join: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        invalid_join["error"]["message"],
        json!("This invite link is invalid or has expired")
    );
}

#[tokio::test]
async fn invite_path_serves_open_graph_metadata_for_valid_invites() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let owner_token = register_and_authenticate(&addr, "owner-og", [63u8; 32]).await;
    let guild_body = json!({ "name": "OG Guild" }).to_string();
    let res = http_post_with_bearer(&addr, "/api/v1/guilds", &guild_body, &owner_token).await;
    assert_eq!(response_status(&res), 201);
    let guild: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let guild_slug = guild["data"]["slug"].as_str().unwrap();

    let create_invite_body = json!({ "type": "reusable" }).to_string();
    let res = http_post_with_bearer(
        &addr,
        &format!("/api/v1/guilds/{guild_slug}/invites"),
        &create_invite_body,
        &owner_token,
    )
    .await;
    assert_eq!(response_status(&res), 201);
    let invite: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    let invite_code = invite["data"]["code"].as_str().unwrap();

    let res = http_response(&addr, &format!("/invite/{invite_code}")).await;
    assert_eq!(response_status(&res), 200);
    let body = response_body(&res);
    assert!(body.contains("og:title"));
    assert!(body.contains("Join OG Guild on Discool"));
    assert!(body.contains(&format!("/?invite={invite_code}")));
}

#[tokio::test]
async fn users_profile_requires_authentication() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let res = http_response(&addr, "/api/v1/users/me/profile").await;
    assert_eq!(response_status(&res), 401);

    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value,
        json!({ "error": { "code": "UNAUTHORIZED", "message": "Missing Authorization header", "details": {} } })
    );
}

#[tokio::test]
async fn users_profile_patch_persists_display_name_and_avatar_color() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "liam", [1u8; 32]).await;
    let patch_body =
        json!({ "display_name": "Liam from Guild", "avatar_color": "#3B82F6" }).to_string();

    let res = http_patch_with_bearer(&addr, "/api/v1/users/me/profile", &patch_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"]["display_name"], json!("Liam from Guild"));
    assert_eq!(value["data"]["avatar_color"], json!("#3B82F6"));

    let res = http_response_with_bearer(&addr, "/api/v1/users/me/profile", &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"]["display_name"], json!("Liam from Guild"));
    assert_eq!(value["data"]["avatar_color"], json!("#3B82F6"));
}

#[tokio::test]
async fn users_profile_patch_rejects_invalid_display_name() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "liam", [1u8; 32]).await;
    let patch_body = json!({ "display_name": "   " }).to_string();

    let res = http_patch_with_bearer(&addr, "/api/v1/users/me/profile", &patch_body, &token).await;
    assert_eq!(response_status(&res), 422);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["error"]["code"], json!("VALIDATION_ERROR"));
}

#[tokio::test]
async fn users_avatar_upload_rejects_unsupported_type() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "liam", [1u8; 32]).await;
    let boundary = "----discool-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.gif\"\r\nContent-Type: image/gif\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"GIF89a");
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let res =
        http_post_multipart_with_bearer(&addr, "/api/v1/users/me/avatar", boundary, &body, &token)
            .await;
    assert_eq!(response_status(&res), 422);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["error"]["code"], json!("VALIDATION_ERROR"));
}

#[tokio::test]
async fn users_avatar_upload_rejects_oversized_file() {
    use serde_json::json;
    use std::io::Write;

    let port = pick_free_port();
    let dir = new_temp_dir();
    let cfg_path = dir.join("config.toml");
    write_server_config(&cfg_path, "127.0.0.1", port, None);
    let mut cfg = fs::OpenOptions::new().append(true).open(&cfg_path).unwrap();
    cfg.write_all(b"\n[avatar]\nmax_size_bytes = 10\n").unwrap();

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "liam", [1u8; 32]).await;
    let boundary = "----discool-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(&[
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4, 5, 6,
    ]);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let res =
        http_post_multipart_with_bearer(&addr, "/api/v1/users/me/avatar", boundary, &body, &token)
            .await;
    assert_eq!(response_status(&res), 422);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["error"]["code"], json!("VALIDATION_ERROR"));
}

#[tokio::test]
async fn users_avatar_upload_accepts_png_and_exposes_avatar_url() {
    use serde_json::json;

    let port = pick_free_port();
    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port, None);
    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_http_status(&mut server.child, &addr, "/readyz", 200).await;

    let token = register_and_authenticate(&addr, "liam", [1u8; 32]).await;
    let boundary = "----discool-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let res =
        http_post_multipart_with_bearer(&addr, "/api/v1/users/me/avatar", boundary, &body, &token)
            .await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value["data"]["avatar_url"],
        json!("/api/v1/users/me/avatar")
    );

    let res = http_response_with_bearer(&addr, "/api/v1/users/me/profile", &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(
        value["data"]["avatar_url"],
        json!("/api/v1/users/me/avatar")
    );

    let res = http_response_with_bearer(&addr, "/api/v1/users/me/avatar", &token).await;
    assert_eq!(response_status(&res), 200);
    let content_type = response_header(&res, "content-type").unwrap_or_default();
    assert!(content_type.starts_with("image/png"));

    let patch_body = json!({ "avatar_color": "#ef4444" }).to_string();
    let res = http_patch_with_bearer(&addr, "/api/v1/users/me/profile", &patch_body, &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert_eq!(value["data"]["avatar_color"], json!("#ef4444"));
    assert!(value["data"]["avatar_url"].is_null());

    let res = http_response_with_bearer(&addr, "/api/v1/users/me/profile", &token).await;
    assert_eq!(response_status(&res), 200);
    let value: serde_json::Value = serde_json::from_str(response_body(&res)).unwrap();
    assert!(value["data"]["avatar_url"].is_null());

    let res = http_response_with_bearer(&addr, "/api/v1/users/me/avatar", &token).await;
    assert_eq!(response_status(&res), 404);
}
