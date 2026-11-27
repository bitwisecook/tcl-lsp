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

//! Compiled locals, `foreach` slots and `dict` search epilogues in procedure
//! bodies, against the tclsh and jimsh captures.

use crate::prelude::*;
use tcl_vm::NativeCommand;

fn ok(value: Value) -> Completion<Value> {
    Completion::new(Code::Ok, value, Value::empty())
}

fn borrowed_source_names() -> BorrowedSourceNames {
    include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/names.tsv")
        .lines()
        .map(|row| {
            let mut fields = tcl_test_support::fixture::tsv_fields(row);
            (
                fields.next().unwrap(),
                (unhex(fields.next().unwrap()), unhex(fields.next().unwrap())),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>()
}

struct OriginalName(Value);
impl NativeCommand for OriginalName {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        ok(self.0.clone())
    }
}

/// The search probe the captures were taken with; its reference counts are
/// not compared.
struct Capture;
impl NativeCommand for Capture {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::empty())
    }
}

fn check_compiled_dictionary_epilogue(engine: &str, row: &str) {
    let profile = tcl_vm::test_support::profile(engine);
    let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
    let (family, action) = fields[0].split_once('-').unwrap();
    let body = match action {
        "normal" => "set ignored OK",
        "error" => "error BODY",
        "break" => "break",
        "continue" => "continue",
        "return" => "return BODY",
        _ => unreachable!(),
    };
    let source = format!(
        "proc p {{}} {{set d {{k V}}; capture before $d;set c [catch {{dict {family} {{k v}} $d {{capture during $d;{body}}}}} r o];capture after $d;list $c $r [dict get $o -code] [dict get $o -level] $o}};p"
    );
    let receipt_start = std::time::Instant::now();
    tcl_test_support::oracle_row_progress("dictionary-epilogues35", engine, fields[0], None);
    tcl_test_support::oracle_phase_progress(
        "dictionary-epilogues35",
        engine,
        fields[0],
        "vm-new-start",
        receipt_start,
    );
    let mut vm = dictionary_epilogue_vm(profile, engine, fields[0], receipt_start);
    vm.register_native_command("capture", Rc::new(Capture));
    tcl_test_support::oracle_phase_progress(
        "dictionary-epilogues35",
        engine,
        fields[0],
        "eval-start",
        receipt_start,
    );
    let completion = vm.try_eval_source_bytes(source.as_bytes()).unwrap();
    tcl_test_support::oracle_phase_progress(
        "dictionary-epilogues35",
        engine,
        fields[0],
        "eval-complete",
        receipt_start,
    );
    assert_eq!(
        completion.code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{engine}/{}",
        fields[0]
    );
    assert_eq!(
        crate::fixture::bytes(&vm, &completion.result).as_slice(),
        unhex(fields[2]).as_slice(),
        "{engine}/{}",
        fields[0]
    );
    drop(vm);
    tcl_test_support::oracle_phase_progress(
        "dictionary-epilogues35",
        engine,
        fields[0],
        "vm-drop-complete",
        receipt_start,
    );
}

struct OriginalScript(Vec<u8>);
impl NativeCommand for OriginalScript {
    fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let mut bytes = b"set {".to_vec();
        bytes.extend_from_slice(&self.0);
        bytes.extend_from_slice(if args.is_empty() {
            b"};set {"
        } else {
            b"} ALTER;set {"
        });
        bytes.extend_from_slice(&self.0);
        bytes.push(b'}');
        ok(Value::from_native_string_bytes(bytes))
    }
}

const BORROWED_SOURCE_FIXTURES: [(&str, &str); 6] = [
    (
        "tcl8.4",
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/9.1.0.tsv"),
    ),
    (
        "jim",
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/Jim.tsv"),
    ),
];

type BorrowedSourceNames = std::collections::BTreeMap<&'static str, (Vec<u8>, Vec<u8>)>;

fn borrowed_source_vm(
    profile: &'static tcl_dialect::DialectProfile,
    engine: &str,
    case: &str,
    receipt_start: std::time::Instant,
) -> Vm {
    let vm = crate::fixture::interpreter(profile);
    tcl_test_support::oracle_phase_progress(
        "borrowed-source240",
        engine,
        case,
        "vm-new-complete",
        receipt_start,
    );
    vm
}

/// Compare a completion with its recorded row before the next command
/// resets the interpreter result, as the probe reports each call.
fn assert_slot_row(
    vm: &mut Vm,
    engine: &str,
    row: &str,
    actual: &tcl_runtime_api::Completion<Value>,
) {
    let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
    let case = fields[..3].join("/");
    assert_eq!(
        actual.code.as_int(),
        fields[3].parse::<i64>().unwrap(),
        "{engine}/{case}"
    );
    assert_eq!(
        crate::fixture::bytes(vm, &actual.result).as_slice(),
        unhex(fields[4]).as_slice(),
        "{engine}/{case}"
    );
}

fn dictionary_callback_vm(
    profile: &'static tcl_dialect::DialectProfile,
    engine: &str,
    case: &str,
    add: bool,
    receipt_start: std::time::Instant,
) -> Vm {
    let mut vm = crate::fixture::interpreter(profile);
    tcl_test_support::oracle_phase_progress(
        "dictionary-callback14",
        engine,
        case,
        "vm-new-complete",
        receipt_start,
    );
    vm.register_native_command(
        "advance",
        tcl_vm::native_conformance::dictionary_search_mutation_driver(
            tcl_native_model::InvocationDialect::of_profile(profile),
            add,
        )
        .unwrap(),
    );
    vm
}

fn dictionary_epilogue_vm(
    profile: &'static tcl_dialect::DialectProfile,
    engine: &str,
    case: &str,
    receipt_start: std::time::Instant,
) -> Vm {
    let vm = crate::fixture::interpreter(profile);
    tcl_test_support::oracle_phase_progress(
        "dictionary-epilogues35",
        engine,
        case,
        "vm-new-complete",
        receipt_start,
    );
    vm
}

fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2));
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn install_original_name_providers(
    vm: &mut Vm,
    first: &[u8],
    second: &[u8],
    duplicate: bool,
) -> Vec<Value> {
    vm.register_native_command(
        "originalA",
        Rc::new(OriginalName(Value::from_native_string_bytes(
            first.to_vec(),
        ))),
    );
    vm.register_native_command(
        "originalB",
        Rc::new(OriginalName(Value::from_native_string_bytes(
            second.to_vec(),
        ))),
    );
    vm.register_native_command("originalBScript", Rc::new(OriginalScript(second.to_vec())));
    let mut formals = vec![Value::from_native_string_bytes(first.to_vec())];
    if duplicate {
        formals.push(Value::from_native_string_bytes(second.to_vec()));
    }
    formals
}

#[test]
fn original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references() {
    // Native proof: naming.procedure.borrowed-slot-duplicate-original-formals
    // docs/design/analysis/name-resolution-proofs/procedure-borrowed-slot-duplicate-original-formals.md
    // Native proof: naming.procedure.borrowed-slot-counted-zero-length
    // docs/design/analysis/name-resolution-proofs/procedure-borrowed-slot-counted-zero-length.md
    // Native proof: naming.procedure.borrowed-slot-modified-zero-and-opaque-bytes
    // docs/design/analysis/name-resolution-proofs/procedure-borrowed-slot-modified-zero-and-opaque-bytes.md
    // Native proof: naming.procedure.borrowed-slot-qualified-formal-boundary
    // docs/design/analysis/name-resolution-proofs/procedure-borrowed-slot-qualified-formal-boundary.md
    // Native proof: naming.procedure.borrowed-slot-array-shaped-formal-boundary
    // docs/design/analysis/name-resolution-proofs/procedure-borrowed-slot-array-shaped-formal-boundary.md
    let names = borrowed_source_names();
    let body = unhex(
        include_str!("../../../../../tcl-vm/tests/data/native_borrowed_frame_slots/body.hex")
            .trim(),
    );
    let engines = BORROWED_SOURCE_FIXTURES;
    let mut compared = 0;
    for (engine, expected) in engines {
        let rows = expected.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 40);
        let profile = tcl_vm::test_support::profile(engine);
        for pair in rows.as_chunks::<2>().0 {
            let fields = tcl_test_support::fixture::tsv_fields(pair[0]).collect::<Vec<_>>();
            let (first, second) = &names[fields[0]];
            let duplicate = fields[1] == "two-original-formals";
            let case = format!("{}/{}", fields[0], fields[1]);
            let receipt_start = std::time::Instant::now();
            tcl_test_support::oracle_row_progress("borrowed-source240", engine, &case, None);
            tcl_test_support::oracle_phase_progress(
                "borrowed-source240",
                engine,
                &case,
                "vm-new-start",
                receipt_start,
            );
            let mut vm = borrowed_source_vm(profile, engine, &case, receipt_start);
            let formals = install_original_name_providers(&mut vm, first, second, duplicate);
            tcl_test_support::oracle_phase_progress(
                "borrowed-source240",
                engine,
                &case,
                "definition-start",
                receipt_start,
            );
            let definition = crate::fixture::call(
                &mut vm,
                "proc",
                &[
                    Value::string("p"),
                    Value::list(formals),
                    Value::from_native_string_bytes(body.clone()),
                ],
            );
            tcl_test_support::oracle_phase_progress(
                "borrowed-source240",
                engine,
                &case,
                "definition-complete",
                receipt_start,
            );
            assert_slot_row(&mut vm, engine, pair[0], &definition);
            compared += 1;
            tcl_test_support::oracle_row_progress(
                "borrowed-source240",
                engine,
                &case,
                Some(compared),
            );
            let mut arguments = vec![Value::string("ONE")];
            if duplicate {
                arguments.push(Value::string("TWO"));
            }
            tcl_test_support::oracle_phase_progress(
                "borrowed-source240",
                engine,
                &case,
                "invoke-start",
                receipt_start,
            );
            let invocation = crate::fixture::call(&mut vm, "p", &arguments);
            tcl_test_support::oracle_phase_progress(
                "borrowed-source240",
                engine,
                &case,
                "invoke-complete",
                receipt_start,
            );
            assert_slot_row(&mut vm, engine, pair[1], &invocation);
            compared += 1;
            tcl_test_support::oracle_row_progress(
                "borrowed-source240",
                engine,
                &case,
                Some(compared),
            );
        }
    }
    assert_eq!(compared, 240);
}

// Native proof: naming.variable.counted-formal-foreach-slot-execution-frontier
// docs/design/analysis/name-resolution-proofs/variable.counted-formal-foreach-slot-execution-frontier.md
#[test]
fn compiled_foreach_matches_twelve_original_native_slot_records() {
    let body = unhex(
        include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/body.hex").trim(),
    );
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../../tcl-vm/tests/data/native_foreach_slots/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, expected) in engines {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::interpreter(profile);
        let first = Value::from_native_string_bytes(b"k\0a".to_vec());
        let second = Value::from_native_string_bytes(b"k\0b".to_vec());
        vm.register_native_command("originalA", Rc::new(OriginalName(first.clone())));
        vm.register_native_command("originalB", Rc::new(OriginalName(second.clone())));
        let rows = expected.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        let definition = crate::fixture::call(
            &mut vm,
            "proc",
            &[
                Value::string("p"),
                Value::list(vec![first, second]),
                Value::from_native_string_bytes(body.clone()),
            ],
        );
        assert_slot_row(&mut vm, engine, rows[0], &definition);
        let invocation =
            crate::fixture::call(&mut vm, "p", &[Value::string("ONE"), Value::string("TWO")]);
        assert_slot_row(&mut vm, engine, rows[1], &invocation);
        compared += 2;
    }
    assert_eq!(compared, 12);
}

#[test]
fn mutated_dictionary_callback_search_preserves_fourteen_native_fatal_boundaries() {
    let mut compared = 0;
    for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_vm::test_support::profile(engine);
        for add in [false, true] {
            let scripts: &[&[u8]] = if engine == "tcl8.5" {
                &[b"catch {advance};set reached yes"]
            } else {
                &[
                    b"catch {advance};set reached yes",
                    b"try {advance} on error {r o} {set caught yes};set reached yes",
                ]
            };
            for source in scripts {
                let case = format!(
                    "{}/{:}",
                    if add { "add" } else { "replace" },
                    if source.starts_with(b"catch") {
                        "catch"
                    } else {
                        "try"
                    }
                );
                let receipt_start = std::time::Instant::now();
                tcl_test_support::oracle_row_progress("dictionary-callback14", engine, &case, None);
                tcl_test_support::oracle_phase_progress(
                    "dictionary-callback14",
                    engine,
                    &case,
                    "vm-new-start",
                    receipt_start,
                );
                let mut vm = dictionary_callback_vm(profile, engine, &case, add, receipt_start);
                tcl_test_support::oracle_phase_progress(
                    "dictionary-callback14",
                    engine,
                    &case,
                    "eval-start",
                    receipt_start,
                );
                let error = vm.try_eval_source_bytes(source).unwrap_err();
                tcl_test_support::oracle_phase_progress(
                    "dictionary-callback14",
                    engine,
                    &case,
                    "eval-complete",
                    receipt_start,
                );
                assert_eq!(
                    error,
                    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::FatalCondition(
                            tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation,
                        ),
                    ),
                    "{engine}"
                );
                assert!(vm.get_var_bytes(b"reached").is_none());
                assert!(vm.get_var_bytes(b"caught").is_none());
                drop(vm);
                tcl_test_support::oracle_phase_progress(
                    "dictionary-callback14",
                    engine,
                    &case,
                    "vm-drop-complete",
                    receipt_start,
                );
                compared += 1;
                tcl_test_support::oracle_row_progress(
                    "dictionary-callback14",
                    engine,
                    &case,
                    Some(compared),
                );
            }
        }
    }
    assert_eq!(compared, 14);
}

#[test]
fn compiled_dictionary_c86_for_error_preserves_original_completion_and_reference_windows() {
    let row = include_str!(
        "../../../../../tcl-vm/tests/data/native_dictionary_search/epilogues/8.6.18.tsv"
    )
    .lines()
    .find(|row| row.starts_with("for-error\t"))
    .expect("original native C86 for-error row");
    check_compiled_dictionary_epilogue("tcl8.6", row);
}

#[test]
fn compiled_dictionary_epilogues_match_35_native_completions_and_reference_windows() {
    let engines = [
        (
            "tcl8.5",
            include_str!(
                "../../../../../tcl-vm/tests/data/native_dictionary_search/epilogues/8.5.19.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../../tcl-vm/tests/data/native_dictionary_search/epilogues/8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../../tcl-vm/tests/data/native_dictionary_search/epilogues/9.0.4.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../../tcl-vm/tests/data/native_dictionary_search/epilogues/9.1.0.tsv"
            ),
        ),
    ];
    let mut compared = 0;
    for (engine, expected) in engines {
        for row in expected.lines() {
            let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            check_compiled_dictionary_epilogue(engine, row);
            compared += 1;
            tcl_test_support::oracle_row_progress(
                "dictionary-epilogues35",
                engine,
                fields[0],
                Some(compared),
            );
        }
    }
    assert_eq!(compared, 35);
}
