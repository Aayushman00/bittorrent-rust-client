use std::fs;

use anyhow::{anyhow, Result};
use reqwest;

use crate::bencode::find_dict_value_range;
use crate::peer::generate_peer_id;
use crate::torrent::{decode_bencoded_value, extract_info_hash, get_i64, get_str};

fn parse_bencode_byte_string(s: &[u8]) -> Result<Vec<u8>> {
    let colon = s
        .iter()
        .position(|&b| b == b':')
        .ok_or_else(|| anyhow!("Missing colon in byte string"))?;
    let len: usize = std::str::from_utf8(&s[..colon])?.parse()?;

    Ok(s[colon + 1..colon + 1 + len].to_vec())
}

pub fn extract_peers_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let (start, end) = find_dict_value_range(bytes, "peers")
        .ok_or_else(|| anyhow!("'peers' not found in tracker response"))?;

    parse_bencode_byte_string(&bytes[start..end])
}

pub fn url_encode_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("%{:02X}", b)).collect()
}

pub fn build_tracker_url(announce: &str, info_hash: &[u8], peer_id: &[u8], left: usize) -> String {
    format!(
        "{}?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left={}&compact=1",
        announce,
        url_encode_bytes(info_hash),
        url_encode_bytes(peer_id),
        left,
    )
}

pub fn get_first_peer(announce: &str, info_hash: &[u8], peer_id: &[u8], length: usize) -> String {
    let url = build_tracker_url(announce, info_hash, peer_id, length);

    let response = reqwest::blocking::get(&url).unwrap().bytes().unwrap();
    let peers = extract_peers_bytes(&response).expect("Failed to extract peers");

    if peers.len() < 6 {
        panic!("No peers found");
    }

    let chunk = &peers[0..6];
    let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
    format!("{}:{}", ip, port)
}

pub fn print_peers(torrent_path: &str) {
    let bytes = fs::read(torrent_path).expect("Failed to read file");

    let decoded = decode_bencoded_value(&bytes);
    let announce = get_str(&decoded, "announce");
    let length = decoded
        .get("info")
        .map(|info| get_i64(info, "length") as usize)
        .unwrap_or(0);

    let info_hash_hex = extract_info_hash(&bytes);
    let info_hash_bytes = hex::decode(info_hash_hex).expect("Invalid info hash");
    let peer_id = generate_peer_id();

    let tracker_url = build_tracker_url(announce, &info_hash_bytes, &peer_id, length);

    let response = reqwest::blocking::get(&tracker_url)
        .expect("Tracker request failed")
        .bytes()
        .expect("Failed to read response");

    let peer_bytes = extract_peers_bytes(&response).expect("Failed to extract peers");

    for chunk in peer_bytes.chunks(6) {
        if chunk.len() < 6 {
            continue;
        }

        let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        println!("{}:{}", ip, port);
    }
}
