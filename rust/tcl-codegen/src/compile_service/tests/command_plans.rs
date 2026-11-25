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

//! Byte command plans: fatal tails, opaque prefixes, entry lookup and the release grammar a runtime script is planned under.

use super::*;

#[test]
fn fatal_tail_preserves_distinct_native_runtime_and_compilation_extents() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let service = BytecodeCompileService::for_profile(profile);
    let decode = |input: &str| {
        input
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut compared = 0;
    for row in
        include_str!("../../../../tcl-registry/tests/data/native_c84_parse_context/8.4.20.tsv")
            .lines()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        if fields[1] == "0" {
            continue;
        }
        let source = decode(fields[6]);
        let start = fields[2].parse::<usize>().unwrap();
        let term = fields[4].parse::<usize>().unwrap();
        let message = String::from_utf8(decode(fields[7])).unwrap();
        let tail = service
            .script_command_plan_bytes_for_profile(
                &tcl_runtime_api::SourceImage::native(source.as_slice()),
                profile,
            )
            .unwrap()
            .fatal_tail
            .unwrap();
        assert_eq!(tail.message, message, "{row}");
        assert_eq!(tail.command_text, source[start..=term], "{row}");
        let end = if term + 1 == source.len() {
            term
        } else {
            source.len()
        };
        assert_eq!(
            tail.compilation_command_text.as_deref(),
            Some(&source[start..end]),
            "{row}"
        );
        assert_eq!(
            tail.line,
            tcl_lexer::LineIndex::from_bytes(&source)
                .line_at(u32::try_from(start).expect("fixture offset fits source"))
                + 1,
            "{row}"
        );
        compared += 1;
    }
    assert_eq!(compared, 26);
}

#[test]
fn fatal_tail_geometry_uses_kind_independently_of_message() {
    let config =
        LexerConfig::from_grammar(tcl_dialect::DialectProfile::find("tcl8.4").unwrap().grammar);
    for source in [
        b"set before OK\nset x \"abc".as_slice(),
        b"set before OK\nset x {abc",
        b"set before OK\nset x [bad",
        b"set before OK\nset x \"\xffabc",
    ] {
        let image = tcl_lexer::SourceImage::native(source);
        let region = tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap());
        let fatal = tcl_lexer::native_script_words_in(image, region, config)
            .unwrap()
            .fatal_tail
            .unwrap();
        let start = fatal.command_start as usize;
        let expected = fatal_tail_frame_bytes(source, start, fatal.cut);
        assert_ne!(expected.command_text.len(), 0, "{source:?}");
        for message in [
            "",
            "different wording with 'quotes'",
            tcl_lexer::MISSING_QUOTE,
            tcl_lexer::MISSING_CLOSE_BRACE,
            tcl_lexer::MISSING_CLOSE_BRACKET,
        ] {
            let mut changed = fatal.cut;
            changed.message = message;
            let observed = fatal_tail_frame_bytes(source, start, changed);
            assert_eq!(observed.message, message);
            assert_eq!(observed.command_text, expected.command_text);
            assert_eq!(
                observed.compilation_command_text,
                expected.compilation_command_text
            );
            assert_eq!(observed.line, expected.line);
        }
        for kind in [
            tcl_lexer::ParseIssueKind::MissingQuote,
            tcl_lexer::ParseIssueKind::MissingBrace,
            tcl_lexer::ParseIssueKind::MissingBracket,
        ] {
            if kind == fatal.cut.kind {
                continue;
            }
            let mut changed = fatal.cut;
            changed.kind = kind;
            let unavailable = fatal_tail_frame_bytes(source, start, changed);
            assert_eq!(unavailable.message, expected.message);
            assert!(unavailable.compilation_command_text.is_none());
            assert_eq!(unavailable.command_text, [] as [u8; 0]);
        }
    }
}

type ParseTailCase<'a> = (&'a [u8], &'a [u8], &'a [u8], u32);

#[test]
fn original_byte_command_plan_keeps_c84_compilation_tail_and_runtime_term() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let service = BytecodeCompileService::for_profile(profile);
    let cases: &[ParseTailCase<'_>] = &[
        (b"set x \"", b"set x \"", b"set x ", 1),
        (b"set x \"abc", b"set x \"", b"set x \"abc", 1),
        (b"set x {abc", b"set x {", b"set x {abc", 1),
        (b"set x [bad", b"set x [", b"set x [bad", 1),
        (
            b"set x [bad x \"abc",
            b"set x [bad x \"",
            b"set x [bad x \"abc",
            1,
        ),
        (b"set x [bad {", b"set x [bad {", b"set x [bad ", 1),
        (
            b"set before OK\nset x \"abc",
            b"set x \"",
            b"set x \"abc",
            2,
        ),
        (
            b"set x \"\xc3\xa9abc",
            b"set x \"",
            b"set x \"\xc3\xa9abc",
            1,
        ),
        (
            b"set x \"\xf0\x9f\x98\x80abc",
            b"set x \"",
            b"set x \"\xf0\x9f\x98\x80abc",
            1,
        ),
        (b"set x \"\xffabc", b"set x \"", b"set x \"\xffabc", 1),
        (b"set x \"\0abc", b"set x \"", b"set x \"\0abc", 1),
        (b"\"", b"\"", b"", 1),
        (b"{", b"{", b"", 1),
    ];
    for &(source, runtime, compilation, line) in cases {
        let source = tcl_runtime_api::SourceImage::native(source);
        let tail = service
            .script_command_plan_bytes_for_profile(&source, profile)
            .unwrap()
            .fatal_tail
            .unwrap();
        assert_eq!(tail.command_text, runtime, "{source:?}");
        assert_eq!(
            tail.compilation_command_text.as_deref(),
            Some(compilation),
            "{source:?}"
        );
        assert_eq!(tail.line, line, "{source:?}");
        if let Ok(text) = std::str::from_utf8(source.bytes()) {
            let text_tail = service
                .script_command_plan_for_profile(text, profile)
                .unwrap()
                .fatal_tail
                .unwrap();
            assert_eq!(tail, text_tail, "{source:?}");
        }
    }
}

#[test]
fn immutable_byte_entry_lookup_keeps_names_incarnations_and_residuals() {
    use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCommandLookupUnavailable, NativeCompilationBinding,
        NativeCompilationNamespace, NativeCompilerHookPresence,
    };
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let point = tcl_registry::model::ingress::default_execution_point_for_profile(profile)
        .expect("actual known native environment execution point");
    let mut entry = policy_entry(profile, point);
    let local = ByteNamespacePath::from_segments([b"N".as_slice()]);
    entry.closed = true;
    entry.current_namespace = 2;
    entry.namespaces = vec![
        NativeCompilationNamespace {
            path: ByteNamespacePath::root(),
            jim_namespace_object: None,
            token: 1,
            visible: true,
            exports: vec![],
            command_path: vec![],
            unknown_handler: None,
        },
        NativeCompilationNamespace {
            path: local.clone(),
            jim_namespace_object: None,
            token: 2,
            visible: false,
            exports: vec![],
            command_path: vec![],
            unknown_handler: None,
        },
        NativeCompilationNamespace {
            path: local.clone(),
            jim_namespace_object: None,
            token: 9,
            visible: true,
            exports: vec![],
            command_path: vec![],
            unknown_handler: None,
        },
    ];
    let binding = |namespace_token, token| NativeCompilationBinding {
        slot: NativeByteCommandSlot::new(local.clone(), NameBytes::from(b"k\xff".as_slice())),
        namespace_token,
        token,
        implementation_generation: token,
        implementation: NativeCommandImplementation::Opaque,
        compiler_hook: NativeCompilerHookPresence::Absent,
        compiler: None,
        procedure_header: None,
        has_execution_trace: false,
    };
    entry.commands = vec![binding(2, 12), binding(9, 19)].into();
    assert_eq!(
        entry
            .lookup_command_bytes(2, b"k\xff\0tail")
            .unwrap()
            .unwrap()
            .token,
        12
    );
    assert_eq!(
        entry
            .lookup_command_bytes(2, b"::N::k\xff")
            .unwrap()
            .unwrap()
            .token,
        19
    );
    assert!(entry.lookup_command_bytes(2, b"absent").unwrap().is_none());
    assert_eq!(
        entry.lookup_command_bytes(2, b"child::k\xff"),
        Err(NativeCommandLookupUnavailable::RetainedDescendant)
    );
    entry.commands.push(binding(2, 22));
    assert_eq!(
        entry.lookup_command_bytes(2, b"k\xff"),
        Err(NativeCommandLookupUnavailable::ConflictingBinding)
    );
    entry.closed = false;
    assert_eq!(
        entry.lookup_command_bytes(2, b"k\xff"),
        Err(NativeCommandLookupUnavailable::OpenTable)
    );
}

#[test]
fn byte_procedure_target_seeds_original_nonunicode_formal_keys() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let source = tcl_runtime_api::SourceImage::native(b"return OK".as_slice());
    let namespace = tcl_core_types::ByteNamespacePath::root();
    let parameters = [
        tcl_core_types::NameBytes::from(b"p\xff".as_slice()),
        tcl_core_types::NameBytes::from(b"a(k)".as_slice()),
    ];
    let module = service
        .compile_procedure_bytes_for_profile(
            tcl_runtime_api::ProcedureCompileTargetBytes {
                source: &source,
                parameters: &parameters,
                namespace: &namespace,
            },
            profile,
            tcl_runtime_api::ProcedureDispatch::Optimised,
        )
        .unwrap();
    assert_eq!(module.source, source);
    assert_eq!(module.source_namespace, namespace);
    assert_eq!(&module.top_level_body.lvt.entries()[..2], &parameters);
}

#[test]
fn original_byte_command_plan_keeps_opaque_prefix_and_exact_bad_tail() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let service = BytecodeCompileService::for_profile(profile);
    let source = tcl_runtime_api::SourceImage::native(b"set \xff V; set \xfe {a}b".as_slice());
    let plan = service
        .script_command_plan_bytes_for_profile(&source, profile)
        .unwrap();
    assert_eq!(plan.complete_prefix_commands, 1);
    assert_eq!(&source.bytes()[..plan.complete_prefix_len], b"set \xff V; ");
    let tail = plan.fatal_tail.unwrap();
    assert_eq!(tail.message, "extra characters after close-brace");
    assert_eq!(tail.command_text, b"set \xfe {a}b");
    assert_eq!(tail.line, 1);
}

#[test]
fn byte_command_plan_preserves_native_and_document_channel_rules() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let service = BytecodeCompileService::for_profile(profile);
    let text = "set a A\\\r\nset b B";
    let native = tcl_runtime_api::SourceImage::native(text.as_bytes());
    let document = tcl_runtime_api::SourceImage::document(text);
    assert_eq!(
        service
            .script_command_plan_bytes_for_profile(&native, profile)
            .unwrap()
            .complete_prefix_commands,
        2
    );
    assert_eq!(
        service
            .script_command_plan_bytes_for_profile(&document, profile)
            .unwrap()
            .complete_prefix_commands,
        1
    );
}

#[test]
fn runtime_script_command_plan_uses_the_requested_release_grammar() {
    let tcl84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(tcl90);
    let source = "set side 1; {*}{set x 2}";

    let old = service
        .script_command_plan_for_profile(source, tcl84)
        .unwrap();
    assert_eq!(&source[..old.complete_prefix_len], "set side 1; ");
    assert_eq!(
        old.fatal_tail
            .expect("8.4 rejects expansion syntax")
            .message,
        "extra characters after close-brace"
    );

    let current = service
        .script_command_plan_for_profile(source, tcl90)
        .unwrap();
    assert_eq!(current.complete_prefix_len, source.len());
    assert!(current.fatal_tail.is_none());
}
