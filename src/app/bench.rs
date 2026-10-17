// SPDX-License-Identifier: GPL-2.0-or-later
//! The render benchmark: `prometheuscs --bench-render FILE [--bench-seconds N]` opens the window
//! with the circuit in FILE, runs the simulation while the view stays put, then while it pans,
//! then while it zooms, and prints how the frames went before quitting (SC-003: 60 frames a
//! second for circuits of up to 500 elements). It also reports the time from the start of the
//! program to its first drawn frame (SC-005: under 2 seconds) and, in a fourth phase, the
//! time from a click to the frame that shows its effect (SC-006: under 50 milliseconds).
//!
//! Frames are timed where they are drawn. Their rate follows the display (vertical sync caps
//! it at the refresh rate), so the time spent drawing a frame is reported too: it tells how
//! much room there is. The first seconds are left out: a window being shown can stall the
//! frame clock for a while (a compositor effect, seen on Wayland).

use super::edit::Mods;
use super::session::Session;
use crate::render::scene::View;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// What the command line asked for.
#[derive(Clone, Debug)]
pub struct BenchConfig {
    /// Circuit file to draw.
    pub path: PathBuf,
    /// Seconds spent in each of the three phases.
    pub seconds: f64,
}

static CONFIG: OnceLock<BenchConfig> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Reads `--bench-render FILE` and `--bench-seconds N` from the command line.
pub fn configure(args: impl IntoIterator<Item = String>) {
    let _ = STARTED.set(Instant::now());
    let mut path = None;
    let mut seconds = 3.0;
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bench-render" => path = args.next().map(PathBuf::from),
            "--bench-seconds" => {
                seconds = args.next().and_then(|v| v.parse().ok()).unwrap_or(seconds);
            }
            _ => {}
        }
    }
    if let Some(path) = path {
        let _ = CONFIG.set(BenchConfig { path, seconds });
    }
}

/// Milliseconds since the program began (0 before `configure` ran).
pub fn since_start() -> f64 {
    STARTED.get().map_or(0.0, |t| t.elapsed().as_secs_f64() * 1000.0)
}

/// The benchmark asked for on the command line, if any.
pub fn config() -> Option<&'static BenchConfig> {
    CONFIG.get()
}

const WARM_UP: f64 = 3.0;
const PHASES: [&str; 3] = ["animate", "pan", "zoom"];
/// How many phases there are, the last one being the input latency test.
const PHASE_COUNT: usize = 4;

#[derive(Default)]
struct PhaseStats {
    /// Time between frames, in milliseconds.
    intervals: Vec<f64>,
    /// Time spent drawing a frame, in milliseconds.
    draws: Vec<f64>,
    /// Time spent bringing the session up to date for a frame, in milliseconds.
    updates: Vec<f64>,
    /// Gaps of more than a quarter of a second between frames (the window was not being
    /// shown, as far as the program can tell), left out of the rest, in milliseconds.
    stalls: Vec<f64>,
    /// Time from a click to the end of the frame drawn after it, in milliseconds.
    latencies: Vec<f64>,
}

/// A gap this long between two frames is a stall, not a slow frame.
const STALL_MS: f64 = 250.0;

/// A benchmark in progress.
pub struct Bench {
    config: BenchConfig,
    elements: usize,
    started: Option<i64>,
    last_frame: Option<i64>,
    last_draw: Option<Instant>,
    base_view: Option<View>,
    /// Milliseconds from the start of the program to the first frame drawn.
    first_draw: Option<f64>,
    /// When the click being timed was made, and how long its handling took.
    click: Option<(Instant, f64)>,
    stats: [PhaseStats; PHASE_COUNT],
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

impl Bench {
    /// A benchmark of a circuit of `elements` elements.
    pub fn new(config: BenchConfig, elements: usize) -> Self {
        Bench {
            config,
            elements,
            started: None,
            last_frame: None,
            last_draw: None,
            base_view: None,
            first_draw: None,
            click: None,
            stats: Default::default(),
        }
    }

    /// Which phase the benchmark is in at `t` seconds: None while warming up, then 0, 1 and 2,
    /// 3 (the input test) and 4 once it is over.
    fn phase(&self, t: f64) -> Option<usize> {
        if t < WARM_UP {
            return None;
        }
        Some((((t - WARM_UP) / self.config.seconds) as usize).min(PHASE_COUNT))
    }

    /// Called once per frame (`now` is the frame clock in microseconds). Moves the view as
    /// the phase needs and returns true when the benchmark is over.
    pub fn on_tick(&mut self, now: i64, s: &mut Session) -> bool {
        let start = *self.started.get_or_insert(now);
        let t = (now - start) as f64 / 1e6;
        let phase = self.phase(t);
        self.last_frame = Some(now);
        // the view the circuit was fitted to is what pan and zoom move around
        if self.base_view.is_none() && !s.fit_pending && s.canvas_size.0 > 0.0 {
            self.base_view = Some(s.view);
        }
        if phase == Some(3) && self.click.is_none() {
            // a click on bare canvas near the corner, timed until the frame that follows it
            let began = Instant::now();
            s.mouse_down(4.0, 4.0, Mods::default(), 1);
            s.mouse_up();
            self.click = Some((began, began.elapsed().as_secs_f64() * 1000.0));
        }
        if let (Some(base), Some(p)) = (self.base_view, phase.filter(|p| *p < 3)) {
            let local = t - WARM_UP - p as f64 * self.config.seconds;
            let angle = local * std::f64::consts::TAU / 2.0;
            s.view = match p {
                1 => View {
                    x: base.x + 150.0 * angle.sin(),
                    y: base.y + 100.0 * angle.cos(),
                    ..base
                },
                2 => View { scale: base.scale * (0.6 * angle.sin()).exp(), ..base },
                _ => base,
            };
        }
        phase == Some(PHASE_COUNT)
    }

    /// Called with the time the session took to catch up for a frame (taking in what the
    /// solver's thread published).
    pub fn on_update(&mut self, now: i64, took: Duration) {
        let Some(start) = self.started else { return };
        if let Some(p @ 0..=2) = self.phase((now - start) as f64 / 1e6) {
            self.stats[p].updates.push(took.as_secs_f64() * 1000.0);
        }
    }

    /// Called after each frame is drawn with the time drawing took. The frames are counted
    /// here, where they are drawn: the frame clock may tick more than once for a frame.
    pub fn on_draw(&mut self, took: Duration) {
        let now = Instant::now();
        if self.first_draw.is_none() {
            tracing::debug!(ms = since_start(), "first frame");
            self.first_draw = STARTED.get().map(|t| now.duration_since(*t).as_secs_f64() * 1000.0);
        }
        if let Some((began, _)) = self.click.take() {
            self.stats[3].latencies.push(now.duration_since(began).as_secs_f64() * 1000.0);
        }
        let before = self.last_draw.replace(now);
        let Some(start) = self.started else { return };
        let Some(last) = self.last_frame else { return };
        let t = (last - start) as f64 / 1e6;
        if let (Some(p @ 0..=2), Some(before)) = (self.phase(t), before) {
            let gap = now.duration_since(before).as_secs_f64() * 1000.0;
            if gap > STALL_MS {
                self.stats[p].stalls.push(gap);
                return;
            }
            self.stats[p].intervals.push(gap);
            self.stats[p].draws.push(took.as_secs_f64() * 1000.0);
        }
    }

    /// The report: one line per phase.
    pub fn report(&self, backend: &str) -> String {
        let mut out = format!(
            "bench-render: {} ({} elements, {backend} canvas, {} s per phase)\n",
            self.config.path.display(),
            self.elements,
            self.config.seconds
        );
        if let Some(ms) = self.first_draw {
            out += &format!("  startup: first frame drawn {ms:.0} ms after the program began\n");
        }
        for (name, st) in PHASES.iter().zip(&self.stats) {
            let mut iv = st.intervals.clone();
            iv.sort_by(f64::total_cmp);
            let mut dr = st.draws.clone();
            dr.sort_by(f64::total_cmp);
            let total: f64 = iv.iter().sum();
            let fps = if total > 0.0 { iv.len() as f64 * 1000.0 / total } else { 0.0 };
            let mean = |v: &[f64]| {
                if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
            };
            let draw_avg = mean(&dr);
            let update_avg = mean(&st.updates);
            // how fast frames could come if the display allowed it
            let room =
                if draw_avg + update_avg > 0.0 { 1000.0 / (draw_avg + update_avg) } else { 0.0 };
            out += &format!(
                "  {name:8} {fps:6.1} fps   frame interval p50 {:5.2} ms  p95 {:5.2} ms  worst {:5.2} ms   drawing avg {draw_avg:5.2} ms  p95 {:5.2} ms  worst {:5.2} ms   update avg {update_avg:5.2} ms   room for {room:6.0} fps\n",
                percentile(&iv, 0.5),
                percentile(&iv, 0.95),
                iv.last().copied().unwrap_or(0.0),
                percentile(&dr, 0.95),
                dr.last().copied().unwrap_or(0.0),
            );
        }
        let mut lat = self.stats[3].latencies.clone();
        lat.sort_by(f64::total_cmp);
        if !lat.is_empty() {
            out += &format!(
                "  input    click to drawn frame ({} clicks): p50 {:.1} ms  p95 {:.1} ms  worst {:.1} ms\n",
                lat.len(),
                percentile(&lat, 0.5),
                percentile(&lat, 0.95),
                lat.last().copied().unwrap_or(0.0)
            );
        }
        let stalls: usize = self.stats.iter().map(|s| s.stalls.len()).sum();
        if stalls > 0 {
            let worst = self.stats.iter().flat_map(|s| &s.stalls).fold(0.0_f64, |m, v| m.max(*v));
            out += &format!(
                "  {stalls} stall(s) of the frame clock left out, the longest {worst:.0} ms\n"
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bench() -> Bench {
        Bench::new(BenchConfig { path: "x.txt".into(), seconds: 2.0 }, 5)
    }

    #[test]
    fn the_phases_follow_the_warm_up_in_order() {
        let b = bench();
        assert_eq!(b.phase(WARM_UP - 0.5), None);
        assert_eq!(b.phase(WARM_UP), Some(0));
        assert_eq!(b.phase(WARM_UP + 1.9), Some(0));
        assert_eq!(b.phase(WARM_UP + 2.1), Some(1));
        assert_eq!(b.phase(WARM_UP + 4.5), Some(2));
        assert_eq!(b.phase(WARM_UP + 6.1), Some(3));
        assert_eq!(b.phase(WARM_UP + 8.1), Some(4));
    }

    #[test]
    fn the_report_gives_frames_per_second_and_drawing_time() {
        let mut b = bench();
        // 60 frames in the animate phase, 16 ms apart, each drawn in 2 ms
        for i in 0..60 {
            b.stats[0].intervals.push(16.0 + f64::from(i % 2) * 2.0);
            b.stats[0].draws.push(2.0);
        }
        let r = b.report("gl");
        assert!(r.contains("5 elements, gl canvas"), "{r}");
        let line = r.lines().find(|l| l.contains("animate")).unwrap();
        // 60 frames in 60 * 17 ms
        assert!(line.contains("58.8 fps"), "{line}");
        assert!(line.contains("drawing avg  2.00 ms"), "{line}");
    }
}
