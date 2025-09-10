# Lab Name Server (Rust)

A simple, lightweight DNS server for labs and local development. It serves authoritative answers for configured zones and can optionally forward other queries to upstream resolvers.

- Language: Rust (edition 2021)
- Crates: `hickory-proto`, `tokio`, `serde_yaml`, `clap`, `tracing`

> Status: Development research sample — not production-ready.

## Why this repo (GitHub Copilot angle)

This project showcases how quickly you can ship a practical tool with GitHub Copilot. The emphasis here is on clear patterns and tests rather than production-hardening. Goal: stand up a small DNS server to answer your lab domains locally and reduce third‑party, traceable round trips. Copilot helped scaffold code, tests, and docs; you can use it to iterate rapidly on features (e.g., more record types, TCP fallback, caching).

NOTE: keep in mind that GitHub Copilot is a tool that can ocassionally make mistakes. Always review before publishing.

## Non‑production disclaimer

This repository is a development research sample. It is not hardened for production and intentionally omits many operational safeguards. Use it only in controlled lab/dev environments. Known gaps include, but are not limited to:

- No TCP fallback to upstreams; UDP only
- No DNSSEC validation or signing
- Minimal CNAME chasing (one hop)
- No dynamic updates or admin endpoints
- Limited error handling, observability, and security hardening
- Config and defaults optimized for simplicity, not resilience

If you need a production DNS server, consider mature projects and services designed and audited for that purpose.

## Quick start

```bash
# 1) Build
cargo build --release

# 2) Copy config and edit
cp config.sample.yaml config.yaml
$EDITOR config.yaml

# 3) Run (non-root port 5353 by default in sample)
./target/release/lab-name-server --config config.yaml

# 4) Query it in another shell
dig @127.0.0.1 -p 5353 example.local. A +noedns +norecurse
```

## Publishing this repo (public)

To publish under your GitHub account (e.g., `luisgizirian`), you can push this folder to a new repository:

```bash
# Initialize (if not already a git repo)
git init
git add .
git commit -m "feat: initial public release"

# Set your GitHub repo URL and push main
git branch -M main
git remote add origin https://github.com/luisgizirian/lab-name-server.git
git push -u origin main
```

Alternatively, using GitHub CLI (`gh`):

```bash
gh repo create luisgizirian/lab-name-server --public --source . --remote origin --push
```

VS Code tasks:
- Build: use the task `cargo build` (Ctrl/Cmd+Shift+B)
- Run (debug): use the task `run server (debug)` which runs `cargo run -- --config config.yaml`

If you want to listen on port 53 on Linux without root, grant the binary the bind capability:

```bash
sudo setcap 'cap_net_bind_service=+ep' ./target/release/lab-name-server
./target/release/lab-name-server --config config.yaml --port 53
```

## Configuration

See `config.sample.yaml` for a full example. High-level structure:

```yaml
listen:
  host: 0.0.0.0
  port: 5353
upstream:
  - 1.1.1.1
  - 8.8.8.8
default_ttl: 300
zones:
  - origin: example.local.
    ttl: 300
    soa: { mname: ns1.example.local., rname: hostmaster.example.local., serial: 2025091001, refresh: 3600, retry: 900, expire: 1209600, minimum: 300 }
    ns: [ ns1.example.local. ]
    records:
      - { name: @, type: A, value: 10.10.10.10 }
      - { name: www, type: CNAME, value: @ }
```

Notes:
- Names are case-insensitive and normalized to absolute form (ending with a dot). Use `@` to refer to the zone origin.
- Supported RR types: A, AAAA, CNAME, TXT, MX, NS, SOA (SOA is configured at zone level).
- Wildcards (e.g., `*.wild`) are supported for A/AAAA/CNAME/TXT.
- For names outside configured zones, queries are forwarded to `upstream` (UDP). If all upstreams fail, SERVFAIL is returned.

Compatibility notes:
- Using `hickory-proto` >= 0.25: record construction uses `Record::from_rdata`, and NS/CNAME rdata require wrapping names with `hickory_proto::rr::rdata::{NS,CNAME}`.
- Tokio UDP sockets are shared via `Arc<UdpSocket>` (no `try_clone()` on `UdpSocket`).

## Logging

```bash
./target/release/lab-name-server --config config.yaml --log DEBUG
```

## Systemd unit (optional)

```
[Unit]
Description=Lab Name Server
After=network.target

[Service]
ExecStart=/usr/local/bin/lab-name-server --config /etc/lab-name-server/config.yaml --log INFO
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
NoNewPrivileges=true
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Limitations

- UDP only (no TCP fallback to upstream yet)
- No DNSSEC validation
- Minimal CNAME chasing (one hop)
- No dynamic updates; edit config and restart

## Development

Inside the dev container (recommended):

```bash
# Fast checks
cargo check

# Debug build & run
cargo build
cargo run -- --config config.yaml --log DEBUG

# Optional lints (if you have clippy installed)
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test
```

Useful queries while testing:

```bash
dig @127.0.0.1 -p 5353 example.local. SOA +norecurse +noedns
dig @127.0.0.1 -p 5353 www.example.local. A +norecurse +noedns
dig @127.0.0.1 -p 5353 foo.wild.example.local. A +norecurse +noedns
```

### Testing

Integration tests live under `tests/` and cover exact answers, wildcards, CNAME behavior, TXT/MX records, and UDP forwarding.

Run all tests with:

```bash
cargo test
```

## Changelog

See `CHANGELOG.md`.

## Project instructions

For day-to-day workflow, docs policy, and troubleshooting, see `INSTRUCTIONS.md`.
