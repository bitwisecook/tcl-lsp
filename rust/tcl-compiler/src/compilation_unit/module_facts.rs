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

//! Whole-module facts every per-function build shares: the class roster, known classes and connection-scoped names.

use super::{ModuleWideFacts, UnitBuildOptions};
use crate::cfg::CfgModule;
use crate::ir::Module as IrModule;
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// The names an iRules `when` handler of the module may find bound on
/// entry: every name any handler binds, with the array each element sits
/// in, and whether a handler writes a computed name
/// ([`crate::value_transfer::ConnectionScoped`]). A handler's variables
/// live as long as its connection, so another event — or an earlier firing
/// of the same one — may have bound them; empty for a module with no
/// handler.
pub(super) fn connection_scoped_names(
    module: &IrModule,
    cfg_module: &CfgModule,
    registry: &CommandRegistry,
    options: UnitBuildOptions<'_>,
) -> crate::value_transfer::ConnectionScoped {
    let config = options.config;
    let mut names: BTreeSet<String> = BTreeSet::new();
    let mut any = false;
    for (id, cfg) in &cfg_module.procedures {
        if !module
            .procedures
            .get(id)
            .is_some_and(|body| body.body_kind == crate::ir::ProcedureBodyKind::Event)
        {
            continue;
        }
        any |= crate::dynamic_names::dynamic_name_barrier(cfg, registry, config).writes;
        for block in cfg.blocks.values() {
            for statement in &block.statements {
                names.extend(crate::ssa::defs_of_with_registry(statement, Some(registry)));
                for script in crate::ir_helpers::nested_bodies(statement) {
                    crate::ir::for_each_statement(script, &mut |inner| {
                        names.extend(crate::ssa::defs_of_with_registry(inner, Some(registry)));
                    });
                }
            }
        }
    }
    let arrays: Vec<String> = names
        .iter()
        .filter_map(|name| {
            tcl_syntax::naming::split_element_ref(name).map(|(base, _)| base.to_owned())
        })
        .collect();
    names.extend(arrays);
    crate::value_transfer::ConnectionScoped {
        names: names.into_iter().collect(),
        any,
    }
}

/// Immutable descriptive class spellings from the genuine source producer.
/// Construction stays private; the actual Module joins every projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedSourceClassFacts {
    pub(super) source: tcl_lexer::SourceImage,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) registry: tcl_registry::RegistrySemanticKey,
    pub(super) input: Option<crate::analyser::ResolvedAnalysisInput>,
    pub(super) entry: crate::command_binding::SourceAnalysisEntry,
    owner: Option<Arc<crate::command_binding::RetainedSourceModuleBindings>>,
    names: Arc<HashSet<String>>,
}

impl RetainedSourceClassFacts {
    pub(super) fn retain(
        module: &IrModule,
        registry: &CommandRegistry,
        names: Arc<HashSet<String>>,
    ) -> Option<Arc<Self>> {
        if let Some(input) = &module.source_metadata_input {
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
            if input.lexer_config() != module.lexer_config {
                return None;
            }
            module
                .retained_source_bindings
                .as_deref()?
                .positioned_for_module(module, registry)?;
        } else if !module.source_entry.metadata_context.is_standalone() {
            return None;
        }
        Some(Arc::new(Self {
            source: module.source.clone(),
            config: module.lexer_config,
            registry: registry.snapshot().semantic_key(),
            input: module.source_metadata_input.clone(),
            entry: module.source_entry.clone(),
            owner: module.retained_source_bindings.clone(),
            names,
        }))
    }

    /// Borrow the complete roster only under its actual source/config/input/store.
    /// Class spellings provide no occupied Native class or receiver allocation.
    pub(crate) fn for_module(
        &self,
        module: &IrModule,
        registry: &CommandRegistry,
    ) -> Option<&HashSet<String>> {
        if self.source != module.source
            || self.config != module.lexer_config
            || self.registry != registry.snapshot().semantic_key()
            || self.input != module.source_metadata_input
            || self.entry != module.source_entry
        {
            return None;
        }
        match (&self.owner, &module.retained_source_bindings) {
            (Some(owner), Some(current)) if Arc::ptr_eq(owner, current) || owner == current => {
                owner.positioned_for_module(module, registry)?;
            }
            (None, None)
                if module.source_metadata_input.is_none()
                    && module.source_entry.metadata_context.is_standalone() => {}
            _ => return None,
        }
        Some(&self.names)
    }
}

impl ModuleWideFacts {
    pub(super) fn collect(source: &str, ir_module: &IrModule, registry: &CommandRegistry) -> Self {
        // Classes come from the signature scanner so this build and the
        // incremental/db build derive an identical set (⇒ identical
        // OBJECT-constructor typing on both paths).
        let roster = collect_known_classes(source, ir_module, registry);
        let available = roster.is_some();
        let known_class_set = Arc::new(roster.unwrap_or_default());
        let source_class_facts = available
            .then(|| {
                RetainedSourceClassFacts::retain(ir_module, registry, Arc::clone(&known_class_set))
            })
            .flatten();
        let mut known_classes: Vec<String> = known_class_set.iter().cloned().collect();
        known_classes.sort_unstable();
        Self {
            known_class_set,
            source_class_facts,
            known_classes,
            // A top-level bare name already lives in the global frame, so a
            // call to a procedure that declares it `global` can reassign it
            // mid-run even though the top-level body never says `global`
            // itself. See `crate::var_observability::scan_module_global_names`.
            top_level_extra_escaping: crate::var_observability::scan_module_global_names(
                ir_module, registry,
            ),
            traced_variable_names: ir_module.traced_variables.iter().cloned().collect(),
        }
    }
}

/// Fully-qualified names of every class defined in `source`.
///
/// Sourced from [`crate::signature_scan`] (which records `oo::class create` and
/// `itcl::class` definitions) so the standalone analyser build and the
/// incremental/db build derive an *identical* set from the same source — the
/// precondition for the two paths agreeing on constructor (`Foo new`) typing.
/// The signature result is a descriptive source inventory, independent of
/// Native allocation or execution. The actual selected Registry grammar, not
/// a spelling probe, determines every recorded declaration.
pub(super) fn collect_known_classes(
    source: &str,
    module: &IrModule,
    registry: &CommandRegistry,
) -> Option<HashSet<String>> {
    #[cfg(test)]
    let trace_enabled =
        tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
    #[cfg(test)]
    let trace = |stage: &str, passed: bool| {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
            eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset=none passed={passed}");
        }
    };
    #[cfg(test)]
    if trace_enabled {
        trace(
            "class_roster.document",
            module.source == tcl_lexer::SourceImage::document(source),
        );
    }
    if module.source != tcl_lexer::SourceImage::document(source) {
        return None;
    }
    let signatures = if let Some(retained) = module.retained_source_bindings.as_deref() {
        let bindings = retained.positioned_for_module(module, registry);
        #[cfg(test)]
        if trace_enabled {
            trace("class_roster.positioned", bindings.is_some());
        }
        let bindings = bindings?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "class_roster.input_config",
                bindings.matches_original_source_image(&module.source, module.lexer_config),
            );
        }
        if !bindings.matches_original_source_image(&module.source, module.lexer_config) {
            return None;
        }
        crate::signature_scan::extract_signatures_with_original_bindings(
            source,
            module.lexer_config,
            registry,
            bindings,
        )
    } else if module.source_metadata_input.is_none()
        && module.source_entry.metadata_context.is_standalone()
    {
        crate::signature_scan::extract_signatures_with_config(source, module.lexer_config, registry)
    } else {
        return None;
    };
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
        let declared = signatures.class_declarations.len();
        let named = signatures
            .class_declarations
            .iter()
            .filter(|declaration| declaration.source_spelling().is_some())
            .count();
        eprintln!(
            "ORIGINAL_FACTORY_GUARD stage=class_roster.selected_result declared={declared} named={named}"
        );
    }
    Some(
        signatures
            .class_declarations
            .into_iter()
            .filter_map(|declaration| declaration.source_spelling())
            .collect(),
    )
}
