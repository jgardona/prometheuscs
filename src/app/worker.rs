// SPDX-License-Identifier: GPL-2.0-or-later
//! The simulation worker: a thread that owns its own copy of the circuit and steps it, so the
//! interface never waits for the solver.
//!
//! The interface sends [`Command`]s and picks up the newest [`FrameSnapshot`] each frame. The
//! circuit crosses the thread boundary as text, because elements are not `Send` and the text
//! carries their running state.

use super::session::Session;
use super::settings::AppSettings;
use crate::elements::registry::Registry;
use crate::solver::snapshot::FrameSnapshot;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Pause between work rounds while time is running.
const RUN_INTERVAL: Duration = Duration::from_millis(3);
/// Longest wait for a command while time is stopped.
const IDLE_INTERVAL: Duration = Duration::from_millis(50);
/// Fewest milliseconds between two published snapshots while time is running.
const PUBLISH_INTERVAL: Duration = Duration::from_millis(8);
/// How long the interface waits for a reply from the worker.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// A request to the worker.
#[derive(Debug)]
pub enum Command {
    /// Replace the circuit with this text (analysing it again). With `keep_time` the clock
    /// continues, which is what an edit of a running circuit needs.
    Load {
        /// Circuit text in the XML format, including element state.
        xml: String,
        /// Keep the simulated time instead of starting from zero.
        keep_time: bool,
    },
    /// Empty the recorded history of one scope.
    ResetScope(usize),
    /// Move a plot of a scope in manual scale mode (while it is being dragged).
    PlotPosition {
        /// Scope index.
        scope: usize,
        /// Plot index.
        plot: usize,
        /// Position in steps.
        pos: i32,
    },
    /// Start or stop time.
    SetRunning(bool),
    /// Reset time and element state, then run.
    Reset,
    /// Set the simulation speed as the original's iteration count factor.
    SetSpeed(f64),
    /// Change one editable field of an element without rebuilding the circuit.
    Edit {
        /// Element index.
        elm: usize,
        /// Field index in the element's edit list.
        n: usize,
        /// New value.
        value: f64,
    },
    /// Move the slider an element owns itself (for example a potentiometer), 0 to 100.
    ElementSlider {
        /// Element index.
        elm: usize,
        /// Slider position from 0 to 100.
        value: i32,
    },
    /// Find the DC operating point.
    FindDc,
    /// Advance this many time steps now and publish the result (headless runs and tests).
    RunSteps(usize),
    /// Send back the circuit with its current state.
    Dump(Sender<String>),
    /// Send back what the audio outputs have recorded.
    Audio(Sender<Vec<AudioClip>>),
    /// Reply once every earlier command has been carried out.
    Sync(Sender<()>),
    /// Stop the thread.
    Shutdown,
}

/// What an audio output recorded.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioClip {
    /// Index of the element.
    pub elm: usize,
    /// Its name.
    pub label: String,
    /// Samples per second.
    pub rate: i32,
    /// The samples, in volts, oldest first.
    pub samples: Vec<f64>,
}

/// Handle of a running worker thread. Dropping it stops the thread.
pub struct Worker {
    tx: Sender<Command>,
    slot: Arc<Mutex<Option<FrameSnapshot>>>,
    join: Option<JoinHandle<()>>,
}

impl Worker {
    /// Starts a worker thread with an empty circuit.
    pub fn spawn(registry: Registry, settings: AppSettings) -> Worker {
        let (tx, rx) = mpsc::channel();
        let slot = Arc::new(Mutex::new(None));
        let thread_slot = slot.clone();
        let join = std::thread::Builder::new()
            .name("simulation".into())
            .spawn(move || run(&rx, &thread_slot, &registry, settings))
            .ok();
        Worker { tx, slot, join }
    }

    /// Sends a command; never blocks.
    pub fn send(&self, command: Command) {
        // a closed channel means the thread already stopped; there is nobody to tell
        let _ = self.tx.send(command);
    }

    /// Takes the newest snapshot published since the last call, if there is one.
    pub fn latest(&self) -> Option<FrameSnapshot> {
        self.slot.lock().ok().and_then(|mut g| g.take())
    }

    /// Asks for the circuit with its current state. Blocks until the worker answers, which is
    /// within one work round.
    pub fn dump(&self) -> Option<String> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Dump(tx));
        rx.recv_timeout(REPLY_TIMEOUT).ok()
    }

    /// What the audio outputs of the circuit have recorded so far.
    pub fn audio(&self) -> Option<Vec<AudioClip>> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Audio(tx));
        rx.recv_timeout(REPLY_TIMEOUT).ok()
    }

    /// Waits until every command sent so far has been carried out.
    pub fn sync(&self) -> bool {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Sync(tx));
        rx.recv_timeout(REPLY_TIMEOUT).is_ok()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        if let Some(j) = self.join.take() {
            // a panic in the worker was already reported by the panic hook
            let _ = j.join();
        }
    }
}

/// Carries out one command; false means stop the thread.
fn handle(s: &mut Session, command: Command, syncs: &mut Vec<Sender<()>>) -> bool {
    match command {
        Command::Load { xml, keep_time } => {
            let (t, running) = (s.sim.time(), s.running);
            let title = s.title.clone();
            let old_scopes = std::mem::take(&mut s.sim.scopes);
            let loaded = s.open_text(&xml, &title, None).is_ok();
            if keep_time {
                s.sim.set_time(t);
                // scopes that did not change keep what they recorded
                let mut old: Vec<Option<crate::scope::Scope>> =
                    old_scopes.into_iter().map(Some).collect();
                for sc in &mut s.sim.scopes {
                    let found =
                        old.iter().position(|o| o.as_ref().is_some_and(|o| o.same_plots(sc)));
                    if let Some(mut old_scope) = found.and_then(|i| old[i].take()) {
                        old_scope.take_settings_from(sc);
                        *sc = old_scope;
                    }
                }
            }
            s.running = running && loaded;
        }
        Command::ResetScope(i) => {
            let t = s.sim.time();
            if let Some(sc) = s.sim.scopes.get_mut(i) {
                sc.reset(t);
            }
        }
        Command::PlotPosition { scope, plot, pos } => {
            if let Some(p) = s.sim.scopes.get_mut(scope).and_then(|sc| sc.plots.get_mut(plot)) {
                p.man_pos = pos;
                p.man_set = true;
            }
        }
        Command::SetRunning(on) => s.set_running(on),
        Command::Reset => s.reset(),
        Command::SetSpeed(ic) => s.options.iter_count = ic,
        Command::Edit { elm, n, value } => s.edit_value(elm, n, value),
        Command::ElementSlider { elm, value } => s.apply_element_slider(elm, value),
        Command::FindDc => {
            // a failure is recorded in the session and reported by the next snapshot
            let _ = s.find_dc();
        }
        Command::RunSteps(n) => s.run_steps_now(n),
        Command::Dump(reply) => {
            let _ = reply.send(s.circuit_xml());
        }
        Command::Audio(reply) => {
            let _ = reply.send(s.audio_clips());
        }
        // answered after the next snapshot is published, so the caller can read it at once
        Command::Sync(reply) => syncs.push(reply),
        Command::Shutdown => return false,
    }
    true
}

/// Runs `f`; a panic inside it stops the simulation with an error that the interface shows,
/// instead of silently killing the thread.
fn guarded<T>(s: &mut Session, f: impl FnOnce(&mut Session) -> T) -> Option<T> {
    match catch_unwind(AssertUnwindSafe(|| f(s))) {
        Ok(v) => Some(v),
        Err(payload) => {
            let what = payload
                .downcast_ref::<&str>()
                .map(|m| (*m).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown".to_string());
            tracing::error!("simulation worker panicked: {what}");
            s.record_stop(&format!("Erro interno do simulador: {what}"), None);
            None
        }
    }
}

fn run(
    rx: &Receiver<Command>,
    slot: &Arc<Mutex<Option<FrameSnapshot>>>,
    registry: &Registry,
    settings: AppSettings,
) {
    let mut s = Session::empty_with(registry, settings);
    let mut sequence = 0_u64;
    let mut last_round = Instant::now();
    let mut last_publish = Instant::now();
    let mut syncs: Vec<Sender<()>> = Vec::new();
    loop {
        let wait = if s.running { RUN_INTERVAL } else { IDLE_INTERVAL };
        let mut changed = false;
        match rx.recv_timeout(wait) {
            Ok(c) => {
                if !guarded(&mut s, |s| handle(s, c, &mut syncs)).unwrap_or(true) {
                    return;
                }
                changed = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(c) = rx.try_recv() {
            if !guarded(&mut s, |s| handle(s, c, &mut syncs)).unwrap_or(true) {
                return;
            }
            changed = true;
        }
        let now = Instant::now();
        let elapsed = now - last_round;
        last_round = now;
        let was_running = s.running;
        if s.running {
            // an error stops the session and is carried by the snapshot
            let _ = guarded(&mut s, |s| s.update(elapsed));
        }
        let due = was_running && now - last_publish >= PUBLISH_INTERVAL;
        if changed || due || was_running != s.running || !syncs.is_empty() {
            sequence += 1;
            let snapshot = s.snapshot(sequence);
            if let Ok(mut g) = slot.lock() {
                *g = Some(snapshot);
            }
            last_publish = now;
        }
        for reply in syncs.drain(..) {
            let _ = reply.send(());
        }
    }
}
