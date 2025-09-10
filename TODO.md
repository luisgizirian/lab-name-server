# Project TODOs and Status

This document tracks current tasks, completion status, and future improvements for Lab Name Server.

Last updated: 2025-09-10

## Completed

- Scaffold project files
  - Rust project initialized: `Cargo.toml`, `src/main.rs`
  - Docs and samples: `README.md`, `config.sample.yaml`, `.gitignore`
- Implement DNS server logic (Rust)
  - Authoritative answers for configured zones
  - Supported RR types: A, AAAA, CNAME, TXT, MX, NS, SOA (SOA via zone.soa)
  - Wildcard records support (e.g., `*.wild`)
  - CNAME: one-hop chase for A/AAAA
  - Forward to upstream resolvers (UDP) for non-local names
  - UDP server using `tokio` with per-request tasks
  - CLI flags: `--config`, `--host`, `--port`, `--log`
- Provide sample config and zones
  - Two example zones: `example.local.` and `lab.` showing most record types
- Usage docs and quick start
  - Build/run steps, `dig` examples, port 53 capability guidance, systemd unit sample
- Dev Container config
  - `.devcontainer/devcontainer.json` using Rust image with host network and bind caps
  - `post-create.sh` installs `dnsutils`, `gdb`, `libcap2-bin`, warms cargo cache, copies config
- VS Code recommendations
  - Rust Analyzer, TOML, crates, C/C++ tools
- Tasks for build/run
  - `cargo build`, `cargo build --release`, and a background run task
- Fix build against current dependencies
  - Replace deprecated `Record::with` with `Record::from_rdata`
  - Wrap NS/CNAME with `hickory_proto::rr::rdata::{NS,CNAME}`
  - Convert SOA fields to expected types (i32) where required
  - Use `Arc<UdpSocket>` instead of `UdpSocket::try_clone()`

## In Progress

- Add more unit tests for additional RRs
  - TXT, MX, NS variations; negative cases and multiple queries per message

## Backlog / Future Improvements

- Upstream TCP fallback when responses are truncated (TC=1) over UDP
- Proper NXDOMAIN vs NODATA distinction with negative caching (SOA in authority)
- Additional RR types: SRV, PTR, CAA, SPF (TXT), etc.
- Multiple queries per message (currently we answer first query only)
- EDNS(0) handling (ignore or minimally reflect)
- Config reload without restart (SIGHUP or inotify)
- Metrics/health endpoints (HTTP) for monitoring
- Structured logs to JSON, log sampling for high rate scenarios
- Rate limiting per client IP to avoid abuse
- Minimal recursion cache for forwarded answers (TTL-aware)
- TCP listener for incoming queries (RFC requires for large answers)
- Unit tests for zone loading, wildcard resolution, and CNAME chase
- Integration tests using `dig` in CI (GitHub Actions) inside container
- Container image (Dockerfile) for deployment
- Systemd hardening options (ProtectSystem, ProtectHome, PrivateDevices)

## Cleanup

- Removed unused code warnings:
  - Deleted `record_type_of` function
  - Removed `default_ttl` field from `ZoneStore`

## How to run

- Build and start:
  ```bash
  cargo build --release
  ./target/release/lab-name-server --config config.yaml --log INFO
  ```
- Query:
  ```bash
  dig @127.0.0.1 -p 5353 example.local. A +noedns +norecurse
  ```

## File map

- `src/main.rs` — DNS server implementation (authoritative + forwarder)
- `Cargo.toml` — Rust crate manifest
- `config.sample.yaml` — Example configuration with zones and records
- `README.md` — Setup and usage
- `.devcontainer/` — Dev Container config and provisioning
- `.vscode/` — Editor tasks and recommendations
- `TODO.md` — This task/status tracker
