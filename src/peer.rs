use reqwest;
use crate::utils::{decode_bencoded_value_with_index};
use crate::utils::{decode_bencoded_value, extract_info_hash, get_str, get_i64};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::fs;

pub fn print_peers(torrent_path: &str) {
    let bytes = fs::read(torrent_path).expect("Failed to read file");

    let decoded = decode_bencoded_value(&bytes);
    let announce = get_str(&decoded, "announce");
    let length = decoded
        .get("info")
        .map(|info| get_i64(info, "length"))
        .unwrap_or(0);

    let info_hash_hex = extract_info_hash(&bytes);
    let info_hash_bytes = hex::decode(info_hash_hex).expect("Invalid info hash");

    let encoded_info_hash = url_encode_bytes(&info_hash_bytes);

    let rand_str: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    let peer_id = format!("-AY0001-{}", rand_str);

    let tracker_url = format!(
        "{}?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left={}&compact=1",
        announce,
        encoded_info_hash,
        peer_id,
        length
    );

    let response = reqwest::blocking::get(&tracker_url)
        .expect("Tracker request failed")
        .bytes()
        .expect("Failed to read response");

    let peer_bytes = extract_peers_bytes(&response);

    for chunk in peer_bytes.chunks(6) {
        if chunk.len() < 6 {
            continue;
        }

        let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        println!("{}:{}", ip, port);
    }
}

pub fn url_encode_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("%{:02X}", b)).collect()
}


// extract the raw peer list (in compact format) from the bencoded tracker response
pub fn extract_peers_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut index = 0;
    assert_eq!(bytes[index] as char, 'd');
    index += 1;

    while bytes[index] as char != 'e' {
        // Parse key
        let key_start = index;
        while bytes[index] as char != ':' {
            index += 1;
        }

        let key_len = std::str::from_utf8(&bytes[key_start..index])
            .unwrap()
            .parse::<usize>()
            .unwrap();
        index += 1;

        let key = &bytes[index..index + key_len];
        index += key_len;

        if key == b"peers" {
            let len_start = index;
            while bytes[index] as char != ':' {
                index += 1;
            }

            let len_str = std::str::from_utf8(&bytes[len_start..index]).unwrap();
            let len = len_str.parse::<usize>().unwrap();
            index += 1;

            return bytes[index..index + len].to_vec();
        } else {
            decode_bencoded_value_with_index(bytes, &mut index); 
        }
    }

    panic!("'peers' not found in tracker response");
}


pub fn get_first_peer(
    announce: &str, 
    info_hash: &[u8], 
    peer_id: &[u8], 
    length: usize
) -> String {
    let encoded_info_hash = info_hash.iter().map(|b| format!("%{:02X}", b)).collect::<String>();
    let encoded_peer_id = peer_id.iter().map(|b| format!("%{:02X}", b)).collect::<String>();

    let url = format!(
        "{}?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left={}&compact=1",
        announce, encoded_info_hash, encoded_peer_id, length
    );

    let response = reqwest::blocking::get(&url).unwrap().bytes().unwrap();
    let peers = extract_peers_bytes(&response);

    if peers.len() < 6 {
        panic!("No peers found");
    }

    let chunk = &peers[0..6];
    let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
    format!("{}:{}", ip, port)
}
/// Generates a peer_id and its encoded form.
pub fn generate_peer_id() -> Vec<u8> {
    thread_rng()
        .sample_iter(rand::distributions::Standard)
        .take(20)
        .collect()
}

