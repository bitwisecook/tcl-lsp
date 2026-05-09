//! Completion provider — minimal Rust port of
//! `lsp/features/completion.py`.
//!
//! Wires the LSP completion surface for the two simplest Tcl
//! completion contexts:
//!
//! * **Variable completion** — when the cursor sits immediately
//!   after `$` (or inside a `${name}` braced reference), suggest
//!   every variable name visible in the global scope.
//! * **Proc-name completion** — at any other cursor position,
//!   suggest every user-defined `proc` name that the analyser
//!   recorded for the document.
//!
//! What is *deferred* (planned as `S-completion-rich` follow-up):
//!
//! * Built-in command completions sourced from
//!   [`tcl_registry::CommandRegistry`].
//! * Subcommand completion (`SIGNATURES.get(cmd)` →
//!   `SubcommandSig` path in `lsp/features/completion.py`).
//! * Switch completion (`-foo`, `-bar` for known switches).
//! * Argument-value completion (registry-driven when arg index
//!   has known values; subcommand-scoped values for things like
//!   `string is <class>`).
//! * iRules `call proc_name` first-arg context, `when EVENT`
//!   value enumeration, and other dialect-specific arg rules.
//! * Workspace-wide proc / RULE_INIT-var enumeration, usage-bucket
//!   sort-text computation, and the `_proc_signature_str` rendering
//!   for proc-completion details.
//!
//! Cache + `spawn_blocking` + cached-analysis read-out (analogous
//! to `S-hover-sync11`) ride on top of this provider in
//! `tcl-lsp-server::Backend::completion`; this module is the
//! pure-CPU computation, no I/O, no async.

use tcl_compiler::analyser::{AnalysisResult, ProcDef, Scope};

/// LSP completion-item kind for our surface.  Keep narrow —
/// extend when richer completion lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    /// Tcl variable.
    Variable,
    /// User-defined proc.
    Function,
}

/// A single completion suggestion.
///
/// Mirrors the subset of `lsprotocol.types.CompletionItem` the
/// minimal port emits today: a label, an insert-text, and a
/// kind.  `detail`, `documentation`, and `sort_text` live in
/// the `S-completion-rich` follow-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Label shown in the completion list — for variables,
    /// `$name` / `${name}`; for procs, the proc's qualified or
    /// simple name.
    pub label: String,
    /// Text to insert when the item is accepted.
    pub insert_text: String,
    /// LSP completion kind.
    pub kind: CompletionKind,
}

/// Compute completions for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller
/// (server) is expected to cache it.  Returns an empty vector
/// when there is no useful suggestion (delimiter run, EOF, etc.).
#[must_use]
pub fn completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<CompletionItem> {
    if let Some((trigger, partial)) = variable_trigger(source, line, character) {
        return variable_completions(&analysis.global_scope, &partial, trigger);
    }
    proc_completions(analysis, &word_partial_at_position(source, line, character))
}

/// Variable-trigger detection — `$prefix` or `${prefix}`.
///
/// Returns `Some((trigger_kind, partial))` where `trigger_kind`
/// is `'$'` for plain `$` triggers and `'{'` for `${name}`
/// braced triggers.  Returns `None` when the cursor is not in
/// a variable-completion context.
///
/// Walks left from the cursor over identifier-continuation
/// characters (alphanumeric, `_`, `:`), then checks whether the
/// character preceding that run forms a recognised trigger:
/// `$` for plain triggers, `${` for braced triggers.
fn variable_trigger(source: &str, line: u32, character: u32) -> Option<(char, String)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());

    // Run of identifier-continuation chars immediately before
    // the cursor — that is the partial.
    let mut start = col;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == ':')
    {
        start -= 1;
    }
    let partial: String = chars[start..col].iter().collect();

    // What sits immediately before the partial run?
    if start == 0 {
        return None;
    }
    if chars[start - 1] == '$' {
        return Some(('$', partial));
    }
    if start >= 2 && chars[start - 2] == '$' && chars[start - 1] == '{' {
        return Some(('{', partial));
    }
    None
}

/// Word-partial extraction for proc-name completion.
///
/// Returns the run of identifier-like chars immediately to the
/// left of the cursor (the "partial" the user has typed so far).
fn word_partial_at_position(source: &str, line: u32, character: u32) -> String {
    let Some(line_text) = source.split('\n').nth(line as usize) else {
        return String::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let mut start = col;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == ':')
    {
        start -= 1;
    }
    chars[start..col].iter().collect()
}

fn variable_completions(scope: &Scope, partial: &str, trigger: char) -> Vec<CompletionItem> {
    let prefix = partial;
    let mut items = Vec::new();
    let mut names: Vec<&str> = scope
        .variables
        .keys()
        .filter(|n| n.starts_with(prefix))
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    for name in names {
        let label = if trigger == '{' {
            format!("${{{name}}}")
        } else {
            format!("${name}")
        };
        items.push(CompletionItem {
            label,
            insert_text: name.to_owned(),
            kind: CompletionKind::Variable,
        });
    }
    items
}

fn proc_completions(analysis: &AnalysisResult, partial: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut names: Vec<(&str, &ProcDef)> = analysis
        .all_procs
        .iter()
        .filter_map(|(qname, proc_def)| {
            // Match either the simple name or the qualified
            // name — the user may have typed the leading `::`.
            if proc_def.name.starts_with(partial) || qname.starts_with(partial) {
                Some((qname.as_str(), proc_def))
            } else {
                None
            }
        })
        .collect();
    names.sort_unstable_by_key(|(qname, _)| *qname);
    for (qname, proc_def) in names {
        items.push(CompletionItem {
            label: proc_def.name.clone(),
            insert_text: qname.to_owned(),
            kind: CompletionKind::Function,
        });
    }
    items
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
    fn dollar_trigger_at_eol_returns_partial() {
        let (trigger, partial) = variable_trigger("set x $", 0, 7).unwrap();
        assert_eq!(trigger, '$');
        assert_eq!(partial, "");
    }

    #[test]
    fn dollar_trigger_with_partial_name() {
        let (trigger, partial) = variable_trigger("set y $foo", 0, 10).unwrap();
        assert_eq!(trigger, '$');
        assert_eq!(partial, "foo");
    }

    #[test]
    fn brace_trigger_recognised() {
        let (trigger, partial) = variable_trigger("set y ${ba", 0, 10).unwrap();
        assert_eq!(trigger, '{');
        assert_eq!(partial, "ba");
    }

    #[test]
    fn space_resets_trigger_context() {
        // The `$` is followed by a space + new identifier — no
        // variable trigger active at the cursor.
        assert!(variable_trigger("set $ ab", 0, 8).is_none());
    }

    #[test]
    fn variable_completion_lists_globals_alphabetically() {
        let src = "set apple 1\nset banana 2\nset $\n";
        let analysis = analyse(src);
        // Cursor after `$` on the third line.
        let items = completions(src, 2, 5, &analysis);
        assert!(!items.is_empty(), "expected variable completions");
        assert_eq!(items[0].kind, CompletionKind::Variable);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Alphabetical order.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted);
        assert!(labels.contains(&"$apple"));
        assert!(labels.contains(&"$banana"));
    }

    #[test]
    fn variable_completion_filters_by_partial() {
        let src = "set apple 1\nset banana 2\nset $b\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 6, &analysis);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["$banana"]);
    }

    #[test]
    fn proc_completion_lists_user_defined_procs() {
        let src = "proc greet {} {}\nproc shout {} {}\ng\n";
        let analysis = analyse(src);
        // Cursor right after `g` on third line.
        let items = completions(src, 2, 1, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].kind, CompletionKind::Function);
        assert_eq!(items[0].label, "greet");
    }

    #[test]
    fn empty_partial_lists_all_procs() {
        let src = "proc alpha {} {}\nproc beta {} {}\n\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 0, &analysis);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"alpha"));
        assert!(labels.contains(&"beta"));
    }
}
