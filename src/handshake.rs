use std::io::{Read, Write};
use std::net::TcpStream;


// The handshake is a message consisting of the following parts as described in the peer protocol:
            // 1. length of the protocol string (BitTorrent protocol) which is 19 (1 byte)
            // 2. the string BitTorrent protocol (19 bytes)
            // 3. eight reserved bytes, which are all set to zero (8 bytes)
            // 4. sha1 infohash (20 bytes) (NOT the hexadecimal representation, which is 40 bytes long)
            // 5. peer id (20 bytes) (generate 20 random byte values)

use crate::utils::extract_info_hash;
use rand::{thread_rng, Rng};
use std::fs;

pub fn send_handshake(torrent_file: &str, peer_addr: &str) {
    let torrent_bytes = fs::read(torrent_file).expect("Failed to read torrent");

    let info_hash_hex = extract_info_hash(&torrent_bytes);
    let info_hash = hex::decode(info_hash_hex).expect("Invalid info hash");

    let peer_id: Vec<u8> = thread_rng()
        .sample_iter(rand::distributions::Standard)
        .take(20)
        .collect();

    let mut handshake = Vec::new();
    handshake.push(19);
    handshake.extend_from_slice(b"BitTorrent protocol");
    handshake.extend_from_slice(&[0u8; 8]);
    handshake.extend_from_slice(&info_hash);
    handshake.extend_from_slice(&peer_id);

    let mut stream = TcpStream::connect(peer_addr).expect("Failed to connect to peer");
    stream.write_all(&handshake).expect("Failed to send handshake");

    let mut response = [0u8; 68];
    stream.read_exact(&mut response).expect("Failed to read handshake");

    let peer_2_peer_id = &response[48..68];
    println!("Peer ID: {}", hex::encode(peer_2_peer_id));
}