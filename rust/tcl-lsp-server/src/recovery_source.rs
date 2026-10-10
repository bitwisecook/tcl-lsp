// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional parser recognition for incomplete documents.

use super::*;
use provider_source::{ProviderCaptureInputs, ProviderSourceUnavailable};
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_lsp_core::package_resolver::{
    PackagePrefer, PackageRequirementAdvice as Requirement, PackageRequirementAdviceKey as Key,
};

pub(super) struct RecoveryWidenCtx<'a> {
    pub cache: &'a Arc<Mutex<RecoveryNameCache>>,
    pub registry: &'a CommandRegistry,
    pub workspace_index: &'a Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    pub package_resolver: &'a Arc<RwLock<PackageResolver>>,
    pub provider_capture: &'a ProviderCaptureInputs<'a>,
    pub prefer: PackagePrefer,
}

#[derive(Clone, PartialEq, Eq)]
struct SharedRecoveryKey {
    index_generation: u64,
    resolver_revision: u64,
    base: Vec<String>,
}

/// Both a successful provider and a typed refusal belong to the captured
/// source/configuration generation. A caller's input cannot replace either.
type RecoveryProviderSource =
    Result<(Arc<str>, Arc<ResolvedAnalysisInput>), ProviderSourceUnavailable>;

#[derive(Clone, PartialEq, Eq)]
struct RecoveryProviderKey {
    path: PathBuf,
    target: Option<tcl_dialect::TclVersion>,
    source: RecoveryProviderSource,
}

#[derive(Clone, PartialEq, Eq)]
struct RecoveryNameKey {
    shared: SharedRecoveryKey,
    input: ResolvedAnalysisInput,
    requires: Vec<Requirement>,
    providers: Vec<RecoveryProviderKey>,
    commands: Vec<String>,
    incomplete: bool,
}

#[derive(Clone)]
pub(super) struct RecoveryNames {
    pub names: Arc<HashSet<String>>,
    /// Recognition candidates are conditional; this is never a complete
    /// command universe or an executable binding receipt.
    pub incomplete: bool,
}

/// Source-independent workspace labels share an allocation across keystrokes.
/// The package layer additionally retains complete caller and provider inputs,
/// source text, requirements and preference, including unavailable providers.
#[derive(Default)]
pub(super) struct RecoveryNameCache {
    shared: Option<(SharedRecoveryKey, Arc<HashSet<String>>)>,
    widened: Option<(RecoveryNameKey, RecoveryNames)>,
}

pub(super) async fn compute_recovery_analysis(
    client: &Client,
    ctx: &SalsaAnalysisCtx<'_>,
    disabled: &HashSet<String>,
    extra: &HashSet<String>,
    mode: NonAsciiMode,
    recovery: &RecoveryWidenCtx<'_>,
) -> ControlFlow<bool, Arc<AnalysisResult>> {
    let (packs, resource, supplied, factories, subclasses) = {
        let db = ctx.db.lock().await;
        (
            ctx.config.spec_pack_key(&*db),
            ResourceAnalyserInputs::from_db_config(ctx.config, &*db),
            ctx.file
                .map(|file| tcl_lsp_db::document_analysis_input(&*db, file, ctx.config)),
            ctx.file
                .and_then(|file| file.workspace_class_factories(&*db).clone()),
            ctx.file
                .and_then(|file| file.workspace_subclass_methods(&*db).clone()),
        )
    };
    let analyser = Backend::configured_analyser(
        disabled.clone(),
        mode,
        extra.clone(),
        packs,
        resource.clone(),
    )
    .with_workspace_class_factories(factories.clone())
    .with_workspace_subclass_methods(subclasses.clone())
    .with_file_path(
        ctx.uri
            .to_file_path()
            .map(|path| path.display().to_string()),
    );
    let text = ctx.text.to_owned();
    let dialect = ctx.dialect;
    let preview = match crate::rt::spawn_blocking(move || {
        with_pack_hooks(|| {
            let input = supplied
                .unwrap_or_else(|| analyser.prepare_analysis_input(dialect.name).map(Arc::new));
            match input {
                Ok(input) => analyser
                    .with_resolved_input(input.as_ref().clone())
                    .structure_only()
                    .analyse(&text, dialect.name),
                Err(miss) => {
                    let mut refused = AnalysisResult::default();
                    refused.dialect = dialect.name.to_owned();
                    refused.analysis_context_unavailable = Some(miss);
                    refused
                }
            }
        })
    })
    .await
    {
        Ok(analysis) => analysis,
        Err(error) => {
            report_analysis_worker_panic(client, ctx.uri, "recovery-input", &error).await;
            return ControlFlow::Break(true);
        }
    };
    if preview.analysis_context_unavailable.is_some() {
        return ControlFlow::Continue(Arc::new(preview));
    }
    let Some(input) = preview.resolved_input.as_ref().cloned() else {
        return ControlFlow::Continue(Arc::new(preview));
    };
    let widened = widen_recovery_extra_commands(recovery, extra, ctx.text, &preview).await;
    let text = ctx.text.to_owned();
    let disabled = disabled.clone();
    let path = ctx
        .uri
        .to_file_path()
        .map(|path| path.display().to_string());
    match crate::rt::spawn_blocking(move || {
        with_pack_hooks(|| {
            let mut analysis =
                Backend::recovery_analyser(disabled, mode, widened.names, packs, resource)
                    .with_file_path(path)
                    .with_workspace_class_factories(factories)
                    .with_workspace_subclass_methods(subclasses)
                    .with_resolved_input(input)
                    .analyse(&text, dialect.name);
            // This guards negative closure only. Recognition labels do not
            // issue bindings, original source words or a Native body entry.
            analysis.has_dynamic_providers |= widened.incomplete;
            Arc::new(analysis)
        })
    })
    .await
    {
        Ok(analysis) => ControlFlow::Continue(analysis),
        Err(error) => {
            report_analysis_worker_panic(client, ctx.uri, "recovery-path", &error).await;
            ControlFlow::Break(true)
        }
    }
}

pub(super) async fn widen_recovery_extra_commands(
    ctx: &RecoveryWidenCtx<'_>,
    base: &HashSet<String>,
    text: &str,
    analysis: &AnalysisResult,
) -> RecoveryNames {
    let Some(input) =
        tcl_compiler::source_graph::current_analysis(text, analysis).and_then(|(_, config)| {
            analysis.resolved_input.as_ref().filter(|input| {
                tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
                    ctx.registry,
                    input,
                    config,
                    analysis.resolved_profile(),
                )
                .is_some()
            })
        })
    else {
        return RecoveryNames {
            names: Arc::new(base.clone()),
            incomplete: true,
        };
    };
    let mut base_key: Vec<String> = base.iter().cloned().collect();
    base_key.sort_unstable();
    let shared_key = SharedRecoveryKey {
        index_generation: ctx.workspace_index.read().await.generation(),
        resolver_revision: ctx.package_resolver.read().await.revision(),
        base: base_key,
    };
    let shared = shared_recovery_names(ctx, &shared_key).await;
    let roots = requirements(analysis, ctx.prefer);
    if roots.is_empty() {
        return RecoveryNames {
            names: shared,
            incomplete: analysis.has_dynamic_providers,
        };
    }
    // Capture configuration/Salsa rows before taking the synchronous resolver
    // read. No resolver callback holds those handles or awaits another task.
    let sources = ctx
        .provider_capture
        .capture(ctx.package_resolver, true)
        .await;
    let resolver = ctx.package_resolver.read().await;
    let (commands, providers, incomplete) =
        with_pack_hooks(|| package_recovery_names(&resolver, &sources, analysis, input, &roots));
    // Compare the newly projected source names too: reporting factory inputs
    // may change without changing a provider's text or availability context.
    let mut command_key: Vec<_> = commands.iter().cloned().collect();
    command_key.sort_unstable();
    let key = RecoveryNameKey {
        shared: shared_key,
        input: input.clone(),
        requires: roots,
        providers,
        commands: command_key,
        incomplete,
    };
    drop(resolver);
    {
        let cached = ctx.cache.lock().await;
        if let Some((cached_key, names)) = cached.widened.as_ref()
            && *cached_key == key
        {
            return names.clone();
        }
    }
    let mut names = (*shared).clone();
    names.extend(commands);
    let result = RecoveryNames {
        names: Arc::new(names),
        incomplete,
    };
    ctx.cache.lock().await.widened = Some((key, result.clone()));
    result
}

fn requirements(analysis: &AnalysisResult, prefer: PackagePrefer) -> Vec<Requirement> {
    analysis
        .package_requires
        .iter()
        .map(|required| {
            Requirement::from_source_requirement(
                required,
                tcl_lsp_core::package_resolver::package_prefer_at(
                    analysis,
                    required.range.start(),
                    prefer,
                ),
            )
        })
        .collect()
}

fn package_recovery_names(
    resolver: &PackageResolver,
    sources: &provider_source::ConfiguredProviderSources,
    analysis: &AnalysisResult,
    input: &ResolvedAnalysisInput,
    roots: &[Requirement],
) -> (HashSet<String>, Vec<RecoveryProviderKey>, bool) {
    let providers = std::cell::RefCell::new(Vec::new());
    let mut incomplete = analysis.has_dynamic_providers;
    let mut names = HashSet::new();
    let target = tcl_registry::InvocationDialect::of_profile(input.unit_profile()).tcl_version;
    for root in roots {
        let version = root
            .key()
            .original()
            .and_then(|key| match key.policy().recipe() {
                tcl_syntax::naming::NativeNameProtocol::C(version) => Some(version),
                tcl_syntax::naming::NativeNameProtocol::Jim084 => None,
            })
            .or(target);
        let inventory = |path: &Path| {
            let result = sources.inventory(path, version, resolver);
            providers.borrow_mut().push(RecoveryProviderKey {
                path: path.to_owned(),
                target: version,
                source: result
                    .as_ref()
                    .map(|row| (Arc::clone(&row.text), Arc::clone(&row.input)))
                    .map_err(Clone::clone),
            });
            result
        };
        let available = resolver.transitive_requirement_advice_with_context(
            std::slice::from_ref(root),
            version,
            &|path, parent| {
                let Ok(row) = inventory(path) else {
                    return vec![Requirement::new(
                        Key::Unknown,
                        Vec::new(),
                        false,
                        parent.prefer(),
                    )];
                };
                let mut dependencies = requirements(&row.analysis, parent.prefer());
                if row.analysis.has_dynamic_providers {
                    dependencies.push(Requirement::new(
                        Key::Unknown,
                        Vec::new(),
                        false,
                        parent.prefer(),
                    ));
                }
                dependencies
            },
        );
        for requirement in &available {
            incomplete |= requirement.key() == &Key::Unknown;
            // An unenumerable loader supplies no negative closure, even when
            // its advertisement exists or another provider supplied names.
            incomplete |= resolver
                .resolve_requirement_advice(requirement, version)
                .is_empty()
                && !registry_requirement(input.borrowed_context_registry().commands(), requirement);
        }
        let declarations = resolver.requirement_advice_defined_commands(
            &available.into_iter().collect::<Vec<_>>(),
            version,
            &|path| {
                inventory(path).map_or_else(
                    |_| Vec::new(),
                    |row| defined_original_commands_from_analysis(&row.text, &row.analysis),
                )
            },
        );
        for declaration in declarations {
            if let Some(spelling) = declaration.source_spelling() {
                tcl_compiler::analyser::utils::insert_qualified_and_tail(&mut names, &spelling);
            }
        }
    }
    let mut providers = providers.into_inner();
    incomplete |= providers.iter().any(|row| row.source.is_err());
    providers.sort_by(|left, right| {
        left.path.cmp(&right.path).then_with(|| {
            left.target
                .map(tcl_dialect::TclVersion::dialect_name)
                .cmp(&right.target.map(tcl_dialect::TclVersion::dialect_name))
        })
    });
    providers.dedup();
    (names, providers, incomplete)
}

fn registry_requirement(registry: &CommandRegistry, requirement: &Requirement) -> bool {
    requirement.key().matches_ascii_metadata("Tcl")
        || match requirement.key() {
            Key::Original(key) => std::str::from_utf8(key.bytes())
                .ok()
                .filter(|name| name.is_ascii())
                .is_some_and(|name| registry.provides_package(name)),
            Key::AuthoredMetadata(name) => registry.provides_package(name),
            Key::Unknown => false,
        }
}

async fn shared_recovery_names(
    ctx: &RecoveryWidenCtx<'_>,
    key: &SharedRecoveryKey,
) -> Arc<HashSet<String>> {
    {
        let cached = ctx.cache.lock().await;
        if let Some((cached_key, names)) = cached.shared.as_ref()
            && cached_key == key
        {
            return Arc::clone(names);
        }
    }
    let mut names: HashSet<String> = key.base.iter().cloned().collect();
    {
        let index = ctx.workspace_index.read().await;
        for proc in index.live_procs() {
            tcl_compiler::analyser::utils::insert_qualified_and_tail(
                &mut names,
                &proc.qualified_name,
            );
        }
        for class in index.live_classes() {
            tcl_compiler::analyser::utils::insert_qualified_and_tail(
                &mut names,
                &class.qualified_name,
            );
        }
    }
    for name in ctx.package_resolver.read().await.auto_command_names() {
        names.insert(name.trim_start_matches("::").to_owned());
    }
    let names = Arc::new(names);
    ctx.cache.lock().await.shared = Some((key.clone(), Arc::clone(&names)));
    names
}

#[cfg(test)]
mod tests;
