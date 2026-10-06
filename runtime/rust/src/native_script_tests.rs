// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::fmt::Write;
use tcl_syntax::jim_script_objects::JimScriptObjectKind;

fn references(value: *mut TclObj) -> obj::TclSize {
    // SAFETY: every caller holds the original header throughout inspection.
    unsafe { (*value).ref_count }
}
fn primary(value: *mut TclObj) -> &'static str {
    let descriptor = obj::obj_type_ptr(value);
    if descriptor.is_null() {
        "NULL"
    } else if std::ptr::eq(descriptor, &JIM_SCRIPT_TYPE) {
        "script"
    } else if std::ptr::eq(descriptor, &JIM_SCRIPT_LINE_TYPE) {
        "scriptline"
    } else if std::ptr::eq(descriptor, &native_source::JIM_SOURCE_TYPE) {
        "source"
    } else if std::ptr::eq(descriptor, &obj::TCL_INT_TYPE) {
        "int"
    } else {
        panic!("unexpected measured native Script cache")
    }
}
fn decode(hex: &str) -> Vec<u8> {
    if hex == "-" {
        return Vec::new();
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn original_script_storage_matches_156_native_windows() {
    crate::counters::reset();
    {
        let dialect = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("jim"),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(
            dialect
                .execution_point()
                .expect("actual Jim execution issuer")
                .grammar(),
        );
        let mut observed = String::new();
        for input in
            include_str!("../../../rust/tcl-syntax/testdata/native_jim_script_objects/inputs.tsv")
                .lines()
        {
            let (case, hex) = input.split_once('\t').unwrap();
            let context = NativeJimObjectContext::new(dialect).unwrap();
            let filename = Owned::fresh(obj::new_string_bytes(b"FILE"));
            let original = Owned::fresh(obj::new_string_bytes(&decode(hex)));
            native_source::install_source(
                original.as_ptr(),
                NativeJimSourceInfo {
                    filename: filename.clone(),
                    line: 7,
                },
                &context,
            )
            .unwrap();
            writeln!(
                observed,
                "SOURCE\t{case}\t{}\t{}",
                references(filename.as_ptr()),
                references(original.as_ptr())
            )
            .unwrap();
            let backing = prepare(original.as_ptr(), &context, config).unwrap();
            writeln!(
                observed,
                "SCRIPT\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                backing.ordinary().unwrap().objects.tokens().len(),
                backing.ordinary().unwrap().first_line,
                backing.ordinary().unwrap().linenr.get(),
                backing
                    .ordinary()
                    .unwrap()
                    .objects
                    .missing
                    .map_or(32, i32::from),
                backing.in_use.get(),
                usize::from(backing.ordinary().unwrap().filename.as_ptr() == filename.as_ptr()),
                references(filename.as_ptr()),
                usize::from(obj::has_string_rep(original.as_ptr()))
            )
            .unwrap();
            for (index, token) in backing
                .ordinary()
                .unwrap()
                .objects
                .tokens()
                .iter()
                .enumerate()
            {
                let kind = match token.kind {
                    JimScriptObjectKind::Line { .. } => 9,
                    JimScriptObjectKind::Word(_) => 10,
                    JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Escaped) => 2,
                    JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::String) => 1,
                    JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Variable) => 3,
                    JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::IndexedVariable) => {
                        4
                    }
                    JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Command) => 5,
                    JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Expression) => 17,
                    _ => panic!("unexpected real native token"),
                };
                let value = token.value.as_ptr();
                write!(
                    observed,
                    "TOKEN\t{case}\t{index}\t{kind}\t{}\t{}\t{}\t{}",
                    primary(value),
                    usize::from(obj::has_string_rep(value)),
                    if obj::has_string_rep(value) {
                        i64::try_from(obj::bytes_of(value).len()).unwrap()
                    } else {
                        -1
                    },
                    references(value)
                )
                .unwrap();
                match token.kind {
                    JimScriptObjectKind::Line { .. } => {
                        let (argc, line) = script_line(value).unwrap();
                        writeln!(observed, "\t{argc}\t{line}").unwrap();
                    }
                    JimScriptObjectKind::Word(_) => {
                        writeln!(observed, "\t{}\t-", obj::wide_of(value)).unwrap()
                    }
                    JimScriptObjectKind::Source(_) => {
                        let info = native_source::pin_source_info(value, &context).unwrap();
                        writeln!(
                            observed,
                            "\t{}\t{}",
                            usize::from(info.filename.as_ptr() == filename.as_ptr()),
                            info.line
                        )
                        .unwrap();
                    }
                }
            }
            let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
            writeln!(
                observed,
                "DUP\t{case}\t{}\t{}\t{}\t{}",
                primary(duplicate.as_ptr()),
                usize::from(obj::has_string_rep(duplicate.as_ptr())),
                obj::bytes_of(duplicate.as_ptr()).len(),
                references(filename.as_ptr())
            )
            .unwrap();
            for (index, token) in backing
                .ordinary()
                .unwrap()
                .objects
                .tokens()
                .iter()
                .enumerate()
                .filter(|(_, token)| matches!(token.kind, JimScriptObjectKind::Line { .. }))
            {
                let duplicate = Owned::fresh(obj::duplicate(token.value.as_ptr()));
                writeln!(
                    observed,
                    "LINE_DUP\t{case}\t{index}\t{}\t{}\t{}",
                    primary(duplicate.as_ptr()),
                    usize::from(obj::has_string_rep(duplicate.as_ptr())),
                    obj::bytes_of(duplicate.as_ptr()).len()
                )
                .unwrap();
            }
            drop(backing);
            drop(original);
            drop(duplicate);
            writeln!(
                observed,
                "RETIRED\t{case}\t{}",
                references(filename.as_ptr())
            )
            .unwrap();
        }
        let context = NativeJimObjectContext::new(dialect).unwrap();
        let backing = prepare(context.empty_object().as_ptr(), &context, config).unwrap();
        writeln!(
            observed,
            "TRUE_EMPTY\t{}\t{}\t{}\t{}",
            primary(context.empty_object().as_ptr()),
            primary(context.null_script_object().as_ptr()),
            backing.ordinary().unwrap().objects.tokens().len(),
            usize::from(
                backing.ordinary().unwrap().filename.as_ptr() == context.empty_object().as_ptr()
            )
        )
        .unwrap();
        let other = Owned::fresh(obj::new_string_bytes(b""));
        prepare(other.as_ptr(), &context, config).unwrap();
        writeln!(
            observed,
            "EQUAL_EMPTY\t{}\t{}",
            primary(other.as_ptr()),
            usize::from(other.as_ptr() == context.null_script_object().as_ptr())
        )
        .unwrap();
        assert_eq!(observed.lines().count(), 156);
        assert_eq!(
            observed,
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_jim_script_objects/observations.tsv"
            )
        );
    }
    assert_eq!(crate::counters::finalize(), 0);
}
