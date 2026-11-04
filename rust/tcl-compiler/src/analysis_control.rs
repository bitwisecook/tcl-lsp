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
pub(crate) fn checkpoint(at: AnalysisCheckpoint) -> bool {
    if interruption().is_some() {
        return true;
    }
    // An inner owned job cannot mask an active outer scheduling owner.
    let active = ACTIVE.with(|active| active.borrow().clone());
    for scope in &active {
        if (scope.control.0)(at) {
            // No enclosing owned job may publish after a nested job stopped,
            // even when its caller chooses to ignore that nested Err.
            for enclosing in &active {
                enclosing.interrupted.set(Some(at));
            }
            return true;
        }
    }
    false
}
pub(crate) fn interrupted() -> bool {
    interruption().is_some()
}
pub(crate) fn run<T>(
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
