
use crate::torrent::decode_bencoded_value_with_index;

/// Advance `index` past one complete bencoded value. Returns the slice that was skipped
pub fn skip_bencoded_value<'a>(bytes: &'a [u8], index: &mut usize) -> &'a [u8] {
    let start = *index;
    decode_bencoded_value_with_index(bytes, index);
    &bytes[start..*index]
}

pub fn find_dict_value_range(s: &[u8], target_key: &str) -> Option<(usize, usize)> {
    let mut index = 0;
    if *s.get(index)? != b'd' {
        return None;
    }

    index += 1;

    while index < s.len() && s[index] != b'e' {
        let key_start = index;

        while s[index] != b':' {
            index += 1;
        }

        let key_len = std::str::from_utf8(&s[key_start..index])
                        .ok()?
                        .parse::<usize>()
                        .ok()?;
        index += 1;
        let key = &s[index..index + key_len];
        index += key_len;

        if key == target_key.as_bytes() {
            let val_start = index;
            skip_bencoded_value(s, &mut index);

            return Some((val_start, index));
        } else {
            skip_bencoded_value(s, &mut index);
        }
    }

    None
}

/// Nested Lookup: e.g keys &["info", "pieces"]
pub fn find_nested_value_range(s: &[u8], keys: &[&str]) -> Option<(usize, usize)> {
    if keys.is_empty() {
        return None;
    }

    let (start, end) = find_dict_value_range(s, keys[0])?;

    if keys.len() == 1 {
        return Some((start, end));
    }

    find_nested_value_range(&s[start..end], &keys[1..])
        .map(|(inner_start, inner_end)| (start + inner_start, start + inner_end))
}

