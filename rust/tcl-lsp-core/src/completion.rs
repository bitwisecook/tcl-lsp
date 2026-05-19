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
//! * iRules `call proc_name` first-arg context and other
//!   dialect-specific arg rules.  (`when EVENT` enumeration
//!   has landed: any command carrying
//!   `Traits::IS_EVENT_HANDLER` triggers event-name completion
//!   from the shared `EventRegistry` at word-index 1.)
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
/// Mirrors the subset of `lsprotocol.types.CompletionItem` we
/// emit today: label, insert-text, kind, and an optional
/// detail line.  The detail is what the editor shows in the
/// right-hand column of the completion list (typically a
/// parameter-list summary for procs).
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
    /// Optional detail text — typically a parameter-list
    /// summary for procs / a synopsis line for built-in
    /// commands.  `None` for items without extra detail.
    pub detail: Option<String>,
    /// Optional sort key.  When `Some`, the editor uses this
    /// string instead of the label for ordering completion
    /// results.  `S-completion-rich`'s usage-bucket scheme:
    /// `A<tier><usage>_<name>` for user procs, `B<usage>_<name>`
    /// for built-in commands.  Lower buckets sort first.
    pub sort_text: Option<String>,
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
///
/// The trigger contexts checked in order:
///
/// 1. **Variable trigger** (`$prefix` / `${prefix}`) — short-
///    circuits to global-scope variable names.
/// 2. **Switch completion** — when the partial starts with `-`
///    and the surrounding command's spec declares matching
///    options.  Requires `registry`.
/// 3. **Subcommand completion** — when the cursor is at word
///    index 1 (i.e. the first argument of a command) and the
///    surrounding command's spec declares subcommands.
///    Requires `registry`.
/// 4. **Command + proc completion** — default fallback:
///    user-defined procs from `analysis.all_procs`, plus all
///    built-in commands the `registry` knows about.
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

    // Context-aware completions — switch + subcommand + event-name.
    // All three require the caller-provided registry to look up
    // the surrounding command's spec.  Without a registry we
    // can't tell which switches / subcommands / events are valid,
    // so fall through to plain command + proc completion.
    if let Some(registry) = registry {
        if let Some((cmd, word_idx)) = command_context_on_line(source, line, character) {
            if let Some(spec) = registry.get(&cmd) {
                // Switch completion fires when the identifier
                // partial is preceded by a literal `-` on the
                // line.  `word_partial_at_position` stops at the
                // dash (it's not an identifier char), so detect
                // the dash here and rebuild the switch partial.
                if let Some(switch_partial) =
                    switch_partial_at_position(source, line, character, &partial)
                {
                    if !spec.options.is_empty() {
                        return switch_completions(spec, &switch_partial);
                    }
                }
                // iRules `when EVENT { body }`: when the cursor is
                // typing the first argument of an event-handler
                // command, enumerate the known event names from the
                // shared event registry.
                if word_idx == 1 && spec.traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER) {
                    return event_name_completions(&partial);
                }
                if word_idx == 1 && !spec.subcommands.is_empty() {
                    return subcommand_completions(spec, &partial);
                }
            }
        }
    }

    let usage = document_usage_counts(analysis);
    let mut items = proc_completions(analysis, &partial, &usage);
    if let Some(registry) = registry {
        items.extend(builtin_completions(registry, &partial, &usage));
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
            detail: None,
            sort_text: None,
        });
    }
    items
}

/// Determine the surrounding command's name and the cursor's
/// word index on the current line.  Returns `(command,
/// word_index)` where `word_index` is the 0-based position of
/// the cursor (0 = typing the command name, 1 = typing the
/// first argument, etc.).
///
/// **Single-line context only.**  Continuation lines, embedded
/// `[…]` / `{…}` token nesting, and `;` command separators are
/// deferred to the same multi-line-aware machinery
/// `S-signature-help-rich` will eventually land — see
/// `core/parsing/find_command_context_*` for the Python
/// reference.  The single-line approach covers the common
/// editor cases (cursor on the same logical line as the
/// command head) and shares its shape with the single-line
/// helper in [`crate::signature_help`].
fn command_context_on_line(source: &str, line: u32, character: u32) -> Option<(String, usize)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let prefix: String = chars[..col].iter().collect();

    // Tokenise on whitespace.  The first token is the command;
    // the count of subsequent tokens (adjusted for whether the
    // cursor sits mid-token) gives the word index.
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let command = tokens[0].to_owned();

    // If the prefix ends with whitespace the cursor is *between*
    // tokens — at the start of a fresh word.  Otherwise the
    // cursor is inside the last token.
    let ends_in_space = prefix.ends_with(|c: char| c.is_whitespace());
    let word_index = if ends_in_space {
        tokens.len()
    } else {
        tokens.len().saturating_sub(1)
    };
    Some((command, word_index))
}

/// Detect a `-switch` partial at the cursor.  Returns
/// `Some(switch_partial)` (including the leading dash) when
/// the character immediately preceding the identifier run
/// before the cursor is a `-`; returns `None` otherwise.
///
/// `partial` is the identifier-only partial computed by
/// [`word_partial_at_position`]; the helper reconstructs the
/// switch-aware partial by prepending the dash without
/// re-walking the line.
fn switch_partial_at_position(
    source: &str,
    line: u32,
    character: u32,
    partial: &str,
) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let start = col.checked_sub(partial.chars().count())?;
    if start == 0 {
        return None;
    }
    if chars[start - 1] != '-' {
        return None;
    }
    Some(format!("-{partial}"))
}

fn switch_completions(spec: &tcl_registry::CommandSpec, partial: &str) -> Vec<CompletionItem> {
    let mut names: Vec<&str> = spec
        .options
        .iter()
        .map(|opt| opt.name)
        .filter(|n| partial.is_empty() || n.starts_with(partial))
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| CompletionItem {
            label: name.to_owned(),
            insert_text: name.to_owned(),
            kind: CompletionKind::Function,
            detail: None,
            sort_text: None,
        })
        .collect()
}

/// Compute iRules event-name completions for `when EVENT { body }`.
/// Returns one [`CompletionItem`] per event registered in the
/// shared [`tcl_registry::events::EventRegistry`] whose name starts
/// with `partial` (case-sensitive — event names are all-uppercase
/// snake-case by convention).
///
/// The registry is built once per call.  `EventRegistry::build`
/// materialises a small static table at runtime; for completion
/// (one call per user keystroke at a `when ` site) that's
/// negligible.  Threading a cached registry through the public
/// `completions()` signature would force every caller to plumb it
/// even when iRules isn't in scope, so we keep it local instead.
fn event_name_completions(partial: &str) -> Vec<CompletionItem> {
    let reg = tcl_registry::events::EventRegistry::build();
    let mut names: Vec<&str> = reg
        .all_event_names()
        .into_iter()
        .filter(|n| partial.is_empty() || n.starts_with(partial))
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| CompletionItem {
            label: name.to_owned(),
            insert_text: name.to_owned(),
            kind: CompletionKind::Function,
            detail: Some("F5 iRules event".to_string()),
            sort_text: None,
        })
        .collect()
}

fn subcommand_completions(spec: &tcl_registry::CommandSpec, partial: &str) -> Vec<CompletionItem> {
    let mut names: Vec<&str> = spec
        .subcommands
        .iter()
        .map(|sub| sub.name)
        .filter(|n| partial.is_empty() || n.starts_with(partial))
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| CompletionItem {
            label: name.to_owned(),
            insert_text: name.to_owned(),
            kind: CompletionKind::Function,
            detail: None,
            sort_text: None,
        })
        .collect()
}

/// Math operators that the registry registers as commands
/// (Tcl 9's `tcl::mathop` exposes them as commands) but that
/// don't make sense as completion items at a command position.
/// Mirrors the same filter applied in
/// `lsp/features/completion.py::426-428`.
const SKIP_BUILTIN_NAMES: &[&str] = &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="];

/// Map a usage count to its sort bucket (lower is better).
/// Mirrors `_usage_bucket` in `lsp/features/completion.py`.
fn usage_bucket(count: usize) -> u8 {
    match count {
        c if c >= 50 => 0,
        c if c >= 20 => 1,
        c if c >= 8 => 2,
        c if c >= 3 => 3,
        c if c >= 1 => 4,
        _ => 5,
    }
}

/// Build a `HashMap<command_name, usage_count>` from the
/// analyser's per-document `command_invocations`.  Used as a
/// best-effort proxy for the Python provider's
/// workspace-wide usage counts until a workspace index lands.
fn document_usage_counts(analysis: &AnalysisResult) -> std::collections::HashMap<String, usize> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for inv in &analysis.command_invocations {
        *counts.entry(inv.name.clone()).or_insert(0) += 1;
        if let Some(q) = inv.resolved_qualified_name.as_deref() {
            if q != inv.name {
                *counts.entry(q.to_owned()).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn builtin_sort_text(name: &str, usage: usize) -> String {
    // `B<usage>_<name>` — built-ins sort after user procs (`A…`)
    // but before any item with no `sort_text`.  The two-digit
    // bucket prevents lexicographic confusion between 1 and 10.
    let rank = usage_bucket(usage);
    format!("B{rank:02}_{name}")
}

fn builtin_completions(
    registry: &CommandRegistry,
    partial: &str,
    usage: &std::collections::HashMap<String, usize>,
) -> Vec<CompletionItem> {
    let mut names: Vec<&str> = registry
        .command_names()
        .filter(|n| partial.is_empty() || n.starts_with(partial))
        .filter(|n| !SKIP_BUILTIN_NAMES.iter().any(|skip| skip == n))
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| {
            let count = usage.get(name).copied().unwrap_or(0);
            CompletionItem {
                label: name.to_owned(),
                insert_text: name.to_owned(),
                kind: CompletionKind::Function,
                detail: None,
                sort_text: Some(builtin_sort_text(name, count)),
            }
        })
        .collect()
}

/// Render a parameter-list summary for a proc completion's
/// `detail` field.  Mirrors Python's `_proc_signature_str`.
/// Returns `"(no args)"` for paramless procs, otherwise a
/// space-separated list with `{name default}` for optional
/// params.
fn proc_signature_str(proc_def: &ProcDef) -> String {
    if proc_def.params.is_empty() {
        return "(no args)".to_string();
    }
    proc_def
        .params
        .iter()
        .map(|p| {
            if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn proc_sort_text(name: &str, usage: usize) -> String {
    // `A<tier><usage>_<name>` — `tier = 0` reserved for
    // single-document user procs (the current minimal port);
    // `tier = 1` opens up once workspace-index lands and
    // we differentiate same-file procs from workspace ones.
    let rank = usage_bucket(usage);
    format!("A0{rank:02}_{name}")
}

fn proc_completions(
    analysis: &AnalysisResult,
    partial: &str,
    usage: &std::collections::HashMap<String, usize>,
) -> Vec<CompletionItem> {
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
        let count = usage
            .get(proc_def.name.as_str())
            .copied()
            .max(usage.get(qname).copied())
            .unwrap_or(0);
        items.push(CompletionItem {
            label: proc_def.name.clone(),
            insert_text: qname.to_owned(),
            kind: CompletionKind::Function,
            detail: Some(proc_signature_str(proc_def)),
            sort_text: Some(proc_sort_text(&proc_def.name, count)),
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

    // -- S-completion-rich: usage-bucket sort-text --------------------

    #[test]
    fn usage_bucket_buckets_per_python_thresholds() {
        // Exhaustive bucket table parity with
        // `lsp/features/completion.py::_usage_bucket`.
        assert_eq!(usage_bucket(0), 5);
        assert_eq!(usage_bucket(1), 4);
        assert_eq!(usage_bucket(2), 4);
        assert_eq!(usage_bucket(3), 3);
        assert_eq!(usage_bucket(7), 3);
        assert_eq!(usage_bucket(8), 2);
        assert_eq!(usage_bucket(19), 2);
        assert_eq!(usage_bucket(20), 1);
        assert_eq!(usage_bucket(49), 1);
        assert_eq!(usage_bucket(50), 0);
        assert_eq!(usage_bucket(1000), 0);
    }

    #[test]
    fn proc_sort_text_has_lower_bucket_for_used_procs() {
        // Three calls to `greet` and zero calls to `shout`.
        // The completion-list partial is empty (cursor on
        // line 6 col 0), so both procs surface — `greet` in
        // bucket 3 (≥3 calls) and `shout` in bucket 5 (no
        // calls).
        let src = "proc greet {} {}\nproc shout {} {}\ngreet\ngreet\ngreet\n\n";
        let analysis = analyse(src);
        let items = completions(src, 5, 0, &analysis, None);
        let greet = items
            .iter()
            .find(|i| i.label == "greet")
            .unwrap_or_else(|| panic!("greet missing from {items:?}"));
        let greet_sort = greet.sort_text.as_deref().unwrap_or("");
        assert!(
            greet_sort.starts_with("A003_"),
            "greet sort_text {greet_sort:?} should land in bucket 3",
        );
        let shout = items
            .iter()
            .find(|i| i.label == "shout")
            .unwrap_or_else(|| panic!("shout missing from {items:?}"));
        let shout_sort = shout.sort_text.as_deref().unwrap_or("");
        assert!(
            shout_sort.starts_with("A005_"),
            "shout sort_text {shout_sort:?} should land in bucket 5",
        );
        // Lower bucket sorts first.
        assert!(greet_sort < shout_sort);
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

    // -- S-completion-rich: subcommand completion --------------------
    //
    // When the cursor sits at word-index 1 of a known command
    // whose spec declares non-empty `subcommands`, the
    // completion surface lists subcommand names (not user procs
    // or unrelated built-ins).

    #[test]
    fn subcommand_completion_surfaces_subcommands_at_word_index_1() {
        // `string l` — partial `l`, cursor at word-index 1 of
        // `string`.  The registry declares many `string`
        // subcommands; expect `length` and similar `l*` ones.
        let src = "string l\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 8, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"length"),
            "expected `length` subcommand; got {labels:?}",
        );
        // No proc / unrelated commands should leak through.
        assert!(
            !labels.contains(&"puts"),
            "subcommand context should not include `puts`; got {labels:?}",
        );
    }

    #[test]
    fn subcommand_completion_lists_all_subcommands_with_empty_partial() {
        // `string ` (cursor just past the space) — empty partial,
        // word-index 1 of `string`.  Should list every `string`
        // subcommand, alphabetically.
        let src = "string \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 7, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // `string` has at minimum `length`, `match`, `range`,
        // `tolower`, `toupper` across all dialects.
        assert!(labels.contains(&"length"), "{labels:?}");
        assert!(labels.contains(&"match"), "{labels:?}");
        // Should be sorted.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted, "expected sorted subcommands");
    }

    #[test]
    fn subcommand_completion_falls_through_when_word_index_not_1() {
        // `string length f` — cursor at word-index 2 (after
        // `length` argument).  Subcommand completion should NOT
        // fire; we fall through to command + proc completion
        // (which would surface built-ins like `foreach`,
        // `format`, etc.).
        let src = "string length f\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 15, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Some f-prefixed command should surface (foreach,
        // format, file…), confirming we fell through.
        assert!(
            labels.iter().any(|l| l.starts_with('f')),
            "expected at least one f-prefixed command via fallback; got {labels:?}",
        );
        // String's `length` subcommand should NOT be in the
        // result — we're past word-index 1.
        assert!(
            !labels.contains(&"length"),
            "should not surface `length` subcommand at word-index 2; got {labels:?}",
        );
    }

    #[test]
    fn subcommand_completion_skipped_when_command_has_no_subcommands() {
        // `puts hel` — `puts` declares no subcommands, so we
        // should fall through to command + proc completion (and
        // ideally find no commands starting with `hel`).
        let src = "proc helper {} {}\nputs hel\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 1, 8, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // The user proc `helper` should surface (fallback path).
        assert!(
            labels.contains(&"helper"),
            "expected `helper` proc via fallback; got {labels:?}",
        );
    }

    // -- S-completion-rich: switch completion ------------------------
    //
    // When the partial starts with `-` and the surrounding
    // command's spec declares matching options, the completion
    // surface lists option names (not subcommands or built-ins).

    #[test]
    fn switch_completion_surfaces_options_for_dash_partial() {
        // `lsearch -n` — partial `-n`.  The `lsearch` spec
        // declares many `-...` options; expect at least one
        // starting with `-n` (`-nocase`).
        let src = "lsearch -n\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 10, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.iter().any(|l| l.starts_with("-n")),
            "expected at least one `-n*` switch; got {labels:?}",
        );
        // No unrelated commands / procs.
        assert!(
            !labels.iter().any(|l| !l.starts_with('-')),
            "all switch-completion items must start with '-'; got {labels:?}",
        );
    }

    #[test]
    fn switch_completion_lists_all_options_for_bare_dash() {
        // `lsearch -` — partial `-`, every option should
        // surface.
        let src = "lsearch -\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 9, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Every result must start with `-`.
        for l in &labels {
            assert!(l.starts_with('-'), "non-switch in result: {l}");
        }
        // Sorted.
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted);
    }

    // -- S-completion-rich: iRules event-name completion -------------
    //
    // When the cursor sits at word-index 1 of an event-handler
    // command (the `when` iRules keyword carries
    // `Traits::IS_EVENT_HANDLER`), the completion surface lists
    // event names from the shared `EventRegistry`.

    fn irules_registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        r
    }

    #[test]
    fn event_completion_surfaces_when_handler_first_arg() {
        // `when HT` inside iRules — should list every event
        // starting with `HT` (HTTP_REQUEST, HTTP_RESPONSE, …).
        let src = "when HT\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 0, 7, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"HTTP_REQUEST"),
            "expected `HTTP_REQUEST`; got {labels:?}",
        );
        assert!(
            labels.iter().all(|l| l.starts_with("HT")),
            "all results should start with `HT`; got {labels:?}",
        );
    }

    #[test]
    fn event_completion_marks_detail_as_irules_event() {
        let src = "when CL\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 0, 7, &analysis, Some(&registry));
        assert!(
            items
                .iter()
                .all(|i| i.detail.as_deref() == Some("F5 iRules event")),
            "every event completion should be labelled as an iRules event; got {items:?}",
        );
    }

    #[test]
    fn event_completion_does_not_fire_in_plain_tcl_dialect() {
        // The default registry doesn't carry the `when` spec —
        // completion should fall through to plain command + proc
        // completion (which won't surface `HTTP_REQUEST`).
        let src = "when HT\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let items = completions(src, 0, 7, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.contains(&"HTTP_REQUEST"),
            "event completion should not fire outside iRules; got {labels:?}",
        );
    }

    #[test]
    fn event_completion_skipped_at_word_index_other_than_1() {
        // `when HTTP_REQUEST f` — word-index 2 (after the event
        // name).  Event completion must not fire.
        let src = "when HTTP_REQUEST f\n";
        let analysis = analyse(src);
        let registry = irules_registry();
        let items = completions(src, 0, 19, &analysis, Some(&registry));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !labels.iter().any(|l| l.contains("HTTP_REQUEST")),
            "event completion should not fire at word-index 2; got {labels:?}",
        );
    }
}
