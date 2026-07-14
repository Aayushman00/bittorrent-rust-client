use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub fn generate_peer_id() -> [u8; 20] {
    let mut id = [0u8; 20];

    id[0..8].copy_from_slice(b"-AY0001-");

    let suffix: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    id[8..20].copy_from_slice(suffix.as_bytes());

    id
}
