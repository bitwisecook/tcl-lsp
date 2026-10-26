// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `textDocument/typeDefinition` — jump to the class that *types* the
//! symbol at the cursor.
//!
//! Two receiver shapes:
//!
//! - **Variable receiver** (`$obj`) — an actual retained object read identifies
//!   its original class allocation. Nominal type candidates cannot supply a
//!   local declaration; foreign source requires its own navigation target.
//! - **Method receiver** — an original member declaration or authentic
//!   lexical self selector identifies the method's owning class.
//!
//! Returns an empty vector when no class types the symbol.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::{LspRange, byte_offset_at, span_to_range};

/// Compute "go-to-type-definition" locations for the symbol at the
/// cursor.
#[must_use]
pub fn type_definition(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let cursor = byte_offset_at(&line_index, source, line, character);
    if !analysis.allows_lexical_declaration_advice() {
        let config = current.config();
        let image = tcl_lexer::SourceImage::document(source);
        if analysis
            .original_variable_root_in_source(&image, config, cursor)
            .is_some()
        {
            return crate::receiver_identity::class_at_read(analysis, source, cursor)
                .map(|class| vec![span_to_range(source, &line_index, class.name_span)])
                .unwrap_or_default();
        }
        return match crate::method_symbol::local_candidate(source, analysis, line, character) {
            std::ops::ControlFlow::Break(Some(method)) => method
                .declaring_class_in(source, analysis)
                .map(|class| {
                    vec![span_to_range(
                        source,
                        &line_index,
                        class.name_input().span(),
                    )]
                })
                .unwrap_or_default(),
            std::ops::ControlFlow::Break(None) | std::ops::ControlFlow::Continue(()) => Vec::new(),
        };
    }

    // The actual object read retains its original class allocation. A
    // singleton candidate type cannot supply a type-navigation declaration.
    if crate::definition::substituting_var_at_position(source, analysis, line, character, cursor)
        .is_some()
    {
        let cursor = byte_offset_at(&line_index, source, line, character);
        return crate::receiver_identity::class_at_read(analysis, source, cursor)
            .map(|class| vec![span_to_range(source, &line_index, class.name_span)])
            .unwrap_or_default();
    }

    analysis
        .logical_source_member_type_at(
            &tcl_lexer::SourceImage::document(source),
            current.config(),
            cursor,
        )
        .map(|class| vec![span_to_range(source, &line_index, class.name_span)])
        .unwrap_or_default()
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
                   method again {} { my greet }\n\
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

    #[test]
    fn lattice_typed_variable_jumps_to_inferred_class() {
        // `b` is typed only by the object-type lattice's
        // method-return edge; go-to-type-definition must reach ::B like
        // every other dispatch consumer.
        let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "$b greet", 1);
        let locs = type_definition(src, l, c + 1, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 1, "must land on ::B: {locs:?}");
    }

    #[test]
    fn variable_jumps_to_class_in_its_own_namespace() {
        // `::A::Widget` and `::B::Widget` share a simple name.  An instance
        // created in `::A` types to `::A::Widget`; go-to-type-definition must
        // land on that class's declaration, never the same-named `::B::Widget`.
        let src = "namespace eval A {\n\
                       oo::class create Widget {}\n\
                       set w [Widget new]\n\
                       $w foo\n\
                   }\n\
                   namespace eval B {\n\
                       oo::class create Widget {}\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `$w` in the `$w foo` call (line 3).
        let (l, c) = pos_of(src, "$w foo", 1);
        let locs = type_definition(src, l, c + 1, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // `::A::Widget`'s declaration is on line 1, not `::B::Widget`'s line 6.
        assert_eq!(locs[0].start_line, 1, "{locs:?}");
    }
}

#[cfg(test)]
mod original_type_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;
    #[test]
    fn original_method_type_navigation_uses_opaque_declaring_class_without_reporting_maps() {

        let source = "oo::class create C\\uD800 {method m\\uD800 {} {}; method m\\uD801 {} {}}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        for name in ["m\\uD800", "m\\uD801"] {
            let cursor = u32::try_from(source.find(name).unwrap()).unwrap();
            let ranges = type_definition(source, 0, cursor, &analysis);
            assert_eq!(ranges.len(), 1);
            let class = analysis.original_class_declarations().next().unwrap();
            assert_eq!(
                ranges[0],
                span_to_range(source, &LineIndex::new(source), class.name_input().span())
            );
        }
        assert!(type_definition(&format!("# changed\n{source}"), 1, 35, &analysis).is_empty());
    }
}

#[cfg(test)]
mod logical_member_type_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn ranges(source: &str, analysis: &AnalysisResult, needle: &str) -> Vec<LspRange> {
        let cursor = u32::try_from(source.rfind(needle).unwrap()).unwrap();
        let position = LineIndex::new(source).position_at_utf16(cursor, source);
        type_definition(source, position.line, position.character.get(), analysis)
    }

    #[test]
    fn logical_declaration_and_self_selector_keep_the_original_member_owner() {
        let source = "oo::class create C {method target {} {}; method caller {} {my target}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        for needle in ["target {}", "target}"] {
            let locations = ranges(source, &analysis, needle);
            assert_eq!(locations.len(), 1, "{needle}: {locations:?}");
            assert_eq!(locations[0].start_character, 17);
        }
    }

    #[test]
    fn logical_unicode_member_declaration_and_self_selector_keep_original_values() {
        for name in ["méthode", "名::méthode", "{名 target}"] {
            let source = format!(
                "oo::class create C {{method {name} {{}} {{}}; method caller {{}} {{my {name}}}}}",
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            assert!(analysis.allows_retained_logical_declaration_advice());
            for needle in [format!("{name} {{}}"), format!("{name}}}")] {
                let locations = ranges(&source, &analysis, &needle);
                assert_eq!(locations.len(), 1, "{name}: {needle}: {locations:?}");
                assert_eq!(locations[0].start_character, 17);
            }
        }
    }

    #[test]
    fn logical_matching_data_and_nested_procedure_cannot_supply_a_member_selector() {
        for body in [
            "puts target",
            "set target target",
            "# target\n",
            "proc nested {} {my target}",
            "puts {my target}",
        ] {
            let source = format!(
                "oo::class create C {{method target {{}} {{}}; method caller {{}} {{{body}}}}}",
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            assert!(ranges(&source, &analysis, "target").is_empty(), "{body}");
        }
    }

    #[test]
    fn logical_member_type_requires_current_source_input_configuration_and_metadata() {
        let source = "oo::class create C {method target {} {}; method caller {} {my target}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let changed = format!("{source} ");
        assert!(ranges(&changed, &analysis, "target}").is_empty());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(ranges(source, &changed, "target}").is_empty());
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(ranges(source, &changed, "target}").is_empty());
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::C")
            .unwrap()
            .methods
            .get_mut("target")
            .unwrap()
            .body_span = tcl_lexer::Span::new(0, 1);
        assert!(ranges(source, &changed, "target}").is_empty());
    }

    #[test]
    fn logical_replaced_self_dispatcher_cannot_borrow_registry_semantics() {
        let source = "proc my {args} {}; oo::class create C {method target {} {}; method caller {} {my target}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(ranges(source, &analysis, "target}").is_empty());
    }
}
