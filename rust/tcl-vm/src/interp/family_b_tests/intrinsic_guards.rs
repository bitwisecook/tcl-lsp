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

//! Family-B intrinsic guards: issuance, the domains a guard covers, the events that stale it, and attestation of registered builtins.

use super::*;

#[test]
fn command_guard_requires_identity_and_has_exact_lifecycle() {
    let mut vm = Vm::new();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    assert_eq!(
        vm.prepare_command_guard("guarded", GuardIdentity::new(1, 42), domains),
        Err(GuardError::IdentityMismatch)
    );
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .unwrap();
    assert!(vm.check_command_guard(token, "guarded"));
    assert!(vm.release_command_guard(token));
    assert!(!vm.release_command_guard(token));
    assert!(!vm.check_command_guard(token, "guarded"));
}

/// A request for a Family-B intrinsic covers the variable-trace domain or
/// is refused, whatever the caller asked for, and a Value member needs no
/// such domain. Covered, the guard is refused while a variable trace
/// exists and goes stale when one is added.
#[test]
fn a_family_b_guard_request_must_cover_the_variable_trace_domain() {
    use tcl_registry::IntrinsicId;
    let mut vm = Vm::new();
    let version = vm.runtime_version();
    let identity = |member: IntrinsicId| {
        GuardIdentity::registry_intrinsic_with_semantics(
            member.stable_id(),
            member.guard_semantics_key(version),
        )
    };
    let (stores, value) = (
        identity(IntrinsicId::DictSet),
        identity(IntrinsicId::ListLength),
    );
    vm.register_guarded_builtin("stores", guarded_builtin, stores);
    vm.register_guarded_builtin("pure", guarded_builtin, value);
    let command = GuardDomains::one(GuardDomain::CommandEnvironment);
    let traced = command.with(GuardDomain::VariableTrace);

    assert_eq!(
        vm.prepare_command_guard("stores", stores, command),
        Err(GuardError::DomainsInsufficient)
    );
    let token = vm
        .prepare_command_guard("pure", value, command)
        .expect("a Value member requires no variable-trace domain");
    assert!(vm.release_command_guard(token));

    let token = vm
        .prepare_command_guard("stores", stores, traced)
        .expect("a request covering the family's domain");
    assert!(vm.check_command_guard(token, "stores"));
    vm.add_var_trace(
        "watched",
        vec!["write".to_owned()],
        "callback".to_owned(),
        false,
    );
    assert!(!vm.check_command_guard(token, "stores"));
    assert_eq!(
        vm.prepare_command_guard("stores", stores, traced),
        Err(GuardError::PrerequisiteUnsatisfied)
    );
    assert!(vm.prepare_command_guard("pure", value, command).is_ok());
}

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
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
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
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
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
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
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
    ok(Value::string(if vm.check_command_guard(token, "guarded") {
        "1"
    } else {
        "0"
    }))
}

/// The attestation belongs to the token the name reaches, so a definition
/// in a namespace that shadows the command drops the guard for calls made
/// from that namespace and only from it.
#[test]
fn a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
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

#[test]
fn trace_registration_invalidates_the_matching_guard_domain() {
    let mut vm = Vm::new();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);

    let command_token = vm
        .prepare_command_guard(
            "guarded",
            GUARDED_IDENTITY,
            GuardDomains::one(GuardDomain::CommandTrace),
        )
        .unwrap();
    let added = vm.add_cmd_trace(
        false,
        "guarded",
        vec!["rename".to_owned()],
        "callback".to_owned(),
    );
    assert_eq!(added.code, Code::Ok);
    assert!(!vm.check_command_guard(command_token, "guarded"));

    let variable_token = vm
        .prepare_command_guard(
            "guarded",
            GUARDED_IDENTITY,
            GuardDomains::one(GuardDomain::VariableTrace),
        )
        .unwrap();
    vm.add_var_trace(
        "watched",
        vec!["write".to_owned()],
        "callback".to_owned(),
        false,
    );
    assert!(!vm.check_command_guard(variable_token, "guarded"));
}

#[test]
fn host_intrinsic_contract_keeps_domains_and_original_binding_owner() {
    let mut vm = Vm::new();
    let intrinsic = tcl_registry::IntrinsicId::StringLength;
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key(vm.runtime_version()),
    );
    vm.register_guarded_builtin("host", guarded_builtin, identity);
    let added = vm.add_cmd_trace(
        false,
        "host",
        vec!["rename".to_owned()],
        "callback".to_owned(),
    );
    assert_eq!(added.code, Code::Ok);
    let domains = GuardDomains::one(GuardDomain::VariableTrace);
    let host = vm.prepare_command_guard("host", identity, domains).unwrap();
    assert!(vm.check_command_guard(host, "host"));
    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::PrerequisiteUnsatisfied)
    );

    // A replacement with the same explicit contract cannot inherit the
    // token even though the requested variable-trace epoch is unchanged.
    vm.register_guarded_builtin("host", guarded_builtin, identity);
    assert!(!vm.check_command_guard(host, "host"));
    let replacement = vm.prepare_command_guard("host", identity, domains).unwrap();
    assert!(vm.check_command_guard(replacement, "host"));
}

#[test]
fn unsupported_guard_domains_are_poisoned() {
    let mut vm = Vm::new();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);

    for domain in [GuardDomain::Interpreter, GuardDomain::ObjectDispatch] {
        assert_eq!(
            vm.prepare_command_guard("guarded", GUARDED_IDENTITY, GuardDomains::one(domain)),
            Err(GuardError::Poisoned),
            "{domain:?} must decline until its mutation ownership is centralised",
        );
    }
}

#[test]
fn string_length_base_guard_declines_and_generic_dispatch_remains_available() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
        .with(GuardDomain::Namespace)
        .with(GuardDomain::CommandTrace)
        .with(GuardDomain::Interpreter);

    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::Poisoned),
    );

    let completion = vm
        .dispatch("string", &[Value::string("length"), Value::string("abc")])
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.as_int().unwrap(), 3);
}

#[test]
fn string_length_intrinsic_uses_the_selected_runtime_character_model() {
    let mut vm = Vm::new();
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    for intrinsic in [
        tcl_registry::IntrinsicId::StringLength,
        tcl_registry::IntrinsicId::StringIndex,
    ] {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(vm.runtime_version()),
        );
        let token = vm
            .prepare_command_guard("string", identity, domains)
            .unwrap();
        assert!(vm.check_command_guard(token, "string"));
    }

    let value = Value::string("é🙂");
    let completion = vm
        .execute_intrinsic(
            tcl_registry::IntrinsicId::StringLength,
            std::slice::from_ref(&value),
        )
        .expect("StringLength is implemented");
    assert_eq!(completion.result.as_int().unwrap(), 2);

    let mut tcl86 = release_vm(tcl_dialect::TclVersion::V8_6);
    let completion = tcl86
        .execute_intrinsic(
            tcl_registry::IntrinsicId::StringLength,
            &[Value::string("é🙂")],
        )
        .expect("StringLength is implemented for Tcl 8");
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.as_int().unwrap(), 3);
    let completion = tcl86
        .dispatch("string", &[Value::string("length"), Value::string("é🙂")])
        .unwrap();
    assert_eq!(completion.result.as_int().unwrap(), 3);

    let completion = vm
        .dispatch("string", &[Value::string("length"), Value::string("é🙂")])
        .unwrap();
    assert_eq!(completion.result.as_int().unwrap(), 2);
    assert!(
        vm.execute_intrinsic(tcl_registry::IntrinsicId::ListLength, &[])
            .is_none()
    );
}

#[test]
fn renaming_spec_registered_string_stales_its_intrinsic_guard() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let token = vm
        .prepare_command_guard(
            "string",
            identity,
            GuardDomains::one(GuardDomain::CommandEnvironment),
        )
        .unwrap();
    let command = vm
        .take_command("string")
        .expect("registered string command");
    vm.register_command("moved", command);
    assert!(!vm.check_command_guard(token, "string"));
    assert!(!vm.check_command_guard(token, "moved"));
    assert_eq!(
        vm.prepare_command_guard(
            "string",
            identity,
            GuardDomains::one(GuardDomain::CommandEnvironment)
        ),
        Err(GuardError::IdentityUnavailable)
    );
}

/// The pin keeps the spec-registered `string`'s attestation for the
/// release it pins: a fresh guard is issued after it, where a pinned VM
/// used to hold none.
#[test]
fn a_profile_pin_keeps_the_spec_registered_string_attested() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let token = vm
        .prepare_command_guard("string", identity, domains)
        .expect("string stays attested after the pin");
    assert!(vm.check_command_guard(token, "string"));
}

/// The sweep reads the generation the VM is pinned to and nothing an
/// overlay installed: a pack's command is not a runtime implementation, and
/// only the runtime may attest.
#[test]
fn identities_come_from_the_pinned_generation_never_an_overlay() {
    use tcl_registry::{CommandSpec, IntrinsicId, SemanticOperationId};
    // Tcl 8.4's `string` is a builtin the sweep attests; later releases
    // install an ensemble, whose stock attestation is checked live.
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_4);
    // A generation an overlay installed under this VM's own profile,
    // holding a spec that declares an intrinsic, and a builtin of that name
    // in the VM.
    let overlaid =
        tcl_registry::registry_for_profile_with_overlay(vm.dialect_profile(), 0xC0DE, |registry| {
            registry.insert(CommandSpec {
                name: "overlay_length",
                semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringLength)),
                ..CommandSpec::DEFAULT
            });
        });
    vm.register_stock_builtin("overlay_length", guarded_builtin);

    vm.attach_identities();
    assert!(
        vm.attested_identities("overlay_length").is_none(),
        "the sweep of the pinned generation attests nothing an overlay declares"
    );
    assert!(
        vm.attested_identities("string").is_some(),
        "and still attests the shipped command"
    );

    // Control: the fixture is sound, a sweep over the overlay's own store
    // would attest the builtin.
    vm.attach_identities_from(&overlaid);
    assert!(vm.attested_identities("overlay_length").is_some());
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
    let attested = |vm: &Vm| {
        vm.attested_identities("string")
            .is_some_and(|identities| identities.contains(&length))
    };
    assert!(attested(&vm));

    eval_value(&mut vm, "rename string original");
    assert!(!attested(&vm), "moved away, the name reaches nothing");
    eval_value(&mut vm, "proc string args {return x}");
    vm.attach_identities();
    assert!(
        !attested(&vm),
        "a procedure at the name gains no attestation"
    );

    eval_value(&mut vm, "rename string {}; rename puts string");
    vm.attach_identities();
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

/// Every builtin this VM registers at the name of a spec that declares an
/// intrinsic is attested for every identity the spec's intrinsics have.
#[test]
fn every_registered_builtin_with_an_intrinsic_is_attested_for_all_of_them() {
    let vm = Vm::new();
    let registry = crate::environment::store_for_profile(vm.dialect_profile());
    let report = tcl_runtime_api::BackingReport::from_entries(vm.backing_report());
    let mut attested_names = 0;
    for name in registry.command_names() {
        let spec = registry.get_exact(name).expect("a named spec");
        let expected: BTreeSet<GuardIdentity> = spec
            .intrinsic_ids()
            .into_iter()
            .flat_map(|id| {
                id.guard_semantics_variants().iter().map(move |semantics| {
                    GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                })
            })
            .collect();
        if expected.is_empty() || report.of(name) != RegisteredBacking::Builtin {
            continue;
        }
        let actual = vm
            .attested_identities(name)
            .unwrap_or_else(|| panic!("{name} is registered and has intrinsics"));
        assert!(expected.is_subset(&actual), "{name}");
        attested_names += 1;
    }
    assert!(attested_names >= 10, "{attested_names}");
    assert!(
        vm.attested_identities("set").is_none(),
        "no intrinsic, nothing"
    );
}

/// Every builtin registered under a spelling the registry names it by has
/// that spelling in the registry, which is what the release surface gates
/// it with.
#[test]
fn a_spelled_builtin_names_a_spec_the_registry_has() {
    let vm = Vm::new();
    let registry = crate::environment::store_for_profile(vm.dialect_profile());
    // Identities are stored unrooted, and a qualified spec may be spelled
    // from the root: the spec the surface gate reaches names the identity.
    let mut qualified = 0;
    for identity in vm.builtin_identities.values() {
        let Some(spec) = registry.get(identity) else {
            continue;
        };
        assert_eq!(
            tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name),
            identity.as_str()
        );
        qualified += usize::from(identity.contains("::"));
    }
    assert!(qualified >= 15, "{qualified}");
}

/// The backing report says what the command table holds: handlers and the
/// engine's `TclOO` roots, absences, and nothing a script defines.
#[test]
fn the_backing_report_says_what_the_command_table_holds() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
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

/// The pin keeps every attestation and the pinned surface decides which of
/// them a guard reaches: a command the release lacks has none, and answers
/// again under a release that has it.
#[test]
fn a_pin_to_a_release_without_the_command_leaves_it_unattested() {
    let mut vm = Vm::new();
    // `lassign` is a command Tcl 8.4 does not have. This is the shipped
    // command: a host handler at the name has no stock release floor.
    let intrinsic = tcl_registry::IntrinsicId::ListAssign;
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key(vm.runtime_version()),
    );
    let domains =
        GuardDomains::one(GuardDomain::CommandEnvironment).with(GuardDomain::VariableTrace);
    let token = vm
        .prepare_command_guard("lassign", identity, domains)
        .expect("attested in the default release");
    assert!(vm.check_command_guard(token, "lassign"));

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    assert!(!vm.check_command_guard(token, "lassign"));
    assert_eq!(
        vm.prepare_command_guard("lassign", identity, domains),
        Err(GuardError::IdentityUnavailable)
    );

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert!(
        vm.prepare_command_guard("lassign", identity, domains)
            .is_ok()
    );
}

#[test]
fn command_trace_stales_the_string_intrinsic_guard() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let token = vm
        .prepare_command_guard(
            "string",
            identity,
            GuardDomains::one(GuardDomain::CommandTrace),
        )
        .unwrap();
    let added = vm.add_cmd_trace(
        false,
        "string",
        vec!["rename".to_owned()],
        "callback".to_owned(),
    );
    assert_eq!(added.code, Code::Ok);
    assert!(!vm.check_command_guard(token, "string"));
}

#[test]
fn stock_guard_reissues_after_unrelated_birth_but_not_worker_replacement() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandTrace);
    let original = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    // An unrelated command birth leaves the guard alone; a namespace birth
    // moves the structural domain every stock guard depends on.
    vm.register("unrelated", guarded_builtin);
    assert!(vm.check_command_guard(original, "string"));
    vm.declare_namespace_key("unrelated");
    assert!(!vm.check_command_guard(original, "string"));
    let fresh = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    assert!(vm.check_command_guard(fresh, "string"));
    vm.register("::tcl::string::length", guarded_builtin);
    assert!(!vm.check_command_guard(fresh, "string"));
    assert!(
        vm.prepare_command_guard("string", identity, domains)
            .is_err()
    );
    let index = tcl_registry::IntrinsicId::StringIndex;
    let index_identity = GuardIdentity::registry_intrinsic_with_semantics(
        index.stable_id(),
        index.guard_semantics_key(vm.runtime_version()),
    );
    let unrelated_worker = vm
        .prepare_command_guard("string", index_identity, domains)
        .unwrap();
    assert!(vm.check_command_guard(unrelated_worker, "string"));
}

#[test]
fn stock_guard_requires_original_ensemble_configuration_at_every_issuance() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandTrace);
    let original = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    let Command::Ensemble(ensemble) = vm.commands.get("string").unwrap().clone() else {
        panic!("modern stock string is an actual ensemble");
    };
    let config = ensemble.config();
    let mut changed = config.clone();
    changed.prefixes = !changed.prefixes;
    ensemble.configure(changed);
    vm.note_ensemble_configuration_changed(&ensemble);
    assert!(!vm.check_command_guard(original, "string"));
    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::IdentityUnavailable)
    );
    ensemble.configure(config);
    vm.note_ensemble_configuration_changed(&ensemble);
    assert!(!vm.check_command_guard(original, "string"));
    let restored = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    assert!(vm.check_command_guard(restored, "string"));
    vm.register("string", guarded_builtin);
    assert!(!vm.check_command_guard(restored, "string"));
    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::IdentityUnavailable)
    );
}
