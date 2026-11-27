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

//! `::static` publication across enrolled iRules worker interpreters,
//! against the recorded BIG-IP observations.
use crate::prelude::*;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    NativeVariableTraceToken,
};

fn vm() -> Vm {
    let profile = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = crate::fixture::interpreter(profile);
    vm.install_irules_timer_simulation();
    vm.install_irules_static_simulation();
    assert_eq!(
        crate::fixture::call(
            &mut vm,
            "::tmm::_static_domain",
            &[Value::string("children")]
        )
        .code,
        Code::Ok
    );
    vm
}

fn eval(vm: &mut Vm, source: &str) -> Completion<Value> {
    crate::fixture::eval(vm, source)
}

/// `source` evaluated in the child `worker`, as `interp eval` does.
fn child(vm: &mut Vm, worker: &str, source: &str) -> Completion<Value> {
    crate::fixture::call(
        vm,
        "interp",
        &[
            Value::string("eval"),
            Value::string(worker),
            Value::string(source),
        ],
    )
}

fn worker(vm: &mut Vm, name: &'static str) -> &'static str {
    assert!(eval(vm, &format!("interp create {name}")).is_ok());
    assert!(
        child(
            vm,
            name,
            "namespace eval ::static {}; ::tmm::_static_enroll"
        )
        .is_ok()
    );
    name
}

fn init(vm: &mut Vm, worker: &str, source: &str) -> Completion<Value> {
    assert!(child(vm, worker, "::tmm::_timer_context_enter RULE_INIT {}").is_ok());
    let result = child(vm, worker, source);
    assert!(child(vm, worker, "::tmm::_timer_context_leave").is_ok());
    result
}

fn query(vm: &mut Vm, worker: &str, source: &str) -> String {
    let result = child(vm, worker, source);
    assert!(result.is_ok(), "{source}: {:?}", result.result);
    result.result.to_str().to_string()
}

fn static_event(vm: &mut Vm, worker: &str, source: &str) -> String {
    assert!(child(vm, worker, "::tmm::_timer_context_enter HTTP_REQUEST {}").is_ok());
    let reached = child(vm, worker, source);
    assert!(reached.is_ok(), "{source}: {:?}", reached.result);
    let bytes = reached.result.to_str().to_string();
    assert!(child(vm, worker, "::tmm::_timer_context_leave").is_ok());
    bytes
}

const RECORDED_STATIC_INITIAL: [&str; 4] = [
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-initial-42100.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-initial-42101.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-initial-42102.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-initial-42103.body"
    ),
];

const RECORDED_STATIC_AFTER_WRITE: [&str; 4] = [
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-write-42100.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-write-42101.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-write-42102.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-write-42103.body"
    ),
];

const RECORDED_STATIC_AFTER_UNSET: [&str; 4] = [
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-unset-42100.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-unset-42101.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-unset-42102.body"
    ),
    include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-after-unset-42103.body"
    ),
];

fn check_recorded_vl_static(vm: &mut Vm, workers: [&str; 4], raw: &[&str; 4]) {
    let mut reached = [false; 4];
    for (tuple, body) in raw.iter().enumerate() {
        let row = tcl_syntax::list::split_list(body).unwrap();
        assert_eq!(row.len(), 11);
        assert_eq!(row[0], "vl1");
        assert_eq!(row[1], "0");
        assert_eq!(row[3], "192.0.2.201");
        let expected_port = (42100 + tuple).to_string();
        assert_eq!(row[4].as_ref(), expected_port.as_str());
        assert_eq!(row[5], "before");
        assert_eq!(row[7], "action");
        assert_eq!(row[8], "read");
        assert_eq!(row[9], "after");
        assert_eq!(row[6], row[10]);
        let unit = row[2].parse::<usize>().unwrap();
        assert!(unit < 4 && !reached[unit]);
        reached[unit] = true;
        assert_eq!(
            query(vm, workers[unit], "set ::static::__tcl_lsp_2286_vl1_cell"),
            row[10],
            "recorded mapped unit 0:{unit}"
        );
    }
    assert_eq!(reached, [true; 4]);
}

fn recorded_static_group_reads(raw: &str) -> [String; 4] {
    let mut expected: [Option<String>; 4] = std::array::from_fn(|_| None);
    let mut responses = 0;
    for line in raw.lines() {
        let record: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(record["status"].as_str(), Some("HTTP/1.0 200 OK"));
        let response = tcl_cmd_core::binary::base64_decode(
            record["response_base64"].as_str().unwrap().as_bytes(),
            true,
        )
        .unwrap();
        let response = std::str::from_utf8(&response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let fields = tcl_syntax::list::split_list(body.trim_end()).unwrap();
        assert_eq!(fields.len(), 11);
        assert_eq!(fields[0].as_ref(), "R2286");
        assert_eq!(fields[1].as_ref(), "r2286_20261006c");
        assert_eq!(fields[2].as_ref(), "static_group");
        assert_eq!(fields[4], "read");
        assert_eq!(fields[7], "0");
        assert_eq!(fields[8], "NONE");
        assert_eq!(fields[5], fields[9]);
        assert_eq!(fields[6], fields[10]);
        let unit = fields[3]
            .strip_prefix("0:")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert!(unit < 4);
        assert_eq!(record["reported_units"].as_array().unwrap().len(), 1);
        assert_eq!(
            record["reported_units"][0].as_str(),
            Some(fields[3].as_ref())
        );
        assert!(headers.contains(&format!("X-R2286-TMM: 0:{unit}\r\n")));
        let actual = format!("{} {}", fields[9], fields[10]);
        if let Some(previous) = &expected[unit] {
            assert_eq!(&actual, previous);
        } else {
            expected[unit] = Some(actual);
        }
        responses += 1;
    }
    assert_eq!(responses, 128);
    expected.map(Option::unwrap)
}

#[test]
fn original_authored_static_fronts_preserve_domain_configuration_and_refused_header() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = vm();
    assert!(eval(&mut vm, "namespace eval ::static {}").is_ok());
    let call = |vm: &mut Vm, name: &str, words: &[Value]| crate::fixture::call(vm, name, words);
    assert_eq!(
        call(&mut vm, "::tmm::_static_domain", &[Value::string("root")]).code,
        Code::Ok
    );
    assert_eq!(call(&mut vm, "::tmm::_static_enroll", &[]).code, Code::Ok);
    let value = Value::string("CONFIGURED");
    let result = call(
        &mut vm,
        "::tmm::_static_configure",
        &[Value::string("::static::source_operand"), value.clone()],
    );
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.string_bytes().as_ref(), b"CONFIGURED");
    assert_eq!(
        vm.get_var("::static::source_operand")
            .unwrap()
            .string_bytes()
            .as_ref(),
        b"CONFIGURED"
    );
    let original = Value::string("root");
    let loan = original.native_lifetime_lease();
    drop(original);
    assert!(
        vm.try_invoke_command("::tmm::_static_domain", std::slice::from_ref(loan.value()))
            .is_err()
    );
}

#[test]
fn rule_owners_share_static_cells_but_events_and_globals_keep_worker_ownership() {
    let mut vm = vm();
    let workers = ["zero", "one", "two", "three"].map(|name| worker(&mut vm, name));
    for (owner, value) in [("/Common/A", "INIT_A"), ("/Common/B", "INIT_B")] {
        for selected in workers {
            {
                let vm = &mut vm;
                assert!(
                    child(
                        vm,
                        selected,
                        &format!("::tmm::_timer_context_enter RULE_INIT {owner}")
                    )
                    .is_ok()
                );
                assert!(
                    child(
                        vm,
                        selected,
                        &format!("set ::static::collision {value}; set ::ordinary {value}")
                    )
                    .is_ok()
                );
                assert!(child(vm, selected, "::tmm::_timer_context_leave").is_ok());
            }
        }
    }
    for selected in workers {
        assert_eq!(
            query(&mut vm, selected, "list $::static::collision $::ordinary"),
            "INIT_B INIT_B"
        );
    }
    {
        let vm = &mut vm;
        assert!(
            child(
                vm,
                workers[2],
                "::tmm::_timer_context_enter HTTP_REQUEST /Common/A"
            )
            .is_ok()
        );
        assert!(
            child(
                vm,
                workers[2],
                "unset ::static::collision; set ::static::collision EVENT; set ::ordinary LOCAL"
            )
            .is_ok()
        );
        assert!(child(vm, workers[2], "::tmm::_timer_context_leave").is_ok());
    }
    for (index, selected) in workers.into_iter().enumerate() {
        assert_eq!(
            query(&mut vm, selected, "list $::static::collision $::ordinary"),
            if index == 2 {
                "EVENT LOCAL"
            } else {
                "INIT_B INIT_B"
            }
        );
    }
    {
        let vm = &mut vm;
        assert!(
            child(
                vm,
                workers[0],
                "::tmm::_timer_context_enter RULE_INIT /Common/A"
            )
            .is_ok()
        );
        assert!(child(vm, workers[0], "set ::static::collision INIT_A").is_ok());
        assert!(child(vm, workers[0], "::tmm::_timer_context_leave").is_ok());
    }
    for (index, selected) in workers.into_iter().enumerate() {
        assert_eq!(
            query(&mut vm, selected, "list $::static::collision $::ordinary"),
            if index == 2 {
                "INIT_A LOCAL"
            } else {
                "INIT_A INIT_B"
            }
        );
    }
}

#[test]
fn rule_init_publishes_resolved_alias_rmw_and_array_cells_only() {
    let mut vm = vm();
    let a = worker(&mut vm, "a");
    let b = worker(&mut vm, "b");
    assert!(init(&mut vm, a, "set ::ordinary 11; set ::static::n 4; proc bump {} {upvar #0 ::static::n alias; incr alias; lappend ::static::items X}; bump; array set ::static::empty {}; set ::static::a(k) OLD").is_ok());
    assert_eq!(
        query(
            &mut vm,
            b,
            "list $::static::n $::static::items [array exists ::static::empty] $::static::a(k) [info exists ::ordinary]"
        ),
        "5 X 1 OLD 0"
    );
    assert!(child(&mut vm, a, "incr ::static::n; set ::ordinary 12").is_ok());
    assert_eq!(query(&mut vm, b, "set ::static::n"), "5");
    assert!(
        init(
            &mut vm,
            b,
            "set ::ordinary 29; set ::static::n $::ordinary; unset ::static::a(k)"
        )
        .is_ok()
    );
    assert_eq!(
        query(
            &mut vm,
            a,
            "list $::static::n $::ordinary [info exists ::static::a(k)]"
        ),
        "29 12 0"
    );
}

#[test]
fn recipient_traces_publish_nested_writes_and_preserve_partial_errors() {
    let mut vm = vm();
    let a = worker(&mut vm, "a");
    let b = worker(&mut vm, "b");
    assert!(child(&mut vm, b, "proc observe {name key op} {set ::static::nested FROM_TRACE; error recipient_failed}; trace add variable ::static::n write observe").is_ok());
    let result = init(
        &mut vm,
        a,
        "set ::static::n FIRST; set ::static::unreached 1",
    );
    assert_eq!(result.code, Code::Error);
    assert!(result.result.to_str().contains("recipient_failed"));
    for id in [a, b] {
        assert_eq!(
            query(
                &mut vm,
                id,
                "list $::static::n $::static::nested [info exists ::static::unreached]"
            ),
            "FIRST FROM_TRACE 0"
        );
    }
    assert!(
        child(
            &mut vm,
            b,
            "trace remove variable ::static::n write observe"
        )
        .is_ok()
    );
    assert!(child(&mut vm, a, "set ::static::nested ONLY_A").is_ok());
    assert_eq!(query(&mut vm, b, "set ::static::nested"), "FROM_TRACE");
}

#[test]
fn enrollment_replays_owned_publications_and_retires_namespace_incarnations() {
    let mut vm = vm();
    let a = worker(&mut vm, "a");
    assert!(init(&mut vm, a, "set ::static::n SEED; array set ::static::empty {}; set ::static::a(k) OLD; unset ::static::a(k)").is_ok());
    let b = worker(&mut vm, "b");
    assert_eq!(
        query(
            &mut vm,
            b,
            "list $::static::n [array exists ::static::empty] [array exists ::static::a] [info exists ::static::a(k)]"
        ),
        "SEED 1 1 0"
    );
    assert!(
        child(
            &mut vm,
            b,
            "namespace delete ::static; namespace eval ::static {};"
        )
        .is_ok()
    );
    assert!(init(&mut vm, a, "set ::static::n NEW").is_ok());
    assert_eq!(query(&mut vm, b, "info exists ::static::n"), "0");
    assert!(child(&mut vm, b, "::tmm::_static_enroll").is_ok());
    assert_eq!(query(&mut vm, b, "set ::static::n"), "NEW");
    assert!(eval(&mut vm, "interp delete b").is_ok());
    let b2 = worker(&mut vm, "b");
    assert_eq!(query(&mut vm, b2, "set ::static::n"), "NEW");
}

/// Writes and unsets `::static` cells through the host's variable store
/// from inside a procedure frame.
struct FacadeStores;

impl tcl_vm::NativeCommand for FacadeStores {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        use tcl_runtime_api::{GLOBAL_FRAME, VarStore};
        VarStore::set(vm, GLOBAL_FRAME, "::static::n", Value::int(7));
        VarStore::set_elem(
            vm,
            GLOBAL_FRAME,
            "::static::a",
            "k",
            Value::string("MEMBER"),
        );
        assert!(VarStore::unset_elem(vm, GLOBAL_FRAME, "::static::a", "k"));
        Completion::new(Code::Ok, Value::empty(), Value::empty())
    }
}

#[test]
fn noncurrent_storage_facades_publish_without_source_trace_callbacks() {
    let mut vm = vm();
    let a = worker(&mut vm, "a");
    let b = worker(&mut vm, "b");
    assert!(vm.register_child_native_command("a", "facade_stores", Rc::new(FacadeStores)));
    assert!(child(&mut vm, a, "set ::source_hits 0; proc source_trace {args} {incr ::source_hits}; trace add variable ::static::n write source_trace; ::tmm::_timer_context_enter RULE_INIT {}").is_ok());
    assert!(child(&mut vm, a, "proc stores {} {facade_stores}; stores").is_ok());
    assert!(child(&mut vm, a, "::tmm::_timer_context_leave").is_ok());
    assert_eq!(query(&mut vm, a, "set ::source_hits"), "0");
    assert_eq!(
        query(
            &mut vm,
            b,
            "list $::static::n [array exists ::static::a] [info exists ::static::a(k)]"
        ),
        "7 1 0"
    );
}

#[test]
fn recipient_aliases_follow_actual_static_cells_without_republishing_or_touching_globals() {
    let mut vm = vm();
    let a = worker(&mut vm, "a");
    let b = worker(&mut vm, "b");
    assert!(child(&mut vm, b,
        "set ::static::target OLD; upvar #0 ::static::target ::static::n; set ::ordinary LOCAL; upvar #0 ::ordinary ::static::foreign"
    ).is_ok());
    assert!(
        init(
            &mut vm,
            a,
            "set ::static::n NEW; set ::static::foreign SOURCE"
        )
        .is_ok()
    );
    assert_eq!(
        query(
            &mut vm,
            b,
            "list $::static::target $::static::n $::ordinary"
        ),
        "NEW NEW LOCAL"
    );
    assert_eq!(
        query(
            &mut vm,
            a,
            "list $::static::n $::static::foreign [info exists ::static::target] [info exists ::ordinary]"
        ),
        "NEW SOURCE 0 0"
    );
    let c = worker(&mut vm, "c");
    assert_eq!(
        query(
            &mut vm,
            c,
            "list $::static::n $::static::foreign [info exists ::static::target]"
        ),
        "NEW SOURCE 0"
    );
}

/// A direct native write observer that counts its calls.
struct CountingObserver(Rc<Cell<usize>>);

impl NativeVariableObserver<Vm> for CountingObserver {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _: &mut Vm,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Write);
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

/// `observe add` registers the observer on `::static::n` in the
/// interpreter it runs in; `observe remove` removes the last registration and
/// returns whether it was still present.
struct ObserverControl {
    observer: Rc<CountingObserver>,
    token: RefCell<Option<NativeVariableTraceToken>>,
}

impl tcl_vm::NativeCommand for ObserverControl {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let name = Value::new_native_string_bytes(b"::static::n".as_slice());
        if args[0].string_bytes().as_ref() == b"add" {
            let token = vm
                .add_native_variable_observer(
                    &name,
                    &[NativeVariableTraceOperation::Write],
                    self.observer.clone(),
                )
                .unwrap();
            *self.token.borrow_mut() = Some(token);
            return Completion::new(Code::Ok, Value::empty(), Value::empty());
        }
        let token = self.token.borrow_mut().take().unwrap();
        let removed = vm.remove_native_variable_observer(&token);
        Completion::new(Code::Ok, Value::int(i64::from(removed)), Value::empty())
    }
}

#[test]
fn direct_native_recipient_observers_see_source_writes_until_removed() {
    let mut vm = vm();
    let source = worker(&mut vm, "source");
    let recipient = worker(&mut vm, "recipient");
    assert!(child(&mut vm, source, "::tmm::_timer_context_enter RULE_INIT {}").is_ok());
    let calls = Rc::new(Cell::new(0));
    assert!(vm.register_child_native_command(
        recipient,
        "observe",
        Rc::new(ObserverControl {
            observer: Rc::new(CountingObserver(Rc::clone(&calls))),
            token: RefCell::new(None),
        })
    ));
    assert_eq!(query(&mut vm, recipient, "observe add"), "");
    assert_eq!(
        query(&mut vm, recipient, "trace info variable ::static::n"),
        ""
    );
    assert!(child(&mut vm, source, "set ::static::n FIRST").is_ok());
    assert_eq!(calls.get(), 1);
    assert!(
        child(
            &mut vm,
            recipient,
            "unset ::static::n; set ::static::n REPLACED"
        )
        .is_ok()
    );
    assert_eq!(query(&mut vm, recipient, "observe remove"), "0");
    assert_eq!(query(&mut vm, recipient, "observe add"), "");
    assert!(child(&mut vm, source, "set ::static::n SECOND").is_ok());
    assert_eq!(calls.get(), 2);
    assert_eq!(query(&mut vm, recipient, "observe remove"), "1");
    assert!(child(&mut vm, source, "set ::static::n THIRD").is_ok());
    assert_eq!(calls.get(), 2);
    assert!(child(&mut vm, source, "::tmm::_timer_context_leave").is_ok());
}

#[test]
fn authored_static_policy_matches_recorded_four_unit_seed_and_target_mutation_states() {
    // naming.bigip.static-state-tmm-locality
    // docs/design/analysis/name-resolution-proofs/bigip-static-state-tmm-locality.md
    // Authored policy regression over retained observations, no appliance execution.
    let phases = [
        include_str!("../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev/traffic-static-group-cmp-yes-read-initial.jsonl"),
        include_str!("../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev/traffic-static-group-cmp-yes-read-after-write.jsonl"),
        include_str!("../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev/traffic-static-group-cmp-yes-read-after-unset.jsonl"),
        include_str!("../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev/traffic-static-group-cmp-yes-read-after-recreate.jsonl"),
    ].map(recorded_static_group_reads);
    let original_rule = include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/r2286_20261006c-fixtures/static_group.conf"
    );
    let report =
        include_str!("../../../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md");
    assert!(
        original_rule
            .contains("set static::__tcl_lsp_probe_2286_r2286_20261006c_cell INIT_r2286_20261006c")
    );
    assert!(report.contains("`static_group` new initialization | delete/recreate exact rule | Four new `RULE_INIT` logs and four fresh `INIT_…` reads"));
    let mut vm = vm();
    let workers = ["zero", "one", "two", "three"].map(|name| worker(&mut vm, name));
    let name = "::static::__tcl_lsp_probe_2286_r2286_20261006c_cell";
    let initialise = format!("set {name} INIT_r2286_20261006c");
    assert!(init(&mut vm, workers[0], &initialise).is_ok());
    for (phase, expected) in phases.into_iter().enumerate() {
        match phase {
            1 => {
                assert_eq!(
                    static_event(&mut vm, workers[2], &format!("set {name} EVENT_0:2")),
                    "EVENT_0:2"
                );
            }
            2 => {
                assert_eq!(
                    static_event(
                        &mut vm,
                        workers[2],
                        &format!("unset {name}; info exists {name}")
                    ),
                    "0"
                );
            }
            3 => {
                assert_eq!(
                    static_event(&mut vm, workers[2], &format!("set {name} RECREATED_0:2")),
                    "RECREATED_0:2"
                );
            }
            _ => {}
        }
        for (unit, selected) in workers.into_iter().enumerate() {
            let script = format!(
                "if {{[info exists {name}]}} {{list 1 [set {name}]}} else {{list 0 MISSING}}"
            );
            assert_eq!(
                query(&mut vm, selected, &script),
                expected[unit],
                "recorded phase {phase}/0:{unit}"
            );
        }
    }
    // This is the explicit policy's reinitialisation, not a native rule loader.
    assert!(init(&mut vm, workers[0], &initialise).is_ok());
    for selected in workers {
        assert_eq!(
            query(&mut vm, selected, &format!("set {name}")),
            "INIT_r2286_20261006c"
        );
    }
}

#[test]
fn authored_static_policy_matches_mapped_unit_missing_read_before_recreation() {
    // naming.bigip.variable-static-event-currency
    // docs/design/analysis/name-resolution-proofs/bigip-variable-static-event-currency.md
    // Only recorded static-cell states; active scalar trace chronology is separate.
    let original_rule = include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/sources/static.conf"
    );
    assert!(original_rule.contains("set static::__tcl_lsp_2286_vl1_cell SEED"));
    assert!(
        original_rule
            .contains("set missing [catch {set static::__tcl_lsp_2286_vl1_cell} missing_result]")
    );
    assert!(original_rule.contains("set static::__tcl_lsp_2286_vl1_cell RECREATED"));
    let mut vm = vm();
    let workers = ["zero", "one", "two", "three"].map(|name| worker(&mut vm, name));
    assert!(
        init(
            &mut vm,
            workers[0],
            "set ::static::__tcl_lsp_2286_vl1_cell SEED"
        )
        .is_ok()
    );
    check_recorded_vl_static(&mut vm, workers, &RECORDED_STATIC_INITIAL);
    assert_eq!(
        static_event(
            &mut vm,
            workers[2],
            "set ::static::__tcl_lsp_2286_vl1_cell MARK_UNIT"
        ),
        "MARK_UNIT"
    );
    check_recorded_vl_static(&mut vm, workers, &RECORDED_STATIC_AFTER_WRITE);

    let missing = include_str!(
        "../../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-static-unset-42101.body"
    );
    let row = tcl_syntax::list::split_list(missing).unwrap();
    assert_eq!(row.len(), 11);
    assert_eq!(row[0], "vl1");
    assert_eq!(row[1], "0");
    assert_eq!(row[2], "2");
    assert_eq!(row[3], "192.0.2.201");
    assert_eq!(row[4], "42101");
    assert_eq!(row[5], "before");
    assert_eq!(row[7], "action");
    assert_eq!(row[9], "after");
    assert_eq!(row[6], "MARK_UNIT");
    assert_eq!(row[10], "RECREATED");
    let action = tcl_syntax::list::split_list(&row[8]).unwrap();
    assert_eq!(action.len(), 5);
    assert_eq!(action[0], "unset");
    assert_eq!(action[1], "missing");
    assert_eq!(action[2], "1");
    assert_eq!(action[3], "missing_result");
    let actual = static_event(
        &mut vm,
        workers[2],
        "unset ::static::__tcl_lsp_2286_vl1_cell; set missing [catch {set static::__tcl_lsp_2286_vl1_cell} missing_result]; list $missing $missing_result [info exists ::static::__tcl_lsp_2286_vl1_cell]",
    );
    let actual = tcl_syntax::list::split_list(&actual).unwrap();
    assert_eq!(actual[0], action[2]);
    assert_eq!(actual[1], action[4]);
    assert_eq!(actual[2], "0");
    assert_eq!(
        query(&mut vm, workers[0], "set ::static::__tcl_lsp_2286_vl1_cell"),
        "SEED"
    );
    assert_eq!(
        static_event(
            &mut vm,
            workers[2],
            "set ::static::__tcl_lsp_2286_vl1_cell RECREATED"
        ),
        "RECREATED"
    );
    check_recorded_vl_static(&mut vm, workers, &RECORDED_STATIC_AFTER_UNSET);
}
