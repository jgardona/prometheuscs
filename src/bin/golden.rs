// SPDX-License-Identifier: GPL-2.0-or-later
//! Developer tool: records what the original CircuitJS1 computes, to compare with this program.
//!
//! The original is the Electron build (by default `/home/jgardona/Bin/circuitjs1`). It does not
//! accept command line switches for remote debugging, so the tool works on a private copy: it
//! copies the program to a temporary folder, adds a few lines to the copy's `main.js` and
//! `preload.js` that load every example, run it to a few simulated times and write the state of
//! every element to a JSON file, and starts that copy once or twice. Examples whose two runs
//! differ are listed in `nondeterministic.json`.
//!
//! Usage: `cargo run --bin golden -- [--app DIR] [--circuits DIR] [--out DIR] [--only a,b]
//! [--times 0.001,0.01,0.05 | --steps 200,2000,10000] [--runs 2] [--limit-ms 20000]` or
//! `--bench [--bench-ms 5000] [--runs 3]` to measure the original on `tests/bench/`. A window opens while it runs.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const PRELOAD: &str = include_str!("../../tests/compat/tools/golden-preload.js");

struct Options {
    app: PathBuf,
    circuits: PathBuf,
    out: PathBuf,
    only: Option<String>,
    times: String,
    steps: Option<String>,
    runs: usize,
    limit_ms: u64,
    /// Measure how fast the original runs the benchmark circuits instead of recording states.
    bench: bool,
    /// Wall-clock milliseconds measured per circuit in the bench mode.
    bench_ms: u64,
}

fn parse_args() -> Result<Options, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut o = Options {
        app: PathBuf::from("/home/jgardona/Bin/circuitjs1"),
        circuits: root.join("assets/circuits"),
        out: root.join("tests/compat/golden"),
        only: None,
        times: "0.001,0.01,0.05".into(),
        steps: None,
        runs: 2,
        limit_ms: 20_000,
        bench: false,
        bench_ms: 5_000,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--app" => o.app = value()?.into(),
            "--circuits" => o.circuits = value()?.into(),
            "--out" => o.out = value()?.into(),
            "--only" => o.only = Some(value()?),
            "--times" => o.times = value()?,
            "--steps" => o.steps = Some(value()?),
            "--bench" => {
                o.bench = true;
                o.circuits = root.join("tests/bench");
                o.out = root.join("tests/bench");
            }
            "--bench-ms" => {
                o.bench_ms = value()?.parse().map_err(|e| format!("--bench-ms: {e}"))?
            }
            "--runs" => o.runs = value()?.parse().map_err(|e| format!("--runs: {e}"))?,
            "--limit-ms" => {
                o.limit_ms = value()?.parse().map_err(|e| format!("--limit-ms: {e}"))?
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(o)
}

/// Copies a folder, leaving out the `._*` files macOS archives add.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("._") {
            continue;
        }
        let target = to.join(&name);
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if kind.is_symlink() {
            let link = fs::read_link(entry.path())?;
            std::os::unix::fs::symlink(link, &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The copy of the program with the recording code added.
fn prepare_app(o: &Options, work: &Path) -> Result<PathBuf, String> {
    let app = work.join("app");
    copy_dir(&o.app, &app).map_err(|e| format!("copying {}: {e}", o.app.display()))?;
    let res = app.join("resources/app");
    let main = fs::read_to_string(res.join("main.js")).map_err(|e| format!("main.js: {e}"))?;
    let user_data = work.join("userdata");
    let patched = main
        .replace(
            "const gotLock = app.requestSingleInstanceLock();",
            &format!(
                "app.setPath('userData', {:?});\nconst gotLock = app.requestSingleInstanceLock();",
                user_data.display().to_string()
            ),
        )
        .replace(
            "ipcMain.on('new-window'",
            "ipcMain.on('golden-done', () => { app.quit(); });\n\nipcMain.on('new-window'",
        );
    if patched == main {
        return Err("main.js has an unexpected layout; the copy was not patched".into());
    }
    fs::write(res.join("main.js"), patched).map_err(|e| e.to_string())?;
    let mut preload = fs::read_to_string(res.join("preload.js")).map_err(|e| e.to_string())?;
    preload.push_str(PRELOAD);
    fs::write(res.join("preload.js"), preload).map_err(|e| e.to_string())?;
    Ok(app.join("circuitjs1"))
}

fn run_once(o: &Options, exe: &Path, out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(exe);
    cmd.env("GOLDEN_IN", &o.circuits)
        .env("GOLDEN_OUT", out)
        .env("GOLDEN_TIMES", &o.times)
        .env("GOLDEN_LIMIT_MS", o.limit_ms.to_string());
    if let Some(steps) = &o.steps {
        cmd.env("GOLDEN_STEPS", steps);
    }
    if o.bench {
        cmd.env("GOLDEN_BENCH_MS", o.bench_ms.to_string());
    }
    if let Some(only) = &o.only {
        cmd.env("GOLDEN_ONLY", only);
    }
    let status = cmd.status().map_err(|e| format!("starting {}: {e}", exe.display()))?;
    if status.success() { Ok(()) } else { Err(format!("{} exited with {status}", exe.display())) }
}

fn read_runs(dir: &Path) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json") {
                if let Some(v) =
                    fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str(&t).ok())
                {
                    map.insert(p.file_stem().unwrap_or_default().to_string_lossy().into_owned(), v);
                }
            }
        }
    }
    map
}

/// True when two recordings of the same circuit differ beyond rounding.
fn differs(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
            (x - y).abs() > 1e-9 * (1.0 + x.abs().max(y.abs()))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() != y.len() || x.iter().zip(y).any(|(p, q)| differs(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() != y.len() || x.iter().any(|(k, v)| y.get(k).is_none_or(|w| differs(v, w)))
        }
        _ => a != b,
    }
}

/// Runs the benchmark circuits in the original and writes `baseline.json` next to them: time
/// steps per second of each, the best of the runs.
fn bench_original(o: &Options, exe: &Path, work: &Path) -> ExitCode {
    let mut best: BTreeMap<String, f64> = BTreeMap::new();
    for i in 0..o.runs.max(1) {
        let out = work.join(format!("bench{i}"));
        eprintln!("golden: bench run {} of {}", i + 1, o.runs.max(1));
        if let Err(e) = run_once(o, exe, &out) {
            eprintln!("golden: {e}");
            return ExitCode::FAILURE;
        }
        for (name, v) in read_runs(&out) {
            let sps = v.get("stepsPerSecond").and_then(Value::as_f64).unwrap_or(0.0);
            let slot = best.entry(name).or_insert(0.0);
            *slot = slot.max(sps);
        }
    }
    let circuits: serde_json::Map<String, Value> =
        best.iter().map(|(k, v)| (k.clone(), Value::from(v.round()))).collect();
    let doc = serde_json::json!({
        "source": "CircuitJS1 (Electron build), run with --bench",
        "unit": "time steps per second",
        "measured_ms_per_circuit": o.bench_ms,
        "circuits": circuits,
    });
    let path = o.out.join("baseline.json");
    match fs::write(&path, serde_json::to_string_pretty(&doc).unwrap_or_default()) {
        Ok(()) => {
            eprintln!("golden: wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("golden: cannot write {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let o = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("golden: {e}");
            return ExitCode::FAILURE;
        }
    };
    let work = std::env::temp_dir().join("prometheuscs-golden");
    let _ = fs::remove_dir_all(&work);
    let exe = match prepare_app(&o, &work) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("golden: {e}");
            return ExitCode::FAILURE;
        }
    };
    if o.bench {
        return bench_original(&o, &exe, &work);
    }
    let mut runs = Vec::new();
    for i in 0..o.runs.max(1) {
        let out = work.join(format!("run{i}"));
        eprintln!("golden: run {} of {}", i + 1, o.runs.max(1));
        if let Err(e) = run_once(&o, &exe, &out) {
            eprintln!("golden: {e}");
            return ExitCode::FAILURE;
        }
        runs.push(read_runs(&out));
    }
    if fs::create_dir_all(&o.out).is_err() {
        eprintln!("golden: cannot create {}", o.out.display());
        return ExitCode::FAILURE;
    }
    let mut nondeterministic = Vec::new();
    let mut written = 0;
    for (name, first) in &runs[0] {
        if runs[1..].iter().any(|r| r.get(name).is_none_or(|v| differs(first, v))) {
            nondeterministic.push(name.clone());
        }
        let text = serde_json::to_string(first).unwrap_or_default();
        if fs::write(o.out.join(format!("{name}.json")), text).is_ok() {
            written += 1;
        }
    }
    // a run of some examples only (--only) keeps what the list says about the others
    let path = o.out.join("../nondeterministic.json");
    if o.only.is_some() {
        let ran: Vec<&String> = runs[0].keys().collect();
        let before: Vec<String> = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let mut kept: Vec<String> = before.into_iter().filter(|n| !ran.contains(&n)).collect();
        kept.extend(nondeterministic.iter().cloned());
        kept.sort();
        nondeterministic = kept;
    }
    let list = serde_json::to_string_pretty(&nondeterministic).unwrap_or_default();
    let _ = fs::write(path, list);
    eprintln!(
        "golden: {written} circuits written to {}; {} differ between runs",
        o.out.display(),
        nondeterministic.len()
    );
    ExitCode::SUCCESS
}
