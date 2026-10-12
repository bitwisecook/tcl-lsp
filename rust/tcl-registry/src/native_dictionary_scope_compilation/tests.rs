// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
use tcl_syntax::native_string::NativeStringProtocol;

fn compile(
    source: &[u8],
    command: NativeDictionaryCommand,
    version: TclVersion,
    frame: NativeCompilationFrame,
) -> NativeControlCompilation<NativeDictionaryScopeInstruction> {
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let native = native_script_words_in(
        SourceImage::native(source),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap()
    .commands
    .remove(0)
    .words;
    let words = NativeCompilerWords::capture(&native, NativeStringProtocol::C(version)).unwrap();
    compile_native_dictionary_scope(
        command,
        &words,
        2,
        version,
        NativeCompilationContext {
            frame,
            ..NativeCompilationContext::default()
        },
    )
    .unwrap()
}

#[test]
fn update_preserves_ordered_original_keys_and_auxiliary_declarations() {
    for version in TclVersion::ALL {
        let selected = compile(
            b"dict update d $first v [second] v {set v VALUE}",
            NativeDictionaryCommand::Update,
            version,
            NativeCompilationFrame::ProcedureCode,
        );
        if version < TclVersion::V8_5 {
            assert_eq!(selected.outcome, NativeControlOutcome::Generic);
            assert!(selected.preparations.is_empty());
            continue;
        }
        assert_eq!(
            &selected.preparations[..5],
            &[
                Step::DeclareLocal(b"d".to_vec()),
                Step::DeclareLocal(b"v".to_vec()),
                Step::DeclareLocal(b"v".to_vec()),
                Step::Word(NativeCompilerWordOperand::Original(3)),
                Step::Word(NativeCompilerWordOperand::Original(5))
            ]
        );
        let NativeControlOutcome::Inline(recipe) = selected.outcome else {
            panic!("native update");
        };
        assert!(
            matches!(recipe.kind, NativeDictionaryScopeKind::Update { targets, .. } if targets == [b"v".to_vec(), b"v".to_vec()])
        );
        assert_eq!(recipe.body, NativeCompilerWordOperand::Original(7));
        assert!(matches!(
            selected.preparations.last(),
            Some(Step::Script {
                context: NativeCompiledBodyContext::ExceptionRange,
                ..
            })
        ));
    }
}

#[test]
fn update_decline_retains_actual_array_base_declaration_before_any_key() {
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let selected = compile(
            b"dict update d [key] a(x) {set a VALUE}",
            NativeDictionaryCommand::Update,
            version,
            NativeCompilationFrame::ProcedureCode,
        );
        assert_eq!(selected.outcome, NativeControlOutcome::Generic);
        assert_eq!(
            selected.preparations,
            [
                Step::DeclareLocal(b"d".to_vec()),
                Step::DeclareLocal(b"a".to_vec())
            ]
        );
    }
}

#[test]
fn with_retains_original_dynamic_receiver_path_and_anonymous_slots() {
    for version in TclVersion::ALL {
        let selected = compile(
            b"dict with $receiver [first] $second {set inner VALUE}",
            NativeDictionaryCommand::With,
            version,
            NativeCompilationFrame::ProcedureCode,
        );
        if version < TclVersion::V8_6 {
            assert_eq!(selected.outcome, NativeControlOutcome::Generic);
            continue;
        }
        assert_eq!(
            &selected.preparations[..6],
            &[
                Step::DeclareAnonymousLocal,
                Step::DeclareAnonymousLocal,
                Step::DeclareAnonymousLocal,
                Step::Word(NativeCompilerWordOperand::Original(2)),
                Step::Word(NativeCompilerWordOperand::Original(3)),
                Step::Word(NativeCompilerWordOperand::Original(4))
            ]
        );
        let NativeControlOutcome::Inline(recipe) = selected.outcome else {
            panic!("native with");
        };
        assert_eq!(
            recipe.receiver,
            NativeDictionaryScopeReceiver::Stack(NativeCompilerWordOperand::Original(2))
        );
        assert_eq!(recipe.body, NativeCompilerWordOperand::Original(5));
    }
}

#[test]
fn with_empty_body_uses_selected_native_units_and_never_treats_comment_as_empty() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        for body in [
            b" \t\r\n".as_slice(),
            b"\x0b\x0c",
            b"\xc0\xa0",
            b"\xc2\xa0",
            b"\xc2",
            b"# comment",
        ] {
            let mut source = b"dict with d {".to_vec();
            source.extend_from_slice(body);
            source.push(b'}');
            let selected = compile(
                &source,
                NativeDictionaryCommand::With,
                version,
                NativeCompilationFrame::ProcedureCode,
            );
            let NativeControlOutcome::Inline(recipe) = selected.outcome else {
                panic!("native with");
            };
            let empty = body == b" \t\r\n" || (version == TclVersion::V9_1 && body == b"\x0b\x0c");
            assert!(
                matches!(recipe.kind, NativeDictionaryScopeKind::With { empty_body, .. } if empty_body == empty),
                "{version:?} {body:?}"
            );
            assert_eq!(
                selected
                    .preparations
                    .iter()
                    .filter(|step| matches!(step, Step::DeclareAnonymousLocal))
                    .count(),
                usize::from(!empty)
            );
            assert_eq!(
                selected
                    .preparations
                    .iter()
                    .filter(|step| matches!(step, Step::Script { .. }))
                    .count(),
                usize::from(!empty)
            );
        }
    }
}

#[test]
fn dictionary_fallback_retains_original_declarations_in_shared_instruction_receipt() {
    use crate::native_compilation::NativeCompilationSelection;
    use crate::native_instruction_plan::{NativeInstructionPlan, native_instruction_plan};
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::for_version(version);
        let source = b"dict update d [key] a(x) {set never VALUE}";
        let parsed = native_script_words_in(
            SourceImage::native(source.as_slice()),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let captured = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        let context = NativeCompilationContext {
            mode: crate::native_compilation::NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..NativeCompilationContext::default()
        };
        let spec = NativeDictionaryCommand::Update.spec(true);
        let selected = spec.select_native_words(&captured, 1, Some(dialect), context);
        let plan = native_instruction_plan(spec, selected, &captured, 1, dialect, context).unwrap();
        let preparations = if version == TclVersion::V8_5 {
            assert_eq!(selected, NativeCompilationSelection::Generic);
            let NativeInstructionPlan::GenericPreparation(preparations) = plan else {
                panic!("original generic receipt")
            };
            preparations
        } else {
            assert!(matches!(
                selected,
                NativeCompilationSelection::NamedInvocation { .. }
            ));
            let NativeInstructionPlan::NamedInvocation(recipe) = plan else {
                panic!("original named receipt")
            };
            recipe.preparations
        };
        assert_eq!(
            preparations,
            [
                Step::DeclareLocal(b"d".to_vec()),
                Step::DeclareLocal(b"a".to_vec())
            ]
        );
    }
}

#[test]
fn dictionary_with_empty_script_selection_uses_original_source_at_script_frame() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let empty = compile(
            b"dict with $receiver $path { \t\r\n}",
            NativeDictionaryCommand::With,
            version,
            NativeCompilationFrame::ScriptCode,
        );
        let NativeControlOutcome::Inline(empty) = empty.outcome else {
            panic!("original empty With")
        };
        assert!(matches!(
            empty.kind,
            NativeDictionaryScopeKind::With {
                empty_body: true,
                ..
            }
        ));
        let substantive = compile(
            b"dict with d {# comment}",
            NativeDictionaryCommand::With,
            version,
            NativeCompilationFrame::ScriptCode,
        );
        assert_eq!(substantive.outcome, NativeControlOutcome::Generic);
        assert!(substantive.preparations.is_empty());
    }
}
