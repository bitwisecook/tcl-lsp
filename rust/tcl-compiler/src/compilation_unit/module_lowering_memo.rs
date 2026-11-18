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

//! The lowered module of one document as a value of its complete lowering
//! inputs, so a host can hand an unchanged document the same module objects
//! (and with them the same procedure identities and source proofs) instead of
//! lowering it again.

use super::build_stages::{RetainedLoweredSource, RetainedLoweringInput, lower_retained_source};
use super::{OriginalBodyCacheInputs, UnitBuildOptions};
use crate::cfg::CfgModule;
use crate::cfg_builder::PreparedCfgContext;
use crate::ir::Module as IrModule;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Every input lowering reads, owned and compared by value. Built only by an
/// exhaustive destructure of the lowering request, so an input added there
/// does not compile until it is added here.
#[derive(Debug, Clone)]
pub struct ModuleLoweringInputs {
    source: Arc<str>,
    registry: tcl_registry::RegistrySnapshot,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    external_call_sites: Option<crate::unit_scope::CallSiteEvidence>,
    declared_commands: Option<tcl_registry::model::DeclaredSurface>,
    entry: Option<crate::command_binding::SourceAnalysisEntry>,
    context: Option<Arc<tcl_registry::model::ContextRegistry>>,
    input: Option<crate::analyser::ResolvedAnalysisInput>,
    source_settings: Option<Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
    /// Process state a generation reads at use rather than at assembly: the
    /// installed pack hook evaluators and the pack overlay epoch.
    evaluator_generation: u64,
    overlay_epoch: u64,
}

impl PartialEq for ModuleLoweringInputs {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.registry == other.registry
            && self.defer_top_level == other.defer_top_level
            && self.config == other.config
            && self.dialect.map(tcl_dialect::DialectProfile::cache_key)
                == other.dialect.map(tcl_dialect::DialectProfile::cache_key)
            && self.external_call_sites == other.external_call_sites
            && self.declared_commands == other.declared_commands
            && self.entry == other.entry
            && match (&self.context, &other.context) {
                (Some(left), Some(right)) => crate::analyser::same_context_generation(left, right),
                (None, None) => true,
                _ => false,
            }
            && match (&self.input, &other.input) {
                (Some(left), Some(right)) => left.same_by_generation(right),
                (None, None) => true,
                _ => false,
            }
            && self.source_settings == other.source_settings
            && self.evaluator_generation == other.evaluator_generation
            && self.overlay_epoch == other.overlay_epoch
    }
}
impl Eq for ModuleLoweringInputs {}

impl Hash for ModuleLoweringInputs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.registry.hash(state);
        self.defer_top_level.hash(state);
        self.config.hash(state);
        self.dialect
            .map(tcl_dialect::DialectProfile::cache_key)
            .hash(state);
        self.entry.hash(state);
        self.context.is_some().hash(state);
        self.evaluator_generation.hash(state);
        self.overlay_epoch.hash(state);
        if let Some(input) = &self.input {
            input.hash_by_generation(state);
        }
    }
}

impl ModuleLoweringInputs {
    /// The command registry lowering resolves against.
    #[must_use]
    pub const fn registry(&self) -> &tcl_registry::RegistrySnapshot {
        &self.registry
    }

    /// The analysis dialect.
    #[must_use]
    pub const fn dialect(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.dialect
    }

    /// The document's lexer configuration.
    #[must_use]
    pub const fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }

    /// These inputs with the source text left out, for a host that keys the
    /// text by its own document identity.
    #[must_use]
    pub fn without_source(&self) -> Self {
        Self {
            source: Arc::from(""),
            ..self.clone()
        }
    }

    /// These inputs over `source`.
    #[must_use]
    pub fn with_source(&self, source: &str) -> Self {
        Self {
            source: Arc::from(source),
            ..self.clone()
        }
    }

    /// The source text lowering reads.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The inputs of a lowering request that has no standalone owner.
    pub(super) fn of_request(request: &RetainedLoweringInput<'_, '_>) -> Option<Self> {
        let RetainedLoweringInput {
            source,
            options:
                UnitBuildOptions {
                    registry,
                    defer_top_level,
                    config,
                    dialect,
                    external_call_sites,
                    declared_commands,
                },
            body_caches: _,
            entry,
            context,
            input,
            source_body_cache: _,
            standalone,
            source_settings,
        } = request;
        if standalone.is_some() {
            return None;
        }
        Some(Self {
            source: Arc::from(*source),
            registry: registry.snapshot(),
            defer_top_level: *defer_top_level,
            config: *config,
            dialect: *dialect,
            external_call_sites: external_call_sites.cloned(),
            declared_commands: declared_commands.cloned(),
            entry: entry.cloned(),
            context: context.clone(),
            input: input.cloned(),
            source_settings: source_settings.cloned(),
            evaluator_generation: u64::from(tcl_registry::pack_hooks::evaluator_generation().0),
            overlay_epoch: tcl_registry::overlay_epoch(),
        })
    }
}

/// One document's lowered module, CFG and prepared CFG context, with the
/// inputs it was lowered from. Consumers take clones; it is never changed in
/// place, so every holder sees the same procedure identities and proofs.
#[derive(Clone)]
pub struct LoweredModule {
    inputs: ModuleLoweringInputs,
    ir_module: IrModule,
    cfg_module: CfgModule,
    tainted_global_writes: HashMap<crate::ir::ProcedureId, HashSet<String>>,
    prepared_cfg_context: PreparedCfgContext,
}

/// Two lowerings of equal inputs are the same module: lowering is a function of
/// its inputs. A host memo keeps the first one, so its holders share objects.
impl PartialEq for LoweredModule {
    fn eq(&self, other: &Self) -> bool {
        self.inputs == other.inputs
    }
}
impl Eq for LoweredModule {}

impl std::fmt::Debug for LoweredModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoweredModule")
            .field("inputs", &self.inputs)
            .finish_non_exhaustive()
    }
}

/// A host's memo of lowered modules by value of their inputs.
pub type ModuleLoweringCache<'a> = dyn Fn(&ModuleLoweringInputs) -> Option<Arc<LoweredModule>> + 'a;

impl LoweredModule {
    /// Lower `inputs` exactly as a unit build would. A lowering whose body
    /// caches reported an integrity failure is not retained.
    #[must_use]
    pub fn lower(
        inputs: &ModuleLoweringInputs,
        body_cache: &crate::lowering::OriginalBodyLoweringCache<'_>,
        source_body_cache: &crate::lowering::SourceBodySyntaxCache<'_>,
    ) -> Option<Self> {
        let registry = inputs.registry.registry();
        let lowered = lower_retained_source(
            RetainedLoweringInput {
                source: &inputs.source,
                options: UnitBuildOptions {
                    registry,
                    defer_top_level: inputs.defer_top_level,
                    config: inputs.config,
                    dialect: inputs.dialect,
                    external_call_sites: inputs.external_call_sites.as_ref(),
                    declared_commands: inputs.declared_commands.as_ref(),
                },
                body_caches: OriginalBodyCacheInputs {
                    legacy: None,
                    original: Some(body_cache),
                    module: None,
                },
                entry: inputs.entry.as_ref(),
                context: inputs.context.clone(),
                input: inputs.input.as_ref(),
                source_body_cache: Some(source_body_cache),
                standalone: None,
                source_settings: inputs.source_settings.as_ref(),
            },
            #[cfg(any(test, debug_assertions))]
            &mut super::SourcePipelineTrace::new("module-memo", inputs.source.len()),
        );
        lowered.integrity.get().then(|| Self {
            inputs: inputs.clone(),
            ir_module: lowered.ir_module,
            cfg_module: lowered.cfg_module,
            tainted_global_writes: lowered.tainted_global_writes,
            prepared_cfg_context: lowered.prepared_cfg_context,
        })
    }

    /// The complete inputs this module was lowered from.
    #[must_use]
    pub const fn inputs(&self) -> &ModuleLoweringInputs {
        &self.inputs
    }

    /// A retained copy for a request with exactly these inputs.
    pub(super) fn retained_for(
        &self,
        inputs: &ModuleLoweringInputs,
    ) -> Option<RetainedLoweredSource> {
        (&self.inputs == inputs).then(|| RetainedLoweredSource {
            ir_module: self.ir_module.clone(),
            cfg_module: self.cfg_module.clone(),
            tainted_global_writes: self.tainted_global_writes.clone(),
            prepared_cfg_context: self.prepared_cfg_context.clone(),
            integrity: std::cell::Cell::new(true),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleLoweringInputs;
    use std::sync::Arc;

    type Variant = (&'static str, Box<dyn Fn(&mut ModuleLoweringInputs)>);

    fn base() -> ModuleLoweringInputs {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let registry = tcl_registry::CommandRegistry::build_default();
        ModuleLoweringInputs {
            source: Arc::from("proc p {} { return 1 }\np\n"),
            registry: registry.snapshot(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
            entry: None,
            context: None,
            input: None,
            source_settings: None,
            evaluator_generation: 0,
            overlay_epoch: 0,
        }
    }

    fn variants() -> Vec<Variant> {
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(Arc::new(tcl_registry::CommandRegistry::build_default())),
        );
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        vec![
            (
                "source",
                Box::new(|inputs| inputs.source = Arc::from("proc p {} { return 2 }\np\n")),
            ),
            (
                "registry",
                Box::new(|inputs| {
                    let mut registry = tcl_registry::CommandRegistry::build_default();
                    registry.insert(tcl_registry::CommandSpec {
                        name: "forge",
                        ..tcl_registry::CommandSpec::DEFAULT
                    });
                    inputs.registry = registry.snapshot();
                }),
            ),
            (
                "defer_top_level",
                Box::new(|inputs| inputs.defer_top_level = true),
            ),
            (
                "config",
                Box::new(|inputs| inputs.config.expand_syntax = !inputs.config.expand_syntax),
            ),
            (
                "dialect",
                Box::new(|inputs| {
                    inputs.dialect = Some(
                        tcl_registry::model::ingress::resolve_environment("tcl8.4")
                            .analyser_profile(),
                    );
                }),
            ),
            (
                "external_call_sites",
                Box::new(|inputs| {
                    inputs.external_call_sites =
                        Some(crate::unit_scope::CallSiteEvidence::default());
                }),
            ),
            (
                "declared_commands",
                Box::new(|inputs| {
                    inputs.declared_commands =
                        Some(tcl_registry::model::DeclaredSurface::default());
                }),
            ),
            (
                "entry",
                Box::new(|inputs| {
                    inputs.entry = Some(crate::command_binding::SourceAnalysisEntry::default());
                }),
            ),
            (
                "context",
                Box::new({
                    let context = Arc::clone(&context);
                    move |inputs| inputs.context = Some(Arc::clone(&context))
                }),
            ),
            (
                "input",
                Box::new({
                    let context = Arc::clone(&context);
                    move |inputs| {
                        inputs.input = Some(crate::analyser::ResolvedAnalysisInput::new(
                            profile,
                            profile,
                            Arc::clone(&context),
                            inputs.config,
                        ));
                    }
                }),
            ),
            (
                "evaluator_generation",
                Box::new(|inputs| inputs.evaluator_generation += 1),
            ),
            (
                "overlay_epoch",
                Box::new(|inputs| inputs.overlay_epoch += 1),
            ),
            (
                "source_settings",
                Box::new(|inputs| {
                    inputs.source_settings =
                        Some(crate::analyser::Analyser::new().source_compilation_settings());
                }),
            ),
        ]
    }

    fn generation(
        store: tcl_registry::CommandRegistry,
    ) -> Arc<tcl_registry::model::ContextRegistry> {
        Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(Arc::new(store)),
        )
    }

    fn over(context: &Arc<tcl_registry::model::ContextRegistry>) -> ModuleLoweringInputs {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut inputs = base();
        inputs.context = Some(Arc::clone(context));
        inputs.input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(context),
            inputs.config,
        ));
        inputs
    }

    /// A host that assembles an equal registry generation again (a pack
    /// overlay rebuilt for each build) must find the module it lowered
    /// before; a generation over a different command store must not.
    #[test]
    fn an_equal_generation_assembled_again_is_the_same_module_key() {
        use std::hash::BuildHasher;
        let hash = |inputs: &ModuleLoweringInputs| {
            std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default()
                .hash_one(inputs)
        };
        let first = generation(tcl_registry::CommandRegistry::build_default());
        let again = generation(tcl_registry::CommandRegistry::build_default());
        assert!(!Arc::ptr_eq(&first, &again));
        assert_eq!(over(&first), over(&again));
        assert_eq!(hash(&over(&first)), hash(&over(&again)));
        let mut store = tcl_registry::CommandRegistry::build_default();
        store.insert(tcl_registry::CommandSpec {
            name: "forge",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        assert_ne!(over(&first), over(&generation(store)));
    }

    /// A memo keyed by fewer inputs than lowering reads would serve one
    /// configuration's module to another, so each input alone must
    /// distinguish the key.
    #[test]
    fn each_lowering_input_alone_distinguishes_the_module_key() {
        let variants = variants();
        let base = base();
        assert_eq!(base, self::base(), "equal inputs share a module");
        for (name, vary) in &variants {
            let mut varied = base.clone();
            vary(&mut varied);
            assert_ne!(varied, base, "{name} alone must distinguish the module key");
        }
        // The variants above name every field: a new input must join them.
        let ModuleLoweringInputs {
            source: _,
            registry: _,
            defer_top_level: _,
            config: _,
            dialect: _,
            external_call_sites: _,
            declared_commands: _,
            entry: _,
            context: _,
            input: _,
            source_settings: _,
            evaluator_generation: _,
            overlay_epoch: _,
        } = base;
        assert_eq!(variants.len(), 13);
    }
}
