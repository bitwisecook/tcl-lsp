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

//! Which builtins the runtime attests: the pinned generation, engine-owned ensembles, intrinsics and the backing report.

use super::*;

/// The sweep reads the generation the interpreter is pinned to and nothing
/// an overlay installed: a pack's command is no runtime implementation, and
/// only the runtime may attest.
#[test]
fn identities_come_from_the_pinned_generation_never_an_overlay() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_registry::{CommandSpec, IntrinsicId, SemanticOperationId};
    leak_free(|i| {
        // A generation an overlay installed under this interpreter's own
        // profile, holding a spec that declares an intrinsic, and a builtin
        // of that name in the interpreter.
        let overlaid = tcl_registry::registry_for_profile_with_overlay(
            i.dialect_profile(),
            0xC0DE,
            |registry| {
                registry.insert(CommandSpec {
                    name: "overlay_length",
                    semantic_operation: Some(SemanticOperationId::Intrinsic(
                        IntrinsicId::StringLength,
                    )),
                    ..CommandSpec::DEFAULT
                });
            },
        );
        i.register_builtin(b"overlay_length", guarded_builtin);

        i.attach_identities();
        assert!(
            i.attested_identities(b"overlay_length").is_none(),
            "the sweep of the pinned generation attests nothing an overlay declares"
        );
        assert!(
            i.attested_identities(b"string").is_some(),
            "and still attests the shipped command"
        );

        // A store cannot attest an ordinary host handler by its spelling.
        i.attach_identities_from(&overlaid);
        assert!(i.attested_identities(b"overlay_length").is_none());
        // Explicit spec registration supplies the independent semantic claim.
        i.register_spec_builtin(
            overlaid.get_exact("overlay_length").unwrap(),
            guarded_builtin,
        );
        assert!(i.attested_identities(b"overlay_length").is_some());
    });
}

/// A name is attested only while the command bound at it is the builtin
/// the runtime registered there: a procedure defined over it, or another
/// builtin renamed into it, gains nothing, and the original, put back,
/// has what it had.
#[test]
fn the_sweep_attests_only_the_builtin_it_registered() {
    leak_free(|i| {
        use tcl_registry::IntrinsicId;
        let length = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let attested = |i: &Interp| {
            i.attested_identities(b"string")
                .is_some_and(|identities| identities.contains(&length))
        };
        assert!(attested(i));

        ok(i, b"rename string original");
        assert!(!attested(i), "moved away, the name reaches nothing");
        ok(i, b"proc string args {return x}");
        i.attach_identities();
        assert!(!attested(i), "a procedure at the name gains no attestation");

        ok(i, b"rename string {}");
        ok(i, b"rename puts string");
        i.attach_identities();
        assert!(
            !attested(i),
            "another builtin renamed into the name gains none"
        );

        ok(i, b"rename string puts");
        ok(i, b"rename original string");
        assert!(attested(i), "the original, restored, is attested as it was");
    });
}

/// Every builtin this runtime registers at the name of a spec that
/// declares an intrinsic or definition contract is attested for every
/// corresponding identity, and no other command is attested.
#[test]
fn engine_owned_ensemble_attestation_requires_original_workers_and_configuration() {
    for change_configuration in [false, true] {
        leak_free(|interpreter| {
            let generation = interpreter
                .shipped_builtin_generation(b"info")
                .expect("actual engine-installed Info ensemble");
            let Some(Command::Ensemble(ensemble)) =
                interpreter.raw_command_by_generation(generation)
            else {
                panic!("original Info ensemble token");
            };
            assert!(interpreter.stock_ensemble_implementation_is_current(generation));
            assert!(interpreter.attested_identities(b"info").is_some());
            let mut configuration = ensemble.config();
            if change_configuration {
                configuration.prefixes = !configuration.prefixes;
                ensemble.configure(configuration);
            } else {
                let worker = configuration
                    .map
                    .as_ref()
                    .expect("actual Info worker map")
                    .iter()
                    .find(|(member, _)| member == b"exists")
                    .expect("actual Info Exists worker")
                    .1[0]
                    .clone();
                interpreter.register_builtin(&worker, guarded_builtin);
            }
            assert!(interpreter
                .stock_ensemble_registry_identity(generation)
                .is_some());
            assert!(!interpreter.stock_ensemble_implementation_is_current(generation));
            assert!(interpreter.shipped_builtin_generation(b"info").is_none());
            interpreter.attach_identities();
            assert!(interpreter.attested_identities(b"info").is_none());
        });
    }
}

#[test]
fn a_public_ensemble_copy_cannot_acquire_engine_intrinsic_attestation() {
    leak_free(|interpreter| {
        let original = interpreter
            .shipped_builtin_generation(b"info")
            .expect("actual engine-installed Info ensemble");
        let Some(Command::Ensemble(ensemble)) = interpreter.raw_command_by_generation(original)
        else {
            panic!("original Info ensemble token");
        };
        let replacement = interpreter
            .create_ensemble(b"info", ensemble.config())
            .expect("actual public replacement ensemble");
        assert_ne!(replacement, original);
        assert!(interpreter
            .stock_ensemble_registry_identity(replacement)
            .is_none());
        assert!(interpreter.shipped_builtin_generation(b"info").is_none());
        interpreter.attach_identities();
        assert!(interpreter.attested_identities(b"info").is_none());
    });
}

#[test]
fn every_registered_builtin_with_an_intrinsic_is_attested_for_all_of_them() {
    leak_free(|i| {
        let registry = crate::environment::store_for_profile(i.dialect_profile());
        let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        let mut attested_names = 0;
        for name in registry.command_names() {
            let spec = registry.get_exact(name).expect("a named spec");
            let expected: std::collections::BTreeSet<GuardIdentity> = spec
                .intrinsic_ids()
                .into_iter()
                .flat_map(|id| {
                    id.guard_semantics_variants().iter().map(move |semantics| {
                        GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                    })
                })
                .chain(
                    spec.native_lowering_at(
                        i.native_invocation_dialect().authoring_query().as_ref(),
                    )
                    .definition_guard_identity(i.native_invocation_dialect()),
                )
                .collect();
            let actual = i.attested_identities(name.as_bytes());
            if expected.is_empty() || report.of(name) != RegisteredBacking::Builtin {
                assert!(actual.is_none(), "{name} has nothing to attest");
            } else {
                assert_eq!(actual.as_ref(), Some(&expected), "{name}");
                attested_names += 1;
            }
        }
        assert!(attested_names >= 10, "{attested_names}");
    });
}

#[test]
fn unicode_backing_catalogue_omits_opaque_names_without_changing_command_identity() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interp| {
        interp.register_builtin(b"opaque_\xff", guarded_builtin);
        interp.register_builtin("café".as_bytes(), guarded_builtin);
        assert!(interp
            .command_names()
            .iter()
            .any(|name| name == b"opaque_\xff"));
        let head = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"opaque_\xff"));
        assert_eq!(interp.dispatch_invoke(&[head.as_ptr()]), Code::Ok);
        assert!(interp.native_execution_refusal().is_none());
        let report = interp.backing_report();
        assert!(report
            .iter()
            .any(|(name, backing)| name == "café" && *backing == RegisteredBacking::Builtin));
        assert!(!report.iter().any(|(name, _)| name == "opaque_�"));
        assert!(!report.iter().any(|(name, _)| name.starts_with("opaque_")));
    });
}

#[test]
fn backing_report_preserves_colon_components_from_actual_placed_slots() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software table placement and diagnostic projection only; no Native dispatch or inverse-name claim.
    leak_free(|interp| {
        let holder = {
            let mut namespaces = interp.namespaces.borrow_mut();
            for name in [b"run".as_slice(), b":run", b"::run"] {
                namespaces.bind(GLOBAL, name, Command::Builtin(guarded_builtin));
            }
            let holder = namespaces.ensure_namespace(GLOBAL, b":");
            namespaces.bind(holder, b"run", Command::Builtin(guarded_builtin));
            namespaces.bind(holder, b":run", Command::Builtin(guarded_builtin));
            assert_eq!(namespaces.command_fqn_at(GLOBAL, b":run"), b":::run");
            assert_eq!(namespaces.command_fqn_at(GLOBAL, b"::run"), b"::::run");
            assert_eq!(namespaces.command_fqn_at(holder, b"run"), b":::::run");
            holder
        };
        let original = interp
            .namespaces
            .borrow()
            .command_generation(holder, b"run");
        assert!(original.is_some());
        let entries = interp.backing_report();
        for expected in ["run", ":run", "::run", ":::run", "::::run"] {
            assert!(
                    entries
                        .iter()
                        .any(|(key, backing)| key == expected
                            && *backing == RegisteredBacking::Builtin),
                    "{expected}"
                );
        }
        let report = tcl_runtime_api::BackingReport::from_unrooted_entries(entries);
        for expected in ["run", ":run", "::run", ":::run", "::::run"] {
            assert_eq!(report.of_unrooted(expected), RegisteredBacking::Builtin);
        }
        assert_eq!(report.of_unrooted(":::::run"), RegisteredBacking::Absent);
        assert_eq!(
            interp
                .namespaces
                .borrow()
                .command_generation(holder, b"run"),
            original
        );
    });
}

/// The backing report says what the handler table holds: handlers, the
/// engine's `TclOO` roots and the commands the object system binds in every
/// object, handlers that only refuse, and absences; and nothing a script
/// defines.
#[test]
fn the_backing_report_says_what_the_handler_table_holds() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        let report = |i: &Interp| tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        let before = report(i);
        assert_eq!(before.of("set"), RegisteredBacking::Builtin);
        assert_eq!(
            before.of("::tcl::string::insert"),
            RegisteredBacking::Builtin
        );
        assert_eq!(before.of("exec"), RegisteredBacking::Unsupported);
        assert_eq!(before.of("oo::class"), RegisteredBacking::Object);
        assert_eq!(before.of("oo::object"), RegisteredBacking::Object);
        assert_eq!(before.of("my"), RegisteredBacking::Object);
        assert_eq!(before.of("zipfs"), RegisteredBacking::Absent);
        // The selected native ensemble owns this real private worker.
        assert_eq!(before.of("tcl::dict::get"), RegisteredBacking::Builtin);
        #[cfg(have_tommath)]
        assert_eq!(before.of("expr"), RegisteredBacking::Builtin);
        #[cfg(not(have_tommath))]
        assert_eq!(before.of("expr"), RegisteredBacking::NeedsNumericTower);

        ok(i, b"proc mine {} {}");
        ok(i, b"oo::class create Mine");
        ok(i, b"interp alias {} aliased {} set");
        let after = report(i);
        for name in ["mine", "Mine", "aliased"] {
            assert_eq!(after.of(name), RegisteredBacking::Absent, "{name}");
        }
        assert_eq!(after, before, "a script adds nothing to the report");
    });
}

/// The commands a build without the numeric tower reports as needing it
/// are the ones a build with it registers as handlers.
#[cfg(have_tommath)]
#[test]
fn the_tower_commands_a_build_without_it_names_are_registered_with_it() {
    leak_free(|i| {
        let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        let names = crate::builtins::tower_command_names();
        assert!(names.len() > 60, "{}", names.len());
        for name in names {
            assert_eq!(report.of(&name), RegisteredBacking::Builtin, "{name}");
        }
    });
}

/// A build without the tower reports each of those commands as needing it,
/// and registers none of them.
#[cfg(not(have_tommath))]
#[test]
fn a_build_without_the_tower_reports_the_commands_it_lacks_as_needing_it() {
    leak_free(|i| {
        let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        for name in crate::builtins::tower_command_names() {
            assert_eq!(
                report.of(&name),
                RegisteredBacking::NeedsNumericTower,
                "{name}"
            );
        }
        assert_eq!(ok(i, b"info commands expr"), b"");
    });
}
