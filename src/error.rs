// SPDX-License-Identifier: GPL-2.0-or-later
//! Error types shared across the engine.

use thiserror::Error;

/// A problem found while reading a circuit description.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum LoadError {
    /// The line starts with a type code that no registered element understands.
    #[error("line {line}: unrecognized element type `{type_code}`")]
    UnknownType {
        /// 1-based line number.
        line: usize,
        /// The type code as written in the file.
        type_code: String,
    },
    /// The line could not be parsed (missing or malformed field).
    #[error("line {line}: malformed `{type_code}` entry: {message}")]
    Malformed {
        /// 1-based line number.
        line: usize,
        /// The type code as written in the file.
        type_code: String,
        /// What was wrong.
        message: String,
    },
    /// The XML circuit could not be parsed.
    #[error("invalid XML circuit: {0}")]
    Xml(String),
}

/// A problem found while preparing a circuit for simulation.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AnalysisError {
    /// The circuit failed validation; `element` is the index of the offending element.
    #[error("{message}")]
    Invalid {
        /// Human-readable reason.
        message: String,
        /// Index of the offending element, when known.
        element: Option<usize>,
    },
}

/// A problem found while advancing the simulation.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum StepError {
    /// The matrix could not be factored.
    #[error("Singular matrix!")]
    Singular,
    /// The nonlinear iteration did not converge.
    #[error("Convergence failed!")]
    NotConverged,
    /// The matrix contains NaN or infinite entries.
    #[error("nan/infinite matrix!")]
    NotFinite,
    /// An element asked the simulation to stop.
    #[error("{message}")]
    Stopped {
        /// Reason given by the element.
        message: String,
        /// Index of the element that stopped the simulation, when known.
        element: Option<usize>,
    },
}
