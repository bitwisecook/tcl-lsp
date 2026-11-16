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

//! Registering builtins and stock ensembles: publication receipts, attestation of the original worker, lookup revisions and guarded replacement.

use super::*;

fn registration_host_worker(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    interpreter.set_result_bytes(b"HOST");
    Code::Ok
}

#[test]
fn namespace_child_birth_and_export_clear_use_the_shared_lookup_revision() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        let namespace = interpreter
            .namespaces_mut()
            .ensure_namespace(GLOBAL, b"::RegistrationExisting");
        interpreter.register_guarded_builtin(
            b"registrationGuarded",
            registration_host_worker,
            GUARDED_IDENTITY,
        );
        let domains = [
            GuardDomain::CommandEnvironment,
            GuardDomain::Namespace,
            GuardDomain::UnknownHandling,
        ];
        let prepare = |interpreter: &mut Interp| {
            domains.map(|domain| {
                interpreter
                    .prepare_command_guard(
                        b"registrationGuarded",
                        GUARDED_IDENTITY,
                        GuardDomains::one(domain),
                    )
                    .unwrap()
            })
        };
        let tokens = prepare(interpreter);
        assert_eq!(
            interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"::RegistrationExisting"),
            namespace
        );
        interpreter.namespaces_mut().clear_exports(namespace);
        interpreter.register_builtin(
            b"::RegistrationExisting::unrelated",
            registration_host_worker,
        );
        assert!(tokens
            .iter()
            .all(|token| interpreter.check_command_guard(*token, b"registrationGuarded")));
        // This is the actual command-publication child birth, rather than
        // an Interp facade adding an invalidation after table mutation.
        interpreter.register_builtin(b"::RegistrationBorn::unrelated", registration_host_worker);
        assert!(tokens.iter().zip(domains).all(|(token, domain)| {
            interpreter.check_command_guard(*token, b"registrationGuarded")
                == (domain == GuardDomain::CommandEnvironment)
        }));
        for token in tokens {
            assert!(interpreter.release_command_guard(token));
        }
        interpreter.namespaces_mut().export(namespace, b"unrelated");
        let tokens = prepare(interpreter);
        interpreter.namespaces_mut().clear_exports(namespace);
        assert!(tokens
            .iter()
            .all(|token| !interpreter.check_command_guard(*token, b"registrationGuarded")));
        for token in tokens {
            assert!(interpreter.release_command_guard(token));
        }
    });
}

#[cfg(debug_assertions)]
unsafe extern "C" fn registration_ensemble_retirement(data: *mut c_void) {
    // The actual old ObjCommand transfers its sole client record here.
    let state = unsafe { Box::from_raw(data.cast::<PublicationRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    match state.mode {
        0 => interpreter.register_builtin(b"retiring", registration_host_worker),
        1 => {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original ensemble delete refusal",
                ),
            );
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("ensemble-publication-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}

#[cfg(debug_assertions)]
#[test]
fn stock_root_installation_keeps_the_actual_publication_receipt_after_retirement() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Runtime callbacks check custody/refusal, not native ensemble chronology.
    for mode in 0..4 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            let state = retirement_state(interpreter, mode);
            let calls = state.calls.clone();
            let old = interpreter
                .create_obj_command(
                    b"retiring",
                    ObjCommand::new(
                        publication_noop,
                        Box::into_raw(state).cast(),
                        Some(registration_ensemble_retirement),
                    ),
                )
                .unwrap();
            interpreter.register_stock_ensemble(
                tcl_registry::invocation_words::EnsembleImplementationFamily::String,
                b"retiring",
                registration_host_worker,
                &[(b"length", registration_host_worker)],
                &[b"length".as_slice()],
            );
            assert_eq!(calls.get(), 1);
            let current = interpreter.resolve_cmd_token(b"retiring");
            if mode == 3 {
                let generation = current.unwrap();
                assert_ne!(generation, old);
                assert!(interpreter.native_execution_refusal().is_none());
                assert_eq!(
                    interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .get(&generation),
                    Some(&b"retiring".to_vec())
                );
                assert!(matches!(
                    interpreter.raw_command_by_generation(generation),
                    Some(Command::Ensemble(_))
                ));
            } else {
                assert!(interpreter.native_execution_refusal().is_some());
                if let Some(generation) = current {
                    assert!(!interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .contains_key(&generation));
                    assert!(!interpreter
                        .registry_builtin_names
                        .borrow()
                        .contains_key(&generation));
                }
                if mode == 0 {
                    assert!(matches!(
                        interpreter.raw_command_by_generation(current.unwrap()),
                        Some(Command::Builtin(_))
                    ));
                }
            }
        });
    }
}

fn registration_nested_retirement(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    REGISTRATION_NESTED_RETIREMENT_CALLS.with(|calls| calls.set(calls.get() + 1));
    match REGISTRATION_NESTED_RETIREMENT_MODE.with(std::cell::Cell::get) {
        0 => interpreter.register_builtin(b"::tcl::binary::encode", registration_host_worker),
        1 => {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original nested ensemble delete refusal",
                ),
            );
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("nested-ensemble-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
    Code::Ok
}

#[test]
fn stock_nested_installation_stops_before_attesting_a_refused_or_recreated_slot() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual command delete trace, not an invented native delete role,
    // supplies this software callback on the installed nested ensemble.
    for mode in 0..4 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            REGISTRATION_NESTED_RETIREMENT_MODE.with(|value| value.set(mode));
            REGISTRATION_NESTED_RETIREMENT_CALLS.with(|value| value.set(0));
            interpreter.register_builtin(
                b"registrationNestedRetirement",
                registration_nested_retirement,
            );
            let name = b"::tcl::binary::encode";
            let old = interpreter.resolve_cmd_token(name).unwrap();
            assert_eq!(
                interpreter.eval_str(
                    b"trace add command ::tcl::binary::encode delete registrationNestedRetirement"
                ),
                Code::Ok
            );
            interpreter.register_stock_nested_ensemble(
                tcl_registry::invocation_words::EnsembleImplementationFamily::Binary,
                name,
                registration_host_worker,
                &[(b"hex", registration_host_worker)],
            );
            assert_eq!(
                REGISTRATION_NESTED_RETIREMENT_CALLS.with(std::cell::Cell::get),
                1
            );
            let current = interpreter.resolve_cmd_token(name);
            if mode == 3 {
                let generation = current.unwrap();
                assert_ne!(generation, old);
                assert!(interpreter.native_execution_refusal().is_none());
                assert_eq!(
                    interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .get(&generation),
                    Some(&name.to_vec())
                );
                assert!(matches!(
                    interpreter.raw_command_by_generation(generation),
                    Some(Command::Ensemble(_))
                ));
            } else {
                assert!(interpreter.native_execution_refusal().is_some());
                if let Some(generation) = current {
                    assert!(!interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .contains_key(&generation));
                }
                if mode == 0 {
                    assert!(matches!(
                        interpreter.raw_command_by_generation(current.unwrap()),
                        Some(Command::Builtin(_))
                    ));
                }
            }
            REGISTRATION_NESTED_RETIREMENT_MODE.with(|value| value.set(3));
            REGISTRATION_NESTED_RETIREMENT_CALLS.with(|value| value.set(0));
        });
    }
}

#[test]
fn dynamic_builtin_registration_preserves_native_namespace_fallback_without_stock_identity() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // These are Runtime registration/lookup controls, not external execution proofs.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interpreter| {
            *interpreter = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interpreter.register_builtin(b"registrationHostWorker", registration_host_worker);
            let generation = interpreter
                .resolve_cmd_token(b"registrationHostWorker")
                .unwrap();
            assert!(!interpreter
                .registry_builtin_names
                .borrow()
                .contains_key(&generation));
            assert!(!interpreter
                .native_compilation
                .borrow()
                .stock
                .contains_key(&generation));
            assert!(interpreter
                .guarded_commands
                .borrow()
                .get(&generation)
                .is_none());
            assert_eq!(
                interpreter.eval_str(b"namespace eval RegistrationHost {registrationHostWorker}"),
                Code::Ok,
                "{profile}: result={:?}, refusal={:?}",
                interpreter.result_bytes(),
                interpreter.native_execution_refusal(),
            );
            assert_eq!(interpreter.result_bytes(), b"HOST", "{profile}");
            assert!(interpreter.native_execution_refusal().is_none());
        });
    }
}

#[test]
fn explicitly_guarded_host_replacement_has_no_stock_release_floor() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        interpreter.register_guarded_builtin(
            b"lassign",
            registration_host_worker,
            GUARDED_IDENTITY,
        );
        let generation = interpreter.resolve_cmd_token(b"lassign").unwrap();
        let token = interpreter
            .prepare_command_guard(
                b"lassign",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(interpreter.eval_str(b"lassign"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"HOST");
        assert!(interpreter.check_command_guard_identity(token, b"lassign", GUARDED_IDENTITY));
        assert!(!interpreter
            .registry_builtin_names
            .borrow()
            .contains_key(&generation));
        assert!(!interpreter
            .native_compilation
            .borrow()
            .stock
            .contains_key(&generation));
        assert!(interpreter.release_command_guard(token));
    });
}

#[test]
fn stock_ensemble_attestation_requires_the_original_worker_at_its_actual_map_slot() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for move_original in [false, true] {
        leak_free(|interpreter| {
            let intrinsic = tcl_registry::IntrinsicId::StringLength;
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                intrinsic.stable_id(),
                intrinsic.guard_semantics_key(interpreter.runtime_version()),
            );
            let token = interpreter
                .prepare_command_guard(
                    b"string",
                    identity,
                    GuardDomains::one(GuardDomain::CommandEnvironment),
                )
                .unwrap();
            let worker = interpreter
                .resolve_cmd_token(b"::tcl::string::length")
                .unwrap();
            if move_original {
                assert_eq!(
                    interpreter
                        .eval_str(b"rename ::tcl::string::length ::registrationOriginalLength"),
                    Code::Ok
                );
                assert!(interpreter
                    .raw_command_location_by_generation(worker)
                    .is_some());
            }
            interpreter.register_builtin(b"::tcl::string::length", registration_host_worker);
            assert!(!interpreter.check_command_guard_identity(token, b"string", identity));
            assert!(interpreter.attested_identities(b"string").is_none());
            assert_eq!(interpreter.eval_str(b"string length abc"), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"HOST");
            assert!(interpreter.release_command_guard(token));
        });
    }
}

#[test]
fn stock_ensemble_attestation_refuses_a_changed_actual_mapping() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        let intrinsic = tcl_registry::IntrinsicId::StringLength;
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(interpreter.runtime_version()),
        );
        let token = interpreter
            .prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        interpreter.register_builtin(b"registrationMappedLength", registration_host_worker);
        assert_eq!(
            interpreter.eval_str(
                b"namespace ensemble configure string -map {length ::registrationMappedLength}",
            ),
            Code::Ok
        );
        assert!(!interpreter.check_command_guard_identity(token, b"string", identity));
        assert_eq!(interpreter.eval_str(b"string length abc"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"HOST");
        assert!(interpreter.release_command_guard(token));
    });
}

#[test]
fn namespace_mutation_owner_synchronises_all_lookup_guard_domains() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mutation in 0..10 {
        leak_free(|interpreter| {
            assert_eq!(interpreter.eval_str(
                    b"namespace eval RegistrationSource {proc target {} {}; namespace export target};
                      namespace eval RegistrationImports {namespace import ::RegistrationSource::target}",
                ), Code::Ok);
            let namespace = interpreter
                .namespaces()
                .find_namespace(GLOBAL, b"RegistrationImports")
                .unwrap();
            if mutation == 4 {
                interpreter.namespaces_mut().export(namespace, b"target");
            } else if mutation == 5 {
                interpreter
                    .namespaces_mut()
                    .set_path(namespace, vec![GLOBAL]);
            } else if mutation == 9 {
                interpreter
                    .namespaces_mut()
                    .begin_namespace_teardown(namespace);
            }
            interpreter.register_guarded_builtin(
                b"registrationGuarded",
                registration_host_worker,
                GUARDED_IDENTITY,
            );
            let tokens = [
                GuardDomain::CommandEnvironment,
                GuardDomain::Namespace,
                GuardDomain::UnknownHandling,
            ]
            .map(|domain| {
                interpreter
                    .prepare_command_guard(
                        b"registrationGuarded",
                        GUARDED_IDENTITY,
                        GuardDomains::one(domain),
                    )
                    .unwrap()
            });
            interpreter.register_builtin(b"registrationUnrelated", registration_host_worker);
            assert!(tokens
                .iter()
                .all(|token| interpreter.check_command_guard(*token, b"registrationGuarded")));
            match mutation {
                0 => interpreter.namespaces_mut().set_path(GLOBAL, vec![GLOBAL]),
                1 => interpreter
                    .namespaces_mut()
                    .export(GLOBAL, b"registrationNewExport"),
                2 => {
                    let handler = obj::Owned::fresh(new_string(b"registrationUnknown"));
                    let previous = interpreter
                        .namespaces_mut()
                        .set_unknown_handler(GLOBAL, Some(handler));
                    drop(previous);
                }
                3 => assert!(interpreter.namespaces_mut().remove_in(namespace, b"target")),
                4 => interpreter.namespaces_mut().clear_exports(namespace),
                5 => interpreter
                    .namespaces_mut()
                    .finish_native_namespace_command_path(namespace),
                6 => interpreter
                    .namespaces_mut()
                    .delete_namespace_by_id(namespace),
                7 => {
                    let retired = interpreter
                        .namespaces_mut()
                        .clear_namespace_ids(&[namespace]);
                    drop(retired);
                }
                8 => interpreter
                    .namespaces_mut()
                    .begin_namespace_variable_teardown(namespace),
                _ => interpreter
                    .namespaces_mut()
                    .finish_namespace_teardown(&[namespace]),
            }
            assert!(tokens
                .iter()
                .zip([
                    GuardDomain::CommandEnvironment,
                    GuardDomain::Namespace,
                    GuardDomain::UnknownHandling,
                ])
                .all(|(token, domain)| {
                    interpreter.check_command_guard(*token, b"registrationGuarded")
                        == (domain == GuardDomain::CommandEnvironment && mutation >= 6)
                }));
            for token in tokens {
                assert!(interpreter.release_command_guard(token));
            }
        });
    }
}

#[test]
fn exhausted_namespace_lookup_revision_permanently_poisons_its_guard_domains() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        interpreter.register_guarded_builtin(
            b"registrationGuarded",
            registration_host_worker,
            GUARDED_IDENTITY,
        );
        let domains = [
            GuardDomain::CommandEnvironment,
            GuardDomain::Namespace,
            GuardDomain::UnknownHandling,
        ];
        let tokens = domains.map(|domain| {
            interpreter
                .prepare_command_guard(
                    b"registrationGuarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(domain),
                )
                .unwrap()
        });
        let independent = interpreter
            .prepare_command_guard(
                b"registrationGuarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .unwrap();
        interpreter
            .namespaces_mut()
            .exhaust_guard_lookup_environment_revision_for_test();
        assert!(tokens
            .iter()
            .all(|token| !interpreter.check_command_guard(*token, b"registrationGuarded")));
        assert!(interpreter.check_command_guard(independent, b"registrationGuarded"));
        interpreter.namespaces_mut().set_path(GLOBAL, vec![GLOBAL]);
        for domain in domains {
            assert_eq!(
                interpreter.prepare_command_guard(
                    b"registrationGuarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(domain),
                ),
                Err(GuardError::Poisoned)
            );
        }
        for token in tokens {
            assert!(interpreter.release_command_guard(token));
        }
        assert!(interpreter.release_command_guard(independent));
    });
}

unsafe extern "C" fn registration_recreate_on_retirement(data: *mut c_void) {
    // SAFETY: the actual retiring command transfers this sole client record.
    let state = unsafe { Box::from_raw(data.cast::<PublicationRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    interpreter.register_builtin(b"retiring", registration_host_worker);
}

#[test]
fn refused_guarded_registration_cannot_attest_a_callback_created_generation() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This checks the Runtime receipt; it claims no native discard chronology.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let state = retirement_state(interpreter, 0);
        let calls = state.calls.clone();
        interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    publication_noop,
                    Box::into_raw(state).cast(),
                    Some(registration_recreate_on_retirement),
                ),
            )
            .unwrap();
        interpreter.register_guarded_builtin(b"retiring", resultless_builtin, GUARDED_IDENTITY);
        assert_eq!(calls.get(), 1);
        assert!(interpreter.native_execution_refusal().is_some());
        let replacement = interpreter.resolve_cmd_token(b"retiring").unwrap();
        assert!(interpreter
            .guarded_commands
            .borrow()
            .get(&replacement)
            .is_none());
        assert!(interpreter.attested_identities(b"retiring").is_none());
        assert!(!interpreter
            .registry_builtin_names
            .borrow()
            .contains_key(&replacement));
    });
}

#[test]
fn renamed_stock_ensemble_keeps_its_release_identity_and_dynamic_replacement_does_not() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        assert_eq!(
            interpreter.eval_str(b"rename dict registrationMovedDict"),
            Code::Ok
        );
        let stock = interpreter
            .resolve_cmd_token(b"registrationMovedDict")
            .unwrap();
        assert!(matches!(
            interpreter.raw_command_by_generation(stock),
            Some(Command::Ensemble(_))
        ));
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert!(interpreter
            .resolve_dispatchable(GLOBAL, b"registrationMovedDict")
            .is_none());
        interpreter.register_builtin(b"registrationMovedDict", registration_host_worker);
        let replacement = interpreter
            .resolve_cmd_token(b"registrationMovedDict")
            .unwrap();
        assert_ne!(replacement, stock);
        assert_eq!(interpreter.eval_str(b"registrationMovedDict"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"HOST");
    });
}

#[test]
fn release_visibility_does_not_donate_a_foreign_list_backing_origin() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The explicit origin refusal is separate from command-table visibility.
    use tcl_syntax::value::ValueOps;
    leak_free(|interpreter| {
        let member = obj::Owned::fresh(new_string(b"A"));
        let list = obj::Owned::fresh(interpreter.new_list(vec![member.as_ptr()]));
        assert_eq!(
            interpreter.list_elements(&list.as_ptr()).unwrap(),
            vec![member.as_ptr()]
        );
        let before = obj::native_object_snapshot(list.as_ptr()).unwrap();
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(matches!(
            interpreter.list_elements(&list.as_ptr()),
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native List string recipe origin"
            ))
        ));
        assert_eq!(obj::native_object_snapshot(list.as_ptr()).unwrap(), before);
    });
}
