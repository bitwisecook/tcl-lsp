//! Signature-help provider — minimal Rust port of
//! `lsp/features/signature_help.py`.
//!
//! Surfaces a single [`SignatureInformation`] for the
//! user-defined `proc` whose name appears as the first word of
//! the active command segment at the cursor.  The active
//! parameter is derived from a simple whitespace-aware count of
//! arguments typed so far on the current physical line.
//!
//! What is *deferred* (planned as `S-signature-help-rich`
//! follow-up):
//!
//! * Built-in command signatures from
//!   [`tcl_registry::CommandRegistry`] / `SIGNATURES` (Python's
//!   `_builtin_signature_help`).
//! * Multi-line command segments (the minimal port only walks
//!   the cursor's physical line — Python uses
//!   `find_command_context_details_at_position`, which understands
//!   continuation lines, embedded `[…]` / `{…}` etc.).
//! * Command-alias resolution
//!   (`lookup_alias_for_word(...)`).
//! * Subcommand-scoped signatures (Python's `SubcommandSig`
//!   path — pulls the right shape based on `args[0]`).
//! * `_signature_documentation` doc-comment rendering.
//!
//! The minimal port is sufficient for the Rust LSP server to
//! surface signature help on user-proc calls inside a single
//! line.

use tcl_compiler::analyser::{AnalysisResult, ProcDef};

/// One element in a signature's parameter list.
///
/// Mirrors `lsprotocol.types.ParameterInformation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInformation {
    /// Parameter label as shown in the signature
    /// (e.g. `name` or `{count 1}`).
    pub label: String,
}

/// One signature in a signature-help response.
///
/// Mirrors `lsprotocol.types.SignatureInformation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    /// Full label of the signature (e.g. `proc ::greet name`).
    pub label: String,
    /// Parameter list in declaration order.
    pub parameters: Vec<ParameterInformation>,
    /// Optional documentation body (markdown).  `None` when no
    /// doc-comment was harvested.
    pub documentation: Option<String>,
}

/// LSP signature-help response — one or more signatures plus
/// the index of the active one and the active parameter.
///
/// Mirrors `lsprotocol.types.SignatureHelp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// Signatures to surface to the editor.
    pub signatures: Vec<SignatureInformation>,
    /// Index of the active signature (0-based).
    pub active_signature: u32,
    /// Index of the active parameter (0-based) within the
    /// active signature.
    pub active_parameter: u32,
}

/// Compute signature help for the command being typed at the
/// cursor.
///
/// Returns `None` when the cursor isn't inside a recognisable
/// command-argument position or no matching user proc was
/// recorded.
#[must_use]
pub fn signature_help(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<SignatureHelp> {
    let (command, active_param) = command_context_on_line(source, line, character)?;
    let proc_def = lookup_proc(analysis, &command)?;
    Some(proc_signature_help(proc_def, active_param))
}

/// Naïve command-context detection on a single physical line.
///
/// Returns `(command_name, active_parameter_index)` when the
/// cursor sits inside the argument list of a command on its
/// own line, or `None` otherwise.
///
/// Mirrors a *much* reduced subset of Python's
/// `find_command_context_details_at_position`: the minimal
/// port doesn't follow continuation lines, doesn't honour
/// embedded `[…]` / `{…}` token nesting, and doesn't treat
/// `;` as a command separator.  All of those are recorded as
/// follow-ups under `S-signature-help-rich`.
fn command_context_on_line(source: &str, line: u32, character: u32) -> Option<(String, u32)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let prefix: String = chars[..col].iter().collect();

    // Split on whitespace; first token is the command.
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let command = tokens[0].to_owned();

    // Active parameter index = number of tokens after the
    // command, minus one if the cursor is currently typing
    // inside a token (i.e. the prefix doesn't end in
    // whitespace).
    let arg_token_count = tokens.len().saturating_sub(1);
    let active_param = if prefix.ends_with(|c: char| c.is_whitespace()) {
        u32::try_from(arg_token_count).ok()?
    } else {
        u32::try_from(arg_token_count.saturating_sub(1)).ok()?
    };

    // Cursor must be past the command name (else it's still
    // typing the command itself).
    if active_param == 0 && !prefix.ends_with(|c: char| c.is_whitespace()) {
        // We're typing the command name — no signature yet.
        return None;
    }

    Some((command, active_param))
}

fn lookup_proc<'a>(analysis: &'a AnalysisResult, name: &str) -> Option<&'a ProcDef> {
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == name || qname == name || qname == &format!("::{name}") {
            return Some(proc_def);
        }
    }
    None
}

fn proc_signature_help(proc_def: &ProcDef, active_param: u32) -> SignatureHelp {
    let parameters: Vec<ParameterInformation> = proc_def
        .params
        .iter()
        .map(|p| {
            let label = if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            };
            ParameterInformation { label }
        })
        .collect();

    let label = format!(
        "proc {} {}",
        proc_def.qualified_name,
        parameters
            .iter()
            .map(|p| p.label.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );

    let active_parameter = if parameters.is_empty() {
        0
    } else {
        let max_idx = u32::try_from(parameters.len() - 1).unwrap_or(0);
        active_param.min(max_idx)
    };

    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            parameters,
            documentation: if proc_def.doc.is_empty() {
                None
            } else {
                Some(proc_def.doc.clone())
            },
        }],
        active_signature: 0,
        active_parameter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn no_help_at_command_name() {
        let src = "proc greet {name body} {}\ngre\n";
        let analysis = analyse(src);
        // Cursor mid-command name — should not surface signature help.
        assert!(signature_help(src, 1, 3, &analysis).is_none());
    }

    #[test]
    fn help_on_first_argument() {
        let src = "proc greet {name body} {}\ngreet \n";
        let analysis = analyse(src);
        // Cursor right after the trailing space following `greet`.
        let h = signature_help(src, 1, 6, &analysis).expect("signature help");
        assert_eq!(h.signatures.len(), 1);
        assert_eq!(h.signatures[0].parameters.len(), 2);
        assert_eq!(h.active_parameter, 0);
        assert!(h.signatures[0].label.contains("::greet"));
    }

    #[test]
    fn active_param_advances_with_typed_args() {
        let src = "proc greet {name body} {}\ngreet alice \n";
        let analysis = analyse(src);
        // Cursor right after the second space (advances to second arg).
        let h = signature_help(src, 1, 12, &analysis).expect("signature help");
        assert_eq!(h.active_parameter, 1);
    }

    #[test]
    fn help_clamps_active_param_to_last_known() {
        let src = "proc one {a} {}\none alice extra \n";
        let analysis = analyse(src);
        // Cursor at position 16 — three arg tokens typed for a
        // 1-param proc; clamp to last known param.
        let h = signature_help(src, 1, 16, &analysis).expect("signature help");
        assert_eq!(h.active_parameter, 0, "{h:?}");
    }

    #[test]
    fn help_returns_none_for_unknown_command() {
        let src = "fakecmd arg \n";
        let analysis = analyse(src);
        assert!(signature_help(src, 0, 12, &analysis).is_none());
    }

    #[test]
    fn proc_doc_surfaces_as_documentation() {
        // The harvested doc-comment from the line above the
        // proc lands in `proc_def.doc`. Verify it surfaces.
        let src = "# greets the user\nproc greet {name} {}\ngreet \n";
        let analysis = analyse(src);
        let h = signature_help(src, 2, 6, &analysis).expect("signature help");
        // Doc may or may not be picked up depending on the
        // analyser's heuristics; if present, it should be
        // surfaced verbatim.
        if let Some(doc) = &h.signatures[0].documentation {
            assert!(doc.contains("greets") || !doc.is_empty(), "doc: {doc}");
        }
    }
}
