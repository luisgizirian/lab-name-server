# Copilot Instructions for lab-name-server

This repo is a small authoritative + forwarding DNS server in Rust. The goal: answer for configured zones authoritatively; otherwise forward to upstreams. Keep answers accurate and wiring simple.

## Project architecture (big picture)
- Binary crate (`src/main.rs`) using:
  - `hickory-proto` for DNS message parsing/building.
  - `tokio` for UDP I/O and async tasks.
  - `serde_yaml` for config, `clap` for CLI, `tracing` for logs.
- Core structs:
  - `Config` → loaded from YAML; defines listen, upstreams, defaults, and `zones`.
  - `ZoneStore` → in-memory index of zones; holds `ZoneData` (origin, defaults, SOA, NS, exact and wildcard maps).
  - `ZoneData` → maps `Name` → `RecordType` → `Vec<(RData, ttl)>` for exact and wildcard suffixes.
- Flow:
  1) `main` loads config, builds `ZoneStore`, binds UDP socket (wrapped in `Arc<UdpSocket>`), spawns a task per packet.
  2) `handle_packet` parses request with `Message::from_vec`.
  3) `ZoneStore::answer` attempts authoritative answer (exact → wildcard), optional one-hop CNAME chase.
  4) If unanswered and `upstream` set, forward via `forward_udp` (UDP, 3s timeout), else reply SERVFAIL.

## Notable implementation patterns
- Hickory API usage (>=0.25):
  - Build records with `Record::from_rdata(name, ttl, rdata)` (not `Record::with`).
  - Wrap NS/CNAME targets with `hickory_proto::rr::rdata::{NS, CNAME}` when constructing `RData::{NS,CNAME}`.
  - SOA `refresh/retry/expire` expect `i32`; code clamps `u32` → `i32`.
- UDP socket is shared with `Arc<UdpSocket>`; no `try_clone()` available.
- Names are normalized to absolute FQDN with trailing dot; `@` refers to zone origin.
- Wildcards stored by suffix name (e.g., `*.wild.example.` stored under `wild.example.`) and matched by suffix scanning.
- CNAME chase: only one hop; if qtype is A/AAAA and CNAME exists, attempt to answer target, add NS/SOA authority for the (target) zone.

## Developer workflows
- Build:
  - VS Code task: `cargo build` (Ctrl/Cmd+Shift+B)
  - Terminal: `cargo build` or `cargo build --release`
- Run:
  - Task: `run server (debug)` → `cargo run -- --config config.yaml`
  - Terminal: `cargo run -- --config config.yaml --log DEBUG`
- Quick tests with dig:
  - `dig @127.0.0.1 -p 5353 example.local. SOA +norecurse +noedns`
  - `dig @127.0.0.1 -p 5353 www.example.local. A +norecurse +noedns`
  - `dig @127.0.0.1 -p 5353 foo.wild.example.local. A +norecurse +noedns`

## Conventions & gotchas
- Config parsing helpers:
  - `ensure_fqdn`, `parse_name`, `value_to_name` enforce absolute names and `@` handling.
- Data structures:
  - `exact: HashMap<Name, HashMap<RecordType, Vec<(RData, u32)>>>`
  - `wild:  HashMap<Name, HashMap<RecordType, Vec<(RData, u32)>>>` keyed by suffix.
- Authority section always includes SOA and NS for the matched zone on positive responses and on NODATA.
- Upstream forwarding is UDP-only with timeout; no TCP fallback yet.
- Logging controlled by `--log` (e.g., `DEBUG`); `tracing_subscriber` is set via env filter.

## External dependencies & versions
- `hickory-proto` (DNS types/messages)
- `tokio` (async runtime and UDP)
- `serde`, `serde_yaml` (config), `clap` (CLI), `tracing` (logs)

## Examples from the code
- Build NS record:
  ```rust
  let ns_rdata = RData::NS(hickory_proto::rr::rdata::NS(ns_name));
  let rec = Record::from_rdata(zone.origin.clone(), ttl, ns_rdata);
  ```
- Build CNAME record:
  ```rust
  let cname = value_to_name(target, &origin);
  let rd = RData::CNAME(hickory_proto::rr::rdata::CNAME(cname));
  let rec = Record::from_rdata(name.clone(), ttl, rd);
  ```
- SOA with i32 fields:
  ```rust
  let rd = RData::SOA(SOA::new(mname, rname, serial, refresh as i32, retry as i32, expire as i32, minimum));
  ```

## When adding features
- Follow FQDN normalization (`ensure_fqdn` / `value_to_name`) for new name inputs.
- Keep authority section behavior consistent (include NS + SOA for authoritative answers and NODATA).
- Update docs: `README.md`, `INSTRUCTIONS.md`, `TODO.md`, and append to `CHANGELOG.md`.
