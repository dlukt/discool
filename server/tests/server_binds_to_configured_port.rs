use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{net::TcpStream, time::sleep};

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

fn write_server_config(path: &Path, host: &str, port: u16) {
    fs::write(
        path,
        format!(
            "[server]\nhost = \"{host}\"\nport = {port}\n\n[log]\nlevel = \"warn\"\nformat = \"json\"\n"
        ),
    )
    .unwrap();
}

#[tokio::test]
async fn server_binds_to_port_from_config_toml() {
    let port = pick_free_port();

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", port);

    let mut server = spawn_server(&dir, |_| {});

    let addr = format!("127.0.0.1:{port}");
    wait_for_bind(&mut server.child, &addr).await;
}

#[tokio::test]
async fn env_vars_override_config_toml() {
    let file_port = pick_free_port();
    let mut env_port = pick_free_port();
    while env_port == file_port {
        env_port = pick_free_port();
    }

    let dir = new_temp_dir();
    write_server_config(&dir.join("config.toml"), "127.0.0.1", file_port);

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
    write_server_config(&dir.join("config.toml"), "127.0.0.1", file_port);
    write_server_config(&custom_path, "127.0.0.1", custom_port);

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
    write_server_config(&dir.join("config.toml"), "127.0.0.1", file_port);
    write_server_config(&custom_path, "127.0.0.1", custom_port);

    let mut server = spawn_server(&dir, |cmd| {
        cmd.env("DISCOOL_CONFIG", custom_path.as_os_str());
        cmd.env("DISCOOL_SERVER__PORT", env_port.to_string());
    });

    let addr = format!("127.0.0.1:{env_port}");
    wait_for_bind(&mut server.child, &addr).await;
}
