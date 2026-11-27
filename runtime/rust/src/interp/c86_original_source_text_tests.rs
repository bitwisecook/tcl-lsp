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
use std::cell::Cell;
use tcl_lexer::{ExecutablePartArena, LexerConfig, SourceChannel, SourceImage, Span};

thread_local! { static MARKS: Cell<usize> = const { Cell::new(0) }; }
fn native() -> Interp {
    Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .unwrap()
}
fn mark(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    MARKS.with(|marks| marks.set(marks.get() + 1));
    interpreter.set_result(obj::new_string_bytes(b"X"));
    Code::Ok
}
#[cfg(debug_assertions)]
fn mutate(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-source-text-test".to_owned(), "1.0".to_owned())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
    interpreter.set_result(obj::new_obj());
    Code::Ok
}

#[test]
fn original_top_level_escapes_preserve_written_and_literal_native_extents() {
    for (script, expected) in [
        (b"set d \\0".as_slice(), b"\xc0\x80".as_slice()),
        (b"set d \\u0000", b"\xc0\x80"),
        (b"set d \\uD800", b"\xed\xa0\x80"),
        (b"set d {\0}", b"\0"),
        (b"set d {\xc0\x80}", b"\xc0\x80"),
        (b"set d {\\0}", b"\\0"),
        (b"set d \\\\0", b"\\0"),
    ] {
        let mut interpreter = native();
        assert_eq!(interpreter.eval_str(script), Code::Ok, "{script:?}");
        assert_eq!(interpreter.result_bytes(), expected, "{script:?}");
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_word_order_preserves_substitution_effects_and_single_object_passthrough() {
    let mut interpreter = native();
    interpreter.register_builtin(b"mark", mark);
    MARKS.with(|marks| marks.set(0));
    assert_eq!(
        interpreter.eval_str(b"set v V; set d \"A\\0$v[mark]\\u0000B\""),
        Code::Ok
    );
    assert_eq!(interpreter.result_bytes(), b"A\xc0\x80VX\xc0\x80B");
    assert_eq!(MARKS.with(Cell::get), 1);
    assert_eq!(
        interpreter.eval_str(b"set v [list a b]; set d $v"),
        Code::Ok
    );
    assert_eq!(interpreter.var_get(b"v"), interpreter.var_get(b"d"));
    MARKS.with(|marks| marks.set(0));
    assert_eq!(interpreter.eval_str(b"list [mark] {broken"), Code::Error);
    assert_eq!(MARKS.with(Cell::get), 0);
}

#[cfg(debug_assertions)]
#[test]
fn selected_source_producer_does_not_fall_back_after_change_and_restore() {
    let mut interpreter = native();
    interpreter.register_builtin(b"mutate", mutate);
    assert_eq!(
        interpreter.eval_str(b"set v V; trace add variable v read mutate; set d \"$v\\0\""),
        Code::Error
    );
    assert!(interpreter.native_execution_refusal().is_some());
    assert!(interpreter.var_get(b"d").is_none());
}

fn mutate_profile(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    interpreter.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
    interpreter.set_result(obj::new_obj());
    Code::Ok
}

#[test]
fn original_text_request_cannot_fall_back_after_prior_argument_changes_profile() {
    let mut interpreter = native();
    interpreter.register_builtin(b"mutateProfile", mutate_profile);
    assert_eq!(
        interpreter.eval_str(b"list [mutateProfile] \\0"),
        Code::Error
    );
    assert!(interpreter.native_execution_refusal().is_some());

    // The captured source operand remains a physical request even if an
    // earlier argument's callback changed the current admission recipe.
    let mut interpreter = native();
    let source = b"set d \\0";
    let commands = parse::parse_script_with_original_text(source, interpreter.lexer_config());
    let _operation = native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    interpreter.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
    assert!(interpreter
        .substitute_word(source, &commands[0].words[2])
        .is_err());
    assert!(interpreter.native_execution_refusal().is_some());
}

#[test]
fn source_channel_and_grammar_refuse_before_decoding_under_actual_operation() {
    for (channel, config) in [
        (SourceChannel::Document, LexerConfig::for_dialect("tcl8.6")),
        (
            SourceChannel::NativeValue,
            LexerConfig::for_dialect("tcl8.5"),
        ),
    ] {
        let mut interpreter = native();
        let original = parse::OriginalEscapedText {
            part_index: 0,
            span: Span::new(0, 2),
            arena: ExecutablePartArena::decompose(
                SourceImage::from_bytes(b"\\0".to_vec(), channel),
                Span::new(0, 2),
                tcl_lexer::word_parts::SubstFlags {
                    vars: false,
                    cmds: false,
                    backslashes: true,
                    ..Default::default()
                },
                config,
            ),
        };
        let _operation =
            native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
        assert_eq!(
            interpreter.original_c_word_text(&original),
            Err(Code::Error)
        );
        assert!(interpreter.native_execution_refusal().is_some());
    }
}

#[test]
fn explicit_logical_and_previous_native_source_routes_keep_their_text_domain() {
    let mut logical = Interp::new();
    logical.set_dialect_profile(crate::environment::profile_for_dialect("irules"));
    assert!(logical.set_logical_name_provider(
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
    ));
    assert!(logical.set_logical_source_word_provider(
        tcl_native_model::LogicalSourceWordProvider::Tcl84CoreSimulation,
        crate::environment::profile_for_dialect("tcl8.6"),
    ));
    assert_eq!(
        logical.eval_str(b"set d \\0"),
        Code::Ok,
        "actual refusal: {:?}; result: {:?}",
        logical.native_execution_refusal(),
        logical.result_bytes()
    );
    assert_eq!(logical.result_bytes(), b"\0");
    for dialect in ["tcl8.4", "tcl8.5", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(dialect),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        assert_eq!(
            interpreter.eval_str(b"set d \\0; split word {}"),
            Code::Ok,
            "{dialect}"
        );
        assert_eq!(interpreter.result_bytes(), b"w o r d", "{dialect}");
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
