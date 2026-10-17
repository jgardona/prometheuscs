// SPDX-License-Identifier: GPL-2.0-or-later
//! Plays a WAV file through the sound server's command line player.
//!
//! The program has no audio library of its own: it writes the recording to a temporary file
//! and hands it to the first player it finds (PipeWire's, PulseAudio's or ALSA's). Playing
//! again stops what is still playing.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// The players to try, in order, with the arguments that go before the file name.
const PLAYERS: [(&str, &[&str]); 3] = [("pw-play", &[]), ("paplay", &[]), ("aplay", &["-q"])];

static CURRENT: Mutex<Option<(Child, PathBuf)>> = Mutex::new(None);

/// Stops what is playing and removes its file.
pub fn stop() {
    if let Ok(mut current) = CURRENT.lock() {
        if let Some((mut child, file)) = current.take() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(file);
        }
    }
}

/// Starts playing `wav` (the bytes of a WAV file). Returns at once; an error says why no
/// player could be started.
pub fn play(wav: &[u8]) -> Result<(), String> {
    stop();
    let file = std::env::temp_dir().join(format!("prometheuscs-audio-{}.wav", std::process::id()));
    std::fs::write(&file, wav).map_err(|e| format!("{}: {e}", file.display()))?;
    for (program, args) in PLAYERS {
        let spawned = Command::new(program)
            .args(args)
            .arg(&file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(child) = spawned {
            if let Ok(mut current) = CURRENT.lock() {
                *current = Some((child, file));
            }
            return Ok(());
        }
    }
    let _ = std::fs::remove_file(&file);
    Err("no audio player found (pw-play, paplay or aplay)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "outputs")]
    fn a_missing_player_is_reported_not_fatal() {
        // whether or not this machine has a player, the call returns and stops cleanly
        let wav = crate::elements::audio::wav_bytes(&[0.0; 800], 8000);
        let _ = play(&wav);
        stop();
    }
}
