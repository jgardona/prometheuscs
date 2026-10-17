# Solver benchmarks (SC-004)

Three circuits, written by `cargo run --release --bin bench_circuits`:

| file         | elements | what it is                                                     |
|--------------|---------:|----------------------------------------------------------------|
| `rlc.txt`    |        6 | series RLC driven by a sine                                    |
| `ladder.txt` |      500 | 166 sections of a series resistor and a capacitor to ground    |
| `mixed.txt`  |     1100 | 110 blocks (AC source, diodes, inductors, capacitors, resistors) sharing one ground: a large nonlinear matrix |

## This program

```
cargo bench --bench solver
```

Reports time steps per second for each circuit (`steps/*`, from `run_steps` after a warm-up)
and the time to parse and analyse the two large ones (`analysis/*`). `ours.json` keeps a run
for reference; it depends on the machine.

## The original

```
cargo run --release --bin golden -- --bench --runs 3
```

Starts a private copy of the Electron build (as the golden recorder does), runs each circuit at
the original's top speed for 5 s after a warm-up, and writes `baseline.json` here with the best
steps per second. A circuit that is light enough shows only the original's own limit (about
64000 steps a second: its speed slider tops out at 399 iterations a frame); a heavy one shows
its real speed. Compare with `ours.json`: SC-004 asks for at least the original's speed on
every circuit and twice as much on the large ones.

## Result (steps per second, same machine)

| circuit | this program | the original | ratio |
|---------|-------------:|-------------:|------:|
| rlc | 7,486,000 | 43,663 | 171x |
| ladder | 108,710 | 18,531 | 6x |
| mixed | 10,036 | 657 | 15x |

SC-004 asks for at least the original's speed on every circuit and twice as much on the large
ones (`ladder`, `mixed`): met with a wide margin. The comparison is not perfectly even: this
program is measured by `cargo bench` running the solver alone, the original by wall-clock time
in its window with drawing and scopes included, and its `rlc` figure is below the 64000 steps
a second of its speed limit because drawing takes a share of each frame. The ratios of the
large circuits, where the solver dominates, are the ones that mean the most.

## Drawing (SC-003)

```
cargo run --release --features gui -- --bench-render tests/bench/ladder.txt [--bench-seconds 3]
```

Opens the window with the circuit and, after three seconds of warm-up, measures three phases of
`--bench-seconds` each: the simulation running with the view still, with the view panning, and
with it zooming in and out. For each it prints the frames per second, the time between frames
(median, 95th percentile, worst), the time spent drawing a frame, the time the session took to
catch up with the solver's thread, and the frame rate the drawing time leaves room for. Gaps of
more than a quarter of a second between frames (a window the compositor is not showing yet)
are counted apart and left out.

The frame rate is capped by the display (vertical sync), so look at the "room for" column to see
what the program could do. One run, 100 Hz display, GPU canvas (X11 and Wayland):

| circuit | elements | drawing avg | room for | at 100 Hz |
|---------|---------:|------------:|---------:|----------:|
| `ladder` | 500 | 5.6 ms | 170 to 190 fps | 95 to 99 fps |
| `mixed` | 1100 | 9.7 ms | 100 to 108 fps | 53 to 66 fps (a frame that takes more than 10 ms misses a vertical sync) |

SC-003 (60 frames a second for up to 500 elements) is met with a wide margin; 1100 elements still
draw in under 16.7 ms, so a 60 Hz display is kept at 60.

## Start-up and input latency (SC-005, SC-006)

The same command also prints two lines the others do not:

- `startup`: milliseconds from the start of the program to the first frame drawn.
- `input`: after the three phases, a fourth one clicks on bare canvas once per frame and times
  each click until the frame drawn after it (the click is handled, the frame is drawn; the
  wait for the display's next vertical sync and the compositor are not included).

| circuit | start-up | click to frame (p50 / worst) |
|---------|---------:|-----------------------------:|
| `ladder` (500 elements) | 0.93 to 1.0 s | 6 ms / 7 ms |
| `mixed` (1100 elements, the largest) | 0.93 to 1.0 s | 10 ms / 11 ms |

Start-up was 2.8 s before the keyboard shortcuts were moved off the application: setting an
accelerator with `set_accels_for_action` makes GTK refresh every menu item, 57 times over, and
that is also what froze the frame clock for about 2.3 s in the drawing benchmark. They are now
one shortcut controller on the window, with the accelerator shown in the menu through the `accel`
item attribute.
