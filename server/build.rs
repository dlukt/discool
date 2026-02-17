use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist_dir = manifest_dir.join("../client/dist");

    if let Err(err) = fs::create_dir_all(&dist_dir) {
        panic!(
            "failed to create client dist directory at {}: {err}",
            dist_dir.display()
        );
    }

    // Release builds must embed the real SPA output.
    if env::var("PROFILE").is_ok_and(|p| p == "release") {
        let index = dist_dir.join("index.html");
        if !index.exists() {
            panic!(
                "missing client build output at {}. Run `cd client && npm run build` before `cargo build --release`.",
                index.display()
            );
        }
    }

    println!("cargo:rerun-if-changed=../client/dist/index.html");
}
