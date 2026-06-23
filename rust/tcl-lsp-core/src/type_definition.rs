//! `textDocument/typeDefinition` — jump to the class that *types* the
//! symbol at the cursor.
//!
//! Two receiver shapes:
//!
//! - **Variable receiver** (`$obj`) — when the analyser has inferred the
//!   variable's class (the Rust analyser records this in
//!   `AnalysisResult::instance_classes`), jump to that `ClassDef`'s
//!   name span.
//! - **Method receiver** — when the cursor sits inside a class body on a
//!   word that names one of that class's methods, jump to the enclosing
//!   class's definition (the method's owning *type*).
//!
//! Returns an empty vector when no class types the symbol.

use tcl_compiler::analyser::{AnalysisResult, ClassDef};
use tcl_lexer::LineIndex;

use crate::definition::{LspRange, byte_offset_at, span_to_range};
use crate::hover::{find_var_at_position, find_word_span_at_position};

/// Compute "go-to-type-definition" locations for the symbol at the
/// cursor.
#[must_use]
pub fn type_definition(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    let line_index = LineIndex::new(source);

    // 1. Variable receiver: `$obj` with a known instance class.
    if let Some(var_name) = find_var_at_position(source, line, character) {
        if let Some(class_q) = analysis.instance_classes.get(&var_name)
            && let Some(cd) = find_class(analysis, class_q)
        {
            return vec![span_to_range(source, &line_index, cd.name_span)];
        }
        return Vec::new();
    }

    // 2. Method receiver: a bare word inside a class body that names a
    //    method of the enclosing class resolves to that class.
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    let cursor = byte_offset_at(source, line, character);
    if let Some(cd) = innermost_class_containing(analysis, cursor)
        && (cd.methods.contains_key(&word) || cd.class_methods.contains_key(&word))
    {
        return vec![span_to_range(source, &line_index, cd.name_span)];
    }
    Vec::new()
}

/// Resolve a class reference to its `ClassDef`, accepting the bare
/// name and either spelling of the qualified name.
fn find_class<'a>(analysis: &'a AnalysisResult, name: &str) -> Option<&'a ClassDef> {
    // Fast path: `instance_classes` stores the qualified name, which
    // is the `all_classes` key.
    if let Some(cd) = analysis.all_classes.get(name) {
        return Some(cd);
    }
    let stripped = name.trim_start_matches(':');
    analysis.all_classes.values().find(|cd| {
        cd.name == name
            || cd.qualified_name == name
            || cd.qualified_name.trim_start_matches(':') == stripped
    })
}

/// The innermost class whose `body_span` contains the cursor offset.
fn innermost_class_containing(analysis: &AnalysisResult, cursor: u32) -> Option<&ClassDef> {
    let mut best: Option<&ClassDef> = None;
    for cd in analysis.all_classes.values() {
        let body = cd.body_span;
        if body.start() < cursor && cursor < body.end() {
            let width = body.end() - body.start();
            if best.is_none_or(|b| width < (b.body_span.end() - b.body_span.start())) {
                best = Some(cd);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
        let mut start = 0;
        for _ in 0..occurrence {
            let idx = src[start..].find(needle).expect("needle not found") + start;
            start = idx + 1;
        }
        let idx = start - 1;
        let prefix = &src[..idx];
        let line = u32::try_from(prefix.matches('\n').count()).unwrap();
        let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
        (line, col)
    }

    #[test]
    fn variable_jumps_to_inferred_class() {
        let src = "oo::class create Dog { method bark {} {} }\n\
                   set d [Dog new]\n\
                   $d bark\n";
        let analysis = analyse(src);
        // Cursor on `$d` in the `$d bark` call (line 2).
        let (l, c) = pos_of(src, "$d bark", 1);
        let locs = type_definition(src, l, c + 1, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `Dog`'s name span is on line 0.
        assert_eq!(locs[0].start_line, 0);
    }

    #[test]
    fn variable_without_known_class_returns_empty() {
        let src = "set x 42\nputs $x\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$x", 1);
        assert!(type_definition(src, l, c + 1, &analysis).is_empty());
    }

    #[test]
    fn method_word_in_class_body_jumps_to_class() {
        let src = "oo::class create Greeter {\n\
                   method greet {} { return hi }\n\
                   method again {} { greet }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on the `greet` call inside `again`'s body (2nd occ).
        let (l, c) = pos_of(src, "greet", 2);
        let locs = type_definition(src, l, c, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 0);
    }

    #[test]
    fn unrelated_word_returns_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "hello", 1);
        assert!(type_definition(src, l, c, &analysis).is_empty());
    }
}
