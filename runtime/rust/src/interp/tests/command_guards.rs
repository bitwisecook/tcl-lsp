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

//! Command guards: the lookup domains each mutation moves, identity and lifecycle checks, and what keeps or drops a guard.

use super::*;
use crate::interp::commands::CommandVisibilityOutcome;

/// Each lookup event moves the domain that covers it, whatever else it
/// leaves alone (#2292): namespace membership moves `Namespace`, and the
/// unknown handler or the command it names moves `UnknownHandling`.
#[test]
fn lookup_events_move_the_namespace_and_unknown_handling_domains() {
    let setup = b"namespace eval src {proc p {} {}; namespace export p}";
    let rows: [(GuardDomain, &[u8], &[u8]); 10] = [
        (GuardDomain::Namespace, b"", b"namespace eval created {}"),
        (GuardDomain::Namespace, b"", b"namespace delete src"),
        (GuardDomain::Namespace, b"", b"namespace export guarded"),
        (GuardDomain::Namespace, b"", b"namespace path ::src"),
        (GuardDomain::Namespace, b"", b"namespace import src::p"),
        (
            GuardDomain::Namespace,
            b"namespace import src::p",
            b"namespace forget src::p",
        ),
        (
            GuardDomain::UnknownHandling,
            b"",
            b"namespace unknown handler",
        ),
        (
            GuardDomain::UnknownHandling,
            b"",
            b"proc unknown args {return 0}",
        ),
        (
            GuardDomain::UnknownHandling,
            b"proc unknown args {}",
            b"rename unknown {}",
        ),
        (
            GuardDomain::UnknownHandling,
            b"proc unknown args {}",
            b"rename unknown moved",
        ),
    ];
    for (domain, prepare, event) in rows {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            ok(i, setup);
            ok(i, prepare);
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, GuardDomains::one(domain))
                .unwrap();
            assert!(i.check_command_guard(token, b"guarded"));
            ok(i, event);
            assert!(
                !i.check_command_guard(token, b"guarded"),
                "{domain:?} kept across {}",
                String::from_utf8_lossy(event)
            );
        });
    }
}

#[test]
fn command_guard_checks_identity_and_exact_lifecycle() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        assert_eq!(
            i.prepare_command_guard(b"guarded", GuardIdentity::new(1, 72), domains),
            Err(GuardError::IdentityMismatch)
        );
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();
        assert!(i.check_command_guard(token, b"guarded"));
        assert!(i.release_command_guard(token));
        assert!(!i.release_command_guard(token));
        assert!(!i.check_command_guard(token, b"guarded"));
    });
}

/// A request for a Family-B intrinsic covers the variable-trace domain or
/// is refused, whatever the caller asked for, and a Value member needs no
/// such domain. Covered, the guard is refused while a variable trace
/// exists and goes stale when one is added.
#[test]
fn a_family_b_guard_request_must_cover_the_variable_trace_domain() {
    use tcl_registry::IntrinsicId;
    leak_free(|i| {
        let version = i.runtime_version();
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
        i.register_guarded_builtin(b"stores", guarded_builtin, stores);
        i.register_guarded_builtin(b"pure", guarded_builtin, value);
        let command = GuardDomains::one(GuardDomain::CommandEnvironment);
        let traced = command.with(GuardDomain::VariableTrace);

        assert_eq!(
            i.prepare_command_guard(b"stores", stores, command),
            Err(GuardError::DomainsInsufficient)
        );
        let token = i
            .prepare_command_guard(b"pure", value, command)
            .expect("a Value member requires no variable-trace domain");
        assert!(i.release_command_guard(token));

        let token = i
            .prepare_command_guard(b"stores", stores, traced)
            .expect("a request covering the family's domain");
        assert!(i.check_command_guard_identity(token, b"stores", stores));
        assert_eq!(
            i.eval_str(b"trace add variable watched write callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard_identity(token, b"stores", stores));
        assert_eq!(
            i.prepare_command_guard(b"stores", stores, traced),
            Err(GuardError::PrerequisiteUnsatisfied)
        );
        assert!(i.prepare_command_guard(b"pure", value, command).is_ok());
    });
}

/// A guard is bound to its command's token. A definition, rename or alias
/// of another command leaves it and its attestation alone; replacing,
/// renaming away or hiding the command drops it, and restoring the same
/// token brings it back; a profile pin keeps the attestation.
#[test]
fn an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();

        i.register_builtin(b"unrelated", guarded_builtin);
        ok(i, b"proc foo {} {return 1}");
        ok(i, b"rename foo bar");
        ok(i, b"interp alias {} baz {} bar");
        ok(i, b"rename baz {}");
        assert!(i.check_command_guard(token, b"guarded"));
        assert!(i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .is_ok());

        // A change to the lookup environment itself stales the token over
        // it, and the attestation stays.
        ok(i, b"namespace eval other {namespace path ::}");
        assert!(!i.check_command_guard(token, b"guarded"));
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .expect("still attested");

        // The profile pin keeps the attestation and a token that does not
        // depend on interpreter policy.
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(
            i.check_command_guard(token, b"guarded"),
            "guard phase/generation/broad(before,after)/policy(before,after)/kernel: {:?}",
            super::super::command_guard_lookup::diagnostic(),
        );
        assert!(i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .is_ok());

        // Renaming the command away drops the guard at its name, and the
        // attestation goes with the command; restoring the name restores it.
        ok(i, b"rename guarded moved");
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
        assert!(i
            .prepare_command_guard(b"moved", GUARDED_IDENTITY, domains)
            .is_ok());
        ok(i, b"rename moved guarded");
        assert!(i.check_command_guard(token, b"guarded"));

        // Hiding it drops the guard, exposing it restores it.
        assert_eq!(
            i.hide_command(b"guarded", b"guarded"),
            CommandVisibilityOutcome::Moved
        );
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.expose_command(b"guarded", b"guarded"),
            CommandVisibilityOutcome::Moved
        );
        assert!(i.check_command_guard(token, b"guarded"));

        // A different command at the name is never attested.
        ok(i, b"proc guarded {} {return 1}");
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
    });
}

#[test]
fn command_currency_retains_original_lookup_but_refuses_path_and_scope_changes() {
    leak_free(|interpreter| {
        let caller = interpreter
            .namespaces_mut()
            .ensure_namespace(GLOBAL, b"caller");
        interpreter.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        interpreter.current_ns.set(caller);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = interpreter
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();
        interpreter.register_builtin(b"::unrelated::worker", guarded_builtin);
        assert!(interpreter.check_command_guard(token, b"guarded"));
        interpreter
            .namespaces_mut()
            .delete_namespace(GLOBAL, b"unrelated");
        assert!(interpreter.check_command_guard(token, b"guarded"));
        interpreter.current_ns.set(GLOBAL);
        assert!(!interpreter.check_command_guard(token, b"guarded"));
        interpreter.current_ns.set(caller);
        assert!(interpreter.check_command_guard(token, b"guarded"));
        interpreter.namespaces_mut().set_path(caller, vec![GLOBAL]);
        assert!(!interpreter.check_command_guard(token, b"guarded"));
        interpreter.namespaces_mut().set_path(caller, Vec::new());
        assert!(!interpreter.check_command_guard(token, b"guarded"));
        assert!(interpreter.release_command_guard(token));
        interpreter.current_ns.set(GLOBAL);
    });
}

/// The attestation belongs to the token the name reaches, so a definition
/// in a namespace that shadows the command drops the guard for calls made
/// from that namespace and only from it.
#[test]
fn a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();

        ok(i, b"namespace eval ns {proc guarded {} {return shadow}}");
        let ns = i
            .namespaces
            .borrow()
            .find_namespace(GLOBAL, b"ns")
            .expect("the namespace exists");
        assert!(i.check_command_guard(token, b"guarded"));
        i.current_ns.set(ns);
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
        i.current_ns.set(GLOBAL);
        assert!(i.check_command_guard(token, b"guarded"));
    });
}

/// The three lookup domains move together on a hidden invocation into a
/// namespace, whether the namespace is named, is the global one, or does not
/// exist yet, and on `oo::copy`: events the guard domain's docs list beside
/// `namespace path` and the rest, which a command-table mutation is not.
#[test]
fn the_lookup_domains_move_on_a_hidden_invocation_into_a_namespace_and_on_oo_copy() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::UnknownHandling);
        let moves = |i: &mut Interp, script: &[u8]| {
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .expect("the lookup domains are guardable");
            ok(i, script);
            assert!(i.attested_identities(b"guarded").is_some());
            !i.check_command_guard(token, b"guarded")
        };

        ok(i, b"interp hide {} lindex");
        assert!(
            moves(i, b"interp invokehidden {} -namespace fresh lindex {a b} 0"),
            "a namespace it creates"
        );
        assert!(
            moves(i, b"interp invokehidden {} -namespace fresh lindex {a b} 0"),
            "a namespace that exists"
        );
        assert!(
            moves(i, b"interp invokehidden {} -global lindex {a b} 0"),
            "the global namespace"
        );
        assert!(
            !moves(i, b"interp invokehidden {} lindex {a b} 0"),
            "a hidden invocation that names no namespace moves nothing"
        );

        ok(
            i,
            b"oo::class create C {variable v; constructor {} {set v 1}}",
        );
        ok(i, b"set o [C new]");
        assert!(moves(i, b"oo::copy $o"), "oo::copy");
    });
}
