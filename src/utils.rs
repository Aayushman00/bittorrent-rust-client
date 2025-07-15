use serde_json;
use std::fs;
use sha1::{Sha1, Digest};

// Helper function to get a string form JSON object
// The lifetime `'a` ensures the returned &str lives as long as the input `map`
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

// decoder function
pub fn decode_bencoded_value(encoded_value: &[u8]) -> serde_json::Value {


    let mut index = 0;
    return decode_bencoded_value_with_index(encoded_value, &mut index)
}
// to extract raw binary bytes from the pieces field
pub fn extract_pieces_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut index = 0;

    assert_eq!(bytes[index] as char, 'd');
    index += 1;

    while bytes[index] as char != 'e' {
        // parse key
        let key_start = index;
        while bytes[index] as char != ':' {
            index += 1;
        }

        let key_len = std::str::from_utf8(&bytes[key_start..index])
                        .unwrap()
                        .parse::<usize>().unwrap();

        index += 1;
        let key = &bytes[index..index + key_len];
        index += key_len;

        if key == b"info" {
            
            assert_eq!(bytes[index] as char, 'd');
            index += 1;

            while bytes[index] as char != 'e' {
                let kstart = index;
                while bytes[index] as char != ':' {
                    index += 1;
                }

                let klen = std::str::from_utf8(&bytes[kstart..index])
                                .unwrap()
                                .parse::<usize>().unwrap();

                index += 1;
                let k = &bytes[index..index + klen];
                index += klen;

                if k == b"pieces" {
                    // Now we are at the value: <len>:<binary>
                    let len_start = index;
                    while bytes[index] as char != ':' {
                        index += 1;
                    }
                    let len_str = std::str::from_utf8(&bytes[len_start..index]).unwrap();

                    let len = len_str.parse::<usize>()
                                .unwrap();

                    index += 1;
                    let data = &bytes[index..index + len];
                    return data.to_vec();

                } else {
                    decode_bencoded_value_with_index(bytes, &mut index);
                }
            }

            panic!("No 'pieces' key in info dict");
        } else {
            decode_bencoded_value_with_index(bytes, &mut index);
        }
    }

    panic!("No 'info' dictionary");
}


// computing exact sha-1 hash for the info dictionary, this is called "info hash"
// this will uniquely identify the torrent
// it's used in bittorrent handshake when connecting to peers

pub fn extract_info_hash(bytes: &[u8]) -> String {
    let mut index = 0;

    assert_eq!(bytes[index] as char, 'd');
    index += 1;

    while bytes[index] as char != 'e' {
        // getting key
        let key_start = index;
        while bytes[index] as char != ':' {
            index += 1;
        }

        let key_len = std::str::from_utf8(&bytes[key_start..index])
                        .unwrap()
                        .parse::<usize>().unwrap();

        index += 1;

        let key = &bytes[index..index + key_len];

        index += key_len;
        if key == b"info" {
            let info_start = index;
            decode_bencoded_value_with_index(bytes, &mut index);
            let info_end = index;
            let info_slice = &bytes[info_start..info_end];

            let mut hasher = Sha1::new();

            hasher.update(info_slice);


            return hex::encode(hasher.finalize());

        } else {
            decode_bencoded_value_with_index(bytes, &mut index); // skip
        }
    }

    panic!("'info' dictionary not found");
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

    let piece_bytes = extract_pieces_bytes(&bytes);

    println!("Piece Hashes:");
    for chunk in piece_bytes.chunks(20) {
        println!("{}", hex::encode(chunk));
    }
}


// recursive parser
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
            *index += 1; // Skip 'e'
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
            *index += 1; // Skip 'e'
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
            *index += 1; // Skip 'e'
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
