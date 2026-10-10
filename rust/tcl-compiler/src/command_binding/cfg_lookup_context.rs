// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retain original CFG lookup holders independently of reporting procedure names.

use super::{
    Binding, BindingInvocationMetadata, BindingKind, BlockId, CfgFunction, CommandRegistry,
    HashMap, MayBinding, ModuleCommandBindings, SourceNamespaceKey, nqn, source_binding,
    source_binding_from_original_input,
};
use crate::ir::{CommandBindingSites, CommandTokens, ExecutionNamespace, Statement};
use crate::signature_scan::scope::SignatureSourceNameInput;

/// Whole original invocation grammar and its separately retained lookup holder.
/// This supplies no variable activation, procedure publication or Native frame.
pub(super) struct CfgPointContext {
    pub(super) namespace: ExecutionNamespace,
    pub(super) config: Option<tcl_lexer::LexerConfig>,
    head: Option<SignatureSourceNameInput>,
    evaluated_head: Option<String>,
    logical_source_head: bool,
}

pub(super) struct CfgLookupContexts {
    points: HashMap<(BlockId, usize), CfgPointContext>,
    body: Option<CfgPointContext>,
    standalone: Option<CfgPointContext>,
}

impl CfgLookupContexts {
    pub(super) fn new(
        cfg: &CfgFunction,
        registry: &CommandRegistry,
        seed: &ModuleCommandBindings,
    ) -> Self {
        let standalone = standalone_namespace(cfg).map(|namespace| CfgPointContext {
            namespace,
            config: None,
            head: None,
            evaluated_head: None,
            logical_source_head: false,
        });
        let mut points = HashMap::new();
        let mut body_namespace_valid = false;
        for (block_id, block) in &cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                let Some(tokens) = CommandBindingSites::unanimous_statement_source_tokens(
                    &cfg.command_binding_sites,
                    statement,
                ) else {
                    continue;
                };
                let Some(point) = original_point(cfg, registry, seed, tokens) else {
                    continue;
                };
                if let Some(namespace) = cfg.namespace_context.as_deref()
                    && let Some(snapshot) = tokens
                        .source_binding
                        .as_ref()
                        .and_then(|binding| binding.lookup_state.as_ref())
                {
                    body_namespace_valid |= retained_namespace(&snapshot.state, namespace);
                }
                points.insert((*block_id, index), point);
            }
        }
        let body = cfg
            .namespace_context
            .as_deref()
            .filter(|namespace| {
                cfg.executed_source.is_some()
                    && (body_namespace_valid || retained_namespace(seed, namespace))
                    && cfg.metadata_context.metadata_context(registry).is_some()
            })
            .map(|namespace| CfgPointContext {
                namespace: ExecutionNamespace::SourceContext(namespace.clone()),
                config: None,
                head: None,
                evaluated_head: None,
                logical_source_head: false,
            });
        Self {
            points,
            body,
            standalone,
        }
    }

    pub(super) fn point(&self, block: BlockId, index: usize) -> Option<&CfgPointContext> {
        self.points
            .get(&(block, index))
            .or(self.standalone.as_ref())
    }

    pub(super) fn metadata<'a>(
        cfg: &'a CfgFunction,
        registry: &CommandRegistry,
    ) -> BindingInvocationMetadata<'a> {
        if matches!(
            &cfg.metadata_context,
            crate::cfg_builder::CfgMetadataContext::Standalone
        ) {
            BindingInvocationMetadata::Standalone
        } else {
            BindingInvocationMetadata::Supplied(
                cfg.metadata_context.metadata_context(registry).flatten(),
            )
        }
    }

    pub(super) fn binding_at(
        &self,
        cfg: &CfgFunction,
        state: &ModuleCommandBindings,
        block: BlockId,
        index: usize,
        command_name: &str,
    ) -> Binding {
        let point = if cfg
            .blocks
            .get(&block)
            .is_some_and(|block| index >= block.statements.len())
        {
            self.body.as_ref().or(self.standalone.as_ref())
        } else {
            self.point(block, index)
        };
        let Some(point) = point else {
            return Binding::of(BindingKind::Unknown);
        };
        let Some(namespace) = point.namespace.for_head_context(command_name) else {
            return Binding::of(BindingKind::Unknown);
        };
        let original_head =
            (point.evaluated_head.as_deref() == Some(command_name)).then_some(point.head.as_ref());
        let (proof, slots) = match original_head {
            Some(Some(input)) => {
                let Some(proof) =
                    source_binding_from_original_input(state, input, namespace.as_ref())
                else {
                    return Binding::of(BindingKind::Unknown);
                };
                let Some(paths) = state.original_command_paths_for_input(namespace.as_ref(), input)
                else {
                    return Binding::of(BindingKind::Unknown);
                };
                let Ok(slots) = state.original_source_keys_for_paths(paths) else {
                    return Binding::of(BindingKind::Unknown);
                };
                (proof, slots)
            }
            // A positive retained Logical producer has no Native naming
            // input. Its own source model remains separate from Native refusal.
            Some(None) if point.logical_source_head => (
                source_binding(state, command_name, namespace.as_ref()),
                state.source_keys(command_name, namespace.as_ref()),
            ),
            Some(None) => return Binding::of(BindingKind::Unknown),
            None => {
                // An independently requested descriptor key is not a written
                // Native name producer. Its ASCII metadata projection still
                // uses the retained exact holder and shared purpose owner.
                if !matches!(&point.namespace, ExecutionNamespace::Exact(_))
                    && !matches!(namespace.as_ref(), SourceNamespaceKey::Authored(_))
                    && (!command_name.is_ascii() || command_name.as_bytes().contains(&0))
                {
                    return Binding::of(BindingKind::Unknown);
                }
                (
                    source_binding(state, command_name, namespace.as_ref()),
                    state.source_keys(command_name, namespace.as_ref()),
                )
            }
        };
        project_binding(state, &proof, &slots)
    }
}

fn original_point(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    seed: &ModuleCommandBindings,
    tokens: &CommandTokens,
) -> Option<CfgPointContext> {
    let binding = tokens.source_binding.as_ref()?;
    let snapshot = binding.lookup_state.as_ref()?;
    (snapshot.state.baseline == seed.baseline
        && snapshot.state.baseline.registry_snapshot.as_ref()
            == Some(&registry.snapshot().semantic_key())
        && tokens.words_align_with_argv_text()
        && retained_namespace(&snapshot.state, &binding.lookup_namespace_key))
    .then_some(())?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let metadata = cfg.metadata_context.metadata_context(registry)?;
    let supplied = metadata
        .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input);
    if let Some(input) = supplied {
        (input.lexer_config().nested().normalized() == config.nested().normalized())
            .then_some(())?;
    }
    let logical_source_head = supplied.is_some_and(|input| {
        input.has_logical_source_name_context()
            && snapshot
                .state
                .baseline
                .logical_source_input
                .as_ref()
                .is_some_and(|original| {
                    std::ptr::eq(
                        input.borrowed_context_registry(),
                        original.borrowed_context_registry(),
                    ) && input.analyser_profile().cache_key()
                        == original.analyser_profile().cache_key()
                        && input.unit_profile().cache_key() == original.unit_profile().cache_key()
                        && original.lexer_config().nested().normalized()
                            == config.nested().normalized()
                })
    });
    Some(CfgPointContext {
        namespace: ExecutionNamespace::SourceContext(binding.lookup_namespace_key.clone()),
        config: Some(config),
        head: binding.original_head_name_input(tokens),
        evaluated_head: binding.evaluated_command_word().map(str::to_owned),
        logical_source_head,
    })
}

fn retained_namespace(state: &ModuleCommandBindings, namespace: &SourceNamespaceKey) -> bool {
    match namespace {
        SourceNamespaceKey::Native(context) => {
            state
                .baseline
                .native_entry
                .as_ref()
                .and_then(|entry| super::runtime_entry::native_namespace_key(entry, context.token))
                .as_ref()
                == Some(namespace)
        }
        SourceNamespaceKey::Allocated { .. } => state.namespaces.contains(namespace),
        SourceNamespaceKey::Authored(_) => {
            state.baseline.native_entry.is_none()
                && !state.baseline.unknown_entry
                && (state.baseline.logical_source_input.is_some()
                    || state.baseline.execution_name_policy.is_some())
        }
    }
}

fn standalone_namespace(cfg: &CfgFunction) -> Option<ExecutionNamespace> {
    (matches!(
        &cfg.metadata_context,
        crate::cfg_builder::CfgMetadataContext::Standalone
    ) && cfg.namespace_context.is_none()
        && cfg.executed_source.is_none()
        && cfg.statement_sources.values().all(Option::is_none)
        && cfg
            .command_binding_sites
            .iter()
            .all(|site| site.source_tokens.is_none())
        && cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter_map(Statement::tokens)
            .all(|tokens| tokens.source_binding.is_none()))
    .then(|| {
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(&cfg.name);
        ExecutionNamespace::Exact(if holder.is_empty() { "::" } else { holder }.to_owned())
    })
}

fn project_binding(
    state: &ModuleCommandBindings,
    proof: &super::SourceInvocationBinding,
    slots: &[super::SourceCommandKey],
) -> Binding {
    let alternatives = slots
        .iter()
        .filter_map(|slot| state.bindings.get(slot))
        .flatten()
        .collect::<Vec<_>>();
    if let [MayBinding::Target(target)] = alternatives.as_slice()
        && !target.terminal
    {
        return Binding {
            kind: BindingKind::Alias,
            target: Some(nqn(&target.command)),
        };
    }
    if !slots.is_empty()
        && slots.iter().all(|slot| {
            state.binding_alternatives(slot) == super::BTreeSet::from([MayBinding::Missing])
        })
    {
        return Binding::of(BindingKind::Opaque);
    }
    if proof.unknown {
        return Binding::of(BindingKind::Unknown);
    }
    let Some(target) = proof.proved_target() else {
        return Binding::of(if proof.targets.is_empty() {
            BindingKind::Opaque
        } else {
            BindingKind::Unknown
        });
    };
    Binding {
        kind: target.kind,
        target: (!target.registry_backed).then(|| target.command.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisEntry, analyse_command_binding};
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;

    fn native_fixture(
        source: &str,
    ) -> (
        tcl_vm::Vm,
        CompilationUnit,
        Arc<tcl_registry::model::ContextRegistry>,
    ) {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = SourceAnalysisEntry {
            native_entry: Some(Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_context_registry(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            Arc::clone(&context),
        );
        (owner, unit, context)
    }

    #[test]
    fn original_cfg_lookup_uses_per_point_native_namespace_not_reported_function_name() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let (_owner, unit, context) = native_fixture("set result VALUE");
        let mut cfg = unit.top_level.cfg.clone();
        let original = CommandBindingSites::unanimous_statement_source_tokens(
            &cfg.command_binding_sites,
            &cfg.blocks[&cfg.entry].statements[0],
        )
        .unwrap();
        assert!(matches!(
            &original
                .source_binding
                .as_ref()
                .unwrap()
                .lookup_namespace_key,
            SourceNamespaceKey::Native(_)
        ));
        assert!(matches!(
            &cfg.metadata_context,
            crate::cfg_builder::CfgMetadataContext::SuppliedSource(_)
        ));
        cfg.name = "::unrelated::display_name".into();
        cfg.namespace_context = Some(Box::new(SourceNamespaceKey::authored("::unrelated")));
        let flow = analyse_command_binding(&cfg, context.commands(), &[]);
        assert!(flow.is_original_builtin_at(cfg.entry, 0, "set"));
        assert!(!flow.has_wildcard());
        assert!(flow.rebound_names().is_empty());

        let mut authored = cfg.clone();
        authored.command_binding_sites[0]
            .source_tokens
            .as_mut()
            .unwrap()
            .source_binding
            .as_mut()
            .unwrap()
            .lookup_namespace_key = SourceNamespaceKey::authored("::");
        assert_eq!(
            analyse_command_binding(&authored, context.commands(), &[])
                .binding_at(authored.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );

        let (foreign_owner, foreign_entry) =
            crate::environment_ingress::captured_native_entry_with_owner(
                tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            );
        let foreign_namespace = SourceNamespaceKey::from_native_entry(&foreign_entry).unwrap();
        let mut foreign = cfg.clone();
        let original_namespace = &foreign.command_binding_sites[0]
            .source_tokens
            .as_ref()
            .unwrap()
            .source_binding
            .as_ref()
            .unwrap()
            .lookup_namespace_key;
        assert_eq!(
            original_namespace.exact_native_path(),
            foreign_namespace.exact_native_path()
        );
        assert_ne!(original_namespace, &foreign_namespace);
        foreign.command_binding_sites[0]
            .source_tokens
            .as_mut()
            .unwrap()
            .source_binding
            .as_mut()
            .unwrap()
            .lookup_namespace_key = foreign_namespace;
        assert_eq!(
            analyse_command_binding(&foreign, context.commands(), &[])
                .binding_at(foreign.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );
        drop(foreign_owner);
    }

    #[test]
    fn original_cfg_lookup_refuses_missing_foreign_and_changed_source_metadata() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let (_owner, unit, context) = native_fixture("set result VALUE");
        let cfg = &unit.top_level.cfg;
        assert!(
            analyse_command_binding(cfg, context.commands(), &[])
                .is_original_builtin_at(cfg.entry, 0, "set")
        );
        let mut missing = cfg.clone();
        missing.metadata_context = crate::cfg_builder::CfgMetadataContext::Unavailable;
        assert_eq!(
            analyse_command_binding(&missing, context.commands(), &[])
                .binding_at(missing.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );
        assert!(analyse_command_binding(&missing, context.commands(), &[]).has_wildcard());

        let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let stale = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let mut changed = cfg.clone();
        changed.metadata_context =
            crate::cfg_builder::CfgMetadataContext::SuppliedSource(Box::new(stale));
        assert_eq!(
            analyse_command_binding(&changed, context.commands(), &[])
                .binding_at(changed.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );

        let other_context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            other_context,
            input.lexer_config(),
        );
        let mut foreign = cfg.clone();
        foreign.metadata_context =
            crate::cfg_builder::CfgMetadataContext::SuppliedSource(Box::new(foreign_input));
        assert_eq!(
            analyse_command_binding(&foreign, context.commands(), &[])
                .binding_at(foreign.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );

        let mut erased = cfg.clone();
        erased.command_binding_sites.clear();
        assert!(erased.namespace_context.is_some());
        assert!(erased.executed_source.is_some());
        assert_eq!(
            analyse_command_binding(&erased, context.commands(), &[])
                .binding_at(erased.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );
    }

    #[test]
    fn original_cfg_logical_lookup_keeps_its_positive_owner_separate_from_native_names() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
        let unit = CompilationUnit::build_with_analysis_input(
            "set result VALUE",
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            &input,
        );
        let mut cfg = unit.top_level.cfg.clone();
        let tokens = CommandBindingSites::unanimous_statement_source_tokens(
            &cfg.command_binding_sites,
            &cfg.blocks[&cfg.entry].statements[0],
        )
        .unwrap();
        let original = tokens.source_binding.as_ref().unwrap();
        assert!(original.original_head_name_input(tokens).is_none());
        assert!(
            original
                .lookup_state
                .as_ref()
                .unwrap()
                .state
                .baseline
                .logical_source_input
                .as_ref()
                .unwrap()
                .has_logical_source_name_context()
        );
        cfg.name = "::unrelated::display_name".into();
        assert!(
            analyse_command_binding(&cfg, context.commands(), &[])
                .is_original_builtin_at(cfg.entry, 0, "set")
        );

        let older_context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        let older =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, older_context, config);
        cfg.metadata_context =
            crate::cfg_builder::CfgMetadataContext::SuppliedSource(Box::new(older));
        assert_eq!(
            analyse_command_binding(&cfg, context.commands(), &[])
                .binding_at(cfg.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );
        cfg.metadata_context = crate::cfg_builder::CfgMetadataContext::Unavailable;
        assert_eq!(
            analyse_command_binding(&cfg, context.commands(), &[])
                .binding_at(cfg.entry, 0, "set")
                .kind,
            BindingKind::Unknown
        );
    }

    #[test]
    fn original_cfg_binding_replay_keeps_native_move_and_delete_horizons() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let (_owner, unit, context) = native_fixture(
            "rename set saved_set\nsaved_set result VALUE\nrename saved_set {}\nset other VALUE",
        );
        let cfg = &unit.top_level.cfg;
        let original_index = |head: &str, occurrence: usize| {
            cfg.blocks[&cfg.entry]
                .statements
                .iter()
                .enumerate()
                .filter(|(_, statement)| {
                    CommandBindingSites::unanimous_statement_source_tokens(
                        &cfg.command_binding_sites,
                        statement,
                    )
                    .and_then(|tokens| tokens.source_binding.as_ref())
                    .and_then(super::super::SourceInvocationBinding::evaluated_command_word)
                        == Some(head)
                })
                .nth(occurrence)
                .unwrap()
                .0
        };
        let renamed = original_index("saved_set", 0);
        let after_delete = original_index("set", 0);
        let flow = analyse_command_binding(cfg, context.commands(), &[]);
        assert_eq!(
            flow.binding_at(cfg.entry, renamed, "set").kind,
            BindingKind::Opaque
        );
        assert_eq!(
            flow.binding_at(cfg.entry, renamed, "saved_set").kind,
            BindingKind::Builtin
        );
        assert_eq!(
            flow.binding_at(cfg.entry, after_delete, "saved_set").kind,
            BindingKind::Opaque
        );
        assert!(flow.rebound_names().contains("::set"));
        assert!(flow.rebound_names().contains("::saved_set"));
    }
}
