// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ExportAsUrlDialog) and from the
// LZ-String algorithm by Pieroxy (MIT licence), `compressToEncodedURIComponent`.
//! Share links: the circuit text compressed with LZ-String and carried in the `ctz` parameter,
//! so links made by the original open here and the other way round.

use std::collections::HashMap;

const ALPHABET: &[u8; 65] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-$";

/// Address of the original online simulator, the base of exported links.
pub const BASE_URL: &str = "https://www.falstad.com/circuit/circuitjs.html";

struct BitWriter {
    out: String,
    val: u32,
    pos: u32,
}

impl BitWriter {
    fn bit(&mut self, b: u32) {
        self.val = (self.val << 1) | b;
        if self.pos == 5 {
            self.pos = 0;
            self.out.push(char::from(ALPHABET[self.val as usize]));
            self.val = 0;
        } else {
            self.pos += 1;
        }
    }

    fn bits(&mut self, mut value: u32, count: u32) {
        for _ in 0..count {
            self.bit(value & 1);
            value >>= 1;
        }
    }

    fn finish(mut self) -> String {
        loop {
            self.val <<= 1;
            if self.pos == 5 {
                self.out.push(char::from(ALPHABET[self.val as usize]));
                return self.out;
            }
            self.pos += 1;
        }
    }
}

/// Compresses `text` the way `LZString.compressToEncodedURIComponent` does.
pub fn compress(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut w = BitWriter { out: String::new(), val: 0, pos: 0 };
    if units.is_empty() {
        return String::new();
    }
    let mut dict: HashMap<Vec<u16>, u32> = HashMap::new();
    let mut to_create: HashMap<Vec<u16>, ()> = HashMap::new();
    let (mut dict_size, mut enlarge_in, mut num_bits) = (3u32, 2u32, 2u32);
    let mut cur: Vec<u16> = Vec::new();

    let emit = |w: &mut BitWriter,
                cur: &Vec<u16>,
                dict: &HashMap<Vec<u16>, u32>,
                to_create: &mut HashMap<Vec<u16>, ()>,
                enlarge_in: &mut u32,
                num_bits: &mut u32| {
        if to_create.remove(cur).is_some() {
            let first = u32::from(cur[0]);
            if first < 256 {
                w.bits(0, *num_bits);
                w.bits(first, 8);
            } else {
                w.bits(1, 1);
                w.bits(0, *num_bits - 1);
                w.bits(first, 16);
            }
            *enlarge_in -= 1;
            if *enlarge_in == 0 {
                *enlarge_in = 1 << *num_bits;
                *num_bits += 1;
            }
        } else {
            w.bits(dict[cur], *num_bits);
        }
        *enlarge_in -= 1;
        if *enlarge_in == 0 {
            *enlarge_in = 1 << *num_bits;
            *num_bits += 1;
        }
    };

    for &u in &units {
        let c = vec![u];
        if !dict.contains_key(&c) {
            dict.insert(c.clone(), dict_size);
            dict_size += 1;
            to_create.insert(c.clone(), ());
        }
        let mut wc = cur.clone();
        wc.push(u);
        if dict.contains_key(&wc) {
            cur = wc;
        } else {
            emit(&mut w, &cur, &dict, &mut to_create, &mut enlarge_in, &mut num_bits);
            dict.insert(wc, dict_size);
            dict_size += 1;
            cur = c;
        }
    }
    if !cur.is_empty() {
        emit(&mut w, &cur, &dict, &mut to_create, &mut enlarge_in, &mut num_bits);
    }
    w.bits(2, num_bits);
    w.finish()
}

struct BitReader<'a> {
    data: &'a [u8],
    index: usize,
    val: u32,
    pos: u32,
}

impl BitReader<'_> {
    fn value_at(&self, i: usize) -> u32 {
        self.data.get(i).and_then(|c| ALPHABET.iter().position(|a| a == c)).map_or(0, |p| p as u32)
    }

    fn bits(&mut self, count: u32) -> u32 {
        let mut out = 0;
        for power in 0..count {
            let resb = self.val & self.pos;
            self.pos >>= 1;
            if self.pos == 0 {
                self.pos = 32;
                self.val = self.value_at(self.index);
                self.index += 1;
            }
            if resb > 0 {
                out |= 1 << power;
            }
        }
        out
    }
}

/// Decompresses text made by `LZString.compressToEncodedURIComponent`. Returns `None` for
/// damaged input.
pub fn decompress(input: &str) -> Option<String> {
    let cleaned: Vec<u8> = input.bytes().map(|b| if b == b' ' { b'+' } else { b }).collect();
    if cleaned.is_empty() {
        return Some(String::new());
    }
    let mut r = BitReader { data: &cleaned, index: 1, val: 0, pos: 32 };
    r.val = r.value_at(0);
    let mut dict: Vec<Vec<u16>> = vec![vec![0], vec![1], vec![2]];
    let (mut enlarge_in, mut num_bits) = (4u32, 3u32);
    let first = match r.bits(2) {
        0 => r.bits(8),
        1 => r.bits(16),
        _ => return Some(String::new()),
    };
    let mut w: Vec<u16> = vec![u16::try_from(first).ok()?];
    dict.push(w.clone());
    let mut result: Vec<u16> = w.clone();
    loop {
        if r.index > cleaned.len() {
            return None;
        }
        let mut c = r.bits(num_bits) as usize;
        match c {
            0 | 1 => {
                let v = if c == 0 { r.bits(8) } else { r.bits(16) };
                dict.push(vec![u16::try_from(v).ok()?]);
                c = dict.len() - 1;
                enlarge_in -= 1;
            }
            2 => return String::from_utf16(&result).ok(),
            _ => {}
        }
        if enlarge_in == 0 {
            enlarge_in = 1 << num_bits;
            num_bits += 1;
        }
        let entry = if let Some(e) = dict.get(c) {
            e.clone()
        } else if c == dict.len() {
            let mut e = w.clone();
            e.push(*w.first()?);
            e
        } else {
            return None;
        };
        result.extend_from_slice(&entry);
        let mut added = w.clone();
        added.push(entry[0]);
        dict.push(added);
        enlarge_in -= 1;
        w = entry;
        if enlarge_in == 0 {
            enlarge_in = 1 << num_bits;
            num_bits += 1;
        }
    }
}

/// The link that opens `circuit_text` in a simulator.
pub fn make_link(circuit_text: &str) -> String {
    format!("{BASE_URL}?ctz={}", compress(circuit_text))
}

/// The circuit text carried by a link (or a bare `ctz` value), if it has one.
pub fn read_link(link: &str) -> Option<String> {
    let value = match link.split_once("ctz=") {
        Some((_, rest)) => rest.split(['&', '#']).next().unwrap_or(rest),
        None => link.trim(),
    };
    decompress(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // vectors produced by lz-string.min.js (the copy bundled with the original)
    const VECTORS: &[(&str, &str)] = &[
        ("", ""),
        ("a", "IZA"),
        ("Hello, World!", "BIUwNmD2A0AEDqkBOYAmBCIA"),
        (
            "$ 1 0.000005 10.2 50 5 43\nr 176 80 384 80 0 10\n",
            "CQAgjCAMB0l3BWcMBMIGXSALAZgFABO4A7AGwgAcmul2VmmYk+QA",
        ),
        ("Ωμ résistor ☃ 😀", "pXAh7gAEBOCXDOCWMAuB7CZDAZGQvBuAA9oA"),
    ];

    #[test]
    fn matches_the_reference_implementation() {
        for (text, packed) in VECTORS {
            assert_eq!(&compress(text), packed, "{text:?}");
            assert_eq!(decompress(packed).as_deref(), Some(*text), "{packed}");
        }
    }

    #[test]
    fn round_trips_ascii_unicode_and_long_text() {
        let long: String =
            (0..4000).map(|i| format!("r {} {} 64 0 0 1000\n", i % 97, i % 31)).collect();
        for t in ["", "a", "abababababab", "Ωμ résistor ☃ 😀", long.as_str()] {
            assert_eq!(decompress(&compress(t)).as_deref(), Some(t));
        }
    }

    #[test]
    fn links_round_trip_and_garbage_is_rejected() {
        let link = make_link("$ 1 5e-6 10 50 5 50\nr 0 0 64 0 0 1000\n");
        assert!(link.starts_with(BASE_URL));
        assert_eq!(read_link(&link).as_deref(), Some("$ 1 5e-6 10 50 5 50\nr 0 0 64 0 0 1000\n"));
        // damaged input must not panic
        let _ = decompress("!!!!");
        let _ = decompress("ZZZZZZZZZZ");
    }
}
