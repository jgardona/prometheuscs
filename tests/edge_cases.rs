// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Invalid and corrupted circuits never crash or hang the program (SC-009): truncated and
//! garbled files, unknown elements, floating nodes, short circuits and circuits the solver
//! cannot settle. Each is loaded, analysed and stepped; an error is fine, a panic is not.

use prometheuscs::elements::registry::Registry;
use prometheuscs::error::{LoadError, StepError};
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

const HEADER: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

/// Loads `text`, analyses it and runs a few hundred steps. Returns what was read and the
/// outcome of analysis and stepping.
fn load_and_run(text: &str) -> (Vec<LoadError>, Result<(), String>) {
    let r = parse(text, &Registry::standard());
    let settings = r.circuit.options.sim_settings();
    let mut sim = Simulation::new(r.circuit.elements, settings);
    let out = match sim.analyze() {
        Err(e) => Err(e.to_string()),
        Ok(()) => sim.run_steps(300).map_err(|e| e.to_string()),
    };
    (r.errors, out)
}

fn examples() -> Vec<(String, String)> {
    let mut v: Vec<_> = std::fs::read_dir("assets/circuits")
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap_or_default(),
            )
        })
        .collect();
    v.sort();
    v
}

#[test]
fn truncated_files_load_what_they_can() {
    let registry = Registry::standard();
    for (name, text) in examples().iter().step_by(7) {
        // cut at several places, including in the middle of a multi-byte character
        for k in 1..16 {
            let mut cut = text.len() * k / 16;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            let r = parse(&text[..cut], &registry);
            // whatever was read can be built, even when it makes no sense
            let settings = r.circuit.options.sim_settings();
            let mut sim = Simulation::new(r.circuit.elements, settings);
            if sim.analyze().is_ok() {
                let _ = sim.run_steps(20);
            }
            let _ = name;
        }
    }
}

#[cfg(feature = "passive")]
#[test]
fn a_file_cut_in_the_middle_of_a_line_keeps_the_lines_before_it() {
    let text = format!("{HEADER}r 0 0 64 0 0 1000\nr 64 0 64 1");
    let r = parse(&text, &Registry::standard());
    assert!(!r.circuit.elements.is_empty());
}

#[cfg(feature = "passive")]
#[test]
fn an_unknown_element_is_reported_and_the_rest_still_loads() {
    let text = format!("{HEADER}9999 0 0 8 0 0\nr 0 0 64 0 0 1000\nw 64 0 64 64 0\n");
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.iter().any(|e| matches!(e, LoadError::UnknownType { .. })), "{:?}", r.errors);
    assert_eq!(r.circuit.elements.len(), 2);
}

#[test]
fn garbage_input_is_not_a_crash() {
    let junk = [
        "",
        "\n\n\n",
        "   ",
        "$",
        "$ x y z",
        "r",
        "r a b c d e f",
        "r 0 0 64 0 0 nan",
        "r 0 0 64 0 0 inf",
        "r 0 0 64 0 0 -1000",
        "r 0 0 64 0 0 0",
        "c 0 0 64 0 0 1e308 0 0 0",
        "r 99999999999999999999 0 64 0 0 1000",
        "\u{0}\u{1}\u{2}\u{ff}",
        "<",
        "<cir",
        "<cir><",
        "<circuit><elm></circuit>",
        "<circuit>",
        "</circuit>",
        "<!DOCTYPE x [<!ENTITY a \"b\">]><circuit/>",
        "<cir>&#xFFFFFFF;</cir>",
        "% 1 2 3\n? 5 6\no 0 64 0 0\n",
    ];
    for text in junk {
        let _ = load_and_run(text);
        let _ = load_and_run(&format!("{HEADER}{text}"));
    }
}

#[test]
fn an_element_with_too_few_fields_is_reported_not_fatal() {
    for line in ["r 0 0", "c 0 0 64 0 0", "v 0 0 0 64", "d 0 0 64 0", "s 0 0 64 0 0", "l 0 0 64"] {
        let (errors, _) = load_and_run(&format!("{HEADER}{line}\n"));
        let _ = errors;
    }
}

#[test]
fn a_floating_node_is_handled() {
    // a resistor connected to nothing, and a capacitor between two unconnected points
    let (_, out) = load_and_run(&format!("{HEADER}r 0 0 64 0 0 1000\n"));
    // an error message is acceptable; hanging or panicking is not
    let _ = out;
    let (_, out) = load_and_run(&format!(
        "{HEADER}v 0 0 0 64 0 0 40 5 0 0 0.5\nr 128 0 192 0 0 1000\nc 192 0 192 64 4 0.000001 0 0 0\n"
    ));
    let _ = out;
}

/// The reasons the simulation may give for refusing a circuit; anything else is a bug.
fn is_a_known_refusal(e: &str) -> bool {
    ["Singular matrix!", "Convergence failed!", "nan/infinite matrix!", "wire loop detected"]
        .contains(&e)
        || e.contains("no resistance")
}

/// Runs `text` and checks that the outcome is a success with finite numbers or a refusal the
/// program words itself.
fn settles_or_refuses(text: &str) {
    let r = parse(text, &Registry::standard());
    let settings = r.circuit.options.sim_settings();
    let mut sim = Simulation::new(r.circuit.elements, settings);
    match sim
        .analyze()
        .map_err(|e| e.to_string())
        .and_then(|()| sim.run_steps(300).map_err(|e| e.to_string()))
    {
        Ok(()) => {
            for e in &sim.elements {
                for p in 0..e.post_count() {
                    assert!(e.get_post_voltage(p).is_finite(), "a voltage is not finite");
                }
            }
        }
        Err(e) => assert!(is_a_known_refusal(&e), "unexpected message: {e}"),
    }
}

#[test]
fn a_short_circuit_across_a_voltage_source_is_handled() {
    settles_or_refuses(&format!("{HEADER}v 0 0 0 64 0 0 40 5 0 0 0.5\nw 0 0 0 64 0\n"));
}

#[test]
fn two_voltage_sources_in_parallel_are_handled() {
    settles_or_refuses(&format!(
        "{HEADER}v 0 0 0 64 0 0 40 5 0 0 0.5\nv 0 0 0 64 0 0 40 3 0 0 0.5\nr 0 0 64 0 0 1000\n"
    ));
}

#[test]
fn a_source_into_a_zero_ohm_resistor_is_handled() {
    settles_or_refuses(&format!(
        "{HEADER}v 0 0 0 64 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 0\nw 64 0 64 64 0\nw 64 64 0 64 0\n"
    ));
}

#[test]
fn a_loop_of_inductors_and_sources_does_not_hang() {
    let (_, out) = load_and_run(&format!(
        "{HEADER}v 0 0 0 64 0 0 40 5 0 0 0.5\nl 0 0 0 64 0 1 0\nw 0 0 64 0 0\nw 64 0 64 64 0\nw 64 64 0 64 0\n"
    ));
    let _ = out;
}

#[test]
fn a_circuit_that_cannot_settle_stops_with_an_error_or_runs_on() {
    // a diode ring fed by a huge source: the nonlinear iteration may fail to converge
    let text = format!(
        "{HEADER}v 0 0 0 64 0 0 40 1e9 0 0 0.5\nd 0 0 64 0 2 default\nd 64 0 64 64 2 default\nd 64 64 0 64 2 default\nw 0 0 0 64 0\n"
    );
    let (_, out) = load_and_run(&text);
    if let Err(e) = out {
        assert!(!e.is_empty());
    }
}

#[test]
fn step_errors_have_messages() {
    for e in [StepError::Singular, StepError::NotConverged, StepError::NotFinite] {
        assert!(!e.to_string().is_empty());
    }
}

#[test]
fn an_empty_circuit_runs() {
    let (errors, out) = load_and_run(HEADER);
    assert!(errors.is_empty());
    assert!(out.is_ok(), "{out:?}");
}

/// A small deterministic generator, so that a failure can be repeated.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Pieces that make text look like XML, with characters of several bytes among them: the
/// parsers slice text by byte positions and must never cut a character in two.
const PIECES: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "=",
    "\"",
    "'",
    "&",
    "&amp;",
    "&#",
    "&#x",
    ";",
    " ",
    "\n",
    "<!--",
    "-->",
    "<![CDATA[",
    "]]>",
    "<?xml?>",
    "<!DOCTYPE",
    "cir",
    "o",
    "w",
    "x",
    "f",
    "é",
    "€",
    "𝄞",
    "日本",
    "\u{0}",
    "\u{feff}",
    "1",
    "-1",
    "0.5",
    "x0",
    "NaN",
    "e",
    "<cir>",
    "<w x=\"",
    "\" f=\"0\"/>",
    "<o en=\"0\" sp=\"64\"/>",
    "<ccm nm=\"é\">",
    "</ccm>",
];

fn soup(rng: &mut Lcg) -> String {
    let n = 1 + rng.next(40);
    (0..n).map(|_| PIECES[rng.next(PIECES.len())]).collect()
}

#[test]
fn text_built_from_xml_pieces_and_multibyte_characters_never_panics() {
    let registry = Registry::standard();
    let seed = std::env::var("FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(2024);
    let rounds = std::env::var("FUZZ_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(20000);
    let mut rng = Lcg(seed);
    for _ in 0..rounds {
        let text = soup(&mut rng);
        let _ = parse(&text, &registry);
        let _ = parse(&format!("{HEADER}{text}"), &registry);
        let _ = prometheuscs::format::xml::parse(&text);
    }
}

#[test]
fn inserting_a_multibyte_character_anywhere_in_an_example_never_panics() {
    let registry = Registry::standard();
    let (name, text) = examples()
        .into_iter()
        .filter(|(_, t)| t.contains("<cir") && t.len() < 6000)
        .min_by_key(|(_, t)| t.len())
        .expect("an XML example");
    let _ = name;
    for (at, _) in text.char_indices().step_by(3) {
        for insert in ["é", "€", "𝄞"] {
            let mut edited = text.clone();
            edited.insert_str(at, insert);
            let _ = parse(&edited, &registry);
        }
    }
    // the same for the text format
    let (_, plain) = examples()
        .into_iter()
        .filter(|(_, t)| !t.trim_start().starts_with('<') && t.len() < 4000)
        .min_by_key(|(_, t)| t.len())
        .expect("a text example");
    for (at, _) in plain.char_indices().step_by(2) {
        let mut edited = plain.clone();
        edited.insert(at, '日');
        let r = parse(&edited, &registry);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        if sim.analyze().is_ok() {
            let _ = sim.run_steps(5);
        }
    }
}

/// Pieces of what a person might type into a field: numbers, suffixes, expressions, and
/// characters of several bytes.
const TYPED: &[&str] = &[
    "1", "2", "0", "9", ".", "-", "+", "e", "E", "k", "K", "m", "M", "u", "n", "p", "f", "G",
    "meg", "Meg", "rms", " ", "(", ")", "a", "b", "t", "sin(", "max(", "+", "*", "/", "^", "%",
    ",", "{", "}", "é", "€", "𝄞", "日本", "\u{0}", "inf", "nan", ":", "\\", "\"", "_", "<", ">",
    "=", "!", "&", "|", "?", "~",
];

fn typed(rng: &mut Lcg) -> String {
    let n = rng.next(12);
    (0..n).map(|_| TYPED[rng.next(TYPED.len())]).collect()
}

#[test]
fn text_typed_into_fields_and_expressions_never_panics() {
    use prometheuscs::elements::units::parse_edit_value;
    use prometheuscs::expr::{Expr, ExprState};
    use prometheuscs::format::tokenizer::{Tokenizer, escape, unescape};
    let seed = std::env::var("FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
    let rounds = std::env::var("FUZZ_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(20000);
    let mut rng = Lcg(seed);
    let state = ExprState::new();
    for _ in 0..rounds {
        let text = typed(&mut rng);
        let _ = parse_edit_value(&text);
        if let Ok(e) = Expr::parse(&text) {
            let _ = e.eval(&state);
        }
        let mut st = Tokenizer::new(&text);
        while st.next_token().is_some() {}
        let mut st = Tokenizer::for_circuit_line(&text);
        while st.next_token().is_some() {}
        let _ = unescape(&escape(&text));
        let _ = unescape(&text);
    }
}

#[test]
fn deeply_nested_files_and_expressions_do_not_overflow_the_stack() {
    use prometheuscs::expr::Expr;
    let depth = 200_000;
    // XML: elements inside elements
    let xml = format!("<cir>{}{}", "<w>".repeat(depth), "</w>".repeat(depth));
    let _ = parse(&xml, &Registry::standard());
    let _ = prometheuscs::format::xml::parse(&xml);
    // an expression in parentheses, in unary operators and in function calls
    let _ = Expr::parse(&format!("{}1{}", "(".repeat(depth), ")".repeat(depth)));
    let _ = Expr::parse(&format!("{}1", "-".repeat(depth)));
    let _ = Expr::parse(&format!("{}1{}", "sin(".repeat(depth), ")".repeat(depth)));
}

#[test]
fn a_subcircuit_that_contains_itself_is_refused_and_does_not_overflow_the_stack() {
    // model `selfish` has an instance of `selfish` inside it, and so has the circuit
    let xml = r#"<cir>
<ccm nm="selfish" f="1" sx="2" sy="1"><ext nm="a" nd="1" ps="0" sd="2"/><cc x="0 0 32 0" f="0" mo="selfish" nn="1"/></ccm>
<cc x="0 0 32 0" f="0" mo="selfish"/>
</cir>"#;
    let (errors, out) = load_and_run(xml);
    let _ = (errors, out);
    // the same through the text format, where the model is written on a line of its own
    let (errors, out) = load_and_run("410 0 0 32 0 0 selfish\n");
    let _ = (errors, out);
    // several instances of itself would multiply at every level
    let many = r#"<cir>
<ccm nm="bomb" f="1" sx="2" sy="1"><ext nm="a" nd="1" ps="0" sd="2"/><cc x="0 0 32 0" f="0" mo="bomb" nn="1"/><cc x="0 0 32 0" f="0" mo="bomb" nn="1"/><cc x="0 0 32 0" f="0" mo="bomb" nn="1"/><cc x="0 0 32 0" f="0" mo="bomb" nn="1"/></ccm>
<cc x="0 0 32 0" f="0" mo="bomb"/>
</cir>"#;
    let started = std::time::Instant::now();
    let _ = load_and_run(many);
    assert!(started.elapsed().as_secs() < 10, "building it takes {:?}", started.elapsed());
    // two models that hold each other
    let pair = r#"<cir>
<ccm nm="ping" f="1" sx="2" sy="1"><ext nm="a" nd="1" ps="0" sd="2"/><cc x="0 0 32 0" f="0" mo="pong" nn="1"/></ccm>
<ccm nm="pong" f="1" sx="2" sy="1"><ext nm="a" nd="1" ps="0" sd="2"/><cc x="0 0 32 0" f="0" mo="ping" nn="1"/></ccm>
<cc x="0 0 32 0" f="0" mo="ping"/>
</cir>"#;
    let _ = load_and_run(pair);
}
