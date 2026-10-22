// SPDX-License-Identifier: GPL-2.0-or-later
//! The checks a document lists (`verificacoes`): a quantity of the example at some time, compared
//! with the value the document states.

use crate::elements::registry::Registry;

/// What is read from the element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    /// The voltage across the element.
    Vd,
    /// The current through it.
    I,
    /// The voltage of terminal `k`.
    V(usize),
}

/// How the quantity is reduced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    /// The value at `t`.
    Value,
    /// The largest value of the window.
    Max,
    /// The smallest value of the window.
    Min,
    /// The root mean square over the window.
    Rms,
    /// The mean over the window.
    Mean,
    /// The frequency of the signal in the window.
    Freq,
}

/// How close the result has to be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tolerance {
    /// A fraction of the expected value.
    Relative(f64),
    /// An amount, in the unit of the quantity.
    Absolute(f64),
}

/// One check of a document.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    /// Simulated time at which the value is read (the end of the window).
    pub t: f64,
    /// Start of the window, for the functions that need one.
    pub from: Option<f64>,
    /// Position of the element in the file.
    pub element: usize,
    /// Class the element must have (a guard against a renumbering).
    pub class: String,
    /// What is read.
    pub quantity: Quantity,
    /// How it is reduced.
    pub func: Func,
    /// The value the document states.
    pub expected: f64,
    /// How close the result has to be.
    pub tolerance: Tolerance,
    /// Elements (switches, logic inputs) flipped once, each at its simulated time, in time order.
    pub toggles: Vec<(usize, f64)>,
}

/// What running a check gave.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The value is what the document states.
    Agrees,
    /// The example does not load without errors in this build (a family is left out).
    Skipped,
}

/// Reads `3,4@0.1` (element, optionally `@` the time in seconds; 0 when there is none), sorted by
/// time.
pub fn parse_toggles(list: &str) -> Result<Vec<(usize, f64)>, String> {
    let mut out = Vec::new();
    for item in list.split(',') {
        let (n, at) = item.split_once('@').unwrap_or((item, "0"));
        let n = n
            .parse::<usize>()
            .map_err(|_| format!("`alterna`: `{n}` não é um número de elemento"))?;
        let at = at.parse::<f64>().map_err(|_| format!("`alterna`: `{at}` não é um tempo"))?;
        out.push((n, at));
    }
    out.sort_by(|a, b| a.1.total_cmp(&b.1));
    Ok(out)
}

/// Flips, in time order, the elements whose time has come, analysing the circuit again after each.
fn apply_due(
    sim: &mut crate::solver::sim::Simulation,
    pending: &mut std::collections::VecDeque<(usize, f64)>,
    until: f64,
) -> Result<(), String> {
    let stop = |e: crate::error::StepError| format!("a simulação parou: {e}");
    while let Some(&(n, at)) = pending.front() {
        if at > until {
            break;
        }
        pending.pop_front();
        if at > sim.time() {
            sim.run_until(at).map_err(stop)?;
        }
        let Some(e) = sim.elements.get_mut(n) else {
            return Err(format!("o exemplo não tem o elemento {n} (alterna)"));
        };
        e.simple_toggle();
        sim.analyze().map_err(|e| format!("o exemplo não pôde ser analisado: {e}"))?;
    }
    Ok(())
}

/// Reads a check from its line: `t=0.02 elemento=2:CapacitorElm grandeza=vd fn=rms de=0.01
/// esperado=0.707 tol=0.03`; `alterna=3,4@0.1` flips element 3 before the simulation and
/// element 4 at 0.1 s (switches, logic inputs).
pub fn parse_check(line: &str) -> Result<Check, String> {
    let number = |key: &str, v: &str| -> Result<f64, String> {
        v.parse::<f64>().map_err(|_| format!("`{key}` não é um número: `{v}`"))
    };
    let (mut t, mut from, mut element, mut quantity) = (None, None, None, None);
    let (mut func, mut expected, mut rel, mut abs) = (Func::Value, None, None, None);
    let mut toggles = Vec::new();
    for token in line.split_whitespace() {
        let Some((key, v)) = token.split_once('=') else {
            return Err(format!("`{token}` não é `chave=valor`"));
        };
        match key {
            "t" => t = Some(number(key, v)?),
            "de" => from = Some(number(key, v)?),
            "esperado" => expected = Some(number(key, v)?),
            "tol" => rel = Some(number(key, v)?),
            "abs" => abs = Some(number(key, v)?),
            "alterna" => toggles = parse_toggles(v)?,
            "elemento" => {
                let (n, class) = v
                    .split_once(':')
                    .ok_or_else(|| format!("`elemento` deve ser `N:Tipo`, não `{v}`"))?;
                let n =
                    n.parse::<usize>().map_err(|_| format!("`elemento`: `{n}` não é um número"))?;
                element = Some((n, class.to_string()));
            }
            "grandeza" => {
                quantity = Some(match v {
                    "vd" => Quantity::Vd,
                    "i" => Quantity::I,
                    _ => match v.strip_prefix('v').and_then(|k| k.parse::<usize>().ok()) {
                        Some(k) => Quantity::V(k),
                        None => {
                            return Err(format!("`grandeza` desconhecida: `{v}` (vd, i, v<k>)"));
                        }
                    },
                });
            }
            "fn" => {
                func = match v {
                    "valor" => Func::Value,
                    "max" => Func::Max,
                    "min" => Func::Min,
                    "rms" => Func::Rms,
                    "media" => Func::Mean,
                    "freq" => Func::Freq,
                    _ => {
                        return Err(format!(
                            "`fn` desconhecida: `{v}` (valor, max, min, rms, media, freq)"
                        ));
                    }
                };
            }
            other => return Err(format!("chave desconhecida `{other}`")),
        }
    }
    let t = t.ok_or("falta `t`")?;
    let (element, class) = element.ok_or("falta `elemento` (N:Tipo)")?;
    let quantity = quantity.ok_or("falta `grandeza`")?;
    let expected = expected.ok_or("falta `esperado`")?;
    if func != Func::Value && from.is_none() {
        return Err("a função sobre uma janela precisa de `de` (início da janela)".into());
    }
    let tolerance = match (rel, abs) {
        (Some(_), Some(_)) => return Err("use `tol` ou `abs`, não os dois".into()),
        (Some(r), None) => Tolerance::Relative(r),
        (None, Some(a)) => Tolerance::Absolute(a),
        (None, None) => Tolerance::Relative(0.02),
    };
    Ok(Check { t, from, element, class, quantity, func, expected, tolerance, toggles })
}

/// Runs the check on the example and compares.
pub fn run_check(
    example_text: &str,
    check: &Check,
    registry: &Registry,
) -> Result<Outcome, String> {
    let parsed = crate::format::parser::parse(example_text, registry);
    if !parsed.errors.is_empty() {
        return Ok(Outcome::Skipped);
    }
    let settings = parsed.circuit.options.sim_settings();
    let mut sim = crate::solver::sim::Simulation::new(parsed.circuit.elements, settings);
    let Some(element) = sim.elements.get(check.element) else {
        return Err(format!("o exemplo não tem o elemento {}", check.element));
    };
    if element.class_name() != check.class {
        return Err(format!(
            "o elemento {} é {}, não {}",
            check.element,
            element.class_name(),
            check.class
        ));
    }
    sim.analyze().map_err(|e| format!("o exemplo não pôde ser analisado: {e}"))?;
    let mut pending: std::collections::VecDeque<_> = check.toggles.iter().copied().collect();
    apply_due(&mut sim, &mut pending, 0.0)?;
    let read = |sim: &crate::solver::sim::Simulation| {
        let e = &sim.elements[check.element];
        match check.quantity {
            Quantity::Vd => e.get_voltage_diff(),
            Quantity::I => e.get_current(),
            Quantity::V(k) => e.get_post_voltage(k),
        }
    };
    let stop = |e: crate::error::StepError| format!("a simulação parou: {e}");
    let got = if check.func == Func::Value {
        apply_due(&mut sim, &mut pending, check.t)?;
        sim.run_until(check.t).map_err(stop)?;
        read(&sim)
    } else {
        let from = check.from.unwrap_or(0.0);
        apply_due(&mut sim, &mut pending, from)?;
        sim.run_until(from).map_err(stop)?;
        let mut samples: Vec<(f64, f64)> = Vec::new();
        while sim.time() < check.t - sim.time_step() * 1e-9 {
            let now = sim.time();
            apply_due(&mut sim, &mut pending, now)?;
            sim.run_steps(1).map_err(stop)?;
            samples.push((sim.time(), read(&sim)));
        }
        reduce(check.func, &samples)?
    };
    let allowed = match check.tolerance {
        Tolerance::Relative(r) => (r * check.expected.abs()).max(1e-12),
        Tolerance::Absolute(a) => a,
    };
    if (got - check.expected).abs() <= allowed {
        Ok(Outcome::Agrees)
    } else {
        Err(format!(
            "o simulador dá {got:.6}, o documento diz {} (tolerância {allowed:.3e})",
            check.expected
        ))
    }
}

/// The samples `(time, value)` of one quantity of one element, one for each simulation step of the
/// window `from..to`, for the author of a document (to look at a waveform or take its spectrum).
pub fn series(
    example_text: &str,
    check: &Check,
    registry: &Registry,
) -> Result<Option<Vec<(f64, f64)>>, String> {
    let parsed = crate::format::parser::parse(example_text, registry);
    if !parsed.errors.is_empty() {
        return Ok(None);
    }
    let settings = parsed.circuit.options.sim_settings();
    let mut sim = crate::solver::sim::Simulation::new(parsed.circuit.elements, settings);
    if sim.elements.get(check.element).is_none_or(|e| e.class_name() != check.class) {
        return Err(format!("o exemplo não tem o elemento {}:{}", check.element, check.class));
    }
    sim.analyze().map_err(|e| format!("o exemplo não pôde ser analisado: {e}"))?;
    let mut pending: std::collections::VecDeque<_> = check.toggles.iter().copied().collect();
    let stop = |e: crate::error::StepError| format!("a simulação parou: {e}");
    let from = check.from.unwrap_or(0.0);
    apply_due(&mut sim, &mut pending, from)?;
    sim.run_until(from).map_err(stop)?;
    let mut out = Vec::new();
    while sim.time() < check.t - sim.time_step() * 1e-9 {
        let now = sim.time();
        apply_due(&mut sim, &mut pending, now)?;
        sim.run_steps(1).map_err(stop)?;
        let e = &sim.elements[check.element];
        out.push((
            sim.time(),
            match check.quantity {
                Quantity::Vd => e.get_voltage_diff(),
                Quantity::I => e.get_current(),
                Quantity::V(k) => e.get_post_voltage(k),
            },
        ));
    }
    Ok(Some(out))
}

/// The terminals of one element: `(index, class, [(x, y)…])`.
pub type ElementPosts = (usize, String, Vec<(i32, i32)>);

/// The position of every terminal of every element of the example, so that the author can
/// rebuild the netlist of a circuit whose elements place their terminals themselves
/// (transistors, chips).
pub fn posts(example_text: &str, registry: &Registry) -> Option<Vec<ElementPosts>> {
    let parsed = crate::format::parser::parse(example_text, registry);
    if !parsed.errors.is_empty() {
        return None;
    }
    Some(
        parsed
            .circuit
            .elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let pts =
                    (0..e.post_count()).filter_map(|k| e.get_post(k)).map(|p| (p.x, p.y)).collect();
                (i, e.class_name().to_string(), pts)
            })
            .collect(),
    )
}

/// What one element reads at a time, for the author of a document.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeRow {
    /// Position of the element in the file.
    pub index: usize,
    /// Class of the element.
    pub class: String,
    /// Voltage across it.
    pub vd: f64,
    /// Current through it.
    pub current: f64,
    /// Voltage of each of its terminals.
    pub posts: Vec<f64>,
}

/// Runs the example until `t` and reads every element (wires left out). `None` when the example
/// does not load without errors in this build.
pub fn probe(
    example_text: &str,
    t: f64,
    toggles: &[(usize, f64)],
    registry: &Registry,
) -> Result<Option<Vec<ProbeRow>>, String> {
    let parsed = crate::format::parser::parse(example_text, registry);
    if !parsed.errors.is_empty() {
        return Ok(None);
    }
    let settings = parsed.circuit.options.sim_settings();
    let mut sim = crate::solver::sim::Simulation::new(parsed.circuit.elements, settings);
    sim.analyze().map_err(|e| format!("o exemplo não pôde ser analisado: {e}"))?;
    let mut pending: std::collections::VecDeque<_> = toggles.iter().copied().collect();
    apply_due(&mut sim, &mut pending, t)?;
    sim.run_until(t).map_err(|e| format!("a simulação parou: {e}"))?;
    Ok(Some(
        sim.elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.class_name() != "WireElm")
            .map(|(index, e)| ProbeRow {
                index,
                class: e.class_name().to_string(),
                vd: e.get_voltage_diff(),
                current: e.get_current(),
                posts: (0..e.post_count()).map(|k| e.get_post_voltage(k)).collect(),
            })
            .collect(),
    ))
}

/// The value of `func` over the samples `(time, value)` of a window.
fn reduce(func: Func, samples: &[(f64, f64)]) -> Result<f64, String> {
    if samples.is_empty() {
        return Err("a janela não tem amostras (de >= t?)".into());
    }
    let n = samples.len() as f64;
    let mean = samples.iter().map(|s| s.1).sum::<f64>() / n;
    match func {
        Func::Value => Ok(samples[samples.len() - 1].1),
        Func::Max => Ok(samples.iter().map(|s| s.1).fold(f64::MIN, f64::max)),
        Func::Min => Ok(samples.iter().map(|s| s.1).fold(f64::MAX, f64::min)),
        Func::Rms => Ok((samples.iter().map(|s| s.1 * s.1).sum::<f64>() / n).sqrt()),
        Func::Mean => Ok(mean),
        Func::Freq => {
            let mut crossings = Vec::new();
            for w in samples.windows(2) {
                let (a, b) = (w[0].1 - mean, w[1].1 - mean);
                if a < 0.0 && b >= 0.0 {
                    crossings.push(w[0].0 + (-a) / (b - a) * (w[1].0 - w[0].0));
                }
            }
            if crossings.len() < 2 {
                return Err("a janela não tem ciclos suficientes para medir a frequência".into());
            }
            Ok((crossings.len() - 1) as f64 / (crossings[crossings.len() - 1] - crossings[0]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_line_is_read_with_its_defaults() {
        let c = parse_check(
            "t=0.02 elemento=2:CapacitorElm grandeza=vd fn=rms de=0.01 esperado=0.707 tol=0.03",
        )
        .unwrap();
        assert_eq!(
            (c.t, c.from, c.element, c.class.as_str()),
            (0.02, Some(0.01), 2, "CapacitorElm")
        );
        assert_eq!((c.quantity, c.func, c.expected), (Quantity::Vd, Func::Rms, 0.707));
        assert_eq!(c.tolerance, Tolerance::Relative(0.03));
        let d = parse_check("t=1 elemento=0:VoltageElm grandeza=v1 esperado=3.75").unwrap();
        assert_eq!(
            (d.quantity, d.func, d.from, d.tolerance),
            (Quantity::V(1), Func::Value, None, Tolerance::Relative(0.02))
        );
        let a = parse_check("t=1 elemento=0:VoltageElm grandeza=i esperado=0 abs=1e-6").unwrap();
        assert_eq!((a.quantity, a.tolerance), (Quantity::I, Tolerance::Absolute(1e-6)));
    }

    #[test]
    fn bad_check_lines_are_errors() {
        for (line, what) in [
            ("elemento=0:X grandeza=vd esperado=1", "t"),
            ("t=1 elemento=0:X grandeza=zz esperado=1", "grandeza"),
            ("t=1 elemento=0:X grandeza=vd", "esperado"),
            ("t=1 elemento=0 grandeza=vd esperado=1", "elemento"),
            ("t=1 elemento=0:X grandeza=vd fn=rms esperado=1", "de"),
            ("t=1 elemento=0:X grandeza=vd esperado=1 coisa=2", "coisa"),
            ("t=1 elemento=0:X grandeza=vd fn=xyz esperado=1", "fn"),
        ] {
            let e = parse_check(line).unwrap_err();
            assert!(e.contains(what), "{line}: {e}");
        }
    }

    #[test]
    fn the_toggle_list_is_read() {
        let c = parse_check("t=1 elemento=0:X grandeza=vd esperado=1 alterna=3,4").unwrap();
        assert_eq!(c.toggles, vec![(3, 0.0), (4, 0.0)]);
        let timed = parse_check("t=1 elemento=0:X grandeza=vd esperado=1 alterna=4@0.2,3").unwrap();
        assert_eq!(timed.toggles, vec![(3, 0.0), (4, 0.2)]);
        assert!(parse_check("t=1 elemento=0:X grandeza=vd esperado=1").unwrap().toggles.is_empty());
        let e = parse_check("t=1 elemento=0:X grandeza=vd esperado=1 alterna=a").unwrap_err();
        assert!(e.contains("alterna"), "{e}");
    }

    #[cfg(all(feature = "passive", feature = "switches", feature = "logic"))]
    #[test]
    fn a_check_can_flip_a_logic_input_before_it_runs() {
        let reg = Registry::standard();
        let circuit = "$ 1 0.000005 10 50 5 50 5e-11\nL 0 0 0 64 0 0 false 5 0\nr 0 0 64 0 0 1000\ng 64 0 64 16 0 0\n";
        let low =
            parse_check("t=0.001 elemento=1:ResistorElm grandeza=vd esperado=0 abs=1e-6").unwrap();
        assert_eq!(run_check(circuit, &low, &reg), Ok(Outcome::Agrees));
        let high =
            parse_check("t=0.001 elemento=1:ResistorElm grandeza=vd esperado=5 abs=1e-6 alterna=0")
                .unwrap();
        assert_eq!(run_check(circuit, &high, &reg), Ok(Outcome::Agrees));
        let none =
            parse_check("t=0.001 elemento=1:ResistorElm grandeza=vd esperado=0 alterna=9").unwrap();
        assert!(run_check(circuit, &none, &reg).unwrap_err().contains("elemento 9"));
    }

    #[cfg(all(feature = "passive", feature = "sources"))]
    #[test]
    fn a_series_has_a_sample_for_each_step_of_the_window() {
        let reg = Registry::standard();
        let c =
            parse_check("t=0.001 de=0.0005 elemento=1:ResistorElm grandeza=v1 esperado=0").unwrap();
        let samples = series(RC, &c, &reg).unwrap().unwrap();
        assert_eq!(samples.len(), 100, "{} samples", samples.len());
        assert!((samples[99].0 - 0.001).abs() < 1e-9);
        assert!(samples.iter().all(|(_, v)| (v - 3.75).abs() < 1e-6));
        let bad = parse_check("t=0.001 elemento=1:CapacitorElm grandeza=v1 esperado=0").unwrap();
        assert!(series(RC, &bad, &reg).is_err());
    }

    #[cfg(all(feature = "passive", feature = "sources"))]
    #[test]
    fn the_posts_of_every_element_are_listed() {
        let rows = posts(RC, &Registry::standard()).unwrap();
        let resistor = rows.iter().find(|(_, c, _)| c == "ResistorElm").unwrap();
        assert_eq!(resistor.2.len(), 2, "{resistor:?}");
        assert_eq!(resistor.2[0], (0, 0));
        assert!(
            posts("$ 1 0.000005 10 50 5 50 5e-11\nzzz 0 0 1 1 0\n", &Registry::standard())
                .is_none()
        );
    }

    #[cfg(all(feature = "passive", feature = "sources"))]
    const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 128 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nr 64 0 64 128 0 3000\nw 64 128 0 128 0\n";
    #[cfg(all(feature = "passive", feature = "sources"))]
    const AC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 128 0 0 0 1 100 5 0 0 0.5\nr 0 0 64 0 0 1000\nr 64 0 64 128 0 3000\nw 64 128 0 128 0\n";

    #[cfg(all(feature = "passive", feature = "sources"))]
    #[test]
    fn a_check_agrees_or_says_what_the_simulator_gave() {
        let reg = Registry::standard();
        let good = parse_check("t=0.001 elemento=1:ResistorElm grandeza=v1 esperado=3.75").unwrap();
        assert_eq!(run_check(RC, &good, &reg), Ok(Outcome::Agrees));
        let vd = parse_check("t=0.001 elemento=1:ResistorElm grandeza=vd esperado=1.25").unwrap();
        assert_eq!(run_check(RC, &vd, &reg), Ok(Outcome::Agrees));
        let i = parse_check("t=0.001 elemento=2:ResistorElm grandeza=i esperado=0.00125").unwrap();
        assert_eq!(run_check(RC, &i, &reg), Ok(Outcome::Agrees));
        let wrong = parse_check("t=0.001 elemento=1:ResistorElm grandeza=v1 esperado=4.5").unwrap();
        let e = run_check(RC, &wrong, &reg).unwrap_err();
        assert!(e.contains("3.75") && e.contains("4.5"), "{e}");
        let class =
            parse_check("t=0.001 elemento=1:CapacitorElm grandeza=v1 esperado=3.75").unwrap();
        assert!(run_check(RC, &class, &reg).unwrap_err().contains("ResistorElm"));
        let nope = parse_check("t=0.001 elemento=9:ResistorElm grandeza=v1 esperado=3.75").unwrap();
        assert!(run_check(RC, &nope, &reg).unwrap_err().contains('9'));
    }

    #[cfg(all(feature = "passive", feature = "sources"))]
    #[test]
    fn the_functions_over_a_window_read_a_sine() {
        let reg = Registry::standard();
        // 5 V peak sine at 100 Hz across the source; 1/4 of it across the first resistor...
        // the source is element 0, the 1 kohm resistor element 1 (1.25 V peak)
        for (line, ok) in [
            ("t=0.06 de=0.02 elemento=0:VoltageElm grandeza=vd fn=max esperado=5 tol=0.01", true),
            ("t=0.06 de=0.02 elemento=0:VoltageElm grandeza=vd fn=min esperado=-5 tol=0.01", true),
            (
                "t=0.06 de=0.02 elemento=0:VoltageElm grandeza=vd fn=rms esperado=3.5355 tol=0.01",
                true,
            ),
            ("t=0.06 de=0.02 elemento=0:VoltageElm grandeza=vd fn=media esperado=0 abs=0.05", true),
            (
                "t=0.06 de=0.02 elemento=1:ResistorElm grandeza=vd fn=freq esperado=100 tol=0.02",
                true,
            ),
            ("t=0.06 de=0.02 elemento=0:VoltageElm grandeza=vd fn=rms esperado=4 tol=0.01", false),
        ] {
            let c = parse_check(line).unwrap();
            assert_eq!(
                run_check(AC, &c, &reg).is_ok(),
                ok,
                "{line}: {:?}",
                run_check(AC, &c, &reg)
            );
        }
    }

    #[cfg(all(feature = "passive", feature = "sources"))]
    #[test]
    fn the_probe_reads_every_element_but_the_wires() {
        let rows = probe(RC, 0.001, &[], &Registry::standard()).unwrap().unwrap();
        assert_eq!(rows.iter().map(|r| r.index).collect::<Vec<_>>(), [0, 1, 2]);
        assert!((rows[1].vd.abs() - 1.25).abs() < 1e-6, "{rows:?}");
        assert!(
            (rows[2].posts[0] - 3.75).abs() < 1e-6 || (rows[2].posts[1] - 3.75).abs() < 1e-6,
            "{rows:?}"
        );
        assert!((rows[1].current.abs() - 0.00125).abs() < 1e-9, "{rows:?}");
        assert_eq!(
            probe(
                "$ 1 0.000005 10 50 5 50 5e-11\nzzz 0 0 1 1 0\n",
                0.001,
                &[],
                &Registry::standard()
            ),
            Ok(None)
        );
    }

    #[test]
    fn an_example_that_does_not_load_is_skipped() {
        let reg = Registry::standard();
        let c = parse_check("t=0.001 elemento=0:ResistorElm grandeza=vd esperado=1").unwrap();
        assert_eq!(
            run_check("$ 1 0.000005 10 50 5 50 5e-11\nzzz 0 0 1 1 0\n", &c, &reg),
            Ok(Outcome::Skipped)
        );
    }
}
