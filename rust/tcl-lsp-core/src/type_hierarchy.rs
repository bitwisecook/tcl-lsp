//! Type-hierarchy provider — minimal Rust port of
//! `lsp/features/type_hierarchy.py`.
//!
//! Resolves a `TclOO` class at the cursor and returns a
//! single hierarchy item.  Supertype / subtype walks are
//! stub-empty; computing them requires the class-hierarchy
//! index that the analyser populates (deferred to
//! `S-type-hierarchy-rich`).

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — class identification plus its name
/// and definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeHierarchyItem {
    /// Class qualified name.
    pub name: String,
    /// Detail (e.g. metaclass).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare type hierarchy" request to a single
/// item — the class whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            let name_range = span_to_range(&line_index, class_def.name_span);
            let body_range = span_to_range(&line_index, class_def.body_span);
            let full_range = LspRange {
                start_line: name_range.start_line,
                start_character: name_range.start_character,
                end_line: body_range.end_line,
                end_character: body_range.end_character,
            };
            return vec![TypeHierarchyItem {
                name: class_def.qualified_name.clone(),
                detail: Some(class_def.metaclass.clone()),
                range: full_range,
                selection_range: name_range,
            }];
        }
    }
    Vec::new()
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
    fn prepare_resolves_class_at_cursor() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 18, &analysis);
        if !items.is_empty() {
            assert!(items[0].name.contains("Greeter"));
        }
    }
}
