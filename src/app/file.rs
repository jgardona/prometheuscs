// SPDX-License-Identifier: GPL-2.0-or-later
//! Document operations of a session: new, open, save, export and automatic save.

use super::session::{Session, SessionError};
use super::settings::autosave_path;
use crate::error::AnalysisError;
use crate::format::circuit::Circuit;
use crate::format::parser::{ParseResult, parse};
use std::path::{Path, PathBuf};

impl Session {
    /// Title of a circuit that has no name yet.
    pub const UNTITLED: &'static str = "Circuito sem título";

    /// Replaces the circuit with an empty one that uses the saved display settings.
    pub fn new_circuit(&mut self) {
        let circuit =
            Circuit { options: Self::default_options(&self.settings), ..Circuit::default() };
        // an empty circuit always analyses
        let _ = self.install(ParseResult { circuit, errors: Vec::new() });
        self.title = Self::UNTITLED.to_string();
        self.path = None;
        self.running = true;
    }

    /// Replaces the circuit with the one in `text`. The circuit is loaded even when analysis
    /// fails; the error says why it cannot run.
    pub fn open_text(
        &mut self,
        text: &str,
        title: &str,
        path: Option<PathBuf>,
    ) -> Result<(), SessionError> {
        let parsed = parse(text, &self.registry);
        // the worker is told the run state while installing, so it must be set first
        self.running = true;
        let result = self.install(parsed);
        self.title = title.to_string();
        self.path = path;
        if result.is_err() {
            self.set_running(false);
        }
        if let Err(SessionError::Analysis(AnalysisError::Invalid { message, element })) = &result {
            self.record_stop(message, *element);
        }
        result
    }

    /// Loads a circuit file; its name becomes the title.
    pub fn open_file(&mut self, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let title = path
            .file_stem()
            .map_or_else(|| Self::UNTITLED.to_string(), |s| s.to_string_lossy().into_owned());
        self.open_text(&text, &title, Some(path.to_path_buf())).map_err(|e| e.to_string())
    }

    /// Writes the circuit to `path` and makes it the current file.
    pub fn save_to(&mut self, path: &Path) -> std::io::Result<()> {
        let text = self.circuit_xml();
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, text)?;
        if let Some(stem) = path.file_stem() {
            self.title = stem.to_string_lossy().into_owned();
        }
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    /// The circuit as text for the Export as Text dialog.
    pub fn export_text(&mut self) -> String {
        self.circuit_xml()
    }

    /// Saves a recovery copy when the circuit has unsaved changes.
    pub fn autosave(&mut self) -> std::io::Result<bool> {
        if !self.dirty {
            return Ok(false);
        }
        let path = autosave_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.circuit_xml())?;
        Ok(true)
    }

    /// Loads the recovery copy, if there is one.
    pub fn recover_autosave(&mut self) -> Result<(), String> {
        let path = autosave_path();
        let text = std::fs::read_to_string(&path)
            .map_err(|_| "Nenhum salvamento automático".to_string())?;
        self.open_text(&text, "Recuperado", None).map_err(|e| e.to_string())?;
        self.dirty = true;
        Ok(())
    }

    /// Finds the DC operating point and keeps simulating from there.
    pub fn find_dc(&mut self) -> Result<(), String> {
        if let Some(w) = &self.worker {
            w.send(super::worker::Command::FindDc);
            return Ok(());
        }
        let result = self.sim.find_dc_operating_point().map_err(|e| e.to_string());
        if let Err(msg) = &result {
            self.record_stop(msg, None);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::registry::Registry;

    const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n";

    fn session() -> Session {
        Session::load(RC, &Registry::standard()).unwrap()
    }

    #[test]
    fn save_then_open_gives_the_same_circuit() {
        let dir = std::env::temp_dir().join(format!("prometheuscs-file-{}", std::process::id()));
        let path = dir.join("rc.xml");
        let mut s = session();
        s.dirty = true;
        s.save_to(&path).unwrap();
        assert!(!s.dirty);
        assert_eq!(s.title, "rc");
        let mut t = Session::empty();
        t.open_file(&path).unwrap();
        assert_eq!(t.sim.elements.len(), 4);
        assert_eq!(t.circuit_xml(), s.circuit_xml());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn new_circuit_is_empty_and_untitled() {
        let mut s = session();
        s.new_circuit();
        assert!(s.sim.elements.is_empty());
        assert_eq!(s.title, Session::UNTITLED);
        assert!(!s.can_undo());
    }

    #[test]
    fn find_dc_keeps_the_circuit_running() {
        let mut s = session();
        s.find_dc().unwrap();
        assert!(s.stop_message.is_none());
    }

    #[test]
    fn a_stop_names_and_highlights_the_offending_element() {
        let mut s = session();
        s.record_stop("Something is wrong", Some(1));
        let msg = s.stop_message.clone().unwrap();
        assert!(msg.starts_with("Something is wrong (") && msg.contains(" em "), "{msg}");
        let i = s.stop_element.expect("element");
        assert_eq!(i, 1);
        assert!(s.sim.elements[i].base().highlight);
        assert!(!s.running);
        // hovering elsewhere must not clear the highlight; restarting does
        s.hover_at(Some((5000.0, 5000.0)));
        assert!(s.sim.elements[i].base().highlight);
        s.set_running(true);
        assert!(s.stop_message.is_none() && !s.sim.elements[i].base().highlight);
    }

    #[test]
    fn opening_a_missing_file_reports_the_path() {
        let mut s = session();
        let err = s.open_file(Path::new("/nonexistent/x.txt")).unwrap_err();
        assert!(err.contains("/nonexistent/x.txt"));
    }
}
