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

//! Guards on an intrinsic: the retained fixture, logical provider changes and the `string length` intrinsic across renames, traces and release pins.

use super::*;

// These helpers exercise epoch policy against an original live registration.
// They do not restore a named lookup, native instruction or hosted engine.
fn retained_fixture_identity(interp: &Interp, generation: u64) -> Option<GuardIdentity> {
    interp.raw_command_location_by_generation(generation)?;
    interp
        .guarded_commands
        .borrow()
        .get(&generation)?
        .contains(&GUARDED_IDENTITY)
        .then_some(GUARDED_IDENTITY)
}

fn prepare_retained_fixture_guard(
    interp: &Interp,
    generation: u64,
    domain: GuardDomain,
) -> GuardToken {
    let observed = retained_fixture_identity(interp, generation);
    interp
        .guards
        .borrow_mut()
        .prepare(
            GUARDED_IDENTITY,
            observed,
            GuardDomains::one(domain),
            generation,
        )
        .unwrap()
}

fn check_retained_fixture_guard(interp: &Interp, token: GuardToken, generation: u64) -> bool {
    interp.guards.borrow().check(
        token,
        retained_fixture_identity(interp, generation),
        &generation,
    )
}

#[test]
fn logical_provider_changes_stale_only_interpreter_policy_guards() {
    fn install(
        interp: &mut Interp,
        host: &'static tcl_dialect::DialectProfile,
        provider: u8,
    ) -> bool {
        match provider {
            0 => interp.set_logical_eval_object_provider(
                tcl_native_model::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation,
                host,
            ),
            1 => interp.set_logical_source_word_provider(
                tcl_native_model::LogicalSourceWordProvider::Tcl84CoreSimulation,
                host,
            ),
            2 => interp.set_logical_expression_parse_provider(
                tcl_native_model::LogicalExpressionParseProvider::Tcl84CoreSimulation,
                host,
            ),
            _ => unreachable!(),
        }
    }
    for provider in [0, 1, 2] {
        leak_free(|interp| {
            let source = tcl_dialect::DialectProfile::irules();
            let host = crate::environment::profile_for_dialect("tcl9.0");
            let replacement = crate::environment::profile_for_dialect("tcl9.1");
            // Retain an actual registration before selecting a hosted
            // source profile that has no command naming capability.
            interp.set_dialect_profile(host);
            interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let generation = interp
                .namespaces
                .borrow()
                .resolve_generation(GLOBAL, b"guarded")
                .unwrap();
            interp.set_dialect_profile(source);
            assert!(matches!(
                interp.prepare_command_guard(
                    b"guarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::Interpreter)
                ),
                Err(GuardError::IdentityUnavailable)
            ));
            let command =
                prepare_retained_fixture_guard(interp, generation, GuardDomain::CommandEnvironment);
            let policy =
                prepare_retained_fixture_guard(interp, generation, GuardDomain::Interpreter);
            assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
            assert!(!install(interp, source, provider));
            assert!(check_retained_fixture_guard(interp, policy, generation));
            assert!(install(interp, host, provider));
            assert!(!check_retained_fixture_guard(interp, policy, generation));
            assert!(interp.release_command_guard(policy));

            let policy =
                prepare_retained_fixture_guard(interp, generation, GuardDomain::Interpreter);
            assert!(install(interp, host, provider));
            assert!(check_retained_fixture_guard(interp, policy, generation));
            assert!(!install(interp, source, provider));
            assert!(check_retained_fixture_guard(interp, policy, generation));
            assert!(install(interp, replacement, provider));
            assert!(!check_retained_fixture_guard(interp, policy, generation));
            assert!(check_retained_fixture_guard(interp, command, generation));
            assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
            assert!(interp
                .native_invocation_dialect()
                .execution_point()
                .is_none());
            assert!(interp.release_command_guard(policy));
            assert!(interp.release_command_guard(command));
        });
    }
}

#[test]
fn string_length_intrinsic_uses_the_selected_runtime_character_model() {
    leak_free(|i| {
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        for intrinsic in [
            tcl_registry::IntrinsicId::StringLength,
            tcl_registry::IntrinsicId::StringIndex,
        ] {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                intrinsic.stable_id(),
                intrinsic.guard_semantics_key(i.runtime_version()),
            );
            let token = i
                .prepare_command_guard(b"string", identity, domains)
                .unwrap();
            assert!(i.check_command_guard(token, b"string"));
        }

        let value = new_string("é🙂".as_bytes());
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[value]),
            Some(Code::Ok)
        );
        assert_eq!(i.result_bytes(), b"2");

        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let live_domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);
        let token = i
            .prepare_command_guard(b"string", identity, live_domains)
            .expect("registry BASE domains are guardable");
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(!i.check_command_guard_identity(token, b"string", identity));
        assert!(i.release_command_guard(token));
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[value]),
            Some(Code::Error)
        );
        assert_eq!(
            i.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native string count cache origin",
                ),
            )
        );
        i.reset_native_compilation_admission();
        let c86_value = new_string("é🙂".as_bytes());
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[c86_value]),
            Some(Code::Ok)
        );
        assert_eq!(i.result_bytes(), b"3");
        assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
        assert_eq!(i.result_bytes(), b"3");

        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
        assert_eq!(i.result_bytes(), b"2");
        drop_fresh(value);
        drop_fresh(c86_value);
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::ListLength, &[]),
            None
        );
    });
}

#[test]
fn renaming_spec_registered_string_stales_its_intrinsic_guard() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let token = i
            .prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        assert_eq!(i.eval_str(b"rename string moved"), Code::Ok);
        assert!(!i.check_command_guard(token, b"string"));
        assert_eq!(
            i.prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment)
            ),
            Err(GuardError::IdentityUnavailable)
        );
    });
}

/// The real runtime's `string length` guard, over the registry's base
/// domains, survives the definition, rename and alias of unrelated commands
/// and falls back once `string` itself is rebound.
#[test]
fn an_unrelated_mutation_keeps_the_string_length_guard() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);
        let token = i
            .prepare_command_guard(b"string", identity, domains)
            .expect("the registry BASE domains are guardable");

        ok(i, b"proc foo {x} {return $x}");
        ok(i, b"rename foo bar");
        ok(i, b"interp alias {} baz {} bar");
        assert!(i.check_command_guard_identity(token, b"string", identity));

        ok(i, b"rename string moved");
        assert!(!i.check_command_guard_identity(token, b"string", identity));
        ok(i, b"rename moved string");
        assert!(i.check_command_guard_identity(token, b"string", identity));
        ok(i, b"proc string args {return shadow}");
        assert!(!i.check_command_guard_identity(token, b"string", identity));
    });
}

/// The pin keeps every attestation and the pinned surface decides which of
/// them a guard reaches: a command the release lacks has none, and answers
/// again under a release that has it.
#[test]
fn a_pin_to_a_release_without_the_command_leaves_it_unattested() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        // This is the actual shipped allocation, not a same-name host handler.
        let intrinsic = tcl_registry::IntrinsicId::ListAssign;
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(i.runtime_version()),
        );
        let domains =
            GuardDomains::one(GuardDomain::CommandEnvironment).with(GuardDomain::VariableTrace);
        let token = i
            .prepare_command_guard(b"lassign", identity, domains)
            .expect("attested in the default release");
        assert!(i.check_command_guard(token, b"lassign"));

        i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert!(!i.check_command_guard(token, b"lassign"));
        assert_eq!(
            i.prepare_command_guard(b"lassign", identity, domains),
            Err(GuardError::IdentityUnavailable)
        );

        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        assert!(i
            .prepare_command_guard(b"lassign", identity, domains)
            .is_ok());
    });
}

#[test]
fn command_trace_stales_the_string_intrinsic_guard() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let token = i
            .prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        assert_eq!(
            i.eval_str(b"trace add command string rename callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard(token, b"string"));
    });
}
