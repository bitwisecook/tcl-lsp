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

thread_local! {
    /// The content hash every call on this thread is forced to, when
    /// set: two different contents in one bucket.
    pub(super) static FORCED_CONTENT_HASH: Cell<Option<u64>> = const { Cell::new(None) };
}

#[test]
fn replaced_receiver_cannot_seed_the_replacements_cache() {
    struct ReplacingHost;
    impl PackHookHost for ReplacingHost {
        fn invoke(&self, _slot: HookSlot, _call: &HookCall<'_>) -> HookAnswer {
            install_host(Rc::new(FixedHost(HookAnswer::Fold("new".into()))));
            HookAnswer::Fold("old".into())
        }
    }
    std::thread::spawn(|| {
        let slot = allocate(HookFamily::ConstFold, &HookInputs::parse(&["words"])).unwrap();
        let fold = const_fold_fn(slot).unwrap();
        install_host(Rc::new(ReplacingHost));
        assert_eq!(fold(&["same"]), Some("old".into()));
        assert_eq!(fold(&["same"]), Some("new".into()));
        assert_eq!(fold(&["same"]), Some("new".into()));
        assert_eq!(cache_stats().hits, 1);
        clear_host();
    })
    .join()
    .unwrap();
}
struct FixedHost(HookAnswer);

impl PackHookHost for FixedHost {
    fn invoke(&self, _slot: HookSlot, _call: &HookCall<'_>) -> HookAnswer {
        self.0.clone()
    }
}

struct CountingHost {
    calls: std::cell::Cell<u32>,
}

impl PackHookHost for CountingHost {
    fn invoke(&self, _slot: HookSlot, _call: &HookCall<'_>) -> HookAnswer {
        self.calls.set(self.calls.get() + 1);
        HookAnswer::Roles(vec![(0, ArgRole::VarWrite)])
    }
}

#[test]
fn owning_receiver_runs_before_shared_cache_and_ignores_global_slot_mode() {
    struct UncachedHost(Cell<usize>);
    impl PackHookHost for UncachedHost {
        fn invoke(&self, _slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
            self.0.set(self.0.get() + 1);
            HookAnswer::Fold(call.words[0].value.to_owned())
        }
        fn requires_uncached_dispatch(&self) -> bool {
            true
        }
    }
    std::thread::spawn(|| {
        let slot = allocate(HookFamily::ConstFold, &HookInputs::parse(&["words"])).unwrap();
        let fold = const_fold_fn(slot).unwrap();
        install_host(Rc::new(FixedHost(HookAnswer::Fold("ordinary".into()))));
        assert_eq!(fold(&["input"]), Some("ordinary".into()));
        assert_eq!(fold(&["input"]), Some("ordinary".into()));
        assert_eq!(cache_stats().hits, 1);
        let actual = Rc::new(UncachedHost(Cell::new(0)));
        install_host(actual.clone());
        let words = literal_words(&["input"]);
        let call = call_of(&words, None);
        let key = shape_key(slot, &call, CacheMode::Content).unwrap();
        SHAPE_CACHE.with(|cache| {
            cache.borrow_mut().entries.insert(
                key,
                CacheEntry {
                    content: CallContent::of(&call, CacheMode::Content),
                    answer: HookAnswer::Fold("stale".into()),
                },
            );
        });
        assert_eq!(fold(&["input"]), Some("input".into()));
        std::thread::spawn(move || {
            CACHE_MODE[slot.family.index()][usize::from(slot.index)]
                .store(CacheMode::Shape.as_u8(), Ordering::Relaxed);
        })
        .join()
        .unwrap();
        assert_eq!(fold(&["different"]), Some("different".into()));
        assert_eq!(actual.0.get(), 2);
        assert_eq!(cache_stats().hits, 0);
        assert_eq!(cache_stats().misses, 0);
        clear_host();
    })
    .join()
    .unwrap();
}

#[test]
fn current_receiver_identity_refuses_generation_aba_and_preserves_foreign_host() {
    std::thread::spawn(|| {
        let first: Rc<dyn PackHookHost> = Rc::new(FixedHost(HookAnswer::Abstain));
        let second: Rc<dyn PackHookHost> = Rc::new(FixedHost(HookAnswer::Abstain));
        assert!(!current_host_is(&first));
        install_host(Rc::clone(&first));
        let first_generation = GENERATION.with(Cell::get);
        assert!(current_host_is(&first));
        install_host(Rc::clone(&second));
        // Reproduce the observable equal-generation state after wrap;
        // equality must never substitute for the actual Rc relation.
        GENERATION.with(|generation| generation.set(first_generation));
        assert!(!current_host_is(&first));
        assert!(!clear_current_host_if(&first));
        assert!(current_host_is(&second));
        assert_eq!(GENERATION.with(Cell::get), first_generation);
        HOST.with(|slot| {
            let _borrowed = slot.borrow_mut();
            assert!(!current_host_is(&second));
            assert!(!clear_current_host_if(&second));
        });
        assert!(current_host_is(&second));
        assert!(clear_current_host_if(&second));
        assert!(!current_host_is(&second));
        assert_eq!(GENERATION.with(Cell::get), EvaluatorGeneration::NO_HOST);
    })
    .join()
    .unwrap();
}

#[test]
fn private_worker_denies_installer_queries_and_receiver_installation() {
    std::thread::spawn(|| {
        assert!(deny_external_evaluators_on_private_thread());
        assert!(private_thread_evaluators_are_closed());
        let queried = Cell::new(false);
        assert!(
            ensure_host_with_installer(|| {
                queried.set(true);
                Some(|| panic!("a private worker must not invoke an external installer"))
            })
            .is_none()
        );
        assert!(!queried.get());
        let host: Rc<dyn PackHookHost> = Rc::new(FixedHost(HookAnswer::Fold("external".into())));
        install_host(Rc::clone(&host));
        install_plan_host(host, 1);
        assert!(private_thread_evaluators_are_closed());
        assert_eq!(evaluator_generation(), EvaluatorGeneration::NO_HOST);
        let slot = HookSlot {
            family: HookFamily::ConstFold,
            index: 0,
        };
        assert!(!slot_available(slot));
        let call = call_of(&[], None);
        assert!(matches!(dispatch(slot, &call), HookAnswer::Abstain));
        assert!(deny_external_evaluators_on_private_thread());
    })
    .join()
    .unwrap();
    assert!(!external_evaluators_blocked());
}

#[test]
fn private_worker_denial_does_not_clear_an_existing_receiver() {
    std::thread::spawn(|| {
        let host: Rc<dyn PackHookHost> = Rc::new(FixedHost(HookAnswer::Abstain));
        install_host(Rc::clone(&host));
        let generation = GENERATION.with(Cell::get);
        assert!(!deny_external_evaluators_on_private_thread());
        assert!(!private_thread_evaluators_are_closed());
        assert_eq!(GENERATION.with(Cell::get), generation);
        assert!(HOST.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|held| Rc::ptr_eq(held, &host))
        }));
    })
    .join()
    .unwrap();
}

#[test]
fn a_thread_with_no_host_abstains() {
    let slot = allocate(HookFamily::ConstFold, &HookInputs::unrestricted())
        .expect("a fold slot is available");
    let fold = const_fold_fn(slot).expect("the slot's family matches");
    assert_eq!(fold(&["abc"]), None);
}

#[test]
fn the_fold_thunk_carries_its_slot_identity() {
    let first = allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
    let second = allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
    assert_ne!(first.index(), second.index());
    let first_fn = const_fold_fn(first).unwrap();
    let second_fn = const_fold_fn(second).unwrap();
    assert!(!std::ptr::fn_addr_eq(first_fn, second_fn));
}

#[test]
fn a_wrong_family_slot_has_no_thunk() {
    let slot = allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
    assert!(arg_role_resolver_fn(slot).is_none());
}

#[test]
fn silence_is_per_family() {
    let sink = allocate(HookFamily::TaintSinkGate, &HookInputs::unrestricted()).unwrap();
    let gate = taint_sink_gate_fn(sink).unwrap();
    install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
    // Silence means the sink applies.
    assert!(gate(&["x"]));
    install_host(Rc::new(FixedHost(HookAnswer::SinkSuppressed)));
    assert!(!gate(&["x"]));
    clear_host();
}

/// A resolver body's facts are read against the call's own words: a
/// literal alias becomes an alias fact; a fact naming a computed word
/// abstains and widens the variable-cell domains for that word; a fact
/// naming a word past the call names nothing; and silence is no
/// transitions. No verb can state any family but the alias one.
#[test]
fn a_resolver_body_states_alias_facts_and_widens_where_it_abstains() {
    use crate::state_transition::{
        CallerFrameSelection, StateTransition, StateTransitionWidening, VariableAliasTarget,
        VariableCellAliasTransition,
    };
    let slot = allocate(
        HookFamily::StateTransitionResolver,
        &HookInputs::unrestricted(),
    )
    .expect("a resolver slot is available");
    let resolver = state_transition_resolver_fn(slot).expect("the slot's family matches");
    install_host(Rc::new(FixedHost(HookAnswer::Transitions(vec![
        PackTransition::Alias {
            local: 2,
            target: 1,
            level: Some(0),
        },
        PackTransition::NamespaceVariable { name: 3 },
        PackTransition::Alias {
            local: 9,
            target: 1,
            level: None,
        },
    ]))));
    let words = [
        InvocationWord::Literal("#0"),
        InvocationWord::Literal("other"),
        InvocationWord::Literal("mine"),
        InvocationWord::Dynamic,
    ];
    let transitions = resolver(InvocationArguments::structured(&words));
    let facts: Vec<&StateTransition> = transitions
        .facts()
        .iter()
        .map(|fact| &fact.transition)
        .collect();
    assert_eq!(
        facts,
        [
            &StateTransition::VariableCellAlias(VariableCellAliasTransition {
                destination: VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::LocatedLiteral {
                    value: "mine".to_owned(),
                    argument_index: 2,
                },
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: CallerFrameSelection::Explicit(TransitionSubject::LocatedLiteral {
                        value: "#0".to_owned(),
                        argument_index: 0,
                    }),
                    variable: TransitionSubject::LocatedLiteral {
                        value: "other".to_owned(),
                        argument_index: 1,
                    },
                },
                writes_value: false,
                words: AliasWords {
                    local: 2,
                    target: 1,
                },
            }),
            &StateTransition::Widen(StateTransitionWidening {
                domains: VARIABLE_ALIAS_DOMAINS.to_vec(),
                subject: TransitionSubject::Unknown {
                    argument_index: 3,
                    word_kind: InvocationWordKind::Dynamic,
                },
            }),
        ]
    );
    install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
    assert!(
        resolver(InvocationArguments::structured(&words))
            .facts()
            .is_empty(),
        "silence is no transitions"
    );
    clear_host();
}

#[test]
fn pack_aliases_preserve_original_known_bytes_and_selected_name_policy() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    // Descriptor facts retain operands; they prove neither a physical
    // variable link nor an entered native frame.
    let name = b"::ns::v\xff\0tail";
    let words = [InvocationWord::KnownBytes(name)];
    let fact = [PackTransition::NamespaceVariable { name: 0 }];
    let arguments = InvocationArguments::structured(&words);
    for dialect in [
        tcl_native_model::InvocationDialect::for_version(TclVersion::V8_6),
        tcl_native_model::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        ),
    ] {
        let transitions = pack_transitions(arguments.with_dialect(dialect), &fact);
        assert_eq!(
            transitions.facts()[0].transition,
            StateTransition::VariableCellAlias(VariableCellAliasTransition {
                destination: VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::LocatedNativeBytes {
                    value: b"v\xff".to_vec(),
                    argument_index: 0,
                },
                target: VariableAliasTarget::CurrentNamespace {
                    variable: TransitionSubject::LocatedNativeBytes {
                        value: name.to_vec(),
                        argument_index: 0,
                    },
                },
                writes_value: false,
                words: AliasWords::same(0),
            }),
        );
    }
    let unknown = pack_transitions(arguments, &fact);
    assert_eq!(
        unknown.facts()[0].transition,
        StateTransition::VariableCellAlias(VariableCellAliasTransition {
            destination: VariableAliasDestination::CurrentNamespaceOrLocal,
            local: TransitionSubject::Unknown {
                argument_index: 0,
                word_kind: InvocationWordKind::KnownBytes,
            },
            target: VariableAliasTarget::CurrentNamespace {
                variable: TransitionSubject::LocatedNativeBytes {
                    value: name.to_vec(),
                    argument_index: 0,
                },
            },
            writes_value: false,
            words: AliasWords::same(0),
        }),
        "a missing dialect cannot select a C or Jim local-name projection",
    );
}

#[test]
fn pack_caller_alias_keeps_counted_operands_and_effective_ordinals() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    let words = [
        InvocationWord::KnownBytes(b"#0"),
        InvocationWord::KnownBytes(b"target\0bytes\xff"),
        InvocationWord::KnownBytes(b"local\0bytes\xfe"),
    ];
    let transitions = pack_transitions(
        InvocationArguments::structured(&words),
        &[PackTransition::Alias {
            local: 2,
            target: 1,
            level: Some(0),
        }],
    );
    assert_eq!(
        transitions.facts()[0].transition,
        StateTransition::VariableCellAlias(VariableCellAliasTransition {
            destination: VariableAliasDestination::CurrentNamespaceOrLocal,
            local: TransitionSubject::LocatedNativeBytes {
                value: b"local\0bytes\xfe".to_vec(),
                argument_index: 2,
            },
            target: VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(TransitionSubject::LocatedNativeBytes {
                    value: b"#0".to_vec(),
                    argument_index: 0,
                }),
                variable: TransitionSubject::LocatedNativeBytes {
                    value: b"target\0bytes\xff".to_vec(),
                    argument_index: 1,
                },
            },
            writes_value: false,
            words: AliasWords {
                local: 2,
                target: 1
            },
        }),
    );
}

#[test]
fn native_byte_hook_kind_cannot_reuse_a_logical_literal_cache_entry() {
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    let words = [HookWord {
        value: "",
        kind: InvocationWordKind::KnownBytes,
    }];
    let call = HookCall {
        words: &words,
        version: None,
        in_event_body: false,
        option: None,
        constraints: None,
        dialect: None,
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    assert_eq!(kind_word(words[0].kind), "known-bytes");
    assert!(!call.all_literal());
    let slot = HookSlot {
        family: HookFamily::ArgRoleResolver,
        index: 0,
    };
    assert!(shape_key(slot, &call, CacheMode::Shape).is_none());
    assert!(shape_key(slot, &call, CacheMode::Content).is_none());
    let literal = [HookWord {
        value: "",
        kind: InvocationWordKind::Literal,
    }];
    assert!(
        shape_key(
            slot,
            &HookCall {
                words: &literal,
                ..call
            },
            CacheMode::Shape
        )
        .is_some()
    );
}

#[test]
fn declared_shape_inputs_are_cacheable_and_words_are_not() {
    assert!(HookInputs::parse(&["nwords", "kinds"]).shape_only());
    assert!(!HookInputs::parse(&["words"]).shape_only());
    assert!(!HookInputs::unrestricted().shape_only());
    // An unknown input word degrades to unrestricted, never to cacheable.
    assert!(!HookInputs::parse(&["nwords", "moon-phase"]).shape_only());
}

#[test]
fn a_shape_cacheable_resolver_reaches_the_host_once_per_shape() {
    let slot = allocate(
        HookFamily::ArgRoleResolver,
        &HookInputs::parse(&["nwords", "kinds"]),
    )
    .unwrap();
    let resolver = arg_role_resolver_fn(slot).unwrap();
    let host = Rc::new(CountingHost {
        calls: std::cell::Cell::new(0),
    });
    install_host(host.clone());
    let first = resolver(&["a", "b"]);
    let second = resolver(&["c", "d"]);
    assert_eq!(first, second);
    assert_eq!(host.calls.get(), 1, "the second call is a shape hit");
    // A different word count is a different shape.
    let _ = resolver(&["a", "b", "c"]);
    assert_eq!(host.calls.get(), 2);
    let stats = cache_stats();
    assert_eq!((stats.hits, stats.misses, stats.entries), (1, 2, 2));
    clear_host();
}

/// A host folding its words, counting its calls.
struct EchoHost {
    calls: Cell<u32>,
}

impl PackHookHost for EchoHost {
    fn invoke(&self, _slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        self.calls.set(self.calls.get() + 1);
        HookAnswer::Fold(
            call.words
                .iter()
                .map(|word| word.value)
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

/// A content-keyed answer is proven on every hit, never found by its
/// hash alone: two calls whose content hashes collide are two misses,
/// each answered for its own words, and the same content again is a
/// hit.
#[test]
fn a_hash_collision_is_not_a_hit() {
    let slot = allocate(HookFamily::ConstFold, &HookInputs::parse(&["words"])).unwrap();
    assert_eq!(cache_mode(slot), CacheMode::Content);
    let fold = const_fold_fn(slot).unwrap();
    let host = Rc::new(EchoHost {
        calls: Cell::new(0),
    });
    install_host(host.clone());
    FORCED_CONTENT_HASH.with(|forced| forced.set(Some(7)));
    assert_eq!(fold(&["abc"]), Some("abc".to_owned()));
    assert_eq!(
        fold(&["xyz"]),
        Some("xyz".to_owned()),
        "one bucket, another content: not a hit"
    );
    assert_eq!(host.calls.get(), 2);
    assert_eq!(fold(&["xyz"]), Some("xyz".to_owned()));
    assert_eq!(host.calls.get(), 2, "the same content is a hit");
    FORCED_CONTENT_HASH.with(|forced| forced.set(None));
    clear_host();
}

/// The evaluator generation names the host and its health: installing
/// one moves the worker off `NO_HOST`, a quarantine gives it a
/// generation no other state has and moves the process's epoch, and
/// clearing the host returns it to `NO_HOST`. Two workers serving one
/// published plan share a generation, so their memoised answers are
/// shared; the worker whose host then quarantines a hook shares it with
/// no one.
#[test]
fn host_install_and_quarantine_bump_the_generation() {
    // A plan no other test in this binary publishes.
    const PLAN: u64 = u64::MAX - 7;
    clear_host();
    assert_eq!(evaluator_generation(), EvaluatorGeneration::NO_HOST);
    install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
    let installed = evaluator_generation();
    assert_ne!(installed, EvaluatorGeneration::NO_HOST);
    install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
    let reinstalled = evaluator_generation();
    assert_ne!(reinstalled, installed, "a host without a plan is its own");
    let epoch = evaluator_epoch();
    note_quarantine();
    assert_ne!(evaluator_generation(), reinstalled);
    assert!(
        evaluator_epoch() > epoch,
        "a quarantine moves the process's epoch"
    );
    clear_host();
    assert_eq!(evaluator_generation(), EvaluatorGeneration::NO_HOST);

    install_plan_host(Rc::new(FixedHost(HookAnswer::Abstain)), PLAN);
    let here = evaluator_generation();
    let (there, quarantined) = std::thread::spawn(|| {
        install_plan_host(Rc::new(FixedHost(HookAnswer::Abstain)), PLAN);
        let shared = evaluator_generation();
        note_quarantine();
        (shared, evaluator_generation())
    })
    .join()
    .expect("the worker ran");
    assert_eq!(there, here, "one plan, one generation");
    assert_ne!(quarantined, here, "a quarantine is its worker's own");
    assert_eq!(
        evaluator_generation(),
        here,
        "and this worker keeps its own"
    );
    clear_host();
}

#[test]
fn an_uncacheable_resolver_reaches_the_host_every_time() {
    let slot = allocate(HookFamily::ArgRoleResolver, &HookInputs::parse(&["words"])).unwrap();
    let resolver = arg_role_resolver_fn(slot).unwrap();
    let host = Rc::new(CountingHost {
        calls: std::cell::Cell::new(0),
    });
    install_host(host.clone());
    let _ = resolver(&["a", "b"]);
    let _ = resolver(&["c", "d"]);
    assert_eq!(host.calls.get(), 2);
    clear_host();
}
