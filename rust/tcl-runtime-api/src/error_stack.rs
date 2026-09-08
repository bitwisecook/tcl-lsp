// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! TIP 348 structured error-stack representation and lifecycle.
//!
//! Runtime engines use different Tcl value representations, but the stack's
//! lazy-reset and tag/value-pair invariants are identical. This owner keeps
//! those rules below both engines; adapters only construct concrete values.

use crate::Code;

/// A rejected explicit `-errorstack` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStackValueError {
    /// The value could not be decoded as a Tcl list.
    NonList,
    /// A flat tag/value list must contain an even number of elements.
    OddSized,
}

/// Validate a decoded explicit `-errorstack` value.
pub fn validate_error_stack<T, E>(
    parts: Result<Vec<T>, E>,
) -> Result<Vec<T>, ErrorStackValueError> {
    let parts = parts.map_err(|_| ErrorStackValueError::NonList)?;
    if !parts.len().is_multiple_of(2) {
        return Err(ErrorStackValueError::OddSized);
    }
    Ok(parts)
}

/// Interpreter-local TIP 348 stack over one engine's concrete Tcl value type.
#[derive(Debug, Clone)]
pub struct ErrorStack<T> {
    entries: Vec<T>,
    reset: bool,
}

impl<T> Default for ErrorStack<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            reset: true,
        }
    }
}

impl<T> ErrorStack<T> {
    /// Whether the next inner-context log starts a new episode.
    #[must_use]
    pub const fn is_reset(&self) -> bool {
        self.reset
    }

    /// Keep the last entries introspectable but make the next inner log replace them.
    pub const fn mark_reset(&mut self) {
        self.reset = true;
    }

    /// Adopt an already-decoded, validated explicit stack.
    pub fn adopt(&mut self, entries: Vec<T>) -> Result<(), ErrorStackValueError> {
        if !entries.len().is_multiple_of(2) {
            return Err(ErrorStackValueError::OddSized);
        }
        self.entries = entries;
        self.reset = false;
        Ok(())
    }

    /// Start the next error episode with one `INNER context` pair.
    pub fn begin_inner(&mut self, tag: T, context: T) -> bool {
        if !self.reset {
            return false;
        }
        self.entries.clear();
        self.entries.push(tag);
        self.entries.push(context);
        self.reset = false;
        true
    }

    /// Discard the active episode and immediately start another `INNER` pair.
    pub fn restart_inner(&mut self, tag: T, context: T) {
        self.reset = true;
        let _ = self.begin_inner(tag, context);
    }

    /// Extend an active episode with another tag/value pair.
    pub fn push_pair(&mut self, tag: T, value: T) -> bool {
        if self.reset {
            return false;
        }
        self.entries.push(tag);
        self.entries.push(value);
        true
    }

    /// Append a procedure `CALL` only when an error unwound through its body.
    ///
    /// A positive-level `return -code error` reaches its settling procedure as
    /// [`Code::Return`] and creates the error there, so that procedure is not a
    /// frame the error unwound through. Outer procedures see [`Code::Error`]
    /// and append normally.
    pub fn push_proc_call(
        &mut self,
        body_code: Code,
        settled_code: Code,
        tag: T,
        invocation: T,
    ) -> bool {
        if settled_code != Code::Error || body_code == Code::Return {
            return false;
        }
        self.push_pair(tag, invocation)
    }

    /// Borrow the flat tag/value entries for engine-specific Tcl-list encoding.
    #[must_use]
    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    /// Borrow the active episode, or `None` while waiting for the next error.
    #[must_use]
    pub fn active_entries(&self) -> Option<&[T]> {
        (!self.reset).then_some(&self.entries)
    }
}

impl<T: Clone> ErrorStack<T> {
    /// Snapshot the active stack, or a carried explicit stack before logging.
    #[must_use]
    pub fn snapshot_or(&self, carried: Option<Vec<T>>) -> Vec<T> {
        if self.reset {
            carried.unwrap_or_default()
        } else {
            self.entries.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_is_lazy_and_pair_shaped() {
        let mut stack = ErrorStack::default();
        assert!(stack.begin_inner("INNER", "first"));
        assert!(!stack.begin_inner("INNER", "ignored"));
        assert!(stack.push_pair("CALL", "p"));
        assert_eq!(stack.entries(), &["INNER", "first", "CALL", "p"]);

        stack.mark_reset();
        assert_eq!(stack.entries(), &["INNER", "first", "CALL", "p"]);
        assert!(stack.begin_inner("INNER", "second"));
        assert_eq!(stack.entries(), &["INNER", "second"]);
    }

    #[test]
    fn validation_is_shared() {
        assert_eq!(
            validate_error_stack::<u8, _>(Err(())),
            Err(ErrorStackValueError::NonList)
        );
        assert_eq!(
            validate_error_stack::<_, ()>(Ok(vec![1])),
            Err(ErrorStackValueError::OddSized)
        );
        assert_eq!(
            validate_error_stack::<_, ()>(Ok(vec![1, 2])),
            Ok(vec![1, 2])
        );
    }

    #[test]
    fn return_boundary_suppresses_only_the_error_creating_proc() {
        let mut stack = ErrorStack::default();
        stack.adopt(Vec::<&str>::new()).unwrap();
        assert!(!stack.push_proc_call(Code::Return, Code::Error, "CALL", "inner"));
        assert!(stack.push_proc_call(Code::Error, Code::Error, "CALL", "outer"));
        assert_eq!(stack.entries(), &["CALL", "outer"]);
    }
}
