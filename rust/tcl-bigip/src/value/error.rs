//! Crate-local error type for the value layer, modelling the `ValueError`
//! raised by the `parse` classmethods on the typed value dataclasses.
//!
//! The message text is kept stable where practical so
//! differential comparisons line up.

use std::fmt;

/// Error raised by a value-type `parse` when the input is invalid.
///
/// The wrapped string is the human-readable message, kept byte-identical
/// to `ValueError` text where practical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueError(pub String);

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ValueError {}
