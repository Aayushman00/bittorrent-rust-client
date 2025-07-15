use std::env;

mod utils;
use utils::print_info;

mod peer;
use peer::print_peers;

mod handshake;
use handshake::send_handshake;

mod download;
use download::{download_cmd, download_piece_cmd};


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: <command> [args]");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "decode" => {
            if args.len() < 3 {
                eprintln!("Usage: decode <bencoded string>");
                std::process::exit(1);
            }
            let encoded_value = args[2].as_bytes();
            let decoded = utils::decode_bencoded_value(encoded_value);
            println!("{}", decoded);
        }

        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: info <torrent file>");
                std::process::exit(1);
            }
            print_info(&args[2]);
        }

        "peers" => {
            if args.len() < 3 {
                eprintln!("Usage: peers <torrent file>");
                std::process::exit(1);
            }
            print_peers(&args[2]);
        }

        "handshake" => {
            if args.len() < 4 {
                eprintln!("Usage: handshake <torrent file> <peer ip:port>");
                std::process::exit(1);
            }
            send_handshake(&args[2], &args[3]);
        }

        "download_piece" => {
            if args.len() < 6 || args[2] != "-o" {
                eprintln!("Usage: download_piece -o <output file> <torrent file> <piece index>");
                std::process::exit(1);
            }

            let output_path = &args[3];
            let torrent_path = &args[4];
            let piece_index: usize = args[5].parse().expect("Invalid piece index");

            download_piece_cmd(output_path, torrent_path, piece_index);
        }

        "download" => {
            if args.len() < 5 || args[2] != "-o" {
                eprintln!("Usage: download -o <output file> <torrent file>");
                std::process::exit(1);
            }
            let output_path = &args[3];
            let torrent_path = &args[4];
            download_cmd(output_path, torrent_path);
        }


        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
