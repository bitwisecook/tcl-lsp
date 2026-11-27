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

//! Family-B intrinsic guards: the events that stale a guard, the domains it
//! covers, and attestation of registered builtins, driven by scripts.

use super::*;
use tcl_runtime_api::RegisteredBacking;
use tcl_runtime_api::guard::{GuardDomain, GuardDomains, GuardError};

/// The registry's members that only observe a variable and still run its
/// traces do so here: `info exists` a read trace, the array queries an
/// array trace, as `tclsh` does.
#[test]
fn every_trace_firing_intrinsic_fires_a_trace_here() {
    use tcl_registry::IntrinsicId;
    for member in IntrinsicId::ALL
        .iter()
        .copied()
        .filter(|member| member.family().fires_traces())
    {
        let (setup, op, command) = match member {
            IntrinsicId::InfoExists => ("set v 1", "read", "info exists v"),
            IntrinsicId::ArrayExists => ("array set v {k 1}", "array", "array exists v"),
            IntrinsicId::ArrayNames => ("array set v {k 1}", "array", "array names v"),
            IntrinsicId::ArraySize => ("array set v {k 1}", "array", "array size v"),
            other => panic!("{other:?} fires traces and needs a probe here"),
        };
        let mut vm = compiled_vm();
        eval_value(
            &mut vm,
            &format!(
                "proc note {{n1 n2 op}} {{lappend ::fired $op}}; set ::fired {{}}; \
                     {setup}; trace add variable v {op} note"
            ),
        );
        eval_value(&mut vm, command);
        assert_eq!(eval_value(&mut vm, "set ::fired"), op, "{member:?}");
    }
}

/// A guard is bound to its command's token. A definition, rename or alias
/// of another command leaves it and its attestation alone; replacing,
/// renaming away or hiding the command drops it, and restoring the same
/// token brings it back; a profile pin keeps the attestation.
#[test]
fn an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it() {
    let mut vm = compiled_vm();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .unwrap();

    vm.register("unrelated", guarded_builtin);
    eval_value(&mut vm, "proc foo {} {return 1}");
    eval_value(&mut vm, "rename foo bar");
    eval_value(&mut vm, "interp alias {} baz {} bar");
    eval_value(&mut vm, "rename baz {}");
    assert!(vm.check_command_guard(token, "guarded"));
    assert!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
            .is_ok()
    );

    // A change to the lookup environment itself stales the token over it,
    // and the attestation stays.
    eval_value(&mut vm, "namespace path ::");
    assert!(!vm.check_command_guard(token, "guarded"));
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .expect("still attested");

    // The profile pin keeps the attestation and a token that does not
    // depend on interpreter policy.
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(vm.check_command_guard(token, "guarded"));
    assert!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
            .is_ok()
    );

    // Renaming the command away drops the guard at its name, and the
    // attestation goes with the command; restoring the name restores it.
    eval_value(&mut vm, "rename guarded moved");
    assert!(!vm.check_command_guard(token, "guarded"));
    assert_eq!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains),
        Err(GuardError::IdentityUnavailable)
    );
    assert!(
        vm.prepare_command_guard("moved", GUARDED_IDENTITY, domains)
            .is_ok()
    );
    eval_value(&mut vm, "rename moved guarded");
    assert!(vm.check_command_guard(token, "guarded"));

    // Hiding it drops the guard, exposing it restores it.
    eval_value(&mut vm, "interp hide {} guarded");
    assert!(!vm.check_command_guard(token, "guarded"));
    eval_value(&mut vm, "interp expose {} guarded");
    assert!(vm.check_command_guard(token, "guarded"));

    // A different command at the name is never attested.
    eval_value(&mut vm, "proc guarded {} {return 1}");
    assert!(!vm.check_command_guard(token, "guarded"));
    assert_eq!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains),
        Err(GuardError::IdentityUnavailable)
    );
}

/// Each lookup event moves the domain that covers it, as in the runtime
/// (#2292): namespace membership moves `Namespace`, and the unknown
/// handler or the command it names moves `UnknownHandling`.
#[test]
fn lookup_events_move_the_namespace_and_unknown_handling_domains() {
    let setup = "namespace eval src {proc p {} {}; namespace export p}";
    let rows: [(GuardDomain, &str, &str); 10] = [
        (GuardDomain::Namespace, "", "namespace eval created {}"),
        (GuardDomain::Namespace, "", "namespace delete src"),
        (GuardDomain::Namespace, "", "namespace export guarded"),
        (GuardDomain::Namespace, "", "namespace path ::src"),
        (GuardDomain::Namespace, "", "namespace import src::p"),
        (
            GuardDomain::Namespace,
            "namespace import src::p",
            "namespace forget src::p",
        ),
        (
            GuardDomain::UnknownHandling,
            "",
            "namespace unknown handler",
        ),
        (
            GuardDomain::UnknownHandling,
            "",
            "proc unknown args {return 0}",
        ),
        (
            GuardDomain::UnknownHandling,
            "proc unknown args {}",
            "rename unknown {}",
        ),
        (
            GuardDomain::UnknownHandling,
            "proc unknown args {}",
            "rename unknown moved",
        ),
    ];
    for (domain, prepare, event) in rows {
        let mut vm = compiled_vm();
        vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
        eval_value(&mut vm, setup);
        eval_value(&mut vm, prepare);
        let token = vm
            .prepare_command_guard("guarded", GUARDED_IDENTITY, GuardDomains::one(domain))
            .unwrap();
        assert!(vm.check_command_guard(token, "guarded"));
        eval_value(&mut vm, event);
        assert!(
            !vm.check_command_guard(token, "guarded"),
            "{domain:?} kept across {event}"
        );
    }
}

/// Check the probed token against the command `guarded` from wherever this
/// runs.
fn probe_guarded(vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    let token = PROBED_TOKEN.get().expect("a token to probe");
    Completion::new(
        Code::Ok,
        Value::string(if vm.check_command_guard(token, "guarded") {
            "1"
        } else {
            "0"
        }),
        Value::empty(),
    )
}

/// The attestation belongs to the token the name reaches, so a definition
/// in a namespace that shadows the command drops the guard for calls made
/// from that namespace and only from it.
#[test]
fn a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace() {
    let mut vm = compiled_vm();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
    vm.register("probe", probe_guarded);
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .unwrap();
    PROBED_TOKEN.set(Some(token));

    assert_eq!(eval_value(&mut vm, "probe"), "1");
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval ns {proc guarded {} {return shadow}; probe}"
        ),
        "0"
    );
    assert_eq!(eval_value(&mut vm, "probe"), "1");
    assert!(vm.check_command_guard(token, "guarded"));
}

/// A name is attested only while the command bound at it is the builtin the
/// VM registered there: a procedure defined over it, or another builtin
/// renamed into it, gains nothing, and the original, put back, has what it
/// had.
#[test]
fn the_sweep_attests_only_the_builtin_it_registered() {
    use tcl_registry::IntrinsicId;
    // Tcl 8.4's `string` is a builtin the sweep attests.
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_4);
    let length = GuardIdentity::registry_intrinsic_with_semantics(
        IntrinsicId::StringLength.stable_id(),
        IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let attested = |vm: &Vm| vm.prepare_command_guard("string", length, domains).is_ok();
    assert!(attested(&vm));

    eval_value(&mut vm, "rename string original");
    assert!(!attested(&vm), "moved away, the name reaches nothing");
    eval_value(&mut vm, "proc string args {return x}");
    assert!(
        !attested(&vm),
        "a procedure at the name gains no attestation"
    );

    eval_value(&mut vm, "rename string {}; rename puts string");
    assert!(
        !attested(&vm),
        "another builtin renamed into the name is attested as itself and not as `string`"
    );

    eval_value(&mut vm, "rename string puts; rename original string");
    assert!(
        attested(&vm),
        "the original, restored, is attested as it was"
    );
}

/// The backing report says what the command table holds: handlers and the
/// engine's `TclOO` roots, absences, and nothing a script defines.
#[test]
fn the_backing_report_says_what_the_command_table_holds() {
    let mut vm = compiled_vm();
    let before = tcl_runtime_api::BackingReport::from_entries(vm.backing_report());
    assert_eq!(before.of("set"), RegisteredBacking::Builtin);
    assert_eq!(before.of("::tcl::dict::get"), RegisteredBacking::Builtin);
    assert_eq!(before.of("oo::class"), RegisteredBacking::Object);
    assert_eq!(before.of("oo::object"), RegisteredBacking::Object);
    assert_eq!(before.of("tclLog"), RegisteredBacking::Absent);

    eval_value(&mut vm, "proc mine {} {}");
    eval_value(&mut vm, "oo::class create Mine");
    eval_value(&mut vm, "interp alias {} aliased {} set");
    let after = tcl_runtime_api::BackingReport::from_entries(vm.backing_report());
    for name in ["mine", "Mine", "aliased"] {
        assert_eq!(after.of(name), RegisteredBacking::Absent, "{name}");
    }
    assert_eq!(after, before, "a script adds nothing to the report");
}
