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
use crate::interp::default_host;
use crate::interp::obj_bytes;
use crate::interp::Param;
use crate::namespace::GLOBAL;
use std::cell::RefCell;

#[test]
fn original_body_word_parts_match_native_concatenation_windows() {
    // Native proof: naming.word-and-subst.original-concatenation
    // docs/design/analysis/name-resolution-proofs/word-and-subst-original-concatenation.md
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    fn primary(value: *mut TclObj) -> &'static str {
        match obj::native_object_snapshot(value).unwrap().cache {
            Cache::None => "none",
            Cache::String { .. } => "string",
            Cache::ByteArray { .. } => "bytearray",
            Cache::List { .. } => "list",
            Cache::Numeric(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Double(_),
            )) => "double",
            other => panic!("unexpected original primary {other:?}"),
        }
    }
    let rows = include_str!(
            "../../../../../rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/observations.tsv"
        );
    for (engine, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        for kind in 0..8 {
            let mut interp = interpreter(engine);
            let dialect = interp.native_invocation_dialect();
            let protocol = dialect.native_string_protocol().unwrap();
            let binary = dialect.byte_array_string_recipe(None).unwrap();
            let x = obj::Owned::fresh(match kind {
                0 => obj::new_string_bytes(b"A"),
                1 => obj::new_native_unicode_obj(Rc::from([0, 0xd800]), dialect).unwrap(),
                2 => obj::new_string_bytes(b"A\0B"),
                3 => crate::bytearray::new_byte_array(b"\0\xff", binary),
                4 => obj::new_string_bytes(b"\xff\xed\xa0\x80"),
                5 => crate::list::new_list_obj_native(&[], protocol),
                6 => obj::new_double_obj(1.5),
                7 => obj::new_string_bytes(b""),
                _ => unreachable!(),
            });
            let y = obj::Owned::fresh(match kind {
                1 => obj::new_string_bytes(b""),
                3 => crate::bytearray::new_byte_array(b"\0\xff", binary),
                _ => obj::new_string_bytes(b"A"),
            });
            for repetition in 0..2 {
                let expected = rows
                    .lines()
                    .skip(1)
                    .map(|row| row.split('\t').collect::<Vec<_>>())
                    .find(|row| {
                        row[0] == version
                            && row[1] == kind.to_string()
                            && row[2] == "0"
                            && row[3] == repetition.to_string()
                    })
                    .unwrap();
                let context = format!("{engine}/{kind}/{repetition}");
                assert_eq!(primary(x.as_ptr()), expected[6], "{context} x before");
                assert_eq!(
                    usize::from(obj::has_string_rep(x.as_ptr())).to_string(),
                    expected[7],
                    "{context} x before storage"
                );
                let result = interp
                    .concatenate_body_values(vec![
                        obj::Owned::retain(x.as_ptr()),
                        obj::Owned::retain(y.as_ptr()),
                    ])
                    .unwrap();
                assert_eq!(
                    primary(result.as_ptr()),
                    expected[10],
                    "{context} result primary"
                );
                assert_eq!(
                    usize::from(obj::has_string_rep(result.as_ptr())).to_string(),
                    expected[11],
                    "{context} result storage"
                );
                assert_eq!(
                    usize::from(result.as_ptr() == x.as_ptr()).to_string(),
                    expected[12],
                    "{context} x identity"
                );
                assert_eq!(
                    usize::from(result.as_ptr() == y.as_ptr()).to_string(),
                    expected[13],
                    "{context} y identity"
                );
                assert_eq!(primary(x.as_ptr()), expected[14], "{context} x after");
                assert_eq!(
                    usize::from(obj::has_string_rep(x.as_ptr())).to_string(),
                    expected[15],
                    "{context} x after storage"
                );
                let bytes = interp.native_object_string_bytes(result.as_ptr()).unwrap();
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                assert_eq!(hex, expected[18], "{context} value");
            }
        }
    }
}

#[test]
fn original_c91_uplevel_artifact_selects_frame_then_enters_original_script() {
    for (source, expected) in [
            (b"set x BEFORE; proc p {} {set x INNER;uplevel 1 {set x OUTER};return $x};list [p] $x".as_slice(), b"INNER OUTER".as_slice()),
            (b"set x BEFORE; proc p {} {set x INNER;uplevel set x OUTER;return $x};list [p] $x".as_slice(), b"INNER OUTER".as_slice()),
            (b"set x BEFORE; proc p {} {uplevel 1 set x OUTER};list [p] $x".as_slice(), b"OUTER OUTER".as_slice()),
            (b"set x BEFORE;proc p {} {catch {uplevel #99 {}} message;set x INNER;return [list $message $x]};list [p] $x".as_slice(), b"{{bad level \"#99\"} INNER} BEFORE".as_slice()),
        ] {
            let mut interp = interpreter("tcl9.1");
            assert_eq!(interp.eval_str(source), Code::Ok, "{}", String::from_utf8_lossy(source));
            assert_eq!(interp.result_bytes(), expected);
            assert!(!interp.host_refusal_pending());
            let procedure = interp.proc_def(b"p").expect("original procedure");
            let artifact = cache(procedure.body.as_ptr()).expect("actual original bytecode");
            assert!(artifact.scripts.values().any(|script| script.commands.iter().any(|command| matches!(command.operation, Operation::Uplevel(..)))));
        }
}

thread_local! {
    static GETTER_ORDER: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}
extern "C" fn update_index(value: *mut TclObj) {
    GETTER_ORDER.with(|events| events.borrow_mut().push("getter"));
    // SAFETY: this exact updater receives the same live original operand.
    unsafe { obj::set_native_updater_string_rep(value, b"k", false) };
}
static INDEX_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"artifactIndex".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_index),
    set_from_any_proc: None,
};
fn later_operand(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    GETTER_ORDER.with(|events| events.borrow_mut().push("value"));
    interp.set_result_bytes(b"2");
    Code::Ok
}

pub(super) fn interpreter(profile: &str) -> Interp {
    let interp = Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("authenticated original core constructor");
    let protocol = interp
        .source_string_protocol()
        .unwrap_or_else(|| panic!("{profile}: selected original source protocol"));
    match protocol {
        tcl_syntax::native_string::NativeStringProtocol::C(_) => assert!(
            interp.native_compiler_cache_epochs(GLOBAL).is_some(),
            "{profile}: actual C compiler epochs"
        ),
        tcl_syntax::native_string::NativeStringProtocol::Jim084 => assert!(
            interp.native_compiler_cache_epochs(GLOBAL).is_none(),
            "{profile}: Jim has no C compiler epoch"
        ),
    }
    interp
}

#[test]
fn original_procedure_artifact_executes_indexed_slots_and_reuses_original_cache() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(
            interp.eval_str(b"proc p {value} {set x $value; incr x; set a(k) $x; return $a(k)}"),
            Code::Ok,
            "{profile}"
        );
        let procedure = interp.proc_def(b"p").expect("original declaration");
        assert!(cache(procedure.body.as_ptr()).is_none());
        assert_eq!(interp.eval_str(b"p 3"), Code::Ok, "{profile}");
        assert_eq!(interp.result_bytes(), b"4", "{profile}");
        let first = cache(procedure.body.as_ptr()).expect("actual emitted original artifact");
        let layout = first
            .compiled_local_layout()
            .expect("actual emitted local slots");
        assert_eq!(
            layout
                .names
                .iter()
                .map(|name| name.as_ref().map(|name| name.as_bytes()))
                .collect::<Vec<_>>(),
            vec![Some(&b"value"[..]), Some(&b"x"[..]), Some(&b"a"[..])],
            "{profile}"
        );
        assert_eq!(interp.eval_str(b"p 7"), Code::Ok, "{profile}");
        assert_eq!(interp.result_bytes(), b"8", "{profile}");
        assert!(
            Rc::ptr_eq(
                &first,
                &cache(procedure.body.as_ptr()).expect("reused original cache")
            ),
            "{profile}"
        );
        let duplicate = obj::Owned::fresh(obj::duplicate(procedure.body.as_ptr()));
        assert!(cache(duplicate.as_ptr()).is_none(), "{profile}");
        assert_eq!(
            obj_bytes(duplicate.as_ptr()),
            obj_bytes(procedure.body.as_ptr())
        );
    }
}

#[test]
fn original_append_artifacts_keep_registered_operands_and_native_local_layout() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (body, expected) in [
            (
                b"set r $input; append r A; return $r".as_slice(),
                b"XA".as_slice(),
            ),
            (
                b"set r $input; append r A B; return $r".as_slice(),
                b"XAB".as_slice(),
            ),
            (
                b"set r $input; lappend r A; return $r".as_slice(),
                b"X A".as_slice(),
            ),
            (
                b"set r $input; lappend r A B; return $r".as_slice(),
                b"X A B".as_slice(),
            ),
            (
                b"set a(k) $input; lappend a(k) A; return $a(k)".as_slice(),
                b"X A".as_slice(),
            ),
        ] {
            let mut interp = interpreter(profile);
            let original = obj::Owned::fresh(new_string(body));
            interp.define_proc(
                b"p",
                vec![Param {
                    name: b"input".to_vec(),
                    default: None,
                }],
                original.as_ptr(),
            );
            assert_eq!(
                interp.eval_str(b"p X"),
                Code::Ok,
                "{profile}/{body:?}: {:?}",
                interp.result_bytes()
            );
            assert_eq!(interp.result_bytes(), expected, "{profile}/{body:?}");
            let artifact = cache(original.as_ptr()).expect("retained original append artifact");
            let script = artifact.scripts.get(&artifact.region).unwrap();
            assert!(
                script.commands.iter().any(|command| matches!(
                    command.operation,
                    Operation::Append(..)
                ) || matches!(
                    command.operation,
                    Operation::Invoke
                )),
                "{profile}: executable selected recipe"
            );
            let layout = artifact.compiled_local_layout().unwrap();
            assert!(
                layout.names.iter().any(|name| name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes() == b"r" || name.as_bytes() == b"a")),
                "{profile}: retained indexed receiver"
            );
            assert_eq!(interp.eval_str(b"p Y"), Code::Ok, "{profile}/{body:?}");
            assert!(Rc::ptr_eq(&artifact, &cache(original.as_ptr()).unwrap()));
        }
    }
}

#[test]
fn registered_append_artifacts_match_43_native_callback_controls() {
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut count = 0;
    for row in include_str!("../../../tests/data/native_registered_append.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 7);
        let profile = fields[0];
        let mut interp = interpreter(profile);
        assert_eq!(interp.eval_str(b"proc watch {n1 n2 op} {lappend ::events $op}; proc arm {} {uplevel 1 {trace add variable r {read write} watch}}; set events {}"), Code::Ok, "{profile}/{}: observer setup", fields[1]);
        let original = obj::Owned::fresh(new_string(&decode(fields[2])));
        interp.define_proc(
            b"p",
            vec![Param {
                name: b"input".to_vec(),
                default: None,
            }],
            original.as_ptr(),
        );
        let head = obj::Owned::fresh(new_string(b"p"));
        let input = obj::Owned::fresh(new_string(&decode(fields[3])));
        let code = interp.dispatch(&[head.as_ptr(), input.as_ptr()]);
        assert_eq!(
            code.as_int(),
            fields[4].parse::<i64>().unwrap(),
            "{profile}/{}: {:?}",
            fields[1],
            interp.result_bytes()
        );
        assert_eq!(
            interp.result_bytes(),
            decode(fields[5]),
            "{profile}/{}: result",
            fields[1]
        );
        let events = interp
            .var_get(b"events")
            .expect("actual callback event variable");
        assert_eq!(
            obj_bytes(events),
            decode(fields[6]),
            "{profile}/{}: observer order",
            fields[1]
        );
        assert!(
            cache(original.as_ptr()).is_some(),
            "{profile}/{}: original executable body",
            fields[1]
        );
        count += 1;
    }
    assert_eq!(count, 43);
}

#[test]
fn original_artifact_keeps_opcode_selection_before_argument_mutation() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(interp.eval_str(b"proc change {} {rename set savedSet; proc set args {return CUSTOM}; return VALUE}; proc p {} {set x [change]; return $x}"), Code::Ok);
        assert_eq!(interp.eval_str(b"p"), Code::Ok, "{profile}");
        assert_eq!(interp.result_bytes(), b"VALUE", "{profile}");
        assert!(
            cache(interp.proc_def(b"p").expect("p").body.as_ptr()).is_some(),
            "{profile}"
        );
    }
}

#[test]
fn original_index_getter_follows_later_store_and_increment_operands() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for body in [
            b"set a($index) [later]; return $a(k)".as_slice(),
            b"set a(k) 1; incr a($index) [later]; return $a(k)".as_slice(),
            b"set $index [later]; return $k".as_slice(),
        ] {
            let mut interp = interpreter(profile);
            interp.register_builtin(b"later", later_operand);
            let original_body = obj::Owned::fresh(new_string(body));
            interp.define_proc(
                b"p",
                vec![Param {
                    name: b"index".to_vec(),
                    default: None,
                }],
                original_body.as_ptr(),
            );
            let index = obj::Owned::fresh(obj::new_obj());
            obj::change_type(index.as_ptr(), &INDEX_TYPE, 0);
            obj::invalidate_string(index.as_ptr());
            let head = obj::Owned::fresh(new_string(b"p"));
            GETTER_ORDER.with(|events| events.borrow_mut().clear());
            assert_eq!(
                interp.dispatch(&[head.as_ptr(), index.as_ptr()]),
                Code::Ok,
                "{profile}/{body:?}"
            );
            GETTER_ORDER.with(|events| {
                assert_eq!(
                    events.borrow().as_slice(),
                    ["value", "getter"],
                    "{profile}/{body:?}"
                )
            });
            assert!(
                cache(interp.proc_def(b"p").unwrap().body.as_ptr()).is_some(),
                "{profile}: actual executable artifact"
            );
        }
    }
}

#[test]
fn procedure_done_in_bracket_exits_without_return_option_state() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(
            interp.eval_str(b"proc p {} {set x [return EARLY]; set ::late YES}; p"),
            Code::Ok,
            "{profile}"
        );
        assert_eq!(interp.result_bytes(), b"EARLY", "{profile}");
        assert!(
            interp.var_get(b"::late").is_none(),
            "{profile}: later store is not entered"
        );
        assert!(cache(interp.proc_def(b"p").unwrap().body.as_ptr()).is_some());
        assert!(
            interp.pending_return_option_objects().is_empty(),
            "{profile}: DONE has no merged options operand"
        );
    }
}

#[test]
fn original_artifact_empty_source_and_parse_failure_keep_native_publication_policy() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        for (name, body) in [
            (b"empty".as_slice(), b"".as_slice()),
            (b"bad", b"{"),
            (b"early", b"return EARLY; {"),
        ] {
            let original = obj::Owned::fresh(new_string(body));
            interp.define_proc(name, Vec::new(), original.as_ptr());
        }
        assert_eq!(interp.eval_str(b"empty"), Code::Ok, "{profile}");
        assert!(
            cache(interp.proc_def(b"empty").expect("empty").body.as_ptr()).is_some(),
            "{profile}"
        );
        assert_eq!(interp.eval_str(b"bad"), Code::Error, "{profile}");
        let modern = profile != "tcl8.4";
        assert_eq!(
            cache(interp.proc_def(b"bad").expect("bad").body.as_ptr()).is_some(),
            modern,
            "{profile}"
        );
        assert_eq!(
            interp.eval_str(b"early"),
            if modern { Code::Ok } else { Code::Error },
            "{profile}"
        );
        if modern {
            assert_eq!(interp.result_bytes(), b"EARLY");
        }
    }
}

#[test]
fn syntax_instruction_reuses_same_stringless_options_dictionary() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        let original = obj::Owned::fresh(new_string(b"{"));
        interp.define_proc(b"bad", Vec::new(), original.as_ptr());
        assert_eq!(interp.eval_str(b"bad"), Code::Error, "{profile}");
        let body = interp.proc_def(b"bad").unwrap().body.as_ptr();
        let first = cache(body).expect("authentic cached SYNTAX program");
        let fatal = first.scripts[&first.region].fatal.as_ref().unwrap();
        let options = first.literals.original(fatal.options).unwrap();
        assert!(core::ptr::eq(
            obj::obj_type_ptr(options),
            &crate::dict::TCL_DICT_TYPE
        ));
        assert!(
            !obj::has_string_rep(options),
            "{profile}: TclAddLiteralObj does not GetString"
        );
        assert_eq!(interp.eval_str(b"bad"), Code::Error, "{profile}");
        assert!(Rc::ptr_eq(&first, &cache(body).unwrap()), "{profile}");
        assert_eq!(first.literals.original(fatal.options), Some(options));
        assert!(core::ptr::eq(
            obj::obj_type_ptr(options),
            &crate::dict::TCL_DICT_TYPE
        ));
        assert!(
            !obj::has_string_rep(options),
            "{profile}: SYNTAX does not reach RETURN_STK List conversion"
        );
    }
}

#[test]
fn original_artifact_legacy_recompilation_uses_actual_roles_and_fresh_body() {
    use tcl_runtime_api::native_procedure_roles::NativeProcedureReference;
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(
            interp.eval_str(b"proc p {{value DEFAULT}} {return $value}; p"),
            Code::Ok
        );
        let first = interp.proc_def(b"p").unwrap();
        let body = first.body.checked_ptr().unwrap();
        let default = first.params[0]
            .default
            .as_ref()
            .unwrap()
            .checked_ptr()
            .unwrap();
        assert!(cache(body).is_some(), "{profile}: genuine warm artifact");
        let role = NativeProcedureReference::acquire(&first)
            .expect("actual retained native procedure role");
        assert_eq!(
            interp.eval_str(b"rename set savedSet; p"),
            Code::Ok,
            "{profile}"
        );
        assert_eq!(interp.result_bytes(), b"DEFAULT");
        let next = interp.proc_def(b"p").unwrap();
        let legacy = matches!(profile, "tcl8.4" | "tcl8.5");
        assert_eq!(
            !Rc::ptr_eq(&first, &next),
            legacy,
            "{profile}: actual client data"
        );
        assert_eq!(
            next.body.as_ptr() != body,
            legacy,
            "{profile}: actual chosen source"
        );
        assert_eq!(
            next.params[0].default.as_ref().unwrap().as_ptr(),
            default,
            "{profile}: same default"
        );
        assert!(
            cache(next.body.as_ptr()).is_some(),
            "{profile}: newly admitted original body"
        );
        drop(role);
    }
}

#[test]
fn original_artifact_recompiles_for_physical_compiler_mutation() {
    let mut interp = interpreter("tcl9.1");
    assert_eq!(
        interp.eval_str(b"proc p {} {set x ONE; return $x}; p"),
        Code::Ok
    );
    let procedure = interp.proc_def(b"p").expect("p");
    let first = cache(procedure.body.as_ptr()).expect("first artifact");
    assert_eq!(
        interp.eval_str(b"proc unrelated {} {return OTHER}; p"),
        Code::Ok
    );
    assert!(Rc::ptr_eq(
        &first,
        &cache(procedure.body.as_ptr()).expect("unchanged compile epoch")
    ));
    assert_eq!(
        interp.eval_str(b"rename set savedSet; proc set args {return CUSTOM}; p"),
        Code::Error
    );
    let second = cache(procedure.body.as_ptr()).expect("new generic artifact");
    assert!(!Rc::ptr_eq(&first, &second));
}
#[test]
fn registered_return_artifacts_match_44_native_controls() {
    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for line in
        include_str!("../../../../../rust/tcl-registry/tests/data/registered-return44.tsv").lines()
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        let mut interp = interpreter(match fields[0] {
            "8.5.19" => "tcl8.5",
            "8.6.18" => "tcl8.6",
            "9.0.4" => "tcl9.0",
            "9.1.0" => "tcl9.1",
            _ => panic!("native version"),
        });
        let mut source = b"proc p {opts msg code} {".to_vec();
        source.extend(bytes(fields[2]));
        source.extend(b"}; p {-custom DYNAMIC} MSG error");
        let code = interp.eval_str(&source);
        assert_eq!(
            code,
            Code::from_int(fields[3].parse().unwrap()),
            "{}/{}: {:?}",
            fields[0],
            fields[1],
            interp.result_bytes()
        );
        assert_eq!(
            interp.result_bytes(),
            bytes(fields[4]),
            "{}/{}",
            fields[0],
            fields[1]
        );
        compared += 1;
    }
    assert_eq!(compared, 44);
}
#[test]
fn original_compiled_generic_expansion_stops_before_real_target_dispatch() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use crate::cmd_control::original_list_conversion_tests as fixture;
    use std::cell::Cell;
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 0);
    assert_eq!(
        interpreter
            .eval_str(b"proc q args {return REACHED}; proc p {input} {q {*}$input}; p {A B}"),
        Code::Ok
    );
    let procedure = interpreter.proc_def(b"p").unwrap();
    let artifact = cache(procedure.body.as_ptr()).expect("actual generic compiled artifact");
    assert!(artifact
        .scripts
        .values()
        .flat_map(|script| &script.commands)
        .any(|command| matches!(&command.operation, Operation::Invoke)
            && command
                .words
                .iter()
                .any(|word| word.original.group().expand)));
    let head = fixture::string(b"p");
    let input = fixture::original(b"A B");
    fixture::MODE.with(|mode| mode.set(1));
    let argv = [head.as_ptr(), input.as_ptr()];
    assert_eq!(interpreter.eval_original_object_vector(&argv), Code::Error);
    fixture::host(&interpreter.native_execution_refusal().unwrap());
    assert_eq!(fixture::FREES.with(Cell::get), 1);
    assert_eq!(fixture::GETTERS.with(Cell::get), 0);
    assert!(!core::ptr::eq(
        obj::obj_type_ptr(input.as_ptr()),
        &crate::list::TCL_LIST_TYPE
    ));
    assert_eq!(unsafe { (*input.as_ptr()).ref_count }, 1);
    assert_eq!(interpreter.frames.borrow().top_level(), 0);
}
