// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use crate::test_wasm_link as wasm_link;

fn runtime_bytes() -> Option<Vec<u8>> {
    Some(std::fs::read(wasm_link::real_link_runtime()?).expect("actual runtime artefact"))
}

fn session(runtime: &WasmRuntime) -> Session {
    let mut session = Session::new(runtime).expect("real instance");
    session
        .arm(
            limits_of(Budget::of_commands(100_000).with_wall_clock(Duration::from_secs(2)))
                .with_first_use(),
        )
        .expect("arm instance");
    session
        .set_release("tcl8.6")
        .expect("selected C86 command recipe");
    session
}

fn evaluate(session: &mut Session, script: &[u8]) {
    let completion = session
        .evaluate(&[b"::eval", script])
        .expect("actual entered script");
    assert_eq!(completion.code, 0, "{:?}", completion.result);
}

#[test]
fn actual_wasm_inventory_retains_colon_brackets_aliases_and_imports() {
    let Some(bytes) = runtime_bytes() else { return };
    let runtime = WasmRuntime::new(&bytes).expect("real runtime");
    let mut session = session(&runtime);
    evaluate(&mut session, b"proc :lost {} {return COLON}; proc lost {} {return PLAIN}; namespace eval {N[0]} {proc {p*} {} {return BRACKET}; namespace export {p*}}; namespace eval InventoryDest {namespace import {::N[0]::p*}}; interp alias {} inventoryAlias {} :lost");
    let (_, rows) = command_inventory(&mut session).expect("actual checked roster");
    for label in [
        ":lost",
        "lost",
        "N[0]::p*",
        "InventoryDest::p*",
        "inventoryAlias",
    ] {
        let (row, _) = rows
            .iter()
            .find(|(_, actual)| actual == label)
            .expect("retained actual binding");
        assert_eq!(row.root_operand.as_deref(), Some(label.as_bytes()));
    }
    assert_ne!(
        rows.iter()
            .find(|(_, name)| name == ":lost")
            .unwrap()
            .0
            .generation,
        rows.iter()
            .find(|(_, name)| name == "lost")
            .unwrap()
            .0
            .generation
    );
    for (label, expected) in [
        (":lost", b"COLON".as_slice()),
        ("lost", b"PLAIN"),
        ("N[0]::p*", b"BRACKET"),
        ("InventoryDest::p*", b"BRACKET"),
        ("inventoryAlias", b"COLON"),
    ] {
        let completion = session
            .evaluate(&[label.as_bytes()])
            .expect("actual exact invocation");
        assert_eq!(completion.code, 0);
        assert_eq!(completion.result, expected);
    }
}

#[test]
fn actual_wasm_inventory_declines_whole_roster_when_a_slot_has_no_root_address() {
    let Some(bytes) = runtime_bytes() else { return };
    let runtime = WasmRuntime::new(&bytes).expect("real runtime");
    let mut session = session(&runtime);
    evaluate(
        &mut session,
        b"proc retained {} {return ROOT}; namespace eval N: {proc x {} {return TRAILING}}",
    );
    let (_, data) = session.command_inventory().expect("actual byte roster");
    assert!(data.rows.iter().any(|row| {
        row.slot
            .namespace
            .as_segments()
            .last()
            .is_some_and(|part| part.as_ref() == b"N:")
            && row.slot.simple.as_ref() == b"x"
            && row.root_operand.is_none()
    }));
    assert!(matches!(
        command_inventory(&mut session),
        Err(DeclineReason::Unsupported)
    ));
    assert_eq!(session.evaluate(&[b"retained"]).unwrap().result, b"ROOT");
}

fn without_inventory_export(module: &[u8]) -> Vec<u8> {
    fn take<'a>(input: &mut &'a [u8], size: usize) -> &'a [u8] {
        let (head, tail) = input.split_at(size);
        *input = tail;
        head
    }
    fn number(input: &mut &[u8]) -> usize {
        let mut value = 0;
        for shift in (0..35).step_by(7) {
            let byte = take(input, 1)[0];
            value |= usize::from(byte & 127) << shift;
            if byte & 128 == 0 {
                return value;
            }
        }
        panic!("invalid actual Wasm unsigned integer")
    }
    fn append_number(output: &mut Vec<u8>, mut value: usize) {
        loop {
            let byte = u8::try_from(value & 127).unwrap();
            value >>= 7;
            output.push(byte | if value == 0 { 0 } else { 128 });
            if value == 0 {
                break;
            }
        }
    }
    let mut input = module;
    let mut output = take(&mut input, 8).to_vec();
    let mut removed = false;
    while !input.is_empty() {
        let id = take(&mut input, 1)[0];
        let size = number(&mut input);
        let payload = take(&mut input, size);
        let payload = if id == 7 {
            let mut exports = payload;
            let count = number(&mut exports);
            let mut kept = Vec::new();
            for _ in 0..count {
                let before = exports;
                let length = number(&mut exports);
                let name = take(&mut exports, length);
                take(&mut exports, 1);
                number(&mut exports);
                if name == b"tcl_engine_visible_command_inventory" {
                    removed = true;
                } else {
                    kept.push(before[..before.len() - exports.len()].to_vec());
                }
            }
            assert!(exports.is_empty());
            let mut result = Vec::new();
            append_number(&mut result, kept.len());
            for export in kept {
                result.extend_from_slice(&export);
            }
            result
        } else {
            payload.to_vec()
        };
        output.push(id);
        append_number(&mut output, payload.len());
        output.extend_from_slice(&payload);
    }
    assert!(removed, "fresh source-built runtime exports the inventory");
    output
}

#[test]
fn actual_old_wasm_artefact_without_inventory_export_declines_only_inventory() {
    let Some(bytes) = runtime_bytes() else { return };
    let old = without_inventory_export(&bytes);
    let runtime = WasmRuntime::new(&old).expect("actual module with inventory export omitted");
    let mut session = session(&runtime);
    let completion = session
        .evaluate(&[b"::list", b":lost"])
        .expect("ordinary engine entry still works");
    assert_eq!(completion.code, 0);
    assert_eq!(completion.result, b":lost");
    assert!(matches!(
        command_inventory(&mut session),
        Err(DeclineReason::Unsupported)
    ));
}
