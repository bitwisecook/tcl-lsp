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
    purpose: tcl_lexer::SourceSegmentationPurpose,
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
        purpose: tcl_lexer::SourceSegmentationPurpose,
    ) -> Option<Self> {
        // Physical provider currency can change independently of an entry's
        // immutable rows. Such entries require a fresh interpretation.
        if options.native_entry.is_some() || source.len() > MAX_SOURCE_BYTES {
            return None;
        }
        Some(Self {
            source: source.clone(),
            config,
            purpose,
            frame: frame.clone(),
            registry: registry.snapshot().semantic_key(),
            entry: SourceAnalysisEntry {
                source_point_entry: options.source_point_entry.cloned(),
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

// Only genuine source-point results use this lease. Its snapshot never owns
// a lease, and the TLS history retains only Weak ownership of this object.
pub(super) struct SourcePointCacheLease {
    key: SourceAnalysisCacheKey,
    snapshot: SourceCommandBindings,
}
impl std::fmt::Debug for SourcePointCacheLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourcePointCacheLease")
    }
}

// Nonauthoritative history identity: the point token has exactly the existing
// issued-key Eq, but cannot expose state, serve a hit or reissue an old key.
struct SourcePointHistoryKey {
    source: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    purpose: tcl_lexer::SourceSegmentationPurpose,
    frame: VariableExecutionFrame,
    registry: tcl_registry::RegistrySemanticKey,
    point: Option<super::OriginalSourcePointOrigin>,
    entry: SourceAnalysisEntry,
}
impl SourcePointHistoryKey {
    fn new(key: &SourceAnalysisCacheKey) -> Self {
        Self {
            source: key.source.clone(),
            config: key.config,
            purpose: key.purpose,
            frame: key.frame.clone(),
            registry: key.registry.clone(),
            point: key
                .entry
                .source_point_entry
                .as_ref()
                .map(super::OriginalSourcePointEntry::source_origin),
            entry: history_entry(&key.entry),
        }
    }
    fn matches(&self, key: &SourceAnalysisCacheKey) -> bool {
        self.source == key.source
            && self.config == key.config
            && self.purpose == key.purpose
            && self.frame == key.frame
            && self.registry == key.registry
            && self.entry_matches(&key.entry)
    }
    fn entry_matches(&self, entry: &SourceAnalysisEntry) -> bool {
        let SourceAnalysisEntry {
            source_point_entry,
            metadata_context,
            hosted_execution_context,
            execution_name_policy,
            logical_source_input,
            vendor_source_input,
            compilation_scope,
            invocation_realm,
            native_entry,
            incoming_formals,
            declared_commands,
            trusted_package_loaders,
            trusted_source_modules,
            unknown_entry,
            invocation_dialect,
            compiled_variable_provider,
            native_compilation,
        } = entry;
        let same_point = match (&self.point, source_point_entry) {
            (None, None) => true,
            (Some(origin), Some(point)) => point.matches_origin_token(origin),
            _ => false,
        };
        same_point
            && self.entry.source_point_entry.is_none()
            && &self.entry.metadata_context == metadata_context
            && &self.entry.hosted_execution_context == hosted_execution_context
            && &self.entry.execution_name_policy == execution_name_policy
            && &self.entry.logical_source_input == logical_source_input
            && &self.entry.vendor_source_input == vendor_source_input
            && &self.entry.compilation_scope == compilation_scope
            && &self.entry.invocation_realm == invocation_realm
            && &self.entry.native_entry == native_entry
            && &self.entry.incoming_formals == incoming_formals
            && &self.entry.declared_commands == declared_commands
            && &self.entry.trusted_package_loaders == trusted_package_loaders
            && &self.entry.trusted_source_modules == trusted_source_modules
            && &self.entry.unknown_entry == unknown_entry
            && &self.entry.invocation_dialect == invocation_dialect
            && &self.entry.compiled_variable_provider == compiled_variable_provider
            && &self.entry.native_compilation == native_compilation
    }
}

fn history_entry(entry: &SourceAnalysisEntry) -> SourceAnalysisEntry {
    let SourceAnalysisEntry {
        source_point_entry: _,
        metadata_context,
        hosted_execution_context,
        execution_name_policy,
        logical_source_input,
        vendor_source_input,
        compilation_scope,
        invocation_realm,
        native_entry,
        incoming_formals,
        declared_commands,
        trusted_package_loaders,
        trusted_source_modules,
        unknown_entry,
        invocation_dialect,
        compiled_variable_provider,
        native_compilation,
    } = entry;
    SourceAnalysisEntry {
        source_point_entry: None,
        metadata_context: metadata_context.clone(),
        hosted_execution_context: *hosted_execution_context,
        execution_name_policy: *execution_name_policy,
        logical_source_input: logical_source_input.clone(),
        vendor_source_input: vendor_source_input.clone(),
        compilation_scope: *compilation_scope,
        invocation_realm: *invocation_realm,
        native_entry: native_entry.clone(),
        incoming_formals: incoming_formals.clone(),
        declared_commands: declared_commands.clone(),
        trusted_package_loaders: trusted_package_loaders.clone(),
        trusted_source_modules: trusted_source_modules.clone(),
        unknown_entry: *unknown_entry,
        invocation_dialect: *invocation_dialect,
        compiled_variable_provider: *compiled_variable_provider,
        native_compilation: *native_compilation,
    }
}

enum CacheEntry {
    Root {
        key: SourceAnalysisCacheKey,
        bindings: Arc<SourceCommandBindings>,
    },
    Point {
        history: SourcePointHistoryKey,
        lease: std::sync::Weak<SourcePointCacheLease>,
        // Intentional bounded keepalive of source identity only, never state.
        _activation_origin: Option<Arc<super::SourceOriginId>>,
    },
}
impl CacheEntry {
    fn matches(&self, key: &SourceAnalysisCacheKey) -> bool {
        match self {
            Self::Root { key: retained, .. } => retained == key,
            Self::Point { history, .. } => history.matches(key),
        }
    }
    fn lookup(&self, key: &SourceAnalysisCacheKey) -> Option<SourceCommandBindings> {
        match self {
            Self::Root {
                key: retained,
                bindings,
            } => (retained == key).then(|| bindings.as_ref().clone()),
            Self::Point { lease, .. } => {
                let lease = lease.upgrade()?;
                // History is never sufficient: authenticate the complete original
                // key against the still-live immutable producer before any facts.
                if &lease.key != key {
                    return None;
                }
                let mut bindings = lease.snapshot.clone();
                bindings.source_point_cache_lease = Some(lease);
                Some(bindings)
            }
        }
    }
    #[cfg(test)]
    fn root_bindings(&self) -> &Arc<SourceCommandBindings> {
        let Self::Root { bindings, .. } = self else {
            panic!("ordinary root history must retain its original strong snapshot");
        };
        bindings
    }
}

thread_local! {
    // Root reuse is unchanged. Point history cannot retain state after the
    // last real bindings/report owner retires; global Eq/Hash stay unchanged.
    static CACHE: RefCell<VecDeque<CacheEntry>> = const { RefCell::new(VecDeque::new()) };
}

pub(super) fn lookup(key: &SourceAnalysisCacheKey) -> Option<SourceCommandBindings> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| entry.matches(key))?;
        let entry = cache.remove(index)?;
        let bindings = entry.lookup(key);
        cache.push_back(entry);
        bindings
    })
}

pub(super) fn retain(key: SourceAnalysisCacheKey, bindings: &mut SourceCommandBindings) {
    if bindings.points.len() > MAX_POINTS {
        return;
    }
    let mut snapshot = bindings.clone();
    snapshot.source_point_cache_lease = None;
    let entry = if key.entry.source_point_entry.is_some() {
        let history = SourcePointHistoryKey::new(&key);
        let origin = snapshot.root_origin.clone();
        let lease = Arc::new(SourcePointCacheLease { key, snapshot });
        let entry = CacheEntry::Point {
            history,
            lease: Arc::downgrade(&lease),
            _activation_origin: origin,
        };
        bindings.source_point_cache_lease = Some(lease);
        entry
    } else {
        bindings.source_point_cache_lease = None;
        CacheEntry::Root {
            key,
            bindings: Arc::new(snapshot),
        }
    };
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        {
            let key = match &entry {
                CacheEntry::Root { key, .. } => key,
                CacheEntry::Point { .. } => {
                    &bindings
                        .source_point_cache_lease
                        .as_ref()
                        .expect("caller owns newly issued lease")
                        .key
                }
            };
            cache.retain(|old| !old.matches(key));
        }
        if cache.len() == MAX_ENTRIES {
            cache.pop_front();
        }
        cache.push_back(entry);
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

        let (source, config, registry, options) = inputs();
        let frame = VariableExecutionFrame::Global;
        let key = SourceAnalysisCacheKey::at_entry(
            &source,
            &frame,
            config,
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .unwrap();
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &source, &frame, config, registry, options,
        )
        .unwrap();
        let cached = CACHE.with(|cache| {
            Arc::clone(
                cache
                    .borrow()
                    .iter()
                    .find(|entry| entry.matches(&key))
                    .unwrap()
                    .root_bindings(),
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
        let key = SourceAnalysisCacheKey::at_entry(
            &source,
            &frame,
            config,
            current.commands(),
            options,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .unwrap();
        let mut bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &source,
            &frame,
            config,
            current.commands(),
            options,
        )
        .unwrap();
        retain(key.clone(), &mut bindings);
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
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .unwrap();
            assert!(lookup(&changed).is_none());
        }
    }

    #[test]
    fn authored_source_memo_bounds_history_and_refuses_oversized_sources() {

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
            let key = SourceAnalysisCacheKey::at_entry(
                &image,
                &frame,
                config,
                registry,
                options,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
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
            SourceAnalysisCacheKey::at_entry(
                &oversized,
                &frame,
                config,
                registry,
                options,
                tcl_lexer::SourceSegmentationPurpose::WholeInput
            )
            .is_none()
        );
    }

    #[test]
    fn authored_source_memo_distinguishes_channel_config_entry_frame_and_registry() {

        let (source, config, registry, options) = inputs();
        let frame = VariableExecutionFrame::Global;
        let key = SourceAnalysisCacheKey::at_entry(
            &source,
            &frame,
            config,
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .unwrap();
        let mut bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &source, &frame, config, registry, options,
        )
        .unwrap();
        retain(key.clone(), &mut bindings);
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
            SourceAnalysisCacheKey::at_entry(
                &other_channel,
                &frame,
                config,
                registry,
                options,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .unwrap(),
            SourceAnalysisCacheKey::at_entry(
                &source,
                &frame,
                other_config,
                registry,
                options,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .unwrap(),
            SourceAnalysisCacheKey::at_entry(
                &source,
                &other_frame,
                config,
                registry,
                options,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .unwrap(),
            SourceAnalysisCacheKey::at_entry(
                &source,
                &frame,
                config,
                registry,
                other_entry,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .unwrap(),
            SourceAnalysisCacheKey::at_entry(
                &source,
                &frame,
                config,
                other_registry,
                options,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
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
                },
                tcl_lexer::SourceSegmentationPurpose::WholeInput
            )
            .is_none()
        );
    }

    fn source_point_for_cache() -> super::super::OriginalSourcePointEntry {
        let source = "source child.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap()
        .source_point_entry(&parent)
        .unwrap()
    }

    fn source_point_bindings(
        point: &super::super::OriginalSourcePointEntry,
        source: &str,
    ) -> (SourceAnalysisCacheKey, SourceCommandBindings) {
        let input = point.input();
        let registry = input.borrowed_context_registry().commands();
        let entry = point.source_analysis_entry(registry, input).unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let key = SourceAnalysisCacheKey::at_entry(
            &image,
            point.frame(),
            input.lexer_config(),
            registry,
            entry.options(),
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .unwrap();
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            point.frame(),
            input.lexer_config(),
            registry,
            entry.options(),
        )
        .unwrap();
        (key, bindings)
    }

    #[test]
    fn source_point_cache_live_lease_keeps_snapshot_acyclic_and_shares_while_live() {
        CACHE.with(|cache| cache.borrow_mut().clear());
        let point = source_point_for_cache();
        let (key, first) = source_point_bindings(&point, "set child VALUE");
        let original = first.source_point_cache_lease.as_ref().unwrap();
        assert!(original.snapshot.source_point_cache_lease.is_none());
        let retired = Arc::downgrade(original);
        let second = lookup(&key).unwrap();
        assert!(Arc::ptr_eq(
            first.source_point_cache_lease.as_ref().unwrap(),
            second.source_point_cache_lease.as_ref().unwrap(),
        ));
        assert_eq!(first, second);
        drop(first);
        assert!(retired.upgrade().is_some());
        assert!(lookup(&key).is_some());
        drop(second);
        assert!(retired.upgrade().is_none());
        assert!(lookup(&key).is_none());
    }

    #[test]
    fn source_point_cache_retirement_drops_parent_payload_but_retains_only_history_origin() {
        CACHE.with(|cache| cache.borrow_mut().clear());
        let point = source_point_for_cache();
        let probe = point.cache_retirement_probe();
        let (key, bindings) = source_point_bindings(&point, "set child RETIRE");
        let kind = Arc::downgrade(&bindings.root_origin.as_ref().unwrap().kind);
        let lease = Arc::downgrade(bindings.source_point_cache_lease.as_ref().unwrap());
        assert!(probe.is_live());
        drop(bindings);
        assert!(lease.upgrade().is_none());
        assert!(lookup(&key).is_none());
        drop(key);
        drop(point);
        assert!(
            !probe.is_live(),
            "history must not retain a parent point payload"
        );
        assert!(
            kind.upgrade().is_some(),
            "history retains only source identity"
        );
        CACHE.with(|cache| cache.borrow_mut().clear());
        assert!(kind.upgrade().is_none());
    }

    #[test]
    fn source_point_cache_rejects_foreign_and_changed_context_keys() {
        CACHE.with(|cache| cache.borrow_mut().clear());
        let point = source_point_for_cache();
        let (key, bindings) = source_point_bindings(&point, "set child ORIGINAL");
        assert!(lookup(&key).is_some());
        let mut changed = key.clone();
        changed.entry.unknown_entry = !changed.entry.unknown_entry;
        assert!(lookup(&changed).is_none());
        changed = key.clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        assert!(lookup(&changed).is_none());
        let foreign = source_point_for_cache();
        changed = key.clone();
        changed.entry.source_point_entry = Some(foreign);
        assert!(lookup(&changed).is_none());
        assert!(lookup(&key).is_some());
        drop(bindings);
        assert!(lookup(&key).is_none());
        CACHE.with(|cache| cache.borrow_mut().clear());
        assert!(lookup(&key).is_none(), "empty history cannot serve facts");
    }

    fn source_point_report_lease(
        report: &crate::analyser::AuthenticatedAnalysisReport,
    ) -> &Arc<SourcePointCacheLease> {
        report
            .analysis()
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref()
            .source_point_cache_lease
            .as_ref()
            .expect("actual full report must pin its genuine source-point lease")
    }

    #[test]
    fn source_point_cache_full_report_cold_warm_and_mixed_stay_equal() {
        for source in [
            "set child VALUE; list $child",
            "proc p {arg} {if {$arg} {return $arg}; return NONE}; set child [p 1]; missing_cache_command",
        ] {
            CACHE.with(|cache| cache.borrow_mut().clear());
            let point = source_point_for_cache();
            let first = crate::analyser::Analyser::new()
                .source_point_report_for_cache_test(source, "tcl8.6", &point);
            let warm = crate::analyser::Analyser::new()
                .source_point_report_for_cache_test(source, "tcl8.6", &point);
            assert!(Arc::ptr_eq(
                source_point_report_lease(&first),
                source_point_report_lease(&warm),
            ));
            assert_eq!(
                first.compare(&warm),
                crate::analyser::AnalysisReportComparison::Equal
            );
            CACHE.with(|cache| cache.borrow_mut().clear());
            let cold = crate::analyser::Analyser::new()
                .source_point_report_for_cache_test(source, "tcl8.6", &point);
            assert!(!Arc::ptr_eq(
                source_point_report_lease(&first),
                source_point_report_lease(&cold),
            ));
            assert_eq!(
                first.compare(&cold),
                crate::analyser::AnalysisReportComparison::Equal
            );
        }
    }
}
