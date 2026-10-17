// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (DataInputElm, AudioInputElm).
//! Sample files loaded for the data input and the audio input.
//!
//! The circuit file does not hold the samples, only the number of the file; the files loaded in
//! this session are kept in a table shared by all threads, so an element rebuilt from the
//! circuit text finds its samples again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// Samples of one loaded file.
#[derive(Clone, Debug)]
pub struct SampleFile {
    /// Name shown on the element (file name without folders or extension).
    pub name: String,
    /// The samples.
    pub data: Arc<Vec<f64>>,
    /// Samples per second (audio files; 0 for plain data files).
    pub rate: u32,
}

struct Table {
    files: HashMap<i32, SampleFile>,
    next: i32,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);

fn table() -> MutexGuard<'static, Option<Table>> {
    TABLE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Keeps a loaded file and returns its number (for `number` 0) or stores it under `number`.
pub fn store(number: i32, file: SampleFile) -> i32 {
    let mut guard = table();
    let t = guard.get_or_insert_with(|| Table { files: HashMap::new(), next: 1 });
    let number = if number == 0 {
        t.next += 1;
        t.next - 1
    } else {
        number
    };
    t.files.insert(number, file);
    number
}

/// The file stored under `number`.
pub fn lookup(number: i32) -> Option<SampleFile> {
    table().as_ref().and_then(|t| t.files.get(&number).cloned())
}

/// Forgets every loaded file.
pub fn clear_cache() {
    *table() = None;
}

/// Name of a file as the element shows it: no folders, no extension.
pub fn display_name(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match file.rfind('.') {
        Some(i) if i > 0 => file[..i].to_string(),
        _ => file.to_string(),
    }
}

/// Reads a data file: one number per line, `#` starts a comment line, blank lines are ignored.
pub fn parse_data_text(text: &str) -> Result<Vec<f64>, String> {
    const HELP: &str = "Expected format:\n- One numeric voltage value per line\n- Lines starting with # are treated as comments\n- Blank lines are ignored\n\nExample:\n# my data\n1.5\n2.3\n-0.5";
    let mut data = Vec::new();
    let mut error = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.trim().parse::<f64>() {
            Ok(v) => data.push(v),
            Err(_) => error = true,
        }
    }
    if error {
        return Err(format!("Error parsing data file.\n\n{HELP}"));
    }
    if data.is_empty() {
        return Err(format!("No data found in file.\n\n{HELP}"));
    }
    Ok(data)
}

/// Decodes the first channel of a PCM or float WAV file into samples between -1 and 1.
pub fn decode_wav(bytes: &[u8]) -> Result<(Vec<f64>, u32), String> {
    let bad = || "not a WAV file this program can read (only PCM and float WAV files)".to_string();
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(bad());
    }
    let le16 = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let le32 = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let mut pos = 12;
    let mut format = None;
    let mut data = None;
    while pos + 8 <= bytes.len() {
        let size = le32(pos + 4) as usize;
        let body = pos + 8;
        let end = (body + size).min(bytes.len());
        match &bytes[pos..pos + 4] {
            b"fmt " if end - body >= 16 => {
                format = Some((le16(body), le16(body + 2), le32(body + 4), le16(body + 14)));
            }
            b"data" => data = Some(&bytes[body..end]),
            _ => {}
        }
        pos = body + size + (size & 1);
    }
    let (tag, channels, rate, bits) = format.ok_or_else(bad)?;
    let data = data.ok_or_else(bad)?;
    let channels = usize::from(channels.max(1));
    let width = usize::from(bits / 8);
    if width == 0 {
        return Err(bad());
    }
    let frame = width * channels;
    let mut out = Vec::with_capacity(data.len() / frame);
    for f in data.chunks_exact(frame) {
        let s = &f[..width];
        let v = match (tag, bits) {
            (1, 8) => (f64::from(s[0]) - 128.0) / 128.0,
            (1, 16) => f64::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0,
            (1, 24) => f64::from(i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) / 8_388_608.0,
            (1, 32) => f64::from(i32::from_le_bytes([s[0], s[1], s[2], s[3]])) / 2_147_483_648.0,
            (3, 32) => f64::from(f32::from_le_bytes([s[0], s[1], s[2], s[3]])),
            _ => return Err(bad()),
        };
        out.push(v);
    }
    Ok((out, rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_lose_their_folders_and_extensions() {
        assert_eq!(display_name("/a/b/data.v2.txt"), "data.v2");
        assert_eq!(display_name("C:\\x\\sound.wav"), "sound");
        assert_eq!(display_name("plain"), "plain");
    }

    #[test]
    fn data_text_is_checked() {
        assert_eq!(parse_data_text("# c\n1.5\r\n\n-2\n").unwrap(), [1.5, -2.0]);
        assert!(parse_data_text("1\nx\n").unwrap_err().starts_with("Error parsing"));
        assert!(parse_data_text("# only\n").unwrap_err().starts_with("No data"));
    }

    #[cfg(feature = "outputs")]
    #[test]
    fn a_wav_made_by_the_audio_output_reads_back() {
        let wav = super::super::audio::wav_bytes(&[0.0, 0.5, -0.5, 1.0], 8000);
        let (samples, rate) = decode_wav(&wav).unwrap();
        assert_eq!(rate, 8000);
        assert_eq!(samples.len(), 4);
        assert!((samples[1] - 0.5).abs() < 1e-3 && (samples[2] + 0.5).abs() < 1e-3);
        assert!(decode_wav(b"not a wav").is_err());
    }

    #[test]
    fn stored_files_get_numbers() {
        let f = SampleFile { name: "x".into(), data: Arc::new(vec![1.0]), rate: 0 };
        let n = store(0, f.clone());
        assert!(n > 0);
        assert_eq!(lookup(n).unwrap().name, "x");
        assert_eq!(store(n, f), n);
    }
}
