#!/bin/bash

# Build and run the BitTorrent client

set -e

# Build the project
cargo build --release

# Run the compiled binary with arguments
./target/release/bittorrent "$@"
