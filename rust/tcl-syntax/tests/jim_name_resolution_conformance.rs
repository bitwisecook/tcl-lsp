// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current Jim naming recipes compared with original native operations.
//! Counted input bytes and namespace objects remain separate from C Tcl paths.

use std::fmt::Write as _;
use tcl_core_types::ByteNamespacePath;
use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol, global_local_name_bytes};
use tcl_test_support::{Jimsh, locate_jimsh, require_jimsh, run_script};

const PROTOCOL: NativeNameProtocol = NativeNameProtocol::Jim084;

fn reference() -> Option<Jimsh> {
    if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim oracle"))
    } else {
        let interpreter = locate_jimsh().expect("valid Jim interpreter override");
        if interpreter.is_none() {
            eprintln!("Jim oracle unavailable; set TCL_LSP_JIMSH to verify counted naming");
        }
        interpreter
    }
}

fn unhex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "complete hexadecimal bytes");
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn hex(value: &[u8]) -> String {
    let mut result = String::new();
    for byte in value {
        write!(result, "{byte:02x}").unwrap();
    }
    result
}

fn rows(table: &str) -> impl Iterator<Item = (&str, Vec<u8>, Vec<u8>)> {
    table
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = line.split('|').collect();
            assert_eq!(fields.len(), 3);
            (fields[0], unhex(fields[1]), unhex(fields[2]))
        })
}

fn native(reference: &Jimsh, source: &str, context: &[u8], name: &[u8]) -> String {
    let source = source
        .replace("@CONTEXT@", &hex(context))
        .replace("@NAME@", &hex(name));
    run_script(&reference.path, source.as_bytes())
        .expect("execute original Jim naming source")
        .strict_text()
        .expect("clean native Jim naming output")
}

#[test]
fn jim_namespace_objects_and_text_helpers_match_native_operations() {
    let Some(reference) = reference() else {
        return;
    };
    let root = ByteNamespacePath::root();
    for (id, namespace, name) in rows(include_str!("data/jim_name_resolution/namespace.txt")) {
        let context = NativeNameContext::with_jim_namespace(&root, &namespace);
        let projected = PROTOCOL
            .jim_namespace_canonical_input(context, &name)
            .unwrap();
        let expected = format!(
            "{}|{}|{}|{}",
            hex(projected.selected()),
            hex(PROTOCOL.namespace_qualifier_bytes(&name)),
            hex(PROTOCOL.namespace_tail_bytes(&name)),
            hex(&PROTOCOL.jim_namespace_parent_bytes(&name).unwrap())
        );
        assert_eq!(
            native(
                &reference,
                include_str!("data/jim_name_resolution/namespace.tcl"),
                &namespace,
                &name
            ),
            expected,
            "{id}: Jim {}",
            reference.patchlevel
        );
    }
}

#[test]
fn jim_publication_preserves_actual_flat_keys_and_procedure_namespaces() {
    let Some(reference) = reference() else {
        return;
    };
    let root = ByteNamespacePath::root();
    for (id, namespace, name) in rows(include_str!("data/jim_name_resolution/publication.txt")) {
        let context = NativeNameContext::with_jim_namespace(&root, &namespace);
        let slot = PROTOCOL.command_publication_slot(context, &name).unwrap();
        assert_eq!(slot.namespace, root, "{id}: Jim table is flat");
        let publication = PROTOCOL.command_publication_input(context, &name).unwrap();
        let key = slot.simple.as_bytes();
        let expected = format!(
            "{}|{}",
            hex(publication.selected()),
            hex(PROTOCOL.jim_procedure_namespace(key).unwrap())
        );
        assert_eq!(
            native(
                &reference,
                include_str!("data/jim_name_resolution/publication.tcl"),
                &namespace,
                &name
            ),
            expected,
            "{id}: Jim {}",
            reference.patchlevel
        );
    }
}

#[test]
fn jim_lookup_searches_its_counted_flat_table_in_native_order() {
    let Some(reference) = reference() else {
        return;
    };
    let root = ByteNamespacePath::root();
    let keys: Vec<_> = include_str!("data/jim_name_resolution/lookup_keys.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| if line == "-" { Vec::new() } else { unhex(line) })
        .collect();
    for (id, namespace, name) in rows(include_str!("data/jim_name_resolution/lookup.txt")) {
        let context = NativeNameContext::with_jim_namespace(&root, &namespace);
        let candidates = PROTOCOL.jim_command_lookup_keys(context, &name).unwrap();
        let winner = candidates
            .iter()
            .find(|candidate| keys.iter().any(|key| key == candidate.as_bytes()));
        let actual = native(
            &reference,
            include_str!("data/jim_name_resolution/lookup.tcl"),
            &namespace,
            &name,
        );
        match winner {
            Some(key) => assert_eq!(
                actual,
                format!("0|{}", hex(key.as_bytes())),
                "{id}: Jim {}",
                reference.patchlevel
            ),
            None => assert!(
                actual.starts_with("1|"),
                "{id}: native lookup unexpectedly found {actual:?}"
            ),
        }
    }
}

#[test]
fn jim_global_alias_local_names_use_original_counted_operands() {
    let Some(reference) = reference() else {
        return;
    };
    for (id, namespace, name) in rows(include_str!("data/jim_name_resolution/global.txt")) {
        let expected = global_local_name_bytes(PROTOCOL, &name)
            .map_or_else(|| "0|".to_owned(), |key| format!("1|{}", hex(&key)));
        assert_eq!(
            native(
                &reference,
                include_str!("data/jim_name_resolution/global.tcl"),
                &namespace,
                &name
            ),
            expected,
            "{id}: Jim {}",
            reference.patchlevel
        );
    }
}

#[test]
fn jim_scalar_and_combined_dictionary_names_match_actual_cells() {
    let Some(reference) = reference() else {
        return;
    };
    for (id, namespace, name) in rows(include_str!("data/jim_name_resolution/variable.txt")) {
        let projection = PROTOCOL.combined_variable_input(&name);
        let element = projection
            .element()
            .map_or_else(|| "SCALAR".to_owned(), |element| hex(element.selected()));
        let expected = format!("{}|{element}", hex(projection.root().selected()));
        assert_eq!(
            native(
                &reference,
                include_str!("data/jim_name_resolution/variable.tcl"),
                &namespace,
                &name
            ),
            expected,
            "{id}: Jim {}",
            reference.patchlevel
        );
    }
}
