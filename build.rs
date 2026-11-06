//! The git hash of the logs' header and --version (`git rev-parse
//! --short HEAD`, not `git describe`, which gives the tag once there is one;
//! "n/a" outside a git checkout).
//! Nothing is linked: crest and the library are all Rust.
use std::process::Command;

fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "n/a".into());
    println!("cargo:rustc-env=CREST_GITHASH={hash}");
}
