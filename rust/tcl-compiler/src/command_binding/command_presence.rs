// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected-slot presence, independently of missing-command fallback execution.

use super::{
    BTreeSet, MayBinding, ModuleCommandBindings, SourceCommandBindings, SourceInvocationBinding,
};

/// Presence of the evaluated command's selected slot at its actual lookup point.
/// This says nothing about alias targets, imported implementations, autoloading,
/// fallback handlers, successful completion or native compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCommandSlotPresence {
    /// Every closed lookup path has a slot.
    Present,
    /// Every closed lookup path lacks a slot.
    Absent,
    /// Closed paths include both a present and an absent slot.
    MayPresent,
    /// Lookup routing or binding evidence is incomplete.
    Unknown,
}

impl SourceCommandBindings {
    /// Diagnostic presence from actual reached operands, or a separately sealed
    /// original source lookup when no reached observation exists. Actual unknown
    /// observations always win. Fixed tables and conditional source lookup supply
    /// no registration, completion, runtime token or compiler authority.
    #[must_use]
    pub fn diagnostic_math_function_presence_at(
        &self,
        registry: &tcl_registry::CommandRegistry,
        function: &str,
        offset: u32,
    ) -> SourceCommandSlotPresence {
        // Proof: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        let Some(origin) = self.root_origin.as_ref() else {
            return SourceCommandSlotPresence::Unknown;
        };
        if let Some(presence) =
            self.fixed_math_diagnostic_presence
                .get(&(origin.clone(), offset, function.to_owned()))
        {
            return *presence;
        }
        // The original expression owner joins reached snapshots through its
        // sealed implicit-function name. A written-head presence query cannot
        // answer for this producer; incomplete reached snapshots still refuse.
        let Some(config) = self.lexer_config else {
            return SourceCommandSlotPresence::Unknown;
        };
        let Some(end) = u32::try_from(function.len())
            .ok()
            .and_then(|length| offset.checked_add(length))
        else {
            return SourceCommandSlotPresence::Unknown;
        };
        let mut original = self
            .original_math_functions_in_source(
                registry,
                origin.source_image(),
                config,
                tcl_lexer::Span::new(offset, end),
            )
            .into_iter()
            .filter(|occurrence| {
                occurrence.span().start() == offset && occurrence.bytes() == function.as_bytes()
            });
        let Some(first) = original.next() else {
            return SourceCommandSlotPresence::Unknown;
        };
        if !original.all(|other| other == first) {
            return SourceCommandSlotPresence::Unknown;
        }
        first.diagnostic_presence()
    }

    pub(super) fn record_fixed_math_diagnostic_presence(
        &mut self,
        function: &str,
        site: u32,
        state: &ModuleCommandBindings,
    ) {
        use SourceCommandSlotPresence as Presence;
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Resolution;
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let presence = if state.opaque_domain {
            Presence::Unknown
        } else if let Some(entry) = &state.baseline.native_entry {
            entry
                .math_functions
                .as_ref()
                .map_or(Presence::Unknown, |table| match table.lookup(function) {
                    Resolution::Present(_) => Presence::Present,
                    Resolution::Absent => Presence::Absent,
                    Resolution::Unknown => Presence::Unknown,
                })
        } else {
            state
                .baseline
                .dialect
                .and_then(|dialect| {
                    tcl_registry::mathfunc::fresh_fixed_function_presence(dialect, function)
                })
                .map_or(Presence::Unknown, |present| {
                    if present {
                        Presence::Present
                    } else {
                        Presence::Absent
                    }
                })
        };
        self.fixed_math_diagnostic_presence
            .entry((origin.clone(), site, function.to_owned()))
            .and_modify(|previous| {
                if *previous != presence {
                    *previous = Presence::Unknown;
                }
            })
            .or_insert(presence);
    }

    /// Diagnostic assistance at an exact offset in the retained document
    /// origin. Separate future worlds can provide advice but never execution
    /// authority. Derived/loaded source offsets cannot alias document offsets.
    #[must_use]
    pub fn diagnostic_slot_presence_at(&self, offset: u32) -> SourceCommandSlotPresence {
        self.diagnostic_command_presence(None, offset)
    }

    /// Diagnostic presence of a name consumed by an actual command at `offset`.
    /// The referenced name is looked up in the post-argv immutable world, not
    /// at its written argument span. Future entry worlds remain independent.
    #[must_use]
    pub fn diagnostic_command_slot_presence_at(
        &self,
        name: &str,
        offset: u32,
    ) -> SourceCommandSlotPresence {
        self.diagnostic_command_presence(Some(name), offset)
    }

    fn diagnostic_command_presence(
        &self,
        name: Option<&str>,
        offset: u32,
    ) -> SourceCommandSlotPresence {
        let select = |binding: &SourceInvocationBinding| {
            name.map_or_else(
                || binding.selected_slot_diagnostic_presence(),
                |name| {
                    binding
                        .lookup_command_word(name)
                        .selected_slot_diagnostic_presence()
                },
            )
        };
        let actual = self.invocation_at_source("", offset);
        if actual.lookup_state.is_some() {
            return select(&actual);
        }
        let mut advice = None;
        for future in self.future_body_inventories() {
            if self.root_origin.as_ref() != Some(&future.source().origin) {
                continue;
            }
            let Some(binding) = future.invocation_at(offset) else {
                continue;
            };
            let selected = select(&binding);
            advice = Some(match advice {
                None => selected,
                Some(previous) if previous == selected => previous,
                Some(SourceCommandSlotPresence::Unknown) => SourceCommandSlotPresence::Unknown,
                Some(_) if selected == SourceCommandSlotPresence::Unknown => selected,
                Some(_) => SourceCommandSlotPresence::MayPresent,
            });
        }
        advice.unwrap_or(SourceCommandSlotPresence::Unknown)
    }
}

impl SourceInvocationBinding {
    /// Actual complete Logical advice input selected at this lookup's entry.
    /// Its absence cannot be repaired from a source site or reporting label.
    #[must_use]
    pub fn logical_source_name_advice_input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.lookup_state
            .as_ref()?
            .state
            .logical_source_name_advice_input()
    }

    /// Diagnostic-only slot advice. An absent slot is actionable only when
    /// the fallback remains the explicit initial handler or is itself absent.
    /// This does not prove that the initial autoload handler will fail.
    #[must_use]
    pub fn selected_slot_diagnostic_presence(&self) -> SourceCommandSlotPresence {
        if self.invocation_site().is_some() && self.logical_source_name_advice_input().is_none() {
            return self.original_selected_slot_diagnostic_presence();
        }
        let presence = self.selected_slot_presence();
        if presence != SourceCommandSlotPresence::Absent {
            return presence;
        }
        if matches!(
            self.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
        ) && self.proved_execution_target().is_some()
        {
            return SourceCommandSlotPresence::Unknown;
        }
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return SourceCommandSlotPresence::Unknown;
        };
        let state = &snapshot.state;
        let Some(head) = self.lookup_word.as_deref() else {
            return SourceCommandSlotPresence::Unknown;
        };
        if !state.has_retained_diagnostic_qualifier(head, &self.lookup_namespace_key) {
            return SourceCommandSlotPresence::Unknown;
        }
        if state
            .unknown_lookup_namespaces
            .contains(&self.lookup_namespace_key)
            || state
                .namespace_unknown_handlers
                .contains(&self.lookup_namespace_key)
        {
            return SourceCommandSlotPresence::Unknown;
        }
        let handlers = if self.logical_source_name_advice_input().is_some() {
            state
                .baseline
                .semantics
                .unresolved_command_handlers()
                .iter()
                .flat_map(|handler| state.source_keys(handler, &self.lookup_namespace_key))
                .collect::<Vec<_>>()
        } else {
            let Some(policy) = state.baseline.dialect.and_then(|dialect| {
                tcl_registry::command_lookup::native_lookup_fallback_policy(
                    dialect,
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                )
            }) else {
                return SourceCommandSlotPresence::Unknown;
            };
            state.source_keys(policy.default_handler, &self.lookup_namespace_key)
        };
        for handler in handlers {
            let expected = match &handler {
                super::SourceCommandKey::Slot { .. } => {
                    state.original_initial_bindings_for_key(&handler)
                }
                super::SourceCommandKey::Authored(_) => {
                    Some(ModuleCommandBindings::unmodified_bindings(
                        &handler,
                        state.baseline.semantics.binding_names(),
                    ))
                }
            };
            let Some(expected) = expected else {
                return SourceCommandSlotPresence::Unknown;
            };
            let actual = state.binding_alternatives(&handler);
            if actual != expected && actual != BTreeSet::from([MayBinding::Missing]) {
                return SourceCommandSlotPresence::Unknown;
            }
        }
        presence
    }

    fn original_selected_slot_diagnostic_presence(&self) -> SourceCommandSlotPresence {
        use SourceCommandSlotPresence as Presence;
        let Some(input) = self.original_recorded_head_name_input() else {
            return Presence::Unknown;
        };
        let Some(snapshot) = &self.lookup_state else {
            return Presence::Unknown;
        };
        let state = &snapshot.state;
        let presence = state.original_slot_presence_for_input(&input, &self.lookup_namespace_key);
        if presence != Presence::Absent {
            return presence;
        }
        if matches!(
            self.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
        ) && self.proved_execution_target().is_some()
        {
            return Presence::Unknown;
        }
        state.original_diagnostic_presence_after_absence(
            &self.lookup_namespace_key,
            input.policy(),
            presence,
            state.original_diagnostic_qualifier_is_retained(&input, &self.lookup_namespace_key),
        )
    }

    /// Query only the retained evaluated head and its positioned lookup world.
    /// A present alias/import wrapper counts as a slot even when its terminal
    /// target is absent. An absent slot can still invoke a custom fallback.
    #[must_use]
    pub fn selected_slot_presence(&self) -> SourceCommandSlotPresence {
        if self.invocation_site().is_some() && self.logical_source_name_advice_input().is_none() {
            let Some(input) = self.original_recorded_head_name_input() else {
                return SourceCommandSlotPresence::Unknown;
            };
            return self.lookup_state.as_ref().map_or(
                SourceCommandSlotPresence::Unknown,
                |snapshot| {
                    snapshot
                        .state
                        .original_slot_presence_for_input(&input, &self.lookup_namespace_key)
                },
            );
        }
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return SourceCommandSlotPresence::Unknown;
        };
        let Some(head) = self.lookup_word.as_deref() else {
            return SourceCommandSlotPresence::Unknown;
        };
        snapshot
            .state
            .source_slot_presence(head, &self.lookup_namespace_key)
    }
}

impl ModuleCommandBindings {
    /// Independently validated complete Logical input for this immutable world.
    #[must_use]
    pub(crate) fn logical_source_name_advice_input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.baseline.logical_source_input.as_ref()
    }

    /// Missing qualified slots are useful advice only inside a namespace
    /// retained at this point. This does not alter execution slot absence.
    fn has_retained_diagnostic_qualifier(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> bool {
        let key = namespace.namespace_key();
        if !matches!(key.as_ref(), super::SourceNamespaceKey::Authored(_)) {
            return self
                .source_keys_checked(head, key.as_ref())
                .is_ok_and(|keys| {
                    keys.iter()
                        .any(|slot| self.namespaces.contains(slot.holder().as_ref()))
                });
        }
        let super::SourceNamespaceKey::Authored(namespace) = key.as_ref() else {
            return false;
        };

        if !tcl_syntax::naming::is_qualified(head.as_bytes()) {
            return true;
        }
        let tail = tcl_syntax::naming::written_command_tail(head.as_bytes());
        let qualifier = &head[..head.len() - tail.len()];
        self.source_lookup_paths(qualifier, namespace)
            .iter()
            .all(|path| {
                path.iter().any(|key| {
                    // The written qualifier ends in a separator. Lookup
                    // therefore supplies an empty command tail after its
                    // constructed namespace; remove only that final join.
                    let Some(spelling) = key.authored_spelling() else {
                        return false;
                    };
                    let Some(parent) = spelling.strip_suffix("::") else {
                        return false;
                    };
                    let parent = if parent.is_empty() { "::" } else { parent };
                    self.namespaces.contains(parent)
                        && !self.unknown_lookup_namespaces.contains(parent)
                })
            })
    }

    /// Pre-fallback slot presence for one retained table and lookup route.
    /// Source diagnostics and executable absence guards share this owner.
    /// Shared routing guard for purpose-only slot and navigation queries.
    /// Absolute names do not require a current namespace; relative names do.
    pub(super) fn source_lookup_is_closed(
        &self,
        word: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> bool {
        !self.has_opaque_domain()
            && self.source_keys_checked(word, namespace).is_ok()
            && (!matches!(
                namespace.namespace_key().as_ref(),
                super::SourceNamespaceKey::Authored(_)
            ) || word.starts_with("::")
                || ((self.source_variables.namespace_known
                    || self.receiver_command_lookup_path(word).is_some())
                    && !self.unknown_lookup_namespaces.contains(namespace)))
    }

    pub(super) fn source_slot_presence(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> SourceCommandSlotPresence {
        if !self.source_lookup_is_closed(head, namespace) {
            return SourceCommandSlotPresence::Unknown;
        }
        let mut present = false;
        let mut absent = false;
        for path in self.source_lookup_paths(head, namespace) {
            let mut fallthrough = true;
            for slot in path {
                let bindings = self.binding_alternatives(&slot);
                if bindings.is_empty() || bindings.contains(&MayBinding::Unknown) {
                    return SourceCommandSlotPresence::Unknown;
                }
                present |= bindings.iter().any(|binding| {
                    matches!(binding, MayBinding::Target(_) | MayBinding::Imported(_))
                });
                fallthrough = bindings.contains(&MayBinding::Missing);
                if !fallthrough {
                    break;
                }
            }
            absent |= fallthrough;
        }
        match (present, absent) {
            (true, false) => SourceCommandSlotPresence::Present,
            (false, true) => SourceCommandSlotPresence::Absent,
            (true, true) => SourceCommandSlotPresence::MayPresent,
            (false, false) => SourceCommandSlotPresence::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_function_advice_requires_the_original_closed_native_table() {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.4");
        let registry = owner.commands();
        let profile = registry.profile().expect("actual Tcl 8.4 fixture");
        for (function, expected) in [
            ("sin", SourceCommandSlotPresence::Present),
            ("min", SourceCommandSlotPresence::Absent),
            ("custom", SourceCommandSlotPresence::Absent),
        ] {
            let source = format!("expr {{{function}(1)}}");
            let site = u32::try_from(source.find(function).unwrap()).unwrap();
            for unknown_entry in [false, true] {
                let bindings = SourceCommandBindings::analyse_with_options(
                    &source,
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    owner.commands(),
                    super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        unknown_entry,
                        ..Default::default()
                    },
                );
                assert_eq!(
                    bindings.diagnostic_math_function_presence_at(registry, function, site),
                    if unknown_entry {
                        SourceCommandSlotPresence::Unknown
                    } else {
                        expected
                    }
                );
                let script = super::super::ExecutedScriptSource::contiguous(
                    std::sync::Arc::clone(bindings.source_origin().unwrap()),
                    &source,
                    0,
                )
                .unwrap();
                assert!(
                    bindings
                        .implicit_math_invocations_for_script(&script)
                        .iter()
                        .all(|call| call.fixed_prerequisite().is_none())
                );
            }
        }
    }

    fn logical_source_input() -> crate::analyser::ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-slot-source-advice",
            &[],
            "Explicit Logical source advice",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
    }

    #[test]
    fn logical_source_slot_advice_uses_the_retained_input_instead_of_site_kind() {
        // naming.consumer.original-workspace-diagnostic-refinement
        // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
        let input = logical_source_input();
        for (source, expected) in [
            ("missing 1 2", SourceCommandSlotPresence::Absent),
            (
                "proc defined {} {}; defined",
                SourceCommandSlotPresence::Present,
            ),
            (
                "proc unknown args {return restored}; missing",
                SourceCommandSlotPresence::Unknown,
            ),
        ] {
            let analysis = crate::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, input.analyser_profile().name);
            let offset = crate::segmenter::segment_commands_with_offset_and_config(
                source,
                0,
                input.lexer_config(),
            )
            .last()
            .unwrap()
            .span
            .start();
            let binding = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", offset);
            assert!(binding.invocation_site().is_some(), "{source}");
            assert!(
                binding.original_recorded_head_name_input().is_none(),
                "{source}"
            );
            assert_eq!(
                binding.logical_source_name_advice_input(),
                Some(&input),
                "{source}"
            );
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                expected,
                "{source}"
            );
            if expected == SourceCommandSlotPresence::Absent {
                assert_eq!(analysis.unresolved_command_sites.len(), 1);
                assert!(
                    analysis
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W123)
                );
            }
        }
        let native = point("missing 1 2", "missing");
        assert!(native.logical_source_name_advice_input().is_none());
        assert!(native.original_recorded_head_name_input().is_some());
        assert_eq!(
            native.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent
        );
    }

    #[test]
    fn logical_source_slot_advice_uses_the_interpreted_head_not_query_labels() {
        // naming.consumer.original-workspace-diagnostic-refinement
        // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
        let input = logical_source_input();
        let source = "missing 1 2";
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let realm = analysis.retained_command_realm().unwrap();
        let first = realm.invocation_at_source("set", 0);
        let second = realm.invocation_at_source("unrelated", 0);
        assert_eq!(first.logical_source_name_advice_input(), Some(&input));
        assert!(first.original_recorded_head_name_input().is_none());
        assert_eq!(first.lookup_word.as_deref(), Some("missing"));
        assert_eq!(second.lookup_word, first.lookup_word);
        assert_eq!(
            first.selected_slot_presence(),
            SourceCommandSlotPresence::Absent
        );
        assert_eq!(
            second.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent
        );
        assert_eq!(analysis.unresolved_command_sites.len(), 1);
        assert_eq!(analysis.unresolved_command_sites[0].1, "missing");
    }

    #[test]
    fn logical_source_slot_advice_requires_authentic_complete_ingress() {
        // naming.consumer.original-workspace-diagnostic-refinement
        // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
        let input = logical_source_input();
        let context = input.context_registry();
        let dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
        let options = super::super::SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let source = "missing";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            input.lexer_config(),
            context.commands(),
            options,
        );
        let genuine = bindings.invocation_at_source("", 0);
        assert_eq!(genuine.logical_source_name_advice_input(), Some(&input));
        assert_eq!(
            genuine.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent
        );
        for options in [
            super::super::SourceAnalysisOptions {
                logical_source_input: None,
                ..options
            },
            super::super::SourceAnalysisOptions {
                execution_name_policy: Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
                    tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
                        tcl_dialect::TclVersion::V8_6,
                    ),
                )),
                ..options
            },
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                input.lexer_config(),
                context.commands(),
                options,
            );
            assert!(
                bindings
                    .invocation_at_source("", 0)
                    .logical_source_name_advice_input()
                    .is_none()
            );
        }
        let mut foreign = input.lexer_config();
        foreign.strict_quoting = !foreign.strict_quoting;
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            foreign,
            context.commands(),
            options,
        );
        assert!(
            bindings
                .invocation_at_source("", 0)
                .logical_source_name_advice_input()
                .is_none()
        );
    }

    #[test]
    fn logical_source_slot_advice_keeps_unknown_and_hosted_contexts_outside_absence() {
        // naming.consumer.original-workspace-diagnostic-refinement
        // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
        let input = logical_source_input();
        let context = input.context_registry();
        let options = super::super::SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                input.unit_profile(),
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let source = "missing";
        let unknown = SourceCommandBindings::analyse_with_options(
            source,
            input.lexer_config(),
            context.commands(),
            super::super::SourceAnalysisOptions {
                unknown_entry: true,
                ..options
            },
        );
        assert_eq!(
            unknown
                .invocation_at_source("", 0)
                .selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Unknown
        );
        let hosted = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("f5-irules")
                .default_context_registry(),
            input.lexer_config(),
        );
        let hosted_context = hosted.context_registry();
        let hosted_options = super::super::SourceAnalysisOptions {
            logical_source_input: Some(&hosted),
            ..options
        };
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            hosted.lexer_config(),
            hosted_context.commands(),
            hosted_options,
        );
        let binding = bindings.invocation_at_source("", 0);
        assert!(binding.logical_source_name_advice_input().is_none());
        assert_eq!(
            binding.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Unknown
        );
    }

    fn point(source: &str, head: &str) -> SourceInvocationBinding {
        point_in(source, head, "tcl8.6")
    }

    fn point_in(source: &str, head: &str, profile: &str) -> SourceInvocationBinding {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let selected = registry.profile().unwrap();
        let result = super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(Some(selected)),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(selected)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
        .invocation_at_source(
            head,
            if head.is_empty() {
                crate::segmenter::segment_commands_with_offset_and_config(
                    source,
                    0,
                    tcl_lexer::LexerConfig::for_profile(Some(selected)),
                )
                .last()
                .unwrap()
                .span
                .start()
            } else {
                u32::try_from(source.rfind(head).unwrap()).unwrap()
            },
        );
        if std::env::var_os("TCL_SLOT_DEBUG").is_some() {
            eprintln!(
                "slot {profile} {source:?} head={:?} targets={:?} unknown={} snapshot={:?}",
                result.lookup_word,
                result.targets,
                result.unknown,
                result.lookup_state.as_ref().map(|snapshot| (
                    snapshot.state.opaque_domain,
                    snapshot.state.opaque_binding_mutation,
                    snapshot.state.loader_handler_unknown,
                    snapshot.state.source_variables.dynamic_traces,
                    snapshot.state.source_variables.namespace_known,
                    result.lookup_namespace.clone(),
                ))
            );
        }
        result
    }

    #[test]
    fn original_slot_diagnostics_keep_opaque_heads_and_refuse_missing_producers() {
        // Implementation contract: naming.compiler.original-slot-presence
        // docs/design/analysis/name-resolution-proofs/original-slot-presence.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = r"missing\uD800 argument";
            let binding = point_in(source, r"missing\uD800", profile);
            assert!(
                binding.lookup_word.is_none(),
                "opaque original head has no UTF-8 report: {profile}"
            );
            assert!(binding.original_recorded_head_name_input().is_some());
            assert_eq!(
                binding.selected_slot_presence(),
                SourceCommandSlotPresence::Absent,
                "{profile}"
            );
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Absent,
                "{profile}"
            );
            let mut presentation = binding.clone();
            presentation.lookup_word = Some("set".to_owned());
            assert_eq!(
                presentation.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Absent
            );
            let mut missing = binding.clone();
            missing.original_written_words = None;
            missing.original_compiler_words = None;
            missing.declaration_layout_observations = None;
            assert_eq!(
                missing.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Unknown
            );
            let mut foreign = binding.clone();
            foreign.original_written_words =
                point_in("other argument", "other", profile).original_written_words;
            assert_eq!(
                foreign.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Unknown
            );
            let custom = point_in(
                r"proc unknown args {return handled}; missing\uD800",
                r"missing\uD800",
                profile,
            );
            assert_eq!(
                custom.selected_slot_presence(),
                SourceCommandSlotPresence::Absent
            );
            assert_eq!(
                custom.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Unknown
            );
            let qualified = point_in(
                r"external::missing\uD800",
                r"external::missing\uD800",
                profile,
            );
            assert_eq!(
                qualified.selected_slot_presence(),
                SourceCommandSlotPresence::Absent
            );
            assert_eq!(
                qualified.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Unknown
            );
        }
    }

    #[test]
    fn known_namespace_missing_head_has_closed_slot_advice() {
        for source in [
            "namespace eval ns {}; ns::missing hello",
            "namespace eval ns {proc cmd {arg} {puts $arg}}; ns::missing hello",
        ] {
            let binding = point(source, "ns::missing");
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Absent,
                "{source}: raw={:?}, snapshot={:?}",
                binding.selected_slot_presence(),
                binding.lookup_state.as_ref().map(|snapshot| (
                    snapshot.state.opaque_domain,
                    snapshot.state.source_variables.namespace_known,
                    snapshot.state.namespaces.iter().collect::<Vec<_>>(),
                )),
            );
        }
    }

    #[test]
    fn qualified_absence_advice_requires_a_retained_namespace() {
        for (source, head, expected) in [
            (
                "external::operation",
                "external::operation",
                SourceCommandSlotPresence::Unknown,
            ),
            (
                "::external::operation",
                "::external::operation",
                SourceCommandSlotPresence::Unknown,
            ),
            (
                "namespace eval external {}; external::operation",
                "external::operation",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "namespace eval external {}; ::external::operation",
                "::external::operation",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "namespace eval external {}; external::::operation",
                "external::::operation",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "namespace eval external {}; external::child::operation",
                "external::child::operation",
                SourceCommandSlotPresence::Unknown,
            ),
            (
                "namespace eval external {}; external::",
                "external::",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "::operation",
                "::operation",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "namespace eval external {}; namespace delete external; external::operation",
                "external::operation",
                SourceCommandSlotPresence::Unknown,
            ),
            (
                "namespace eval external {}; namespace delete external; namespace eval external {}; external::operation",
                "external::operation",
                SourceCommandSlotPresence::Absent,
            ),
        ] {
            let binding = point(source, head);
            assert_eq!(
                binding.selected_slot_presence(),
                SourceCommandSlotPresence::Absent,
                "{source}"
            );
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn slot_presence_does_not_follow_missing_alias_targets() {
        let binding = point("interp alias {} short {} missing; short", "short");
        assert_eq!(
            binding.selected_slot_presence(),
            SourceCommandSlotPresence::Present
        );
    }

    #[test]
    fn absence_does_not_claim_custom_unknown_failure() {
        let binding = point("proc unknown args {return handled}; missing", "missing");
        assert_eq!(
            binding.selected_slot_presence(),
            SourceCommandSlotPresence::Absent
        );
        assert!(
            !binding.targets.is_empty(),
            "fallback execution remains independent"
        );
        assert_eq!(
            binding.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Unknown
        );
    }

    #[test]
    fn math_slot_advice_uses_the_reached_post_operand_world() {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        for (source, function, expected) in [
            (
                "expr {missing()}",
                "missing",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "proc ::tcl::mathfunc::f {} {return 1}; expr {f()}",
                "f",
                SourceCommandSlotPresence::Present,
            ),
            (
                "rename ::tcl::mathfunc::sin {}; expr {sin(1)}",
                "sin",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "proc unknown args {return HANDLED}; expr {missing()}",
                "missing",
                SourceCommandSlotPresence::Unknown,
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind(&format!("{function}(")).unwrap()).unwrap();
            assert_eq!(
                bindings.diagnostic_math_function_presence_at(registry, function, offset),
                expected,
                "{source}",
            );
        }
    }

    #[test]
    fn missing_snapshot_or_head_has_no_presence_proof() {
        assert_eq!(
            SourceInvocationBinding::unknown().selected_slot_presence(),
            SourceCommandSlotPresence::Unknown
        );
    }

    #[test]
    fn absence_guards_and_presence_share_unresolved_lookup_routing() {
        let mut state = point("set checkpoint 1", "set")
            .lookup_state
            .unwrap()
            .state
            .clone();
        assert!(state.definitely_absent("missing", "::"));
        std::sync::Arc::make_mut(&mut state.source_variables).namespace_known = false;
        assert_eq!(
            state.source_slot_presence("missing", "::"),
            SourceCommandSlotPresence::Unknown
        );
        assert!(!state.definitely_absent("missing", "::"));
        assert!(state.definitely_absent("::missing", "::"));
        state.opaque_domain = true;
        assert!(!state.definitely_absent("::missing", "::"));
    }

    #[test]
    fn import_wrapper_and_closed_default_fallback_are_separate() {
        let imported = point(
            "namespace eval n {proc helper {} {}; namespace export helper}; namespace import ::n::*; helper",
            "helper",
        );
        assert_eq!(
            imported.selected_slot_presence(),
            SourceCommandSlotPresence::Present
        );
        let missing = point("missing", "missing");
        assert_eq!(
            missing.selected_slot_presence(),
            SourceCommandSlotPresence::Absent
        );
        assert_eq!(
            missing.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent
        );
        assert!(
            !missing.targets.is_empty(),
            "default autoload remains possible"
        );
    }
    #[test]
    fn native_empty_and_trailing_tail_slots_use_the_common_lookup_owner() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for (source, head) in [
                ("proc {} {} {return yes}; {}", ""),
                (
                    "namespace eval n {}; proc ::n:: {} {return yes}; ::n::",
                    "::n::",
                ),
                ("proc tail: {} {return yes}; tail:", "tail:"),
            ] {
                let binding = point_in(source, head, profile);
                assert_eq!(
                    binding.selected_slot_presence(),
                    SourceCommandSlotPresence::Present,
                    "{profile}: {source}"
                );
                assert!(
                    binding.command_reference(head).is_some(),
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn selected_native_fallback_head_retains_its_lookup_namespace() {
        let source = "namespace eval n {proc unknown args {return handled}; missing}";
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let binding = point_in(source, "missing", profile);
            assert_eq!(
                binding.selected_slot_presence(),
                SourceCommandSlotPresence::Absent
            );
            let expected = if profile == "jim" {
                SourceCommandSlotPresence::Unknown
            } else {
                SourceCommandSlotPresence::Absent
            };
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                expected,
                "{profile}"
            );
        }
    }
}
