//! Completion provider — Rust port of
//! `lsp/features/completion.py`.
//!
//! Wires the LSP completion surface for the three Tcl
//! completion contexts the analyser surfaces today:
//!
//! * **Variable completion** — when the cursor sits immediately
//!   after `$` (or inside a `${name}` braced reference), suggest
//!   every variable name visible in the global scope.
//! * **Proc-name completion** — at any other cursor position,
//!   suggest every user-defined `proc` name that the analyser
//!   recorded for the document.
//! * **Built-in command completion** — at the same cursor
//!   contexts as proc-name completion, also suggest every
//!   command registered in the caller-provided
//!   [`tcl_registry::CommandRegistry`].  This is part of the
//!   `S-completion-rich` follow-up; the caller (server)
//!   threads its already-built per-dialect registry through.
//!   When no registry is provided, the completion surface
//!   degrades cleanly to the minimal port's proc-only set.
//!
//! What is *still deferred* (planned as further
//! `S-completion-rich` follow-ups):
//!
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
use tcl_registry::CommandRegistry;

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
/// (server) is expected to cache it.  `registry`, when `Some`,
/// extends proc-name completion with built-in command names —
/// every command registered in the caller's dialect-aware
/// registry surfaces at the same cursor contexts.  Returns an
/// empty vector when there is no useful suggestion (delimiter
/// run, EOF, etc.).
#[must_use]
pub fn completions(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Vec<CompletionItem> {
    if let Some((trigger, partial)) = variable_trigger(source, line, character) {
        return variable_completions(&analysis.global_scope, &partial, trigger);
    }
    let partial = word_partial_at_position(source, line, character);
    let mut items = proc_completions(analysis, &partial);
    if let Some(registry) = registry {
        items.extend(builtin_completions(registry, &partial));
    }
    items
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

/// Math operators that the registry registers as commands
/// (Tcl 9's `tcl::mathop` exposes them as commands) but that
/// don't make sense as completion items at a command position.
/// Mirrors the same filter applied in
/// `lsp/features/completion.py::426-428`.
const SKIP_BUILTIN_NAMES: &[&str] = &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="];

fn builtin_completions(registry: &CommandRegistry, partial: &str) -> Vec<CompletionItem> {
    let mut names: Vec<&str> = registry
        .command_names()
        .filter(|n| partial.is_empty() || n.starts_with(partial))
        .filter(|n| !SKIP_BUILTIN_NAMES.iter().any(|skip| skip == n))
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| CompletionItem {
            label: name.to_owned(),
            insert_text: name.to_owned(),
            kind: CompletionKind::Function,
        })
        .collect()
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
        let items = completions(src, 2, 5, &analysis, None);
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
        let items = completions(src, 2, 6, &analysis, None);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["$banana"]);
    }

    #[test]
    fn proc_completion_lists_user_defined_procs() {
        let src = "proc greet {} {}\nproc shout {} {}\ng\n";
        let analysis = analyse(src);
        // Cursor right after `g` on third line.
        let items = completions(src, 2, 1, &analysis, None);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].kind, CompletionKind::Function);
        assert_eq!(items[0].label, "greet");
    }

    #[test]
    fn empty_partial_lists_all_procs() {
        let src = "proc alpha {} {}\nproc beta {} {}\n\n";
        let analysis = analyse(src);
        let items = completions(src, 2, 0, &analysis, None);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"alpha"));
        assert!(labels.contains(&"beta"));
    }

    // -- S-completion-rich: built-in command completion --------------
    //
    // Tests pin the contract that a non-`None` registry parameter
    // extends proc-name completion with every command the
    // registry knows about, filtered by the partial typed at the
    // cursor.

    #[test]
    fn builtin_completion_lists_registry_commands_at_command_position() {
        // No user-defined procs, partial `pu` → `puts` (and any
        // other registered command beginning with `pu`) should
        // surface.
        let src = "pu\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 2, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"puts"),
            "expected `puts` from registry; got {labels:?}",
        );
    }

    #[test]
    fn builtin_completion_filters_by_partial() {
        // Partial `whi` should yield `while` but not unrelated
        // commands like `puts`.
        let src = "whi\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 3, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"while"),
            "expected `while`; got {labels:?}"
        );
        assert!(
            !labels.contains(&"puts"),
            "should NOT contain `puts` for partial `whi`; got {labels:?}",
        );
    }

    #[test]
    fn builtin_completion_skips_math_operators() {
        // The registry registers `+` / `-` / `*` / etc. as
        // commands (Tcl 9 `tcl::mathop`), but they don't make
        // sense as completion items at a command position.
        let src = "+\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 1, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for op in &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="] {
            assert!(
                !labels.contains(op),
                "math operator `{op}` should be filtered out; got {labels:?}",
            );
        }
    }

    #[test]
    fn builtin_completion_none_registry_keeps_minimal_behaviour() {
        // Passing `None` for the registry must not regress the
        // minimal port's behaviour — proc-name completion alone.
        let src = "proc helper {} {}\nhe\n";
        let analysis = analyse(src);
        let items_no_registry = completions(src, 1, 2, &analysis, None);
        let labels_no_registry: Vec<&str> =
            items_no_registry.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels_no_registry.contains(&"helper"),
            "expected `helper` proc; got {labels_no_registry:?}",
        );
        // No registry commands surface without the registry.
        assert_eq!(items_no_registry.len(), 1, "{items_no_registry:?}");
    }

    #[test]
    fn builtin_completion_merges_procs_and_registry() {
        // Both a user-defined proc and built-in commands should
        // surface when both apply.
        let src = "proc parade {} {}\npar\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 3, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"parade"),
            "expected user proc `parade`; got {labels:?}",
        );
        // `parray` is a stdlib command starting with `par`.
        assert!(
            labels.contains(&"parray"),
            "expected built-in `parray`; got {labels:?}",
        );
    }

    #[test]
    fn builtin_completion_skipped_inside_variable_trigger() {
        // Variable trigger (`$par`) must take precedence and
        // suppress built-in command completions even when a
        // registry is supplied.  Mirrors the
        // `variable_completions` short-circuit at the top of
        // `completions`.
        let src = "set apple 1\nset $par\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 8, &analysis, Some(&registry));
        // Only variable completions allowed here.
        for it in &items {
            assert_eq!(
                it.kind,
                CompletionKind::Variable,
                "variable trigger should suppress built-ins; got {it:?}",
            );
        }
    }
}
