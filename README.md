# Rust BitTorrent Client

A BitTorrent client built from scratch in Rust. Implements:

- `.torrent` parsing
- Tracker communication
- Peer handshake
- Piece download and verification

> Originally inspired by Codecrafters, now extended as a standalone project.

## Run

```bash
cargo run sample.torrent
```

```bash
cargo build --release
./target/release/bittorrent-rust-client path/to/file.torrent
```

## Project Structure:

- src/torrent.rs – Torrent file parsing and info hash computation

- src/tracker.rs – Tracker communication via HTTP GET requests

- src/peer.rs – Peer connection, handshake, and piece requests

- src/download.rs – Piece management and writing to disk

- src/utils.rs – Common helper functions

- src/main.rs – CLI entry point using clap

## Dependencies
### This project uses the following crates:

- serde, serde_bencode, serde_json, serde_urlencoded – for parsing and encoding data

- reqwest, tokio – for async HTTP tracker communication

- sha1, rand, bytes – for protocol implementation

- clap – for CLI interface

- anyhow, thiserror – for error handling

## Author
Aayushman – https://github.com/Aayushman00
