use std::io::{Read, Write};
use std::net::TcpStream;
use anyhow::{Context, Result};

// The handshake is a message consisting of the following parts as described in the peer protocol:
            // 1. length of the protocol string (BitTorrent protocol) which is 19 (1 byte)
            // 2. the string BitTorrent protocol (19 bytes)
            // 3. eight reserved bytes, which are all set to zero (8 bytes)
            // 4. sha1 infohash (20 bytes) (NOT the hexadecimal representation, which is 40 bytes long)
            // 5. peer id (20 bytes) (generate 20 random byte values)

use crate::torrent::extract_info_hash;
use rand::{thread_rng, Rng};
use std::fs;

pub fn build_handshake_message(info_hash: &[u8], peer_id: &[u8]) -> Vec<u8> {
    let mut handshake = vec![19];
    handshake.extend_from_slice(b"BitTorrent protocol");
    handshake.extend_from_slice(&[0u8; 8]);
    handshake.extend_from_slice(&info_hash);
    handshake.extend_from_slice(&peer_id);
    handshake
}

pub fn perform_handshake(
    stream: &mut TcpStream,
    info_hash: &[u8],
    peer_id: &[u8],
) -> Result<[u8; 20]> {
    stream.write_all(&build_handshake_message(info_hash, peer_id))
        .context("failed to send handshake")?;

    let mut response = [0u8; 68];
    stream.read_exact(&mut response).context("failed to read handshake")?;

    let mut remote_id = [0u8; 20];
    remote_id.copy_from_slice(&response[48..68]);
    Ok(remote_id)
}

pub fn send_handshake(torrent_file: &str, peer_addr: &str) {
    let torrent_bytes = fs::read(torrent_file).expect("Failed to read torrent");

    let info_hash_hex = extract_info_hash(&torrent_bytes);
    let info_hash = hex::decode(info_hash_hex).expect("Invalid info hash");

    let peer_id: Vec<u8> = thread_rng()
        .sample_iter(rand::distributions::Standard)
        .take(20)
        .collect();


    let mut stream = TcpStream::connect(peer_addr).expect("Failed to connect to peer");
    

    let peer_2_peer_id = perform_handshake(&mut stream, &info_hash, &peer_id).expect("Handshake failed");
    println!("Peer ID: {}", hex::encode(peer_2_peer_id));
}