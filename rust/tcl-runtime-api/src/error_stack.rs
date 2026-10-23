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

use std::sync::Arc;

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

/// Actual call-frame projection supplied by a reached native command log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorStackFrame<T> {
    /// Root or special frame with no reportable invocation words.
    Unreported,
    /// Redirected native variable frame, retaining its concrete level-delta value.
    Redirect(T),
    /// Ordinary non-root frame, retaining its original invocation list.
    Call(T),
}

/// Original execution-frame role retained while its variable frame is redirected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftedErrorStackFrame {
    /// The original special frame has no invocation words, so it logs no frame entry.
    Unreported,
    /// An original invocation-bearing frame executes through another variable frame.
    Redirect(usize),
}

/// Interpreter-local TIP 348 stack over one engine's concrete Tcl value type.
#[derive(Debug, Clone)]
pub struct ErrorStack<T> {
    entries: Arc<Vec<T>>,
    reset: bool,
    shifted_contexts: Vec<(usize, ShiftedErrorStackFrame)>,
}

impl<T> Default for ErrorStack<T> {
    fn default() -> Self {
        Self {
            entries: Arc::new(Vec::new()),
            reset: true,
            shifted_contexts: Vec::new(),
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
        self.entries = Arc::new(entries);
        self.reset = false;
        Ok(())
    }

    /// Start the next error episode with one `INNER context` pair.
    pub fn begin_inner(&mut self, tag: T, context: T) -> bool {
        if !self.reset {
            return false;
        }
        self.entries = Arc::new(vec![tag, context]);
        self.reset = false;
        true
    }

    /// Discard the active episode and immediately start another `INNER` pair.
    pub fn restart_inner(&mut self, tag: T, context: T) {
        self.reset = true;
        let _ = self.begin_inner(tag, context);
    }

    /// Extend an active episode with another tag/value pair.
    pub fn push_pair(&mut self, tag: T, value: T) -> bool
    where
        T: Clone,
    {
        if self.reset {
            return false;
        }
        let entries = Arc::make_mut(&mut self.entries);
        entries.push(tag);
        entries.push(value);
        true
    }

    /// Record the actual call-frame role at a reached command-log operation.
    ///
    /// The caller's already-logged guard owns whether this operation is reached.
    /// Catching an error inside a procedure still records its invocation here;
    /// procedure exit is not the owner of this entry.
    pub fn log_frame(&mut self, frame: ErrorStackFrame<T>, mut tag: impl FnMut(&str) -> T) -> bool
    where
        T: Clone,
    {
        match frame {
            ErrorStackFrame::Unreported => false,
            ErrorStackFrame::Redirect(value) => self.push_pair(tag("UP"), value),
            ErrorStackFrame::Call(value) => self.push_pair(tag("CALL"), value),
        }
    }

    /// Enter an `uplevel`-style redirect, identified by the concrete runtime's
    /// target frame count and the logical level delta recorded by TIP 348.
    pub fn enter_shifted_context(
        &mut self,
        frame_count: usize,
        original_frame: ShiftedErrorStackFrame,
    ) {
        self.shifted_contexts.push((frame_count, original_frame));
    }

    /// Leave the innermost `uplevel`-style redirect.
    pub fn leave_shifted_context(&mut self) {
        let _ = self.shifted_contexts.pop();
    }

    /// The innermost active shift whose target is the command being logged.
    #[must_use]
    pub fn shifted_context_frame(&self, frame_count: usize) -> Option<ShiftedErrorStackFrame> {
        self.shifted_contexts
            .iter()
            .rev()
            .find_map(|(target, frame)| (*target == frame_count).then_some(*frame))
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
    /// Snapshot the active stack, an explicit carried stack, or the retained
    /// prior stack when a reset episode supplies no new inner context.
    #[must_use]
    pub fn snapshot_or(&self, carried: Option<Vec<T>>) -> Vec<T> {
        if self.reset {
            carried.unwrap_or_else(|| self.entries.as_ref().clone())
        } else {
            self.entries.as_ref().clone()
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
    fn saving_backing_does_not_clone_children_before_mutation() {
        use std::cell::Cell;
        use std::rc::Rc;

        struct Counted(Rc<Cell<usize>>);
        impl Clone for Counted {
            fn clone(&self) -> Self {
                self.0.set(self.0.get() + 1);
                Self(Rc::clone(&self.0))
            }
        }
        let count = Rc::new(Cell::new(0));
        let mut stack = ErrorStack::default();
        stack.begin_inner(Counted(Rc::clone(&count)), Counted(Rc::clone(&count)));
        let saved = stack.clone();
        assert_eq!(count.get(), 0);
        stack.push_pair(Counted(Rc::clone(&count)), Counted(Rc::clone(&count)));
        assert_eq!(count.get(), 2);
        assert_eq!(saved.entries().len(), 2);
        assert_eq!(stack.entries().len(), 4);
        stack.mark_reset();
        let reset_saved = stack.clone();
        stack.begin_inner(Counted(Rc::clone(&count)), Counted(Rc::clone(&count)));
        assert_eq!(count.get(), 2);
        assert!(reset_saved.is_reset());
        assert_eq!(reset_saved.entries().len(), 4);
        assert_eq!(stack.entries().len(), 2);
    }

    #[test]
    fn reset_snapshot_retains_prior_entries_until_a_new_inner_context() {
        let mut stack = ErrorStack::default();
        assert!(stack.begin_inner("INNER", "first"));
        stack.mark_reset();

        assert_eq!(stack.snapshot_or(None), ["INNER", "first"]);
        assert_eq!(
            stack.snapshot_or(Some(vec!["INNER", "explicit"])),
            ["INNER", "explicit"]
        );

        assert!(stack.begin_inner("INNER", "second"));
        assert_eq!(stack.snapshot_or(None), ["INNER", "second"]);
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
    fn reached_log_records_call_before_any_procedure_exit() {
        let mut stack = ErrorStack::default();
        stack.begin_inner("INNER", "error BODY");
        assert!(
            stack.log_frame(ErrorStackFrame::Call("p"), |tag| match tag {
                "CALL" => "CALL",
                "UP" => "UP",
                _ => unreachable!(),
            })
        );
        assert!(!stack.log_frame(ErrorStackFrame::Unreported, |_| unreachable!()));
        assert_eq!(stack.entries(), &["INNER", "error BODY", "CALL", "p"]);
    }

    #[test]
    fn shifted_context_uses_the_innermost_matching_target() {
        let mut stack = ErrorStack::<&str>::default();
        stack.enter_shifted_context(2, ShiftedErrorStackFrame::Redirect(1));
        stack.enter_shifted_context(2, ShiftedErrorStackFrame::Redirect(3));
        assert_eq!(
            stack.shifted_context_frame(2),
            Some(ShiftedErrorStackFrame::Redirect(3))
        );
        assert_eq!(stack.shifted_context_frame(1), None);
        stack.leave_shifted_context();
        assert_eq!(
            stack.shifted_context_frame(2),
            Some(ShiftedErrorStackFrame::Redirect(1))
        );
        stack.enter_shifted_context(2, ShiftedErrorStackFrame::Unreported);
        assert_eq!(
            stack.shifted_context_frame(2),
            Some(ShiftedErrorStackFrame::Unreported)
        );
        stack.leave_shifted_context();
        assert_eq!(
            stack.shifted_context_frame(2),
            Some(ShiftedErrorStackFrame::Redirect(1))
        );
    }
}
