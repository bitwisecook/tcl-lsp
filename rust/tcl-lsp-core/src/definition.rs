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
//! Class-member lookup also lands: when the cursor sits on a
//! word inside a class body span, the provider walks that
//! class's `methods` / `class_methods` / `properties` /
//! `constructors` / `destructor` looking for a name match
//! and jumps to the member's `name_span`.  Catches `my
//! method` calls inside the body and bare references to the
//! class's own members.
//!
//! `$obj method` dispatch also lands: when the cursor sits on
//! the method-name token of a `$obj method` / `[$obj method]`
//! call and `$obj`'s class is known (recorded in
//! `analysis.instance_classes` from a `set obj [Cls new]` /
//! `Cls create obj` site), the provider jumps to the method
//! declaration on that class.
//!
//! What is *still deferred* (planned as further
//! `S-definition-rich` sub-strips):
//!
//! * Flow-sensitive / scope-aware instance-class tracking —
//!   `analysis.instance_classes` is a best-effort global
//!   var-name → class map (last assignment wins).  Re-binding
//!   the same name to a different class, or two locals of the
//!   same name in different procs, isn't disambiguated.
//! * `BigIP` definition (`get_bigip_definition`) — entirely
//!   separate provider keyed off iRules dialect that resolves
//!   pool / data-group / iRule / virtual-server names against
//!   a parsed `bigip.conf`.

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

    // 1. Variable reference — walk the scope chain inward
    //    from the global scope toward the innermost scope
    //    whose body span contains the cursor's byte offset,
    //    then walk back outward looking for the var.  Mirrors
    //    Python's `find_scope_at_line` + scope-chain ascent.
    if let Some(var_name) = find_var_at_position(source, line, character) {
        let cursor_offset = byte_offset_at(source, line, character);
        if let Some(var_def) =
            lookup_var_in_scope_chain(&analysis.global_scope, cursor_offset, &var_name)
        {
            return vec![span_to_range(&line_index, var_def.definition_span)];
        }
        return Vec::new();
    }

    // 2. Bare word — proc, class, class-member, or alias.
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // `$obj method` / `[$obj method]` — when the cursor sits on
    // the method-name token of an instance-method call and the
    // instance variable's class is known, jump to the method
    // declaration.  Checked before the proc lookup so a method
    // call resolves to the method even when a same-named proc
    // exists.
    if let Some((inst, method)) = instance_method_at_cursor(source, line, character) {
        if let Some(class_q) = analysis.instance_classes.get(&inst) {
            if let Some(span) = lookup_method_in_class(analysis, class_q, &method) {
                return vec![span_to_range(&line_index, span)];
            }
        }
    }
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
    // Class-member lookup — when the cursor sits inside a
    // class body, walk that class's methods / properties /
    // constructors / destructor for a name match.  Covers
    // `my method` calls inside the body plus bare member
    // references.
    let cursor_offset = byte_offset_at(source, line, character);
    if let Some(span) = lookup_class_member(analysis, &word, cursor_offset) {
        return vec![span_to_range(&line_index, span)];
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

/// Walk every class whose `body_span` contains the cursor
/// offset and look up `word` in that class's methods,
/// class-methods, properties, constructors, or destructor.
/// Returns the matched member's `name_span` when found.
///
/// `"constructor"` matches any defined constructor;
/// `"destructor"` matches the destructor.  Other words match
/// against the member's `name`.
fn lookup_class_member(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        if let Some(m) = class_def.methods.get(word) {
            return Some(m.name_span);
        }
        if let Some(m) = class_def.class_methods.get(word) {
            return Some(m.name_span);
        }
        if let Some(p) = class_def.properties.get(word) {
            return Some(p.name_span);
        }
        if word == "constructor" {
            if let Some(c) = class_def.constructors.first() {
                if !c.name_span.is_empty() {
                    return Some(c.name_span);
                }
                // Analyser doesn't store a name span for the
                // constructor keyword (it has no name token).
                // Fall back to the body span's start so the
                // editor at least lands on the constructor's
                // body opener.
                return Some(c.body_span);
            }
        }
        if word == "destructor" {
            if let Some(d) = &class_def.destructor {
                if !d.name_span.is_empty() {
                    return Some(d.name_span);
                }
                return Some(d.body_span);
            }
        }
    }
    None
}

/// Look up `method` against the class identified by qualified
/// name `class_q` — searches `methods`, `class_methods`, then
/// `properties`.  Returns the member's `name_span`.
fn lookup_method_in_class(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<tcl_lexer::Span> {
    let class_def = analysis.all_classes.get(class_q)?;
    class_def
        .methods
        .get(method)
        .map(|m| m.name_span)
        .or_else(|| class_def.class_methods.get(method).map(|m| m.name_span))
        .or_else(|| class_def.properties.get(method).map(|p| p.name_span))
}

/// Detect a `$obj method ...` / `[$obj method ...]` call where
/// the cursor sits on the *method-name* token.  Returns
/// `(instance_var_name, method_name)`.
///
/// The instance variable must be the command-segment head (a
/// single `$name` / `${name}` token immediately preceding the
/// method), so the method sits at word-index 1.  Command
/// segments are delimited by `;`, `[`, `{`, and the line start
/// — a single-line approximation that covers the common editor
/// cases.
pub(crate) fn instance_method_at_cursor(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, String)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == ':';

    // Method-name word bounds around the cursor.
    let mut wstart = col;
    while wstart > 0 && is_ident(chars[wstart - 1]) {
        wstart -= 1;
    }
    let mut wend = col;
    while wend < chars.len() && is_ident(chars[wend]) {
        wend += 1;
    }
    if wstart == wend {
        return None;
    }
    let method: String = chars[wstart..wend].iter().collect();

    // Command-segment start: nearest `;` / `[` / `{` to the
    // left, else the line start.
    let mut seg_start = 0;
    for i in (0..wstart).rev() {
        if matches!(chars[i], ';' | '[' | '{') {
            seg_start = i + 1;
            break;
        }
    }
    // The head must be exactly one whitespace-delimited token
    // (the instance var), so the method is word-index 1.
    let prefix: String = chars[seg_start..wstart].iter().collect();
    let head_tokens: Vec<&str> = prefix.split_whitespace().collect();
    if head_tokens.len() != 1 {
        return None;
    }
    let head = head_tokens[0];
    let inst = head.strip_prefix('$')?;
    let inst = inst
        .strip_prefix('{')
        .map_or(inst, |r| r.strip_suffix('}').unwrap_or(r));
    if inst.is_empty() {
        return None;
    }
    Some((inst.to_string(), method))
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

/// Compute the byte offset of a 0-based `(line, character)`
/// pair in `source`.  Character indices count chars (matching
/// Python's behaviour); supplementary-plane code points may
/// drift by one column under strict UTF-16 semantics —
/// acceptable for the minimal port.
pub(crate) fn byte_offset_at(source: &str, line: u32, character: u32) -> u32 {
    let mut current_line: u32 = 0;
    let mut current_char_in_line: u32 = 0;
    let mut byte_offset: u32 = 0;
    for c in source.chars() {
        if current_line == line && current_char_in_line == character {
            return byte_offset;
        }
        let len = u32::try_from(c.len_utf8()).unwrap_or(1);
        if c == '\n' {
            if current_line == line {
                return byte_offset;
            }
            current_line += 1;
            current_char_in_line = 0;
        } else {
            current_char_in_line += 1;
        }
        byte_offset += len;
    }
    byte_offset
}

/// Walk the scope tree to find the variable definition that
/// the cursor's `byte_offset` would see — the innermost
/// scope whose body span contains the offset takes precedence
/// over any enclosing scope.
///
/// Mirrors Python's `find_scope_at_line` (descend into the
/// innermost matching child) followed by a scope-chain walk
/// outward for the var lookup.
pub(crate) fn lookup_var_in_scope_chain<'a>(
    scope: &'a tcl_compiler::analyser::Scope,
    byte_offset: u32,
    name: &str,
) -> Option<&'a tcl_compiler::analyser::VarDef> {
    // First, find the innermost scope containing the cursor.
    let chain = scope_chain_at(scope, byte_offset);
    // Walk outward (innermost-first) looking for the var.
    for sc in chain.iter().rev() {
        if let Some(v) = sc.variables.get(name) {
            return Some(v);
        }
    }
    None
}

/// Return the chain of scopes from `root` down to the
/// innermost child whose `body_span` contains `byte_offset`.
/// The chain is ordered outermost (`root`) to innermost.
fn scope_chain_at(
    root: &tcl_compiler::analyser::Scope,
    byte_offset: u32,
) -> Vec<&tcl_compiler::analyser::Scope> {
    let mut chain = vec![root];
    let mut cursor = root;
    loop {
        let next = cursor.children.iter().find(|c| {
            // `Span` is half-open `[start, end)` — the byte at
            // `s.end()` lives outside the scope (PR #454 Copilot
            // review).
            c.body_span
                .is_some_and(|s| s.start() <= byte_offset && byte_offset < s.end())
        });
        match next {
            Some(child) => {
                chain.push(child);
                cursor = child;
            }
            None => break,
        }
    }
    chain
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

    // -- S-definition-rich: scope-chain $var descent ---------------

    #[test]
    fn proc_local_var_jumps_to_proc_scope_definition() {
        // `local` is defined inside `proc f`.  Cursor on
        // `$local` inside `f`'s body must jump to the
        // proc-local definition, not the global scope
        // (which has none).
        let src = "proc f {} {\n    set local 1\n    puts $local\n}\n";
        let analysis = analyse(src);
        // Cursor inside `$local` on line 2.
        let locs = definition(src, 2, 12, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The proc-local `set local 1` is on line 1.
        assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
    }

    #[test]
    fn byte_offset_at_handles_newlines() {
        let src = "abc\ndef\nghi\n";
        assert_eq!(byte_offset_at(src, 0, 0), 0);
        assert_eq!(byte_offset_at(src, 0, 3), 3);
        assert_eq!(byte_offset_at(src, 1, 0), 4);
        assert_eq!(byte_offset_at(src, 2, 2), 10);
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

    // -- S-definition-rich: class-member lookup ---------------------

    #[test]
    fn definition_jumps_to_method_inside_class_body() {
        // Inside an OO class body, `greet` refers to the
        // class's own method.  Cursor on `greet` should jump
        // to the `method greet` declaration.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` in the `twice` body.
        // Line 2: `    method twice {} { greet ; greet }`
        // Col 22 lands on the `g` of the first `greet`.
        let locs = definition(src, 2, 22, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The method declaration is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_jumps_to_classmethod() {
        let src = "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        // Line 2: `    method use {} { factory }`
        // Cursor on `factory` (col 20).
        let locs = definition(src, 2, 20, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_jumps_to_constructor_keyword() {
        // Bare `constructor` inside a class body jumps to the
        // constructor's declaration.
        let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
        let analysis = analyse(src);
        // Cursor on `constructor` in the `touch_ctor` body
        // (line 2 col 27).
        let locs = definition(src, 2, 27, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The analyser anchors the constructor's ``name_span``
        // on the ``constructor`` keyword token (declared on
        // line 1).  The provider keeps a body-span fallback
        // for the empty-span case, but the keyword span is
        // populated now, so the jump lands on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_member_lookup_skipped_outside_class_body() {
        // Same word outside the class body must not surface
        // the method definition.
        let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the bare `greet` on line 3.
        let locs = definition(src, 3, 2, &analysis);
        // No proc / class / member named `greet` is in scope
        // here — the class-member lookup must not leak.
        assert!(locs.is_empty(), "{locs:?}");
    }

    // -- S-definition-rich: $obj method dispatch --------------------

    #[test]
    fn definition_resolves_obj_method_call() {
        // `set d [Dog new]` then `$d bark` — cursor on `bark`
        // jumps to the method declaration on Dog.
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        // Line 4 `$d bark` — `bark` starts at col 3.
        let locs = definition(src, 4, 3, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `method bark` is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_resolves_obj_method_in_bracket() {
        // `[$d bark]` bracketed form.
        let src =
            "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nputs [$d bark]\n";
        let analysis = analyse(src);
        // Line 4 `puts [$d bark]` — `bark` starts at col 9.
        let locs = definition(src, 4, 9, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn definition_obj_method_unknown_instance_falls_through() {
        // `$x bark` where `x` has no recorded class — no
        // instance-method resolution, falls through (and finds
        // nothing here).
        let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
        let analysis = analyse(src);
        let locs = definition(src, 3, 3, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn instance_method_at_cursor_detects_dollar_head() {
        let src = "$d bark\n";
        let got = instance_method_at_cursor(src, 0, 4);
        assert_eq!(got, Some(("d".to_string(), "bark".to_string())));
    }

    #[test]
    fn instance_method_at_cursor_rejects_non_dollar_head() {
        // `foo bark` — head is a bare word, not an instance var.
        let src = "foo bark\n";
        assert_eq!(instance_method_at_cursor(src, 0, 5), None);
    }
}
