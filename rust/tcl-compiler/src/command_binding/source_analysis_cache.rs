// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded reuse of immutable authored-source interpretations.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use super::{SourceAnalysisEntry, SourceAnalysisOptions, SourceCommandBindings};
use crate::var_resolve::VariableExecutionFrame;

const MAX_ENTRIES: usize = 16;
const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_POINTS: usize = 4096;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SourceAnalysisCacheKey {
    source: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    frame: VariableExecutionFrame,
    registry: tcl_registry::RegistrySemanticKey,
    entry: SourceAnalysisEntry,
}

impl SourceAnalysisCacheKey {
    pub(super) fn at_entry(
        source: &tcl_lexer::SourceImage,
        frame: &VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        // Physical provider currency can change independently of an entry's
        // immutable rows. Such entries require a fresh interpretation.
        if options.native_entry.is_some() || source.len() > MAX_SOURCE_BYTES {
            return None;
        }
        Some(Self {
            source: source.clone(),
            config,
            frame: frame.clone(),
            registry: registry.snapshot().semantic_key(),
            entry: SourceAnalysisEntry {
                metadata_context: options.retained_metadata_context(),
                hosted_execution_context: options.hosted_execution_context,
                execution_name_policy: options.execution_name_policy,
                logical_source_input: options.logical_source_input.cloned(),
                vendor_source_input: options.vendor_source_input.cloned(),
                compilation_scope: options.compilation_scope,
                invocation_realm: options.invocation_realm,
                native_entry: None,
                incoming_formals: options.incoming_formals.to_vec(),
                declared_commands: options.declared_commands.cloned(),
                trusted_package_loaders: options.trusted_package_loaders.to_vec(),
                trusted_source_modules: options.trusted_source_modules.to_vec(),
                unknown_entry: options.unknown_entry,
                invocation_dialect: options.invocation_dialect,
                compiled_variable_provider: options.compiled_variable_provider,
                native_compilation: options.native_compilation,
            },
        })
    }
}

struct CacheEntry {
    key: SourceAnalysisCacheKey,
    bindings: Arc<SourceCommandBindings>,
}

thread_local! {
    // Derived state is outside every semantic equality/hash. A worker retains
    // a bounded history; no lock is held while the shared driver runs.
    static CACHE: RefCell<VecDeque<CacheEntry>> = const { RefCell::new(VecDeque::new()) };
}

pub(super) fn lookup(key: &SourceAnalysisCacheKey) -> Option<SourceCommandBindings> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| &entry.key == key)?;
        let entry = cache.remove(index)?;
        let bindings = entry.bindings.as_ref().clone();
        cache.push_back(entry);
        Some(bindings)
    })
}

pub(super) fn retain(key: SourceAnalysisCacheKey, bindings: &SourceCommandBindings) {
    if bindings.points.len() > MAX_POINTS {
        return;
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|entry| entry.key != key);
        if cache.len() == MAX_ENTRIES {
            cache.pop_front();
        }
        cache.push_back(CacheEntry {
            key,
            bindings: Arc::new(bindings.clone()),
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> (
        tcl_lexer::SourceImage,
        tcl_lexer::LexerConfig,
        &'static tcl_registry::CommandRegistry,
        SourceAnalysisOptions<'static>,
    ) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let profile = registry.profile().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let options = SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        (
            tcl_lexer::SourceImage::document("set original VALUE; list $original"),
            config,
            registry,
            options,
        )
    }

    #[test]
    fn authored_source_memo_matches_complete_input_and_independent_builder_clones() {
        // Implementation contract: naming.source.authored-interpretation-memo
        // docs/design/analysis/name-resolution-proofs/authored-interpretation-memo.md
        let (source, config, registry, options) = inputs();
        let frame = VariableExecutionFrame::Global;
        let key =
            SourceAnalysisCacheKey::at_entry(&source, &frame, config, registry, options).unwrap();
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &source, &frame, config, registry, options,
        )
        .unwrap();
        let cached = CACHE.with(|cache| {
            Arc::clone(
                &cache
                    .borrow()
                    .iter()
                    .find(|entry| entry.key == key)
                    .unwrap()
                    .bindings,
            )
        });
        let mut changed = bindings;
        Arc::make_mut(&mut changed.final_state).mark_opaque_binding_mutation();
        changed.declaration_layouts.clear();
        let reused = lookup(&key).unwrap();
        assert!(Arc::ptr_eq(&cached.final_state, &reused.final_state));
        assert!(!reused.final_state.opaque_domain);
        assert!(!reused.declaration_layouts.is_empty());
        assert!(changed.final_state.opaque_domain);
        assert!(changed.declaration_layouts.is_empty());
    }

    #[test]
    fn authored_source_memo_keeps_supplied_metadata_generation_and_missing_tag() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&current),
            config,
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        let older_input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, older, config);
        let source = tcl_lexer::SourceImage::document("set item VALUE");
        let frame = VariableExecutionFrame::Global;
        let options = SourceAnalysisOptions {
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(&input),
            ),
            ..Default::default()
        };
        let key =
            SourceAnalysisCacheKey::at_entry(&source, &frame, config, current.commands(), options)
                .unwrap();
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &source,
            &frame,
            config,
            current.commands(),
            options,
        )
        .unwrap();
        retain(key.clone(), &bindings);
        assert!(lookup(&key).is_some());
        for metadata_context in [
            crate::registry_invocation::InvocationMetadataInput::SuppliedSource(Some(&older_input)),
            crate::registry_invocation::InvocationMetadataInput::SuppliedSource(None),
            crate::registry_invocation::InvocationMetadataInput::Standalone,
        ] {
            let changed = SourceAnalysisCacheKey::at_entry(
                &source,
                &frame,
                config,
                current.commands(),
                SourceAnalysisOptions {
                    metadata_context,
                    ..options
                },
            )
            .unwrap();
            assert!(lookup(&changed).is_none());
        }
    }

    #[test]
    fn authored_source_memo_bounds_history_and_refuses_oversized_sources() {
        // Implementation contract: naming.source.authored-interpretation-memo
        // docs/design/analysis/name-resolution-proofs/authored-interpretation-memo.md
        let (_, config, registry, options) = inputs();
        let frame = VariableExecutionFrame::Global;
        CACHE.with(|cache| cache.borrow_mut().clear());
        let sources = [
            "set item A",
            "set item B",
            "set item C",
            "set item D",
            "set item E",
            "set item F",
            "set item G",
            "set item H",
            "set item I",
            "set item J",
            "set item K",
            "set item L",
            "set item M",
            "set item N",
            "set item O",
            "set item P",
            "set item Q",
        ];
        let mut keys = Vec::new();
        for source in sources {
            let image = tcl_lexer::SourceImage::document(source);
            let key = SourceAnalysisCacheKey::at_entry(&image, &frame, config, registry, options)
                .unwrap();
            SourceCommandBindings::analyse_image_in_frame_with_options(
                &image, &frame, config, registry, options,
            )
            .unwrap();
            keys.push(key);
        }
        assert_eq!(CACHE.with(|cache| cache.borrow().len()), MAX_ENTRIES);
        assert!(lookup(&keys[0]).is_none());
        assert!(lookup(keys.last().unwrap()).is_some());
        let oversized = tcl_lexer::SourceImage::document(&"x".repeat(MAX_SOURCE_BYTES + 1));
        assert!(
            SourceAnalysisCacheKey::at_entry(&oversized, &frame, config, registry, options)
                .is_none()
        );
    }

    #[test]
    fn authored_source_memo_distinguishes_channel_config_entry_frame_and_registry() {
        // Implementation contract: naming.source.authored-interpretation-memo
        // docs/design/analysis/name-resolution-proofs/authored-interpretation-memo.md
        let (source, config, registry, options) = inputs();
        let frame = VariableExecutionFrame::Global;
        let key =
            SourceAnalysisCacheKey::at_entry(&source, &frame, config, registry, options).unwrap();
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &source, &frame, config, registry, options,
        )
        .unwrap();
        retain(key.clone(), &bindings);
        let other_channel = tcl_lexer::SourceImage::native(source.bytes());
        let mut other_config = config;
        other_config.strict_quoting = !other_config.strict_quoting;
        let other_frame = VariableExecutionFrame::NamespaceActivation {
            namespace: "::".to_owned(),
            identity: "other".to_owned(),
        };
        let other_entry = SourceAnalysisOptions {
            unknown_entry: true,
            ..options
        };
        let other_registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        for changed in [
            SourceAnalysisCacheKey::at_entry(&other_channel, &frame, config, registry, options)
                .unwrap(),
            SourceAnalysisCacheKey::at_entry(&source, &frame, other_config, registry, options)
                .unwrap(),
            SourceAnalysisCacheKey::at_entry(&source, &other_frame, config, registry, options)
                .unwrap(),
            SourceAnalysisCacheKey::at_entry(&source, &frame, config, registry, other_entry)
                .unwrap(),
            SourceAnalysisCacheKey::at_entry(&source, &frame, config, other_registry, options)
                .unwrap(),
        ] {
            assert!(lookup(&changed).is_none());
        }
        let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(
            registry.profile().unwrap(),
        );
        assert!(
            SourceAnalysisCacheKey::at_entry(
                &source,
                &frame,
                config,
                registry,
                SourceAnalysisOptions {
                    native_entry: Some(&native),
                    ..options
                }
            )
            .is_none()
        );
    }
}
