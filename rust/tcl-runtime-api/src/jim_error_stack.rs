// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Jim's error capture over evaluation frames, independent of variable frames.

/// Selected native automatic-error presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeErrorStackProtocol {
    /// Tcl's textual error-info and version-selected structured error stack.
    Tcl,
    /// Jim 0.84's flat procedure/file/line/argv records.
    Jim084,
}

/// Source location retained by a Jim script object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimScriptLocation<T> {
    /// Actual script filename object, including an explicitly empty name.
    pub file: T,
    /// Current native script line, rather than a variable-frame line.
    pub line: u32,
}

/// One active native evaluation frame, ordered innermost first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimEvaluationFrame<T> {
    /// Procedure nesting depth; uplevel does not change this axis.
    pub procedure_level: u32,
    /// Resolved command slot name; unavailable for a failed command lookup.
    pub command_name: Option<T>,
    /// Whether the actual selected command is a procedure.
    pub is_procedure: bool,
    /// Current script object's location, if this evaluation owns a script.
    pub script: Option<JimScriptLocation<T>>,
    /// Evaluated invocation list, empty when lookup failed before argv capture.
    pub invocation: T,
}

/// One native Jim error record. Adapters construct a four-element Tcl list
/// from these fields without adding C annotations or clipping command bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimErrorFrame<T> {
    /// Actual selected caller procedure name, or the empty object.
    pub procedure: T,
    /// Script object's filename, or the empty object.
    pub file: T,
    /// Native script line, defaulting to one for a vector evaluation.
    pub line: u32,
    /// Materialized invocation list captured before unwinding.
    pub invocation: T,
}

/// Capture Jim's first automatic error stack before evaluation frames unwind.
/// `script_failure` is supplied only by a script parser's failure boundary.
/// A missing procedure-level frame abstains; it never invents a caller frame.
#[must_use]
pub fn capture_jim_error_frames<T: Clone>(
    procedure_level: u32,
    frames: &[JimEvaluationFrame<T>],
    script_failure: Option<&JimScriptLocation<T>>,
    empty: &T,
) -> Option<Vec<JimErrorFrame<T>>> {
    if procedure_level == 0
        && let Some(script) = script_failure
    {
        return Some(vec![JimErrorFrame {
            procedure: empty.clone(),
            file: script.file.clone(),
            line: script.line,
            invocation: empty.clone(),
        }]);
    }
    let mut result = Vec::new();
    for level in (0..=procedure_level).rev() {
        let index = if level == procedure_level {
            0
        } else {
            frames
                .iter()
                .position(|frame| frame.procedure_level == level)?
        };
        let frame = frames.get(index)?;
        let procedure = if index == 0 || frame.command_name.is_some() {
            frames[index + 1..].iter().find_map(|caller| {
                caller
                    .is_procedure
                    .then_some(caller.command_name.as_ref())
                    .flatten()
            })
        } else {
            None
        };
        result.push(JimErrorFrame {
            procedure: procedure.unwrap_or(empty).clone(),
            file: frame
                .script
                .as_ref()
                .map_or(empty, |script| &script.file)
                .clone(),
            line: frame.script.as_ref().map_or(1, |script| script.line),
            invocation: frame.invocation.clone(),
        });
    }
    Some(result)
}

/// First-capture lifecycle for Jim's cached trace. Explicit traces are raw
/// values: Jim does not require a list or a multiple of four elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JimErrorTrace<T> {
    /// A raw explicit `error`/`return -errorinfo` trace.
    Explicit(T),
    /// A trace captured from actual native evaluation frames.
    Automatic(Vec<JimErrorFrame<T>>),
}

/// Interpreter-local Jim trace cache. Starting a new evaluation marks capture
/// pending while keeping the last trace available to `info stacktrace`.
#[derive(Debug, Clone)]
pub struct JimErrorStack<T> {
    trace: Option<JimErrorTrace<T>>,
    pending: bool,
}

impl<T> Default for JimErrorStack<T> {
    fn default() -> Self {
        Self {
            trace: None,
            pending: true,
        }
    }
}

impl<T> JimErrorStack<T> {
    /// Begin a selected native script/catch evaluation, retaining cached data.
    pub const fn mark_reset(&mut self) {
        self.pending = true;
    }

    /// Replace the trace with an unvalidated explicit native value.
    pub fn adopt_explicit(&mut self, value: T) {
        self.trace = Some(JimErrorTrace::Explicit(value));
        self.pending = false;
    }

    /// Capture once, before unwinding. Incomplete frame evidence leaves the
    /// episode pending instead of presenting a fabricated stack.
    pub fn capture(&mut self, capture: impl FnOnce() -> Option<Vec<JimErrorFrame<T>>>) -> bool {
        if !self.pending {
            return false;
        }
        let Some(frames) = capture() else {
            return false;
        };
        self.trace = Some(JimErrorTrace::Automatic(frames));
        self.pending = false;
        true
    }

    /// Capture the actual stack-trace object once while evaluation argv is live.
    /// The adapter constructs its native list storage without a later reconstruction.
    pub fn capture_object(&mut self, capture: impl FnOnce() -> Option<T>) -> bool {
        if !self.pending {
            return false;
        }
        let Some(value) = capture() else {
            return false;
        };
        self.trace = Some(JimErrorTrace::Explicit(value));
        self.pending = false;
        true
    }

    /// Last captured or explicit trace, including a previous evaluation's data.
    #[must_use]
    pub const fn trace(&self) -> Option<&JimErrorTrace<T>> {
        self.trace.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        level: u32,
        name: &str,
        procedure: bool,
        invocation: &str,
    ) -> JimEvaluationFrame<String> {
        JimEvaluationFrame {
            procedure_level: level,
            command_name: Some(name.into()),
            is_procedure: procedure,
            script: Some(JimScriptLocation {
                file: "stdin".into(),
                line: level + 1,
            }),
            invocation: invocation.into(),
        }
    }

    #[test]
    fn capture_selects_proc_levels_and_retains_materialized_invocations() {
        let frames = [
            frame(2, "error", false, "error BOOM"),
            frame(2, "eval", false, "eval {error BOOM}"),
            frame(1, "inner", true, "inner BOOM"),
            frame(0, "savedOuter", true, "savedOuter"),
        ];
        let captured = capture_jim_error_frames(2, &frames, None, &String::new()).unwrap();
        assert_eq!(
            captured
                .iter()
                .map(|entry| entry.procedure.as_str())
                .collect::<Vec<_>>(),
            ["inner", "savedOuter", ""]
        );
        assert_eq!(
            captured
                .iter()
                .map(|entry| entry.invocation.as_str())
                .collect::<Vec<_>>(),
            ["error BOOM", "inner BOOM", "savedOuter"]
        );
    }

    #[test]
    fn script_failure_has_no_command_and_missing_frame_is_unknown() {
        let script = JimScriptLocation {
            file: "script.tcl".to_owned(),
            line: 7,
        };
        let captured = capture_jim_error_frames(0, &[], Some(&script), &String::new()).unwrap();
        assert_eq!(captured[0].invocation, "");
        assert_eq!(captured[0].file, "script.tcl");
        assert_eq!(captured[0].line, 7);
        assert!(
            capture_jim_error_frames(
                1,
                &[frame(1, "error", false, "error BOOM")],
                None,
                &String::new()
            )
            .is_none()
        );
        let explicit = JimErrorTrace::<String>::Explicit("P file 3".into());
        assert!(matches!(explicit, JimErrorTrace::Explicit(_)));
    }

    #[test]
    fn explicit_trace_survives_unwinding_until_a_new_evaluation_captures() {
        let mut stack = JimErrorStack::default();
        stack.adopt_explicit("P file 3".to_owned());
        assert!(!stack.capture(|| panic!("explicit trace is already captured")));
        stack.mark_reset();
        assert!(
            matches!(stack.trace(), Some(JimErrorTrace::Explicit(value)) if value == "P file 3")
        );
        assert!(stack.capture(|| Some(Vec::new())));
        assert!(!stack.capture(|| panic!("unwinding cannot replace the first capture")));
        assert!(
            matches!(stack.trace(), Some(JimErrorTrace::Automatic(frames)) if frames.is_empty())
        );
    }
}
