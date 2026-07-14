use std::fs;

use anyhow::{anyhow, Result};
use serde_json;
use sha1::{Digest, Sha1};

use crate::bencode::{find_dict_value_range, find_nested_value_range};

pub fn get_str<'a>(map: &'a serde_json::Value, key: &str) -> &'a str {
    map.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>")
}

pub fn get_i64(map: &serde_json::Value, key: &str) -> i64 {
    map.get(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

pub fn decode_bencoded_value(encoded_value: &[u8]) -> serde_json::Value {
    let mut index = 0;
    decode_bencoded_value_with_index(encoded_value, &mut index)
}

pub fn decode_bencoded_value_with_index(s: &[u8], index: &mut usize) -> serde_json::Value {
    if *index >= s.len() {
        panic!("Unexpected end of input at index {}", *index);
    }

    match s[*index] as char {
        'i' => {
            *index += 1;
            let start = *index;
            while *index < s.len() && s[*index] as char != 'e' {
                *index += 1;
            }
            if *index >= s.len() {
                panic!("Unterminated integer at index {}", start);
            }
            let number = std::str::from_utf8(&s[start..*index])
                .unwrap()
                .parse::<i64>()
                .unwrap();
            *index += 1;
            serde_json::Value::Number(number.into())
        }

        'l' => {
            *index += 1;
            let mut list = Vec::new();
            while *index < s.len() && s[*index] as char != 'e' {
                list.push(decode_bencoded_value_with_index(s, index));
            }
            if *index >= s.len() {
                panic!("Unterminated list starting at index {}", *index);
            }
            *index += 1;
            serde_json::Value::Array(list)
        }

        'd' => {
            *index += 1;
            let mut map = serde_json::Map::new();
            while *index < s.len() && s[*index] as char != 'e' {
                let key = match decode_bencoded_value_with_index(s, index) {
                    serde_json::Value::String(k) => k,
                    _ => panic!("Dictionary key must be string at index {}", *index),
                };
                let value = decode_bencoded_value_with_index(s, index);
                map.insert(key, value);
            }
            if *index >= s.len() {
                panic!("Unterminated dictionary starting at index {}", *index);
            }
            *index += 1;
            serde_json::Value::Object(map)
        }

        c if c.is_ascii_digit() => {
            let start = *index;
            while *index < s.len() && s[*index] as char != ':' {
                *index += 1;
            }
            if *index >= s.len() {
                panic!("Invalid string length at index {}", start);
            }
            let len_str = std::str::from_utf8(&s[start..*index]).unwrap();
            let len = len_str.parse::<usize>().unwrap();
            *index += 1;

            if *index + len > s.len() {
                panic!(
                    "String length {} out of bounds at index {}, input length {}",
                    len, *index, s.len()
                );
            }

            let bytes = &s[*index..*index + len];
            *index += len;

            match std::str::from_utf8(bytes) {
                Ok(v) => serde_json::Value::String(v.to_string()),
                Err(_) => serde_json::Value::String(format!("<{} binary bytes>", len)),
            }
        }

        _ => panic!("Unknown token '{}' at index {}", s[*index] as char, *index),
    }
}

fn parse_bencode_byte_string(s: &[u8]) -> Result<Vec<u8>> {
    let colon = s
        .iter()
        .position(|&b| b == b':')
        .ok_or_else(|| anyhow!("Missing colon in byte string"))?;
    let len: usize = std::str::from_utf8(&s[..colon])?.parse()?;

    Ok(s[colon + 1..colon + 1 + len].to_vec())
}

pub fn extract_pieces_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let (start, end) = find_nested_value_range(bytes, &["info", "pieces"])
        .ok_or_else(|| anyhow!("'pieces' not found in info dict"))?;

    parse_bencode_byte_string(&bytes[start..end])
}

pub fn extract_info_hash(bytes: &[u8]) -> String {
    let (start, end) = find_dict_value_range(bytes, "info").expect("'info' dictionary not found");

    let mut hasher = Sha1::new();
    hasher.update(&bytes[start..end]);
    hex::encode(hasher.finalize())
}

pub fn print_info(torrent_file: &str) {
    let bytes = fs::read(torrent_file).expect("Failed to read file");

    let decoded = decode_bencoded_value(&bytes);

    let announce = get_str(&decoded, "announce");
    let length = decoded
        .get("info")
        .map(|info| get_i64(info, "length"))
        .unwrap_or(0);

    println!("Tracker URL: {}", announce);
    println!("Length: {}", length);

    let info_hash = extract_info_hash(&bytes);
    println!("Info Hash: {}", info_hash);

    let info = decoded.get("info").expect("missing info");

    let piece_length = get_i64(info, "piece length");
    println!("Piece Length: {}", piece_length);

    let piece_bytes = extract_pieces_bytes(&bytes).expect("Failed to extract pieces");

    println!("Piece Hashes:");
    for chunk in piece_bytes.chunks(20) {
        println!("{}", hex::encode(chunk));
    }
}
