// SPDX-License-Identifier: GPL-2.0-or-later
//! Writes the circuits of the solver benchmarks to `tests/bench/`: a small RLC circuit, a ladder
//! of 500 elements and a mixed circuit of 1100 elements with diodes, inductors and an AC
//! source in each of 110 blocks. The same files are measured here (`cargo bench`) and in the
//! original (`cargo run --bin golden -- --bench`), so the two are compared on equal terms.
//!
//! Usage: `cargo run --bin bench_circuits [-- OUT_DIR]`.

use std::fmt::Write;
use std::path::PathBuf;

/// Circuit header: time step 5 us and the largest simulation speed of the original (its slider
/// tops out at 399 iterations a frame, 64000 steps a second): a circuit that needs longer than
/// that per step shows the original's real speed, a light one shows only its speed limit.
const HEADER: &str = "$ 1 5e-6 399 50 5 50 5e-11\n";

fn rlc() -> String {
    let mut s = String::from(HEADER);
    s += "v 0 96 0 32 0 1 100 5 0 0 0.5\ng 0 96 0 112 0\n";
    s += "r 0 32 96 32 0 10\nl 96 32 96 96 0 0.1 0\nc 96 96 0 96 0 1e-5 0\n";
    s
}

/// A source, then `sections` times a series resistor with a capacitor to ground.
fn ladder(sections: usize) -> String {
    let mut s = String::from(HEADER);
    s += "v 0 64 0 0 0 0 40 5 0 0 0.5\ng 0 64 0 80 0\n";
    for i in 0..sections {
        let (x0, x1) = (32 * i, 32 * (i + 1));
        let _ = writeln!(s, "r {x0} 0 {x1} 0 0 100");
        let _ = writeln!(s, "c {x1} 0 {x1} 32 0 1e-6 0");
        let _ = writeln!(s, "g {x1} 32 {x1} 48 0");
    }
    s
}

/// `blocks` copies of: AC source, resistor, diode, inductor, then a capacitor and two more
/// resistors to ground. All grounds are one node, so this is one large nonlinear matrix.
fn mixed(blocks: usize) -> String {
    let mut s = String::from(HEADER);
    for k in 0..blocks {
        let (y, y2, y3) = (128 * k, 128 * k + 64, 128 * k + 80);
        let _ = writeln!(s, "v 0 {y2} 0 {y} 0 1 1000 5 0 0 0.5");
        let _ = writeln!(s, "g 0 {y2} 0 {y3} 0");
        let _ = writeln!(s, "r 0 {y} 64 {y} 0 100");
        let _ = writeln!(s, "d 64 {y} 128 {y} 2 default");
        let _ = writeln!(s, "l 128 {y} 192 {y} 0 0.01 0");
        let _ = writeln!(s, "c 192 {y} 192 {y2} 0 1e-6 0");
        let _ = writeln!(s, "g 192 {y2} 192 {y3} 0");
        let _ = writeln!(s, "r 192 {y} 256 {y} 0 1000");
        let _ = writeln!(s, "r 256 {y} 256 {y2} 0 1000");
        let _ = writeln!(s, "g 256 {y2} 256 {y3} 0");
    }
    s
}

fn main() {
    let dir: PathBuf = std::env::args().nth(1).map_or_else(|| "tests/bench".into(), PathBuf::from);
    let _ = std::fs::create_dir_all(&dir);
    for (name, text) in [("rlc", rlc()), ("ladder", ladder(166)), ("mixed", mixed(110))] {
        let path = dir.join(format!("{name}.txt"));
        match std::fs::write(&path, text) {
            Ok(()) => println!("wrote {}", path.display()),
            Err(e) => eprintln!("cannot write {}: {e}", path.display()),
        }
    }
}
