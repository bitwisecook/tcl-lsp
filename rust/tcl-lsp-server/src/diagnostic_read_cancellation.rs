// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed cancellation evidence scoped to one actual direct provider pass.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ReadFailure {
    #[default]
    Unobserved,
    PendingWriteOnly,
    Terminal,
}

#[derive(Clone, Default)]
pub(super) struct DiagnosticReadCancellations(Arc<Mutex<ReadFailure>>);

impl DiagnosticReadCancellations {
    pub(super) fn observe(&self, reason: &salsa::Cancelled) {
        let next = match reason {
            salsa::Cancelled::PendingWrite => ReadFailure::PendingWriteOnly,
            _ => ReadFailure::Terminal,
        };
        if let Ok(mut state) = self.0.lock() {
            *state = if *state == ReadFailure::Terminal || next == ReadFailure::Terminal {
                ReadFailure::Terminal
            } else {
                next
            };
        }
    }

    pub(super) fn terminal(&self) {
        if let Ok(mut state) = self.0.lock() {
            *state = ReadFailure::Terminal;
        }
    }

    pub(super) fn pending_write_only(&self) -> bool {
        self.0
            .lock()
            .is_ok_and(|state| *state == ReadFailure::PendingWriteOnly)
    }
}

pub(super) fn observe(scope: Option<&DiagnosticReadCancellations>, reason: &salsa::Cancelled) {
    if let Some(scope) = scope {
        scope.observe(reason);
    }
}

pub(super) fn terminal(scope: Option<&DiagnosticReadCancellations>) {
    if let Some(scope) = scope {
        scope.terminal();
    }
}

pub(super) fn terminal_if_context(
    scope: Option<&DiagnosticReadCancellations>,
    completion: &Result<(), super::DiagnosticUnavailable>,
) {
    if matches!(completion, Err(super::DiagnosticUnavailable::Context)) {
        terminal(scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_actual_pending_write_is_recoverable() {
        for (reason, pending) in [
            (salsa::Cancelled::PendingWrite, true),
            (salsa::Cancelled::Local, false),
            (salsa::Cancelled::PropagatedPanic, false),
        ] {
            let scope = DiagnosticReadCancellations::default();
            assert!(!scope.pending_write_only());
            let result: Result<(), salsa::Cancelled> = Err(reason);
            assert!(
                result
                    .inspect_err(|reason| observe(Some(&scope), reason))
                    .ok()
                    .is_none()
            );
            assert_eq!(scope.pending_write_only(), pending);
        }
    }

    #[test]
    fn terminal_joined_producers_dominate_in_either_order_and_scopes_are_independent() {
        for terminal_reason in [salsa::Cancelled::Local, salsa::Cancelled::PropagatedPanic] {
            for terminal_first in [false, true] {
                let scope = DiagnosticReadCancellations::default();
                if terminal_first {
                    scope.observe(&terminal_reason);
                }
                scope.observe(&salsa::Cancelled::PendingWrite);
                if !terminal_first {
                    scope.observe(&terminal_reason);
                }
                assert!(!scope.pending_write_only());
                let independent = DiagnosticReadCancellations::default();
                independent.observe(&salsa::Cancelled::PendingWrite);
                assert!(independent.pending_write_only());
                independent.terminal();
                assert!(!independent.pending_write_only());
            }
        }
    }

    #[test]
    fn actual_unavailable_provider_completion_prevents_pending_write_recovery() {
        use std::ops::ControlFlow;
        for cross_file in [false, true] {
            let scope = DiagnosticReadCancellations::default();
            scope.observe(&salsa::Cancelled::PendingWrite);
            let completion = super::super::deep_provider_completion(
                cross_file,
                None,
                &ControlFlow::Continue(None),
                &ControlFlow::Continue(None),
            );
            assert!(matches!(
                completion,
                Err(super::super::DiagnosticUnavailable::Context)
            ));
            terminal_if_context(Some(&scope), &completion);
            assert!(!scope.pending_write_only());
        }
    }
}
