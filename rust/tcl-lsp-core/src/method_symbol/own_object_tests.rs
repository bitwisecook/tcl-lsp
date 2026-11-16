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

use super::*;
use tcl_compiler::analyser::Analyser;

fn at(
    source: &str,
    analysis: &AnalysisResult,
    needle: &str,
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    let offset = u32::try_from(source.rfind(needle).unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    local_candidate(source, analysis, position.line, position.character.get())
}

#[test]
fn original_own_object_candidates_keep_allocation_and_canonical_worker_without_class_tables() {
    // Implementation contract: naming.editor.original-own-object-member-navigation
    // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
    let source = r"oo::class create C {method shared {} {return CLASS}}; C create object; C create sibling; oo::objdefine object {method shared {} {}; method p\uD800 {} {}; method p\uD801 {} {}}; object shared; object p\uD800; object p\uD801; sibling shared";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        analysis.instance_classes.clear();
        analysis.created_instance_commands.clear();
        let ControlFlow::Break(Some(own)) = at(source, &analysis, "shared; object") else {
            panic!("own method has its own source owner");
        };
        assert!(own.own_object().is_some());
        assert!(own.has_call_point_entry());
        assert!(own.declaring_class_in(source, &analysis).is_none());
        let ControlFlow::Break(Some(sibling)) = at(source, &analysis, "shared") else {
            panic!("sibling keeps the class-owned declaration");
        };
        assert!(sibling.own_object().is_none());
        assert!(!own.same_declaration(&sibling));
        let ControlFlow::Break(Some(a)) = at(source, &analysis, r"p\uD800;") else {
            panic!("first opaque own key");
        };
        let ControlFlow::Break(Some(b)) = at(source, &analysis, r"p\uD801;") else {
            panic!("second opaque own key");
        };
        assert!(!a.same_declaration(&b));
        assert!(a.hover().unwrap().value.contains("Own method"));
        let changed = format!("{source} ");
        assert!(matches!(
            at(&changed, &analysis, r"p\uD800;"),
            ControlFlow::Break(None)
        ));
        let mut changed_config = analysis.clone();
        changed_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting ^= true;
        assert!(matches!(
            at(source, &changed_config, r"p\uD800;"),
            ControlFlow::Break(None)
        ));
    }
}

#[test]
fn original_own_object_generation_and_private_entries_block_class_advice() {
    // Implementation contract: naming.editor.original-own-object-member-navigation
    // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
    let source = "oo::class create C {method shared {} {return CLASS}; method Hidden {} {}; export Hidden}; C create object; oo::objdefine object {method shared {} {return FIRST}; method Hidden {} {}}; object shared; oo::objdefine object method shared {} {return SECOND}; object shared; object Hidden";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let first = u32::try_from(source.find("object shared;").unwrap()).unwrap() + 7;
    let position = LineIndex::new(source).position_at_utf16(first, source);
    let ControlFlow::Break(Some(first)) =
        local_candidate(source, &analysis, position.line, position.character.get())
    else {
        panic!("first own generation");
    };
    let ControlFlow::Break(Some(last)) = at(source, &analysis, "shared; object Hidden") else {
        panic!("replacement own worker");
    };
    assert!(!first.same_declaration(&last));
    assert_eq!(
        first.own_object().unwrap().instance().allocation(),
        last.own_object().unwrap().instance().allocation()
    );
    assert!(matches!(
        at(source, &analysis, "Hidden"),
        ControlFlow::Break(None)
    ));
    let complete_prefix = source.strip_suffix("object Hidden").unwrap();
    let ambiguous = format!("{complete_prefix} unknown; object shared");
    let analysis = Analyser::new().analyse(&ambiguous, "tcl8.6");
    assert!(matches!(
        at(&ambiguous, &analysis, "shared"),
        ControlFlow::Break(None)
    ));
}
