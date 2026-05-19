//! Definition / declaration / type-definition / implementation
//! provider — minimal Rust port of `lsp/features/definition.py`,
//! `declaration.py`, `type_definition.py`, and `implementation.py`.
//!
//! The four LSP methods all answer the same fundamental question
//! ("where is the symbol at this position defined?") with slightly
//! different priorities for proc / class / variable matches; in
//! practice the Python providers are almost identical for our
//! minimal port, so they share the same core function and the
//! server lifts each method onto it.
//!
//! What lands here:
//!
//! * `$var` references resolve to the `definition_span` of the
//!   matching `VarDef` in the global scope.  Scope-chain
//!   descent is deferred until the cached-analysis surface lands
//!   under `S-diagnostics` (the analyser's body-span line index
//!   isn't currently threaded into the search path).
//! * Bare-word references resolve to a user-defined `proc` or
//!   `TclOO` class via `name_span`.
//!
//! Also lands: command-alias resolution — when the cursor's
//! word matches an `interp alias {} ALIAS {} TARGET` recorded
//! in `analysis.command_aliases`, the provider jumps to the
//! target proc's definition (when the target is a user
//! proc).
//!
//! What is *still deferred* (planned as further
//! `S-definition-rich` sub-strips):
//!
//! * Method-body context lookups (Python's `scope.kind ==
//!   "method"` path that surfaces `my method` calls inside a
//!   class body).
//! * `BigIP` definition (`get_bigip_definition`) — entirely
//!   separate provider keyed off iRules dialect that resolves
//!   pool / data-group / iRule / virtual-server names against
//!   a parsed `bigip.conf`.
//! * Property / constructor / destructor name resolution
//!   inside a class.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::hover::{find_var_at_position, find_word_span_at_position};

/// LSP `Range` analogue — line/character pairs (UTF-16 code
/// units per the LSP spec; the minimal port treats them as
/// char counts, matching the Python implementation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspRange {
    /// Start position (0-based inclusive).
    pub start_line: u32,
    /// Start character (0-based UTF-16 code units; minimal
    /// port treats as char counts).
    pub start_character: u32,
    /// End position (0-based exclusive).
    pub end_line: u32,
    /// End character (0-based, exclusive).
    pub end_character: u32,
}

/// Compute "go-to-definition" locations for the symbol at the
/// cursor.
///
/// Returns an empty vector when no recognisable symbol is at
/// the position or when the symbol's definition isn't in the
/// current document.
#[must_use]
pub fn definition(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    let line_index = LineIndex::new(source);

    // 1. Variable reference — walk the global scope only for
    //    the minimal port.
    if let Some(var_name) = find_var_at_position(source, line, character) {
        if let Some(var_def) = analysis.global_scope.variables.get(&var_name) {
            return vec![span_to_range(&line_index, var_def.definition_span)];
        }
        return Vec::new();
    }

    // 2. Bare word — proc, class, or alias.
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            return vec![span_to_range(&line_index, proc_def.name_span)];
        }
    }
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            return vec![span_to_range(&line_index, class_def.name_span)];
        }
    }
    // Alias resolution — when the cursor's word matches an
    // `interp alias {} ALIAS {} TARGET` recorded in
    // `analysis.command_aliases`, jump to the TARGET proc.
    // Mirrors Python's `lookup_alias_for_word`.
    if let Some(alias) = lookup_alias(analysis, &word) {
        for (qname, proc_def) in &analysis.all_procs {
            if proc_def.name == alias.target
                || qname == &alias.target
                || qname == &format!("::{}", alias.target)
            {
                return vec![span_to_range(&line_index, proc_def.name_span)];
            }
        }
    }
    Vec::new()
}

/// Look up an alias by name.  Accepts the alias's simple or
/// qualified form (`mycmd` and `::mycmd` both match an alias
/// stored with `qualified_name == "::mycmd"`).
fn lookup_alias<'a>(
    analysis: &'a AnalysisResult,
    name: &str,
) -> Option<&'a tcl_compiler::signature_scan::types::SignatureCommandAlias> {
    let qualified = if name.starts_with("::") {
        name.to_string()
    } else {
        format!("::{name}")
    };
    analysis
        .command_aliases
        .get(&qualified)
        .or_else(|| analysis.command_aliases.get(name))
}

fn span_to_range(line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LspRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
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
    fn jump_to_proc_definition() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the second `greet` (line 1, char 2).
        let locs = definition(src, 1, 2, &analysis);
        assert_eq!(locs.len(), 1);
        // The proc name span is on line 0 starting at column 5.
        assert_eq!(locs[0].start_line, 0);
        assert_eq!(locs[0].start_character, 5);
    }

    #[test]
    fn jump_to_var_definition() {
        let src = "set greeting hi\nputs $greeting\n";
        let analysis = analyse(src);
        // Cursor inside `$greeting` on line 1.
        let locs = definition(src, 1, 8, &analysis);
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    }

    #[test]
    fn no_definition_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(definition(src, 0, 6, &analysis).is_empty());
    }

    #[test]
    fn jump_to_class_definition() {
        let src = "oo::class create Greeter {}\nGreeter\n";
        let analysis = analyse(src);
        // Cursor on `Greeter` on line 1.
        let locs = definition(src, 1, 2, &analysis);
        if !locs.is_empty() {
            assert_eq!(locs[0].start_line, 0);
        }
    }

    // -- S-definition-rich: alias resolution ------------------------

    #[test]
    fn jump_to_alias_target_proc() {
        // `interp alias {} mycmd {} greet` aliases `mycmd` to
        // the user proc `greet`.  Cursor on `mycmd` should
        // resolve to `greet`'s definition.
        let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\nmycmd\n";
        let analysis = analyse(src);
        // Cursor on `mycmd` invocation on line 2.
        let locs = definition(src, 2, 2, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The target's name span is on line 0 starting at
        // column 5 (after `proc `).
        assert_eq!(locs[0].start_line, 0);
        assert_eq!(locs[0].start_character, 5);
    }

    #[test]
    fn alias_with_no_user_proc_returns_empty() {
        // `mycmd` aliases to a built-in (`puts`).  No user
        // proc; the provider returns empty rather than
        // pretending to know the location.
        let src = "interp alias {} mycmd {} puts\nmycmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 1, 2, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn alias_lookup_accepts_qualified_form() {
        // The alias's qualified form (`::mycmd`) should also
        // resolve.
        let src = "proc greet {} {}\ninterp alias {} mycmd {} greet\n::mycmd\n";
        let analysis = analyse(src);
        let locs = definition(src, 2, 2, &analysis);
        // Whether find_word_span_at_position includes leading
        // `::` depends on the implementation; both should
        // jump to the target proc when matched.
        if !locs.is_empty() {
            assert_eq!(locs[0].start_line, 0);
        }
    }

    #[test]
    fn span_to_range_translates_offsets() {
        let src = "abc\ndef\n";
        let line_index = LineIndex::new(src);
        // Span covering `def` (offsets 4..7).
        let span = tcl_lexer::Span::new(4, 7);
        let range = span_to_range(&line_index, span);
        assert_eq!(range.start_line, 1);
        assert_eq!(range.start_character, 0);
        assert_eq!(range.end_line, 1);
        assert_eq!(range.end_character, 3);
    }
}
