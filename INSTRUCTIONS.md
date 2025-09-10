# Project Instructions

This document provides a concise, practical guide for day-to-day work on Lab Name Server: how to build, run, document, and troubleshoot. Keep this file up-to-date alongside other docs.

Last updated: 2025-09-10

## Documentation policy

Always keep these documents current:
- `README.md`: Quick start, configuration, usage, compatibility notes, and links.
- `TODO.md`: Task tracker with Completed/In Progress/Backlog and cleanup items.
- `CHANGELOG.md`: Notable changes per version/date (fixes, features, docs).
- `INSTRUCTIONS.md`: This workflow and troubleshooting guide.

When you make changes that affect behavior or setup:
- Update the appropriate section in `README.md`.
- Add/modify items in `TODO.md` (move between sections as status changes).
- Append an entry to `CHANGELOG.md` with date, type (Added/Changed/Fixed), and brief notes.

## Build and run

Using VS Code tasks (recommended):
- Build: task `cargo build` (Ctrl/Cmd+Shift+B)
- Run (debug): task `run server (debug)` which executes `cargo run -- --config config.yaml`

Using terminal:
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run with config
cargo run -- --config config.yaml --log INFO

# Example queries
Dig against localhost UDP 5353:
dig @127.0.0.1 -p 5353 example.local. SOA +norecurse +noedns
dig @127.0.0.1 -p 5353 www.example.local. A +norecurse +noedns
dig @127.0.0.1 -p 5353 foo.wild.example.local. A +norecurse +noedns
```

Listen on port 53 without root (Linux):
```bash
sudo setcap 'cap_net_bind_service=+ep' ./target/release/lab-name-server
./target/release/lab-name-server --config config.yaml --port 53
```

## Configuration primer

See `config.sample.yaml` for a full example. Key points:
- Use `@` for the zone origin; all names are normalized to absolute (with trailing dot).
- Supported RR types: A, AAAA, CNAME, TXT, MX, NS, SOA (SOA is per-zone via `soa`).
- Wildcards supported (e.g., `*.wild`).
- Unknown names within zones return NODATA (SOA in authority). Unknown zones are forwarded to `upstream`.

## Compatibility notes

- Hickory >= 0.25: use `Record::from_rdata`; NS/CNAME require `hickory_proto::rr::rdata::{NS,CNAME}` wrappers.
- SOA fields `refresh`/`retry`/`expire` are `i32` in Hickory; we clamp from `u32`.
- Tokio UDP socket is shared via `Arc<UdpSocket>`; there is no `UdpSocket::try_clone()`.

## Development workflow

- Validate quickly:
```bash
cargo check
cargo clippy --all-targets -- -D warnings   # optional if clippy installed
cargo test
```
- Run with verbose logs:
```bash
cargo run -- --config config.yaml --log DEBUG
```
- Make a change → update docs:
  - Behavior/API/config change → update `README.md` and `CHANGELOG.md`.
  - New tasks or decisions → update `TODO.md` (move items across sections).

## Testing (suggested)

Add tests for:
- Exact match vs wildcard resolution
- One-hop CNAME chase for A/AAAA
- NODATA responses with SOA in authority section
- Upstream forwarding fallback when no local answer

## Troubleshooting

- Build fails with Hickory type mismatches:
  - Use `Record::from_rdata` and wrap NS/CNAME as `rdata::{NS,CNAME}`.
  - Ensure SOA `refresh`/`retry`/`expire` are `i32` (clamp as needed).
- Server not receiving on port 53:
  - Use `setcap` as shown above or run as root. Verify the port with `ss -ulpn | grep 53`.
- Queries time out:
  - Check firewall rules, ensure host listens on the right interface/port in `config.yaml`.
  - Enable DEBUG logs and inspect output.
- Wildcard not matching:
  - Confirm the wildcard pattern is configured (`*.sub`) and the queried name is under the same zone.

## Release checklist

- [ ] Build succeeds (debug and release)
- [ ] Unit tests pass (`cargo test`)
- [ ] Smoke test queries pass
- [ ] `README.md`, `TODO.md`, `CHANGELOG.md`, `INSTRUCTIONS.md` updated
- [ ] Version/date updated in `CHANGELOG.md`
