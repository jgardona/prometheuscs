// SPDX-License-Identifier: GPL-2.0-or-later
//! Immutable views of a simulation handed from the solver to the UI.

/// What the simulation is doing.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Time advances.
    Running,
    /// Time is stopped by the user.
    Stopped,
    /// The simulation stopped itself: singular matrix, no convergence, or an element asked.
    Errored {
        /// Reason shown to the user.
        reason: String,
        /// Index of the offending element, when known.
        element: Option<usize>,
    },
}

/// New samples of one scope since the previous snapshot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScopeSamples {
    /// Index of the scope.
    pub scope: usize,
    /// The samples, oldest first.
    pub values: Vec<f64>,
}

/// An error with the element that caused it, for the banner and the highlight.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotError {
    /// Reason shown to the user.
    pub message: String,
    /// Index of the offending element, when known.
    pub element: Option<usize>,
}

/// Everything the UI needs to draw one frame. Immutable once published: the UI draws the
/// newest snapshot and never blocks the solver.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameSnapshot {
    /// Monotonic frame number.
    pub sequence: u64,
    /// Simulated time in seconds.
    pub time: f64,
    /// Current time step in seconds.
    pub time_step: f64,
    /// Running, stopped or errored.
    pub status: Status,
    /// Voltage of every node; index 0 is ground.
    pub node_voltages: Vec<f64>,
    /// Voltage at every post (and internal node) of every element, in element order, so the
    /// UI can draw elements without knowing the solver's node numbering.
    pub element_volts: Vec<Vec<f64>>,
    /// Current through every element, in element order.
    pub element_currents: Vec<f64>,
    /// New scope samples since the last snapshot.
    pub scope_samples: Vec<ScopeSamples>,
    /// The error, if the status is `Errored`.
    pub error: Option<SnapshotError>,
}
