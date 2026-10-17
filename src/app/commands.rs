// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Menus, CommandManager).
//! The application's commands, the menus that expose them and their disabled states.
//!
//! Commands that only change the session are carried out by [`dispatch`], which needs no
//! display; commands that need a dialog, a file chooser or the clipboard come back as
//! [`Outcome::Ui`] for the window to handle (contract C6).

use super::edit::{Toggle, Tool, Transform};
use super::session::Session;
use super::settings::autosave_path;
use crate::elements::registry::Registry;

/// Everything a menu entry, toolbar button or shortcut can do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// Open another window.
    NewWindow,
    /// Start an empty circuit.
    NewCircuit,
    /// Choose a circuit file to open.
    Open,
    /// Import a circuit from text.
    ImportText,
    /// Save the circuit.
    Save,
    /// Save under a new name.
    SaveAs,
    /// Show a share link.
    ExportLink,
    /// Show the circuit as text.
    ExportText,
    /// Save a PNG picture.
    ExportImage,
    /// Copy a picture to the clipboard.
    CopyImage,
    /// Save an SVG picture.
    ExportSvg,
    /// Turn a selection into a subcircuit.
    CreateSubcircuit,
    /// Find the DC operating point.
    FindDc,
    /// Load the automatic save.
    Recover,
    /// Print the circuit.
    Print,
    /// Toggle full screen.
    Fullscreen,
    /// Show the About dialog.
    About,
    /// Undo.
    Undo,
    /// Redo.
    Redo,
    /// Cut the selection.
    Cut,
    /// Copy the selection.
    Copy,
    /// Paste.
    Paste,
    /// Duplicate the selection.
    Duplicate,
    /// Select everything.
    SelectAll,
    /// Delete the selection.
    Delete,
    /// Find a component by name.
    Find,
    /// Fit the circuit to the window.
    Center,
    /// Zoom to 100%.
    Zoom100,
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
    /// Mirror left to right.
    MirrorX,
    /// Mirror top to bottom.
    MirrorY,
    /// Rotate counter-clockwise.
    RotateCcw,
    /// Rotate clockwise.
    RotateCw,
    /// Drag out a new element of this registry id.
    Place(String),
    /// Choose a non-placing tool.
    SetTool(Tool),
    /// Open a scope for the selected element.
    ViewInScope,
    /// Add sliders for the fields of the selected element.
    AddSliders,
    /// Turn the selected scope element back into a docked scope.
    DockScope,
    /// Remove every scope.
    RemoveScopes,
    /// Stack all scopes.
    StackAll,
    /// Unstack all scopes.
    UnstackAll,
    /// Combine all scopes.
    CombineAll,
    /// Separate all scopes.
    SeparateAll,
    /// Flip an Options switch.
    Toggle(Toggle),
    /// Show the keyboard shortcuts.
    Shortcuts,
    /// Show the other options dialog.
    OtherOptions,
    /// Convert wires to routed wires.
    ConvertWires,
    /// Open the subcircuit manager.
    SubcircuitManager,
    /// Load a bundled example by file name.
    Example(String),
    /// Choose the interface language (a code of `locale::LANGUAGES`).
    Language(String),
}

const TOGGLES: &[(Toggle, &str)] = &[
    (Toggle::ShowCurrent, "show-current"),
    (Toggle::ShowVoltage, "show-voltage"),
    (Toggle::ShowPower, "show-power"),
    (Toggle::ShowValues, "show-values"),
    (Toggle::SmallGrid, "small-grid"),
    (Toggle::Toolbar, "toolbar"),
    (Toggle::Crosshair, "crosshair"),
    (Toggle::EuroResistors, "euro-resistors"),
    (Toggle::ShowOhm, "show-ohm"),
    (Toggle::IecGates, "iec-gates"),
    (Toggle::WhiteBackground, "white-background"),
    (Toggle::ConventionalCurrent, "conventional-current"),
    (Toggle::DisableEditing, "disable-editing"),
    (Toggle::MouseWheelEdit, "mouse-wheel-edit"),
];

const TOOLS: &[(Tool, &str)] = &[
    (Tool::Select, "tool-select"),
    (Tool::DragAll, "tool-drag-all"),
    (Tool::DragRow, "tool-drag-row"),
    (Tool::DragColumn, "tool-drag-column"),
    (Tool::DragSelected, "tool-drag-selected"),
    (Tool::DragPost, "tool-drag-post"),
];

impl Cmd {
    /// The name of the window action that carries this command (without the `win.` prefix).
    /// Commands with a parameter share one action name; see [`Cmd::target`].
    pub fn action_name(&self) -> String {
        let fixed = match self {
            Cmd::NewWindow => "new-window",
            Cmd::NewCircuit => "new-circuit",
            Cmd::Open => "open",
            Cmd::ImportText => "import-text",
            Cmd::Save => "save",
            Cmd::SaveAs => "save-as",
            Cmd::ExportLink => "export-link",
            Cmd::ExportText => "export-text",
            Cmd::ExportImage => "export-image",
            Cmd::CopyImage => "copy-image",
            Cmd::ExportSvg => "export-svg",
            Cmd::CreateSubcircuit => "create-subcircuit",
            Cmd::FindDc => "find-dc",
            Cmd::Recover => "recover",
            Cmd::Print => "print",
            Cmd::Fullscreen => "fullscreen",
            Cmd::About => "about",
            Cmd::Undo => "undo",
            Cmd::Redo => "redo",
            Cmd::Cut => "cut",
            Cmd::Copy => "copy",
            Cmd::Paste => "paste",
            Cmd::Duplicate => "duplicate",
            Cmd::SelectAll => "select-all",
            Cmd::Delete => "delete",
            Cmd::Find => "find",
            Cmd::Center => "center",
            Cmd::Zoom100 => "zoom-100",
            Cmd::ZoomIn => "zoom-in",
            Cmd::ZoomOut => "zoom-out",
            Cmd::MirrorX => "mirror-x",
            Cmd::MirrorY => "mirror-y",
            Cmd::RotateCcw => "rotate-ccw",
            Cmd::RotateCw => "rotate-cw",
            Cmd::Place(_) => "place",
            Cmd::ViewInScope => "view-in-scope",
            Cmd::DockScope => "dock-scope",
            Cmd::AddSliders => "add-sliders",
            Cmd::RemoveScopes => "remove-scopes",
            Cmd::StackAll => "stack-all",
            Cmd::UnstackAll => "unstack-all",
            Cmd::CombineAll => "combine-all",
            Cmd::SeparateAll => "separate-all",
            Cmd::Shortcuts => "shortcuts",
            Cmd::OtherOptions => "other-options",
            Cmd::ConvertWires => "convert-wires",
            Cmd::SubcircuitManager => "subcircuit-manager",
            Cmd::Example(_) => "example",
            Cmd::Language(_) => "language",
            Cmd::Toggle(t) => TOGGLES.iter().find(|(x, _)| x == t).map_or("toggle", |(_, n)| n),
            Cmd::SetTool(t) => TOOLS.iter().find(|(x, _)| x == t).map_or("tool", |(_, n)| n),
        };
        fixed.to_string()
    }

    /// The string parameter of commands that carry one (`Place`, `Example`).
    pub fn target(&self) -> Option<&str> {
        match self {
            Cmd::Place(id) | Cmd::Example(id) | Cmd::Language(id) => Some(id),
            _ => None,
        }
    }

    /// Rebuilds a command from an action name and its optional parameter.
    pub fn from_action(name: &str, target: Option<&str>) -> Option<Cmd> {
        if let Some((t, _)) = TOGGLES.iter().find(|(_, n)| *n == name) {
            return Some(Cmd::Toggle(*t));
        }
        if let Some((t, _)) = TOOLS.iter().find(|(_, n)| *n == name) {
            return Some(Cmd::SetTool(t.clone()));
        }
        Some(match name {
            "place" => Cmd::Place(target?.to_string()),
            "example" => Cmd::Example(target?.to_string()),
            "language" => Cmd::Language(target?.to_string()),
            "new-window" => Cmd::NewWindow,
            "new-circuit" => Cmd::NewCircuit,
            "open" => Cmd::Open,
            "import-text" => Cmd::ImportText,
            "save" => Cmd::Save,
            "save-as" => Cmd::SaveAs,
            "export-link" => Cmd::ExportLink,
            "export-text" => Cmd::ExportText,
            "export-image" => Cmd::ExportImage,
            "copy-image" => Cmd::CopyImage,
            "export-svg" => Cmd::ExportSvg,
            "create-subcircuit" => Cmd::CreateSubcircuit,
            "find-dc" => Cmd::FindDc,
            "recover" => Cmd::Recover,
            "print" => Cmd::Print,
            "fullscreen" => Cmd::Fullscreen,
            "about" => Cmd::About,
            "undo" => Cmd::Undo,
            "redo" => Cmd::Redo,
            "cut" => Cmd::Cut,
            "copy" => Cmd::Copy,
            "paste" => Cmd::Paste,
            "duplicate" => Cmd::Duplicate,
            "select-all" => Cmd::SelectAll,
            "delete" => Cmd::Delete,
            "find" => Cmd::Find,
            "center" => Cmd::Center,
            "zoom-100" => Cmd::Zoom100,
            "zoom-in" => Cmd::ZoomIn,
            "zoom-out" => Cmd::ZoomOut,
            "mirror-x" => Cmd::MirrorX,
            "mirror-y" => Cmd::MirrorY,
            "rotate-ccw" => Cmd::RotateCcw,
            "rotate-cw" => Cmd::RotateCw,
            "view-in-scope" => Cmd::ViewInScope,
            "dock-scope" => Cmd::DockScope,
            "add-sliders" => Cmd::AddSliders,
            "remove-scopes" => Cmd::RemoveScopes,
            "stack-all" => Cmd::StackAll,
            "unstack-all" => Cmd::UnstackAll,
            "combine-all" => Cmd::CombineAll,
            "separate-all" => Cmd::SeparateAll,
            "shortcuts" => Cmd::Shortcuts,
            "other-options" => Cmd::OtherOptions,
            "convert-wires" => Cmd::ConvertWires,
            "subcircuit-manager" => Cmd::SubcircuitManager,
            _ => return None,
        })
    }

    /// True when the feature behind the command is not available in this build yet.
    pub fn unavailable(&self) -> bool {
        // converting wires needs the wires of the passive family
        cfg!(not(feature = "passive")) && matches!(self, Cmd::ConvertWires)
    }

    /// Whether the command can be used right now (contract C3).
    pub fn enabled(&self, s: &Session) -> bool {
        if self.unavailable() {
            return false;
        }
        let editing = s.editing_allowed();
        let has_elements = !s.sim.elements.is_empty();
        match self {
            Cmd::Undo => editing && s.can_undo(),
            Cmd::Redo => editing && s.can_redo(),
            Cmd::Cut | Cmd::Delete => editing && s.has_selection(),
            Cmd::Copy => s.has_selection(),
            Cmd::Duplicate => editing && s.has_selection(),
            Cmd::Paste => editing && s.clipboard.is_some(),
            Cmd::ViewInScope => !s.scope_targets().is_empty(),
            Cmd::DockScope => editing && s.selected_scope_element().is_some(),
            Cmd::AddSliders => editing && s.selected_slider_element().is_some(),
            Cmd::CreateSubcircuit | Cmd::ConvertWires => editing && has_elements,
            Cmd::RemoveScopes | Cmd::StackAll | Cmd::UnstackAll | Cmd::SeparateAll => {
                s.scope_count() > 0
            }
            Cmd::CombineAll => s.scope_count() > 1,
            Cmd::SelectAll
            | Cmd::Find
            | Cmd::Center
            | Cmd::Print
            | Cmd::ExportImage
            | Cmd::CopyImage
            | Cmd::ExportSvg => has_elements,
            Cmd::MirrorX | Cmd::MirrorY | Cmd::RotateCcw | Cmd::RotateCw => editing && has_elements,
            Cmd::Save => s.dirty,
            Cmd::Recover => autosave_path().is_file(),
            Cmd::Place(_) => editing,
            _ => true,
        }
    }
}

/// What the window must still do after [`dispatch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing more to do.
    Done,
    /// Put this text on the system clipboard.
    Clipboard(String),
    /// The command needs the window (dialog, file chooser, clipboard read, print, ...).
    Ui(Cmd),
}

/// Carries out the commands that only change the session.
pub fn dispatch(s: &mut Session, cmd: &Cmd) -> Outcome {
    if !cmd.enabled(s) {
        return Outcome::Done;
    }
    match cmd {
        Cmd::NewCircuit => s.new_circuit(),
        Cmd::StackAll => s.stack_scopes(),
        Cmd::UnstackAll => s.unstack_scopes(),
        Cmd::CombineAll => s.combine_scopes(),
        Cmd::SeparateAll => s.separate_scopes(),
        Cmd::FindDc => {
            // the failure is kept in `stop_message` and shown by the banner
            let _ = s.find_dc();
        }
        Cmd::Undo => {
            s.undo();
        }
        Cmd::Redo => {
            s.redo();
        }
        Cmd::Cut => return s.cut_selected().map_or(Outcome::Done, Outcome::Clipboard),
        Cmd::Copy => return s.copy_selected().map_or(Outcome::Done, Outcome::Clipboard),
        Cmd::Duplicate => {
            s.duplicate();
        }
        Cmd::SelectAll => s.select_all(),
        Cmd::ViewInScope => {
            s.view_in_scope();
        }
        Cmd::DockScope => {
            if let Some(i) = s.selected_scope_element() {
                s.dock_scope(i);
            }
        }
        #[cfg(feature = "passive")]
        Cmd::ConvertWires => {
            s.convert_wires();
        }
        Cmd::RemoveScopes => s.remove_scopes(),
        Cmd::Delete => {
            s.delete_selected();
        }
        Cmd::Center => s.center_circuit(),
        Cmd::Zoom100 => s.zoom_100(),
        Cmd::ZoomIn => s.zoom_by(1.25),
        Cmd::ZoomOut => s.zoom_by(0.8),
        Cmd::MirrorX => {
            s.transform(Transform::MirrorX);
        }
        Cmd::MirrorY => {
            s.transform(Transform::MirrorY);
        }
        Cmd::RotateCcw => {
            s.transform(Transform::RotateCcw);
        }
        Cmd::RotateCw => {
            s.transform(Transform::RotateCw);
        }
        Cmd::Place(id) => s.set_tool(Tool::Place(id.clone())),
        Cmd::SetTool(t) => s.set_tool(t.clone()),
        Cmd::Toggle(t) => {
            let on = !s.toggle_state(*t);
            s.set_toggle(*t, on);
        }
        other => return Outcome::Ui(other.clone()),
    }
    Outcome::Done
}

/// Controls of the side panel (contract C6): Restart, Stop and the sliders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Reset time and element state, then run.
    Restart,
    /// Start (`true`) or stop (`false`) time.
    Run(bool),
    /// Start if stopped, stop if running.
    ToggleRun,
    /// Move a circuit slider to a position from 0 to 100.
    Slider {
        /// Index of the slider.
        index: usize,
        /// Position from 0 to 100.
        position: f64,
    },
    /// Simulation speed slider position.
    Speed(f64),
    /// Current-dot speed slider position.
    CurrentSpeed(f64),
    /// Power brightness slider position.
    Brightness(f64),
}

/// Carries out a side-panel action.
pub fn apply(s: &mut Session, action: Action) {
    match action {
        Action::Restart => s.reset(),
        Action::Run(on) => s.set_running(on),
        Action::ToggleRun => {
            let on = !s.running;
            s.set_running(on);
        }
        Action::Slider { index, position } => {
            s.set_slider(index, position);
        }
        Action::Speed(v) => s.set_speed_bar(v),
        Action::CurrentSpeed(v) => s.set_current_bar(v),
        Action::Brightness(v) => s.options.power_bar = v as i32,
    }
}

/// One entry of a menu.
#[derive(Clone, Debug)]
pub enum Node {
    /// A command: label key (translated with `ls`), glyph name for the icon layer, and accelerator.
    Item(Cmd, String, Option<&'static str>, Option<String>),
    /// A switch from the Options menu.
    Check(Toggle, &'static str),
    /// A group of entries separated from its neighbours.
    Section(Vec<Node>),
    /// A submenu.
    Sub(String, Vec<Node>),
}

fn item(cmd: Cmd, label: &str, glyph: Option<&'static str>, accel: Option<&str>) -> Node {
    Node::Item(cmd, label.to_string(), glyph, accel.map(str::to_string))
}

/// The File menu.
pub fn file_menu() -> Vec<Node> {
    vec![
        Node::Section(vec![
            item(Cmd::NewWindow, "New Window...", Some("window"), Some("<Control>n")),
            item(Cmd::NewCircuit, "New Blank Circuit", Some("doc-new"), None),
            item(Cmd::Open, "Open File...", Some("folder"), Some("<Control>o")),
            item(Cmd::ImportText, "Import From Text...", Some("doc-text"), None),
        ]),
        Node::Section(vec![
            item(Cmd::Save, "Save", Some("floppy"), Some("<Control>s")),
            item(Cmd::SaveAs, "Save As...", Some("floppy"), Some("<Control><Shift>s")),
        ]),
        Node::Section(vec![
            item(Cmd::ExportLink, "Export As Link...", Some("export"), None),
            item(Cmd::ExportText, "Export As Text...", Some("export"), None),
            item(Cmd::ExportImage, "Save As Image...", Some("image"), None),
            item(Cmd::CopyImage, "Copy Circuit Image to Clipboard", Some("image"), None),
            item(Cmd::ExportSvg, "Save As SVG...", Some("image"), None),
        ]),
        Node::Section(vec![
            item(Cmd::CreateSubcircuit, "Create Subcircuit...", Some("microchip"), None),
            item(Cmd::FindDc, "Find DC Operating Point", Some("magic"), None),
            item(Cmd::Recover, "Recover Auto-Save", Some("back-in-time"), None),
            item(Cmd::Print, "Print...", Some("print"), Some("<Control>p")),
        ]),
        Node::Section(vec![item(
            Cmd::Fullscreen,
            "Toggle Full Screen",
            Some("resize-full-alt"),
            Some("F11"),
        )]),
        Node::Section(vec![item(Cmd::About, "About...", Some("info-circled"), None)]),
    ]
}

/// The Edit menu.
pub fn edit_menu() -> Vec<Node> {
    vec![
        Node::Section(vec![
            item(Cmd::Undo, "Undo", Some("reply"), Some("<Control>z")),
            item(Cmd::Redo, "Redo", Some("forward"), Some("<Control>y")),
        ]),
        Node::Section(vec![
            item(Cmd::Cut, "Cut", Some("scissors"), Some("<Control>x")),
            item(Cmd::Copy, "Copy", Some("copy"), Some("<Control>c")),
            item(Cmd::Paste, "Paste", Some("paste"), Some("<Control>v")),
            item(Cmd::Duplicate, "Duplicate", Some("clone"), Some("<Control>d")),
            item(Cmd::Delete, "Delete", None, Some("Delete")),
        ]),
        Node::Section(vec![
            item(Cmd::SelectAll, "Select All", Some("select-all"), Some("<Control>a")),
            item(Cmd::Find, "Find Component...", Some("search"), Some("slash")),
            item(Cmd::Center, "Center Circuit", Some("target"), None),
            item(Cmd::Zoom100, "Zoom 100%", Some("zoom-11"), Some("0")),
            item(Cmd::ZoomIn, "Zoom In", Some("zoom-in"), Some("plus")),
            item(Cmd::ZoomOut, "Zoom Out", Some("zoom-out"), Some("minus")),
        ]),
        Node::Section(vec![
            item(Cmd::MirrorX, "Mirror X", Some("flip-x"), None),
            item(Cmd::MirrorY, "Mirror Y", Some("flip-y"), None),
            item(Cmd::RotateCcw, "Rotate CCW", Some("ccw"), None),
            item(Cmd::RotateCw, "Rotate CW", Some("cw"), None),
        ]),
    ]
}

/// The Time Diagrams (scopes) menu.
pub fn scopes_menu() -> Vec<Node> {
    vec![
        Node::Section(vec![
            item(Cmd::ViewInScope, "View in Scope", None, None),
            item(Cmd::RemoveScopes, "Remove All Scopes", None, None),
        ]),
        item(Cmd::StackAll, "Stack All", Some("lines"), None),
        item(Cmd::UnstackAll, "Unstack All", Some("columns"), None),
        item(Cmd::CombineAll, "Combine All", Some("object-group"), None),
        item(Cmd::SeparateAll, "Separate All", Some("object-ungroup"), None),
    ]
}

/// The Options menu.
pub fn options_menu() -> Vec<Node> {
    vec![
        Node::Section(vec![
            Node::Check(Toggle::ShowCurrent, "Show Current"),
            Node::Check(Toggle::ShowVoltage, "Show Voltage"),
            Node::Check(Toggle::ShowPower, "Show Power"),
            Node::Check(Toggle::ShowValues, "Show Values"),
            Node::Check(Toggle::SmallGrid, "Small Grid"),
            Node::Check(Toggle::Toolbar, "Toolbar"),
            Node::Check(Toggle::Crosshair, "Show Cursor Crosshair"),
            Node::Check(Toggle::EuroResistors, "European Resistors"),
            Node::Check(Toggle::ShowOhm, "Show \u{3a9} Unit"),
            Node::Check(Toggle::IecGates, "IEC Gates"),
            Node::Check(Toggle::WhiteBackground, "White Background"),
            Node::Check(Toggle::ConventionalCurrent, "Conventional Current Motion"),
            Node::Check(Toggle::DisableEditing, "Disable Editing"),
            Node::Check(Toggle::MouseWheelEdit, "Edit Values With Mouse Wheel"),
        ]),
        Node::Section(vec![Node::Sub(
            "Language".into(),
            crate::locale::LANGUAGES
                .iter()
                .map(|(code, name)| {
                    let mark =
                        if *code == crate::locale::current_language() { "\u{2713} " } else { "" };
                    Node::Item(
                        Cmd::Language((*code).to_string()),
                        format!("{mark}{name}"),
                        None,
                        None,
                    )
                })
                .collect(),
        )]),
        Node::Section(vec![
            item(Cmd::Shortcuts, "Shortcuts...", None, None),
            item(Cmd::OtherOptions, "Other Options...", None, None),
        ]),
    ]
}

/// The Tools menu.
pub fn tools_menu() -> Vec<Node> {
    vec![
        item(Cmd::ConvertWires, "Convert Wires to Routed Wires", None, None),
        item(Cmd::SubcircuitManager, "Subcircuit Manager", None, None),
    ]
}

/// The Circuit Examples menu: New Blank Circuit, then the catalogue as nested submenus.
pub fn examples_menu(list: &super::examples::ExampleList) -> Vec<Node> {
    fn convert(items: &[super::examples::ExampleItem]) -> Vec<Node> {
        use super::examples::ExampleItem;
        items
            .iter()
            .map(|i| match i {
                ExampleItem::Circuit { file, title } => {
                    Node::Item(Cmd::Example(file.clone()), title.clone(), None, None)
                }
                ExampleItem::Group { title, items } => Node::Sub(title.clone(), convert(items)),
            })
            .collect()
    }
    vec![
        Node::Section(vec![item(Cmd::NewCircuit, "New Blank Circuit", Some("doc-new"), None)]),
        Node::Section(convert(&list.items)),
    ]
}

/// The Draw menu, generated from the registry (contract C2): one entry per registered element
/// in its category, then the drag modes.
pub fn draw_menu(registry: &Registry) -> Vec<Node> {
    let mut top: Vec<Node> = Vec::new();
    let mut subs: Vec<Node> = Vec::new();
    for group in registry.menu() {
        let entries: Vec<Node> = group
            .items
            .iter()
            .filter_map(|id| registry.get(id))
            .map(|d| {
                let accel = d.shortcut.map(|c| match c {
                    c if c.is_ascii_uppercase() => format!("<Shift>{}", c.to_ascii_lowercase()),
                    // the original's shifted-digit shortcuts, as GTK spells them
                    '!' => "<Shift>1".to_string(),
                    '@' => "<Shift>2".to_string(),
                    '#' => "<Shift>3".to_string(),
                    '$' => "<Shift>4".to_string(),
                    '%' => "<Shift>5".to_string(),
                    '^' => "<Shift>6".to_string(),
                    '&' => "<Shift>7".to_string(),
                    '*' => "<Shift>8".to_string(),
                    c => c.to_string(),
                });
                Node::Item(Cmd::Place(d.id.to_string()), d.display_name.to_string(), None, accel)
            })
            .collect();
        if group.category.is_empty() {
            top.extend(entries);
        } else {
            subs.push(Node::Sub(group.category.to_string(), entries));
        }
    }
    let mut out = vec![Node::Section(top), Node::Section(subs)];
    out.push(Node::Section(vec![
        Node::Sub(
            "Drag".to_string(),
            vec![
                item(Cmd::SetTool(Tool::DragAll), "Drag All", None, None),
                item(Cmd::SetTool(Tool::DragRow), "Drag Row", None, None),
                item(Cmd::SetTool(Tool::DragColumn), "Drag Column", None, None),
                item(Cmd::SetTool(Tool::DragSelected), "Drag Selected", None, None),
                item(Cmd::SetTool(Tool::DragPost), "Drag Post", None, None),
            ],
        ),
        item(Cmd::SetTool(Tool::Select), "Select/Drag Sel", None, Some("space")),
    ]));
    out
}

/// Every command in the menus that has an accelerator, for registering shortcuts and for the
/// shortcuts dialog (contract C4): `(command, accelerator, label key)`.
pub fn accelerators(registry: &Registry) -> Vec<(Cmd, String, String)> {
    fn walk(nodes: &[Node], out: &mut Vec<(Cmd, String, String)>) {
        for n in nodes {
            match n {
                Node::Item(cmd, label, _, Some(accel)) => {
                    out.push((cmd.clone(), accel.clone(), label.clone()))
                }
                Node::Section(v) | Node::Sub(_, v) => walk(v, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for menu in [file_menu(), edit_menu(), draw_menu(registry)] {
        walk(&menu, &mut out);
    }
    out
}

/// Every command that appears in a menu.
pub fn all_commands(registry: &Registry) -> Vec<Cmd> {
    fn walk(nodes: &[Node], out: &mut Vec<Cmd>) {
        for n in nodes {
            match n {
                Node::Item(c, ..) => out.push(c.clone()),
                Node::Check(t, _) => out.push(Cmd::Toggle(*t)),
                Node::Section(v) | Node::Sub(_, v) => walk(v, out),
            }
        }
    }
    let mut out = Vec::new();
    for m in
        [file_menu(), edit_menu(), scopes_menu(), options_menu(), tools_menu(), draw_menu(registry)]
    {
        walk(&m, &mut out);
    }
    out
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
    fn every_menu_command_round_trips_through_its_action_name() {
        let r = Registry::standard();
        for c in all_commands(&r) {
            let back = Cmd::from_action(&c.action_name(), c.target());
            assert_eq!(back.as_ref(), Some(&c), "{c:?}");
        }
    }

    #[test]
    fn draw_menu_has_one_entry_per_registered_menu_element() {
        let r = Registry::standard();
        let placed: Vec<String> = all_commands(&r)
            .into_iter()
            .filter_map(|c| if let Cmd::Place(id) = c { Some(id) } else { None })
            .collect();
        let expected: Vec<&str> = r.menu().iter().flat_map(|g| g.items.clone()).collect();
        assert_eq!(placed.len(), expected.len());
        for id in expected {
            assert!(placed.iter().any(|p| p == id), "{id} missing");
        }
    }

    #[test]
    fn disabled_states_follow_the_session() {
        let mut s = session();
        assert!(!Cmd::Undo.enabled(&s));
        assert!(!Cmd::Cut.enabled(&s));
        assert!(!Cmd::Paste.enabled(&s));
        assert!(!Cmd::Save.enabled(&s));
        assert!(!Cmd::StackAll.enabled(&s));
        dispatch(&mut s, &Cmd::SelectAll);
        assert!(Cmd::Cut.enabled(&s));
        assert!(Cmd::Copy.enabled(&s));
        dispatch(&mut s, &Cmd::Copy);
        assert!(Cmd::Paste.enabled(&s));
        dispatch(&mut s, &Cmd::Delete);
        assert!(Cmd::Undo.enabled(&s) && Cmd::Save.enabled(&s));
        s.set_toggle(Toggle::DisableEditing, true);
        assert!(!Cmd::Undo.enabled(&s) && !Cmd::Paste.enabled(&s));
    }

    #[test]
    fn dispatch_applies_session_commands_and_defers_ui_ones() {
        let mut s = session();
        let n = s.sim.elements.len();
        assert_eq!(dispatch(&mut s, &Cmd::SelectAll), Outcome::Done);
        assert!(
            matches!(dispatch(&mut s, &Cmd::Copy), Outcome::Clipboard(t) if t.contains("<cir"))
        );
        assert_eq!(dispatch(&mut s, &Cmd::Duplicate), Outcome::Done);
        assert_eq!(s.sim.elements.len(), 2 * n);
        assert_eq!(dispatch(&mut s, &Cmd::Undo), Outcome::Done);
        assert_eq!(s.sim.elements.len(), n);
        assert_eq!(dispatch(&mut s, &Cmd::Open), Outcome::Ui(Cmd::Open));
        dispatch(&mut s, &Cmd::Place("ResistorElm".into()));
        assert_eq!(s.tool, Tool::Place("ResistorElm".into()));
        dispatch(&mut s, &Cmd::SetTool(Tool::Select));
        assert_eq!(s.tool, Tool::Select);
        dispatch(&mut s, &Cmd::Toggle(Toggle::ShowPower));
        assert!(s.toggle_state(Toggle::ShowPower));
        dispatch(&mut s, &Cmd::NewCircuit);
        assert!(s.sim.elements.is_empty());
    }

    #[cfg(feature = "passive")]
    #[test]
    fn accelerators_cover_the_contract_shortcuts() {
        let r = Registry::standard();
        let acc = accelerators(&r);
        let has = |c: Cmd, a: &str| acc.iter().any(|(x, y, _)| *x == c && y == a);
        assert!(has(Cmd::Undo, "<Control>z"));
        assert!(has(Cmd::Redo, "<Control>y"));
        assert!(has(Cmd::Open, "<Control>o"));
        assert!(has(Cmd::Place("WireElm".into()), "w"));
        assert!(has(Cmd::Place("ResistorElm".into()), "r"));
        let mut seen = std::collections::HashSet::new();
        for (_, a, l) in &acc {
            assert!(seen.insert(a.clone()), "accelerator {a} used twice ({l})");
        }
    }
}
