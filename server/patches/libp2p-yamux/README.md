# Patched `libp2p-yamux`

This directory contains a patched copy of `libp2p-yamux` 0.47.0 that removes
support for the vulnerable `yamux` 0.12.x line.

## Why this patch exists

Dependabot flagged `yamux` < 0.13.10 as vulnerable to a remote panic
(CVE-2026-32314 / GHSA-vxx9-2994-q338). Upstream `libp2p-yamux` 0.47.0 still
depends on both `yamux` 0.12.1 and `yamux` 0.13.x for backwards compatibility.

Discool only uses `yamux::Config::default()`, which selects the yamux 0.13.x
path at runtime. The vulnerable 0.12.x code is therefore never executed, but it
is still compiled into the binary and reported by dependency scanners.

This patch drops the 0.12.x path entirely and uses only `yamux` 0.13.10+.

## What changed

- Removed the `yamux012` dependency alias.
- Removed deprecated `Config::client()`, `Config::server()`,
  `WindowUpdateMode`, and other yamux 0.12-only configuration APIs.
- Simplified `Muxer`, `Stream`, `Config`, and `Error` to use only yamux 0.13.x
  types.

## When to remove

This patch can be removed once `libp2p-yamux` publishes a version that no
longer depends on `yamux` 0.12.x. At that point:

1. Delete this directory.
2. Remove the `[patch.crates-io]` entry in `server/Cargo.toml`.
3. Run `cargo update -p libp2p-yamux` in `server/`.
