// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Operational interruption of an owned analysis, independent of source facts.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// Reached compiler work, not an execution or source-authority claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisCheckpoint {
    /// Before starting an owned analysis.
    Start,
    /// Before constructing a command realm.
    Realm,
    /// Before analysing an original command.
    Command,
    /// Before analysing a body.
    Body,
    /// Before another analysis iteration.
    Fixpoint,
    /// Before collecting diagnostics.
    Diagnostics,
}
/// No complete result was produced; no partial result may be published.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisInterrupted {
    /// The checkpoint at which the host requested interruption.
    pub checkpoint: AnalysisCheckpoint,
}
/// A non-reentrant host scheduling probe. `true` requests interruption.
#[derive(Clone)]
pub struct AnalysisControl(Arc<dyn Fn(AnalysisCheckpoint) -> bool + Send + Sync>);
impl AnalysisControl {
    /// The callback must not consume or dispatch input or re-enter analysis.
    pub fn new(probe: impl Fn(AnalysisCheckpoint) -> bool + Send + Sync + 'static) -> Self {
        Self(Arc::new(probe))
    }
}
struct Active {
    control: AnalysisControl,
    interrupted: Cell<Option<AnalysisCheckpoint>>,
}
thread_local! { static ACTIVE: RefCell<Vec<Rc<Active>>> = const { RefCell::new(Vec::new()) }; }
struct Scope(usize);
impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.borrow_mut().truncate(self.0));
    }
}
fn interruption() -> Option<AnalysisCheckpoint> {
    ACTIVE.with(|active| {
        active
            .borrow()
            .iter()
            .find_map(|scope| scope.interrupted.get())
    })
}
/// Probe every active scheduling owner at `at`; `true` once any owner has
/// stopped the enclosing jobs. A probe may also unwind instead of answering.
#[must_use]
pub fn checkpoint(at: AnalysisCheckpoint) -> bool {
    if interruption().is_some() {
        return true;
    }
    // An inner owned job cannot mask an active outer scheduling owner. Scopes
    // are read one at a time so a probe on the hot path never allocates.
    let mut index = 0;
    while let Some(scope) = ACTIVE.with(|active| active.borrow().get(index).cloned()) {
        if (scope.control.0)(at) {
            // No enclosing owned job may publish after a nested job stopped,
            // even when its caller chooses to ignore that nested Err.
            ACTIVE.with(|active| {
                for enclosing in active.borrow().iter() {
                    enclosing.interrupted.set(Some(at));
                }
            });
            return true;
        }
        index += 1;
    }
    false
}
/// A stop point inside work that cannot publish a partial result. Probes may
/// unwind here (a host cancelling the whole job), but a `true` answer is
/// deliberately ignored: nothing is marked interrupted, so the work completes
/// exactly as it would have without this point.
pub fn unwind_point(at: AnalysisCheckpoint) {
    let mut index = 0;
    while let Some(scope) = ACTIVE.with(|active| active.borrow().get(index).cloned()) {
        let _ = (scope.control.0)(at);
        index += 1;
    }
}
pub(crate) fn interrupted() -> bool {
    interruption().is_some()
}
/// Run `operation` under `control`; `Err` when any checkpoint stopped it.
pub fn run<T>(
    control: AnalysisControl,
    operation: impl FnOnce() -> T,
) -> Result<T, AnalysisInterrupted> {
    let active = Rc::new(Active {
        control,
        interrupted: Cell::new(None),
    });
    let depth = ACTIVE.with(|stack| {
        let mut stack = stack.borrow_mut();
        let depth = stack.len();
        stack.push(Rc::clone(&active));
        depth
    });
    let scope = Scope(depth);
    let result = operation();
    let stopped = interruption();
    drop(scope);
    match stopped {
        Some(checkpoint) => Err(AnalysisInterrupted { checkpoint }),
        None => Ok(result),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    #[test]
    fn an_unwind_point_never_marks_the_job_interrupted() {
        let probes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = std::sync::Arc::clone(&probes);
        let job = run(
            AnalysisControl::new(move |_| {
                seen.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                true
            }),
            || {
                unwind_point(AnalysisCheckpoint::Body);
                interrupted()
            },
        );
        assert_eq!(job, Ok(false), "a `true` probe answer is ignored here");
        assert_eq!(probes.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn ignoring_a_nested_only_interruption_cannot_complete_the_outer_job() {
        let outer = run(AnalysisControl::new(|_| false), || {
            let stopped = Analyser::new().try_analyse_owned(
                "proc inner_stop {} {}",
                "tcl8.6",
                None,
                AnalysisControl::new(|at| at == AnalysisCheckpoint::Command),
            );
            assert_eq!(stopped.unwrap_err().checkpoint, AnalysisCheckpoint::Command);
            // Deliberately ignore the nested failure and attempt more work.
            assert!(
                Analyser::new()
                    .analyse("proc ignored_error {} {}", "tcl8.6")
                    .all_procs
                    .is_empty()
            );
            "attempted outer success"
        });
        assert_eq!(outer.unwrap_err().checkpoint, AnalysisCheckpoint::Command);
        assert!(!interrupted());
    }

    #[test]
    fn nested_owned_and_default_analysis_inherit_a_sticky_outer_interruption() {
        let outer = run(
            AnalysisControl::new(|at| at == AnalysisCheckpoint::Command),
            || {
                assert!(checkpoint(AnalysisCheckpoint::Command));
                let default = Analyser::new().analyse("proc default_hidden {} {}", "tcl8.6");
                assert!(default.all_procs.is_empty());
                let controlled = Analyser::new().try_analyse_owned(
                    "proc controlled_hidden {} {}",
                    "tcl8.6",
                    None,
                    AnalysisControl::new(|_| false),
                );
                assert_eq!(
                    controlled.unwrap_err().checkpoint,
                    AnalysisCheckpoint::Command
                );
                assert!(interrupted());
            },
        );
        assert_eq!(outer.unwrap_err().checkpoint, AnalysisCheckpoint::Command);
        assert!(!interrupted());
        let complete = Analyser::new()
            .try_analyse_owned(
                "proc after_scope {} {}",
                "tcl8.6",
                None,
                AnalysisControl::new(|_| false),
            )
            .unwrap();
        assert!(complete.all_procs.contains_key("::after_scope"));
    }

    #[test]
    fn noninterrupting_owned_analysis_preserves_results_and_source_currency() {
        let source = "proc p {x} {return $x}\np 1\n";
        let expected = Analyser::new().analyse(source, "tcl");
        let got = Analyser::new()
            .try_analyse_owned(source, "tcl", None, AnalysisControl::new(|_| false))
            .unwrap();
        assert_eq!(got.diagnostics, expected.diagnostics);
        assert_eq!(
            got.all_procs
                .keys()
                .collect::<std::collections::BTreeSet<_>>(),
            expected
                .all_procs
                .keys()
                .collect::<std::collections::BTreeSet<_>>()
        );
        assert_eq!(
            got.global_scope.children.len(),
            expected.global_scope.children.len()
        );
        assert!(crate::source_graph::current_analysis(source, &got).is_some());
    }
    #[test]
    fn interruption_is_typed_and_a_fresh_retry_cannot_read_a_partial_source_memo() {
        for (stop, source) in [
            (
                AnalysisCheckpoint::Realm,
                "# realm\nproc p {x} {return $x}\np 1\n",
            ),
            (
                AnalysisCheckpoint::Command,
                "# command\nproc p {x} {return $x}\np 1\n",
            ),
            (
                AnalysisCheckpoint::Body,
                "# body\nproc p {x} {return $x}\np 1\n",
            ),
            (
                AnalysisCheckpoint::Fixpoint,
                "# fixpoint\nproc p {x} {if {$x} {return yes}; return no}\np 1\n",
            ),
        ] {
            let result = Analyser::new().try_analyse_owned(
                source,
                "tcl",
                None,
                AnalysisControl::new(move |at| at == stop),
            );
            assert_eq!(result.unwrap_err().checkpoint, stop);
            let retry = Analyser::new()
                .try_analyse_owned(source, "tcl", None, AnalysisControl::new(|_| false))
                .unwrap();
            assert!(crate::source_graph::current_analysis(source, &retry).is_some());
            assert!(!retry.global_scope.children.is_empty());
        }
    }
}
