use std::io::{Read, Write};
use std::net::TcpStream;
use std::fs::write;
use crate::utils::{decode_bencoded_value, extract_info_hash, extract_pieces_bytes, get_str, get_i64};
use crate::peer::{get_first_peer, generate_peer_id};
use sha1::{Sha1, Digest};

/// Used by: `./your_program download_piece -o file piece_index`
pub fn download_piece_cmd(
    output_path: &str,
    torrent_path: &str,
    piece_index: usize,
) {
    let torrent_bytes = std::fs::read(torrent_path).expect("Failed to read .torrent file");
    let decoded = decode_bencoded_value(&torrent_bytes);

    let announce = get_str(&decoded, "announce");
    let info = decoded.get("info").expect("Missing 'info' dict");

    let piece_length = get_i64(info, "piece length") as usize;
    let total_length = get_i64(info, "length") as usize;
    let num_pieces = (total_length + piece_length - 1) / piece_length;

    if piece_index >= num_pieces {
        panic!("Invalid piece index");
    }

    let piece_hashes = extract_pieces_bytes(&torrent_bytes);
    let piece_hash = &piece_hashes[piece_index * 20..(piece_index + 1) * 20];

    let info_hash_bytes = hex::decode(extract_info_hash(&torrent_bytes)).unwrap();
    let peer_id = generate_peer_id();
    let peer_addr = get_first_peer(announce, &info_hash_bytes, &peer_id, total_length);

    let piece_data = download_piece_to_buffer(
        &peer_addr,
        piece_index as u32,
        piece_length,
        total_length,
        &info_hash_bytes,
        &peer_id,
        piece_hash,
    );

    write(output_path, &piece_data).expect("Failed to write piece to disk");
    println!("Piece downloaded and saved to {}", output_path);
}

/// Used by: `./your_program download -o file`
pub fn download_cmd(
    output_path: &str,
    torrent_path: &str,
) {
    let torrent_bytes = std::fs::read(torrent_path).expect("Failed to read .torrent file");
    let decoded = decode_bencoded_value(&torrent_bytes);

    let announce = get_str(&decoded, "announce");
    let info = decoded.get("info").expect("Missing 'info' dict");

    let piece_length = get_i64(info, "piece length") as usize;
    let total_length = get_i64(info, "length") as usize;

    let info_hash_bytes = hex::decode(extract_info_hash(&torrent_bytes)).unwrap();
    let peer_id = generate_peer_id();
    let peer_addr = get_first_peer(announce, &info_hash_bytes, &peer_id, total_length);

    let piece_hashes = extract_pieces_bytes(&torrent_bytes);
    let num_pieces = (total_length + piece_length - 1) / piece_length;

    let mut final_data = vec![0u8; total_length];

    for i in 0..num_pieces {
        let piece_offset = i * piece_length;
        let piece_size = piece_length.min(total_length - piece_offset);
        let piece_hash = &piece_hashes[i * 20..(i + 1) * 20];

        println!("Downloading piece {}/{}", i, num_pieces);

        let piece_data = download_piece_to_buffer(
            &peer_addr,
            i as u32,
            piece_length,
            total_length,
            &info_hash_bytes,
            &peer_id,
            piece_hash,
        );

        final_data[piece_offset..piece_offset + piece_size].copy_from_slice(&piece_data);
    }

    write(output_path, final_data).expect("Failed to write full file");
    println!("Full file downloaded and saved to {}", output_path);
}

/// Internal logic used by both full and partial download
pub fn download_piece_to_buffer(
    peer_addr: &str,
    piece_index: u32,
    piece_length: usize,
    total_length: usize,
    info_hash: &[u8],
    peer_id: &[u8],
    expected_hash: &[u8],
) -> Vec<u8> {
    let mut stream = TcpStream::connect(peer_addr).expect("Failed to connect to peer");

    // we are doing handshake here (for more info refer handshake.rs) 
    let mut handshake = Vec::new();
    handshake.push(19);
    handshake.extend_from_slice(b"BitTorrent protocol");
    handshake.extend_from_slice(&[0u8; 8]);
    handshake.extend_from_slice(info_hash);
    handshake.extend_from_slice(peer_id);
    stream.write_all(&handshake).unwrap();

    let mut buf = [0u8; 68];
    stream.read_exact(&mut buf).unwrap();

    // this represents the bitfield
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).unwrap();
    let msg_len = u32::from_be_bytes(len_buf);
    let mut msg_id = [0u8; 1];
    stream.read_exact(&mut msg_id).unwrap();
    if msg_id[0] != 5 {
        panic!("Expected bitfield");
    }
    let mut bitfield = vec![0u8; (msg_len - 1) as usize];
    stream.read_exact(&mut bitfield).unwrap();
    
    stream.write_all(&[0, 0, 0, 1, 2]).unwrap();

    // wait for unchoke
    stream.read_exact(&mut len_buf).unwrap();
    stream.read_exact(&mut msg_id).unwrap();
    if msg_id[0] != 1 {
        panic!("Expected unchoke");
    }

    // donwloading piece here
    let block_size = 16 * 1024;
    let mut piece_data = vec![0u8; piece_length.min(total_length - (piece_index as usize * piece_length))];
    let mut offset = 0;

    while offset < piece_data.len() {
        let len = block_size.min(piece_data.len() - offset);

        // requesting a block from the peer 
        let mut req = Vec::new();
        req.extend_from_slice(&13u32.to_be_bytes());
        req.push(6);
        req.extend_from_slice(&piece_index.to_be_bytes());
        req.extend_from_slice(&(offset as u32).to_be_bytes());
        req.extend_from_slice(&(len as u32).to_be_bytes());
        stream.write_all(&req).unwrap();

        // recieve the requested block
        stream.read_exact(&mut len_buf).unwrap();
        stream.read_exact(&mut msg_id).unwrap();
        if msg_id[0] != 7 {
            panic!("Expected piece");
        }

        // header is the index + begin
        let mut header = [0u8; 8]; 
        stream.read_exact(&mut header).unwrap();
        stream.read_exact(&mut piece_data[offset..offset + len]).unwrap();
        offset += len;
    }

    // validate the hash if the piece hash is correctly recieved or not
    let mut hasher = Sha1::new();
    hasher.update(&piece_data);
    let hash = hasher.finalize();
    if hash[..] != expected_hash[..] {
        panic!("Piece hash mismatch!");
    }

    piece_data
}
