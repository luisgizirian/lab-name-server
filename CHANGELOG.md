# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2025-09-10

### Tests
- Moved tests from `src/main.rs` into integration tests under `tests/` (`answers.rs`, `forwarding.rs`)

### Fixed
- Build failures with latest dependencies:
  - Replaced `Record::with` with `Record::from_rdata` (hickory-proto >= 0.25)
  - Wrapped NS and CNAME targets using `hickory_proto::rr::rdata::{NS,CNAME}`
  - Converted SOA `refresh`/`retry`/`expire` fields to `i32` as required by Hickory
  - Removed `UdpSocket::try_clone()` usage; share socket with `Arc<UdpSocket>`
  - Fixed wildcard lookup to avoid returning references to locals

### Docs
- Expanded README with VS Code tasks, development commands, and compatibility notes
- Updated TODO with testing plans and cleanup tasks
  
### Chore
- Removed unused `ZoneStore::default_ttl` field and `record_type_of` function to eliminate build warnings

### Tests
- Added initial unit tests covering exact, wildcard, CNAME one-hop, NODATA, and a `forward_udp` echo path

## [0.1.0] - 2025-09-01

### Added
- Initial release: authoritative DNS server with optional forwarding
- Zone config file with A, AAAA, CNAME, TXT, MX, NS, SOA
- Wildcard support and one-hop CNAME chase
