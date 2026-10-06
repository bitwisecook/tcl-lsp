// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proved native compilation failures, independent of a bytecode representation.

use std::fmt::Write;

/// Native entry protocol, independent of the active physical variable frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationAdmissionScope {
    /// Admit the complete script before its first effect.
    Script,
    /// Admit the procedure body before checking or binding formal parameters.
    ProcedureBody,
}

/// Admission obligation independent of whether an exact error is presentable.
/// A host may discharge this through its genuine native compiler provider;
/// ordinary command dispatch cannot discharge an entry-time compiler failure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NativeCompilationPreflight {
    /// No retained compiler-failure obligation requires a provider.
    #[default]
    NotRequired,
    /// Compilation may fail or requires an unresolved compiler capability.
    /// This does not assert a Tcl error; a genuine provider must admit entry.
    ProviderRequired,
    /// Compilation definitely fails, but its complete native error presentation
    /// is not proved. Refuse execution until the provider resolves this entry.
    UnpresentedDefiniteFailure,
}

/// Host admission failure, deliberately separate from Tcl completion codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompilationAdmissionError {
    /// A genuine native compiler provider must resolve the retained obligation.
    NativePreflightRequired,
    /// Foreign error metadata does not establish its complete native contexts.
    InvalidErrorPresentation,
}

impl std::fmt::Display for NativeCompilationAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NativePreflightRequired => "native compiler preflight provider required",
            Self::InvalidErrorPresentation => "invalid native compilation error presentation",
        })
    }
}

impl std::error::Error for NativeCompilationAdmissionError {}

/// One source command whose native compilation contributed an error frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationErrorCommand {
    /// Original written command, preserving qualification, quoting and whitespace.
    pub text: String,
    /// One-based command line in its own compilation chunk.
    pub line: u32,
    /// Authored expression-compiler annotations preceding this command frame.
    pub before_context: Vec<String>,
    /// Authored child-compiler annotations appended after this command frame.
    pub after_context: Vec<String>,
}

/// Exact error result and source contexts retained at a compilation boundary.
///
/// This is a proved failure, not a declaration that compilation might fail.
/// Emitters only construct it when every command context is established. The
/// runtime supplies the actual procedure invocation name before formal binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationError {
    /// Proved native error result.
    pub message: String,
    /// Error-code list explicitly set by the compiler. Absence retains the
    /// native reset/error lifecycle rather than claiming an explicit NONE.
    pub error_code: Option<String>,
    /// Compilation commands, ordered from the rejected inner script outwards.
    pub command_contexts: Vec<NativeCompilationErrorCommand>,
    /// One-based line attributed to the outer procedure compilation boundary.
    pub body_line: u32,
}

impl NativeCompilationError {
    /// Render C Tcl 8.4's compiler frames using the actual procedure name.
    ///
    /// Missing command contexts or line evidence abstains. The invocation frame
    /// is added by the ordinary runtime command boundary, once, after this
    /// compiler presentation. Script-object entry passes `None`.
    #[must_use]
    pub fn error_info_for_procedure(&self, procedure: Option<&str>) -> Option<String> {
        if self.command_contexts.is_empty()
            || self
                .command_contexts
                .iter()
                .any(|command| command.line == 0)
            || (procedure.is_some() && self.body_line == 0)
        {
            return None;
        }
        let mut information = self.message.clone();
        for command in &self.command_contexts {
            for note in &command.before_context {
                information.push_str(note);
            }
            let (text, ellipsis) = compiler_excerpt(&command.text, 150);
            write!(information, "\n    while compiling\n\"{text}{ellipsis}\"")
                .expect("write compilation context to String");
            for note in &command.after_context {
                information.push_str(note);
            }
        }
        if let Some(procedure) = procedure {
            let (name, ellipsis) = compiler_excerpt(procedure, 50);
            write!(
                information,
                "\n    (compiling body of proc \"{name}{ellipsis}\", line {})",
                self.body_line
            )
            .expect("write procedure compilation context to String");
        }
        Some(information)
    }
    /// Render a compiler frame with the original native procedure-name bytes.
    /// C Tcl 8.4 reports the `CString` extent, clips at 50 bytes, and backs over
    /// continuation bytes without requiring host Unicode decoding.
    #[must_use]
    pub fn error_info_for_procedure_bytes(&self, procedure: Option<&[u8]>) -> Option<Vec<u8>> {
        let mut information = self.error_info_for_procedure(None)?.into_bytes();
        if let Some(procedure) = procedure {
            if self.body_line == 0 {
                return None;
            }
            let name = tcl_syntax::naming::native_procedure_compilation_name_input(
                tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4),
                procedure,
            )
            .ok()?;
            let (excerpt, boundary_ellipsis) =
                tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(
                    tcl_dialect::TclVersion::V8_4,
                )
                .command_log_excerpt(name, name.len().min(50))?;
            let ellipsis = name.len() > 50 || boundary_ellipsis;
            information.extend_from_slice(b"\n    (compiling body of proc \"");
            information.extend_from_slice(excerpt);
            if ellipsis {
                information.extend_from_slice(b"...");
            }
            information.extend_from_slice(format!("\", line {})", self.body_line).as_bytes());
        }
        Some(information)
    }
}

fn compiler_excerpt(text: &str, limit: usize) -> (&str, &str) {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], if end == text.len() { "" } else { "..." })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(text: &str) -> NativeCompilationError {
        NativeCompilationError {
            message: "wrong # args: should be \"set varName ?newValue?\"".into(),
            error_code: Some("NONE".into()),
            command_contexts: vec![NativeCompilationErrorCommand {
                text: text.into(),
                line: 1,
                before_context: Vec::new(),
                after_context: Vec::new(),
            }],
            body_line: 1,
        }
    }

    #[test]
    fn native_compiler_usage_and_written_command_have_separate_presentations() {
        let error = failure("native_set x extra bad");
        assert_eq!(
            error.error_info_for_procedure(Some("p")).as_deref(),
            Some(
                "wrong # args: should be \"set varName ?newValue?\"\n    while compiling\n\"native_set x extra bad\"\n    (compiling body of proc \"p\", line 1)"
            )
        );
        assert_eq!(
            error.error_info_for_procedure(None).as_deref(),
            Some(
                "wrong # args: should be \"set varName ?newValue?\"\n    while compiling\n\"native_set x extra bad\""
            )
        );
    }

    #[test]
    fn compiler_excerpt_limits_match_native_command_and_procedure_frames() {
        let command = format!("{}éextra", "x".repeat(149));
        let name = format!("{}éextra", "p".repeat(49));
        let information = failure(&command)
            .error_info_for_procedure(Some(&name))
            .unwrap();
        assert!(information.contains(&format!("\"{}...\"", "x".repeat(149))));
        assert!(information.contains(&format!("\"{}...\", line 1)", "p".repeat(49))));
        assert!(!information.contains('é'));
    }

    #[test]
    fn procedure_reporting_keeps_non_unicode_and_native_cstring_extent() {
        let error = failure("set x extra bad");
        let information = error
            .error_info_for_procedure_bytes(Some(b"p\xFF\0ignored"))
            .unwrap();
        assert!(information.ends_with(b"(compiling body of proc \"p\xFF\", line 1)"));
        let mut name = vec![b'p'; 49];
        name.extend_from_slice(b"\xC0\x80tail");
        let information = error.error_info_for_procedure_bytes(Some(&name)).unwrap();
        assert!(information.ends_with(
            format!("(compiling body of proc \"{}...\", line 1)", "p".repeat(49)).as_bytes()
        ));
    }

    #[test]
    fn unknown_compiler_context_never_becomes_a_message_only_error_frame() {
        let mut error = failure("set x extra bad");
        error.body_line = 0;
        assert!(error.error_info_for_procedure(Some("p")).is_none());
        error.command_contexts.clear();
        assert!(error.error_info_for_procedure(None).is_none());
    }
}
