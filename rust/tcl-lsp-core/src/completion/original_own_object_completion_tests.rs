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

fn items(source: &str, analysis: &AnalysisResult, head: &str) -> Vec<CompletionItem> {
    let start = u32::try_from(source.rfind(head).unwrap()).unwrap();
    let cursor = start + u32::try_from(head.len()).unwrap();
    let record = crate::original_oo::instance_method_completion_record(analysis, source, cursor)
        .expect("current actual object class");
    original_instance_method_candidate_items_at(analysis, record, source, cursor, "", None)
}

#[test]
fn original_own_object_completion_keeps_allocations_and_opaque_routes_after_reporting_clear() {
    // Implementation contract: naming.core.original-own-object-completion
    // docs/design/analysis/name-resolution-proofs/core-original-own-object-completion.md
    let source = r"oo::class create C {method shared {} {}}; C create object; C create sibling; oo::objdefine object {method shared {} {}; method p\uD800 {} {}; method p\uD801 {} {}}; object shared; sibling shared";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_classes.clear();
    analysis.superseded_classes.clear();
    analysis.global_scope.classes.clear();
    let own = items(source, &analysis, "object shared");
    // tclsh 8.6.18, 9.0.4 and 9.1.0 (ASCII names for the escapes):
    //   lsort [info object methods object -all]  ->  destroy pa pb shared
    // The built-in `destroy` comes from the family's registry row.
    let (builtin, own): (Vec<_>, Vec<_>) =
        own.into_iter().partition(|item| item.label == "destroy");
    assert_eq!(builtin.len(), 1);
    assert_eq!(own.len(), 3);
    assert!(own.iter().all(
            |item| item.detail.as_deref() == Some("method — per-object declaration candidate")
        ));
    let class = analysis.original_class_declarations().next().unwrap();
    let policy = class.name_input().policy();
    let values = own
        .iter()
        .map(|item| {
            let image = tcl_lexer::SourceImage::document(&item.insert_text);
            let plan = tcl_lexer::native_script_words_in(
                image,
                tcl_lexer::Span::new(0, u32::try_from(item.insert_text.len()).unwrap()),
                class.name_input().lexer_config(),
            )
            .unwrap();
            tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0],
                class.name_input().word_value_rules(),
                policy,
            )
            .unwrap()
            .bytes()
            .to_vec()
        })
        .collect::<Vec<_>>();
    assert!(values.contains(&b"p\xed\xa0\x80".to_vec()));
    assert!(values.contains(&b"p\xed\xa0\x81".to_vec()));
    let sibling = items(source, &analysis, "sibling shared");
    // tclsh 8.6.18 and 9.1.0: lsort [info object methods sibling -all]
    //   ->  destroy shared
    assert_eq!(
        sibling
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["destroy", "shared"]
    );
    assert_eq!(
        sibling[1].detail.as_deref(),
        Some("method — source declaration candidate")
    );
    assert!(matches!(
        crate::original_oo::own_object_completion_methods(
            &analysis,
            &format!("#{source}"),
            u32::try_from(source.len()).unwrap()
        ),
        std::ops::ControlFlow::Break(None)
    ));
}

#[test]
fn original_own_object_completion_requires_the_actual_generation_and_canonical_declaration() {
    // Implementation contract: naming.core.original-own-object-completion
    // docs/design/analysis/name-resolution-proofs/core-original-own-object-completion.md
    let source = "oo::class create C {method shared {} {}}; C create object; oo::objdefine object {method own {} {}}; object own";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        items(source, &analysis, "object own")
            .iter()
            .any(|item| item.label == "own")
    );
    analysis.original_object_configuration_metadata.clear();
    assert_eq!(
        items(source, &analysis, "object own"),
        [] as [crate::completion::CompletionItem; 0]
    );
    let unknown =
        "oo::class create C {}; C create object; oo::objdefine object {unknown_worker}; object own";
    let analysis = Analyser::new().analyse(unknown, "tcl8.6");
    let offset = u32::try_from(unknown.rfind("object own").unwrap()).unwrap();
    let structure = crate::source_structure::SourceStructure::capture(
        unknown,
        Some(&analysis),
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap();
    let command = structure
        .commands
        .iter()
        .find(|command| {
            command
                .argv
                .first()
                .is_some_and(|head| head.span.start() == offset)
        })
        .unwrap();
    assert!(
        crate::receiver_identity::class_at_command_head(&analysis, unknown, command).is_none(),
        "conditional source construction cannot supply the missing current receiver generation"
    );
}
