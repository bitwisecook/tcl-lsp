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

use super::{
    OriginalSourceProcedureDeclarationValues, OriginalSourceTransitionAdviceLookup,
    OriginalSourceTransitionAdviceTape, source_class,
};
use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use crate::ir::CommandTokens;
use std::sync::Arc;

const SOURCE: &str = "proc helper {} {return VALUE}";

struct Fixture {
    source: String,
    analysis: AnalysisResult,
    command: crate::segmenter::SegmentedCommand,
    tokens: CommandTokens,
    original: Vec<tcl_lexer::NativeWord>,
    tape: OriginalSourceTransitionAdviceTape,
}
impl Fixture {
    fn values(&self) -> Option<OriginalSourceProcedureDeclarationValues> {
        let realm = self.analysis.retained_command_realm()?;
        self.tape.procedure_declaration_values(
            realm.source_bindings_ref().original_analysis_owner()?,
            self.analysis.resolved_input.as_ref()?,
            0,
            &self.original,
            self.tokens.source_binding.as_ref()?,
        )
    }
    fn selected(&self) -> crate::registry_invocation::source_structure::OriginalRegistryWords {
        crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.analysis,
            &self.command,
        )
        .unwrap()
    }
}
fn fixture(source: &str) -> Fixture {
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let realm = analysis.retained_command_realm().unwrap();
    let input = analysis.resolved_input.as_ref().unwrap();
    let image = realm.original_source_image().unwrap();
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config())
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        &command,
    );
    realm.stamp_original_tokens(&mut tokens);
    let original = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        0,
        input.lexer_config(),
    )
    .unwrap();
    let tape = realm
        .source_bindings_ref()
        .original_source_transition_advice_tape(input.borrowed_context_registry())
        .unwrap();
    Fixture {
        source: source.to_owned(),
        analysis,
        command,
        tokens,
        original,
        tape,
    }
}

#[test]
fn primary_declaration_values_keep_the_final_record_and_command_refusal() {
    let fixture = fixture(SOURCE);
    assert!(matches!(
        fixture.tape.lookup(0),
        OriginalSourceTransitionAdviceLookup::Refused
    ));
    let values = fixture
        .values()
        .expect("genuine final same-tape declaration values");
    let (record, shape) = fixture
        .tape
        .conditional_procedure_declarations
        .get(&0)
        .unwrap()
        .as_ref()
        .unwrap();
    assert!(Arc::ptr_eq(&values.advice, &record.declaration));
    assert_eq!(values.shape, *shape);
    let input = fixture.analysis.resolved_input.as_ref().unwrap();
    let realm = fixture.analysis.retained_command_realm().unwrap();
    assert!(
        realm
            .original_source_transition_advice(input.borrowed_context_registry(), &fixture.tokens,)
            .is_none()
    );
    let selected = fixture.selected();
    let values = realm
        .original_source_procedure_declaration_values(
            input,
            input.borrowed_context_registry(),
            &fixture.tokens,
        )
        .unwrap();
    let pair = values
        .head_correspondence(SOURCE, input.lexer_config(), input, &selected)
        .unwrap();
    assert!(pair.matches_selected_source(&selected));
    assert!(matches!(
        fixture.tape.lookup(0),
        OriginalSourceTransitionAdviceLookup::Refused
    ));
}

#[test]
fn declaration_values_refuse_absent_and_sticky_conflicted_final_records() {
    let mut absent = fixture(SOURCE);
    assert!(absent.values().is_some());
    assert!(
        absent
            .tape
            .conditional_procedure_declarations
            .remove(&0)
            .is_some()
    );
    assert!(absent.values().is_none());
    assert!(matches!(
        absent.tape.lookup(0),
        OriginalSourceTransitionAdviceLookup::Refused
    ));
    let mut conflict = fixture(SOURCE);
    let actual = conflict
        .tape
        .conditional_procedure_declarations
        .get(&0)
        .cloned()
        .unwrap();
    assert!(actual.is_some());
    source_class::merge_receipt(
        &mut conflict.tape.conditional_procedure_declarations,
        0,
        None,
    );
    source_class::merge_receipt(
        &mut conflict.tape.conditional_procedure_declarations,
        0,
        actual,
    );
    assert!(
        conflict
            .tape
            .conditional_procedure_declarations
            .get(&0)
            .unwrap()
            .is_none()
    );
    assert!(conflict.values().is_none());
}

#[test]
fn declaration_values_refuse_registry_barrier_and_equal_foreign_owner() {
    let mut barred = fixture(SOURCE);
    assert!(barred.values().is_some());
    barred.tape.registry_barriers.insert(0);
    assert!(barred.values().is_none());
    let current = fixture(SOURCE);
    let foreign = fixture(SOURCE);
    let input = current.analysis.resolved_input.as_ref().unwrap();
    let foreign_owner = foreign
        .analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref()
        .original_analysis_owner()
        .unwrap();
    assert!(
        current
            .tape
            .procedure_declaration_values(
                foreign_owner,
                input,
                0,
                &current.original,
                current.tokens.source_binding.as_ref().unwrap(),
            )
            .is_none()
    );
    let realm = current.analysis.retained_command_realm().unwrap();
    assert!(
        realm
            .original_source_procedure_declaration_values(
                input,
                input.borrowed_context_registry(),
                &foreign.tokens,
            )
            .is_none()
    );
}

#[test]
fn declaration_values_refuse_changed_input_source_and_selected_operands() {
    let current = fixture(SOURCE);
    let input = current.analysis.resolved_input.as_ref().unwrap();
    let selected = current.selected();
    assert!(
        current
            .values()
            .unwrap()
            .head_correspondence(
                "proc helper {} {return OTHER}",
                input.lexer_config(),
                input,
                &selected,
            )
            .is_none()
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let changed = ResolvedAnalysisInput::new(profile, profile, input.context_registry(), config);
    let realm = current.analysis.retained_command_realm().unwrap();
    assert!(
        realm
            .original_source_procedure_declaration_values(
                &changed,
                changed.borrowed_context_registry(),
                &current.tokens,
            )
            .is_none()
    );
    let other = fixture("proc helper {argument} {return $argument}");
    assert!(
        current
            .values()
            .unwrap()
            .head_correspondence(SOURCE, input.lexer_config(), input, &other.selected(),)
            .is_none()
    );
    let mut wrong_shape = current.values().unwrap();
    wrong_shape.shape.name = usize::MAX;
    assert!(
        wrong_shape
            .head_correspondence(SOURCE, input.lexer_config(), input, &selected,)
            .is_none()
    );
}
