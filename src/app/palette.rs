// SPDX-License-Identifier: GPL-2.0-or-later
//! The model of the Find Component palette: what it lists, in which groups, and how the typed
//! text filters it (contract "Find Component result groups"). It needs no display, so the
//! window only draws what [`Palette::filter`] returns.

use super::commands::{
    Cmd, Node, draw_menu, edit_menu, file_menu, options_menu, scopes_menu, subcircuit_items,
    tools_menu,
};
use crate::elements::registry::Registry;
use crate::locale::ls;

/// How many recent picks the palette remembers.
pub const MAX_RECENT: usize = 10;

/// The groups of the palette, in the order they are shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The last picks, newest first.
    Recent,
    /// Every element the Draw menu places, subcircuits included.
    Components,
    /// Every other menu command.
    Commands,
    /// The elements already in the circuit.
    Elements,
}

impl Group {
    /// The English title of the group (translated with `ls`).
    pub fn title(self) -> &'static str {
        match self {
            Group::Recent => "Recent",
            Group::Components => "Components",
            Group::Commands => "Commands",
            Group::Elements => "Elements in this circuit",
        }
    }
}

/// What choosing an entry does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Run a command (placing a component is the command [`Cmd::Place`]).
    Run(Cmd),
    /// Select the circuit element with this index and centre the view on it.
    Focus(usize),
}

impl Pick {
    /// The name a pick is remembered by in the recent list; element picks are not remembered,
    /// since their indices change as the circuit is edited.
    pub fn key(&self) -> Option<String> {
        match self {
            Pick::Run(cmd) => Some(match cmd.target() {
                Some(t) => format!("{}:{t}", cmd.action_name()),
                None => cmd.action_name(),
            }),
            Pick::Focus(_) => None,
        }
    }
}

/// One line of the palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The group the line is shown under.
    pub group: Group,
    /// The text shown, already translated.
    pub label: String,
    /// What choosing it does.
    pub pick: Pick,
}

/// Everything the palette can list, before filtering.
#[derive(Clone, Debug, Default)]
pub struct Palette {
    components: Vec<Entry>,
    commands: Vec<Entry>,
    elements: Vec<Entry>,
}

impl Palette {
    /// A palette over these entries; their groups are set from the argument they come in.
    pub fn new(
        components: Vec<(String, Cmd)>,
        commands: Vec<(String, Cmd)>,
        elements: Vec<(usize, String)>,
    ) -> Palette {
        let run =
            |group| move |(label, cmd): (String, Cmd)| Entry { group, label, pick: Pick::Run(cmd) };
        Palette {
            components: components.into_iter().map(run(Group::Components)).collect(),
            commands: commands.into_iter().map(run(Group::Commands)).collect(),
            elements: elements
                .into_iter()
                .map(|(i, label)| Entry { group: Group::Elements, label, pick: Pick::Focus(i) })
                .collect(),
        }
    }

    /// The palette of the menus: components from the Draw menu and the subcircuits made so
    /// far, commands from every other menu (and the Draw menu's tools), and the circuit's
    /// elements as `(index, label)`.
    pub fn from_menus(
        registry: &Registry,
        subcircuits: &[String],
        elements: Vec<(usize, String)>,
    ) -> Palette {
        fn walk(nodes: &[Node], out: &mut Vec<(String, Cmd)>) {
            for n in nodes {
                match n {
                    Node::Item(cmd, label, ..) => out.push((ls(label), cmd.clone())),
                    Node::Check(t, label) => out.push((ls(label), Cmd::Toggle(*t))),
                    Node::Section(v) | Node::Sub(_, v) => walk(v, out),
                }
            }
        }
        let mut drawn = Vec::new();
        walk(&draw_menu(registry), &mut drawn);
        walk(&subcircuit_items(subcircuits), &mut drawn);
        let (components, mut commands): (Vec<_>, Vec<_>) = drawn
            .into_iter()
            .partition(|(_, c)| matches!(c, Cmd::Place(_) | Cmd::PlaceSubcircuit(_)));
        let mut menus = Vec::new();
        for m in [file_menu(), edit_menu(), scopes_menu(), options_menu(), tools_menu()] {
            walk(&m, &mut menus);
        }
        // the palette itself is not something to find in it
        menus.retain(|(_, c)| *c != Cmd::Find && !c.unavailable());
        menus.append(&mut commands);
        Palette::new(components, menus, elements)
    }

    /// The entries whose label contains `query` (ignoring case), grouped in the order of
    /// [`Group`]. Recent picks come first, newest first, and are not repeated in their own group.
    pub fn filter(&self, query: &str, recent: &[String]) -> Vec<Entry> {
        let q = query.to_lowercase();
        let hit = |e: &&Entry| e.label.to_lowercase().contains(&q);
        let mut out: Vec<Entry> = recent
            .iter()
            .filter_map(|key| {
                self.components
                    .iter()
                    .chain(&self.commands)
                    .find(|e| e.pick.key().as_deref() == Some(key.as_str()))
            })
            .filter(hit)
            .map(|e| Entry { group: Group::Recent, ..e.clone() })
            .collect();
        let shown = |e: &&Entry| e.pick.key().is_none_or(|k| !recent.contains(&k));
        for list in [&self.components, &self.commands, &self.elements] {
            out.extend(list.iter().filter(hit).filter(shown).cloned());
        }
        out
    }
}

/// Puts `key` at the front of the recent list, without duplicates, keeping at most
/// [`MAX_RECENT`] entries.
pub fn record(recent: &mut Vec<String>, key: String) {
    recent.retain(|k| *k != key);
    recent.insert(0, key);
    recent.truncate(MAX_RECENT);
}

/// How close together two Shift presses must be to open the palette.
pub const DOUBLE_SHIFT: std::time::Duration = std::time::Duration::from_millis(400);

/// Recognises a double Shift press: two Shift presses within [`DOUBLE_SHIFT`] with no other
/// key between them (so typing capitals never opens the palette).
#[derive(Clone, Copy, Debug, Default)]
pub struct DoubleShift {
    last: Option<std::time::Instant>,
}

impl DoubleShift {
    /// Takes one key press; true when it completes a double Shift.
    pub fn press(&mut self, is_shift: bool, now: std::time::Instant) -> bool {
        if !is_shift {
            self.last = None;
            return false;
        }
        match self.last.take() {
            Some(t) if now.duration_since(t) <= DOUBLE_SHIFT => true,
            _ => {
                self.last = Some(now);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Palette {
        Palette::new(
            vec![
                ("Add Resistor".into(), Cmd::Place("ResistorElm".into())),
                ("Add Capacitor".into(), Cmd::Place("CapacitorElm".into())),
            ],
            vec![("Undo".into(), Cmd::Undo), ("Rotate CW".into(), Cmd::RotateCw)],
            vec![(0, "Resistor 1k (16, 32)".into()), (1, "Wire (0, 0)".into())],
        )
    }

    #[test]
    fn filter_is_a_case_insensitive_substring_match() {
        let found = sample().filter("RESIS", &[]);
        let labels: Vec<&str> = found.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["Add Resistor", "Resistor 1k (16, 32)"]);
    }

    #[test]
    fn groups_come_in_the_contract_order() {
        let recent = vec!["rotate-cw".to_string()];
        let groups: Vec<Group> = sample().filter("", &recent).iter().map(|e| e.group).collect();
        assert_eq!(
            groups,
            [
                Group::Recent,
                Group::Components,
                Group::Components,
                Group::Commands,
                Group::Elements,
                Group::Elements,
            ]
        );
    }

    #[test]
    fn recent_picks_are_listed_newest_first_and_not_repeated() {
        let recent = vec!["place:CapacitorElm".to_string(), "undo".to_string()];
        let found = sample().filter("", &recent);
        assert_eq!(found[0].label, "Add Capacitor");
        assert_eq!(found[1].label, "Undo");
        assert_eq!(found.iter().filter(|e| e.label == "Undo").count(), 1);
        // a recent pick that does not match the text is not shown
        assert!(sample().filter("resis", &recent).iter().all(|e| e.group != Group::Recent));
    }

    #[test]
    fn recent_list_keeps_ten_newest_first_without_duplicates() {
        let mut recent = Vec::new();
        for i in 0..12 {
            record(&mut recent, format!("k{i}"));
        }
        assert_eq!(recent.len(), MAX_RECENT);
        assert_eq!(recent[0], "k11");
        record(&mut recent, "k5".into());
        assert_eq!(recent[0], "k5");
        assert_eq!(recent.iter().filter(|k| *k == "k5").count(), 1);
        assert_eq!(recent.len(), MAX_RECENT);
    }

    #[test]
    fn no_match_gives_an_empty_list() {
        assert!(sample().filter("zzz", &["undo".into()]).is_empty());
    }

    #[test]
    fn element_picks_are_not_remembered() {
        assert_eq!(Pick::Focus(3).key(), None);
        assert_eq!(Pick::Run(Cmd::Place("ResistorElm".into())).key().unwrap(), "place:ResistorElm");
    }

    #[test]
    fn double_shift_needs_two_quick_presses_with_nothing_between() {
        use std::time::{Duration, Instant};
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut d = DoubleShift::default();
        assert!(!d.press(true, t0));
        assert!(d.press(true, ms(300)));
        // a third press starts over
        assert!(!d.press(true, ms(350)));
        // too slow
        let mut d = DoubleShift::default();
        assert!(!d.press(true, t0));
        assert!(!d.press(true, ms(500)));
        assert!(d.press(true, ms(600)));
        // another key between the two
        let mut d = DoubleShift::default();
        assert!(!d.press(true, t0));
        assert!(!d.press(false, ms(100)));
        assert!(!d.press(true, ms(200)));
    }

    #[test]
    fn the_menus_give_components_and_commands() {
        let p = Palette::from_menus(&Registry::standard(), &["amp".into()], Vec::new());
        let all = p.filter("", &[]);
        let has = |g: Group, cmd: &Cmd| {
            all.iter().any(|e| e.group == g && e.pick == Pick::Run(cmd.clone()))
        };
        assert!(has(Group::Components, &Cmd::Place("ResistorElm".into())));
        assert!(has(Group::Components, &Cmd::PlaceSubcircuit("amp".into())));
        assert!(has(Group::Commands, &Cmd::Undo));
        assert!(has(Group::Commands, &Cmd::SwapTerminals));
        assert!(!all.iter().any(|e| e.pick == Pick::Run(Cmd::Find)));
    }
}
