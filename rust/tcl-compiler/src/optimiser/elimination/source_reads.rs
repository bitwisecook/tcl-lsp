// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Possible hidden reads from the original evaluated source vectors.

use crate::compilation_unit::FunctionUnit;
use crate::registry_invocation::InvocationMetadataContext;
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

/// Unknown reads are retained independently of names discovered so far.
/// They are suppression facts, never receipts for a Native read or cell.
#[derive(Default)]
pub(crate) struct OriginalReadFootprint {
    pub(crate) names: HashSet<String>,
    pub(crate) opaque: bool,
}

impl OriginalReadFootprint {
    /// Every symbolic contents name may be consumed by an unresolved read.
    /// This only prevents a diagnostic/removal; it creates no SSA use/version.
    pub(crate) fn possible_names(mut self, function: &FunctionUnit) -> HashSet<String> {
        if self.opaque {
            self.names.extend(function.ssa.var_names().iter().cloned());
        }
        self.names
    }
}

pub(crate) fn original_hidden_reads(
    function: &FunctionUnit,
    registry: &CommandRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
) -> OriginalReadFootprint {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return OriginalReadFootprint {
            opaque: true,
            ..Default::default()
        };
    };
    let mut out = OriginalReadFootprint::default();
    for (block_id, block) in &function.cfg.blocks {
        for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
            let Some(tokens) = function.cfg.source_tokens_at(*block_id, index) else {
                if index != usize::MAX {
                    let statement = &block.statements[index];
                    if statement.source_edit_span().is_some()
                        && !statement
                            .tokens()
                            .is_some_and(|tokens| !tokens.evaluates_words())
                    {
                        out.opaque = true;
                    }
                }
                continue;
            };
            let parent_reads = tokens.source_binding.as_ref().and_then(|binding| {
                let site = binding.invocation_site()?;
                let source = site.source.source_image().try_text().ok()?;
                let footprint = binding.original_materialized_footprint(
                    tokens,
                    source,
                    registry,
                    Some(metadata),
                )?;
                Some(footprint.reevaluated_reads())
            });
            if let Some(reads) = parent_reads {
                out.opaque |= reads.opaque;
                out.names.extend(reads.read_names);
            } else {
                out.opaque = true;
            }
            for (_, binding) in &tokens.nested_bindings {
                let Some((_, nested)) = binding.original_recorded_command() else {
                    out.opaque = true;
                    continue;
                };
                let Some(site) = binding.invocation_site() else {
                    out.opaque = true;
                    continue;
                };
                let Ok(source) = site.source.source_image().try_text() else {
                    out.opaque = true;
                    continue;
                };
                let Some(footprint) = binding.original_materialized_footprint(
                    &nested,
                    source,
                    registry,
                    Some(metadata),
                ) else {
                    out.opaque = true;
                    continue;
                };
                let reads = footprint.invocation_reads();
                out.opaque |= reads.opaque;
                out.names.extend(reads.read_names);
            }
        }
    }
    out
}

/// Ordinary source substitutions obey the actual grammar and protected braces.
/// By-name command reads are supplied separately by original_hidden_reads.
pub(crate) fn original_textual_reads(
    source: &str,
    function: &FunctionUnit,
    registry: &CommandRegistry,
) -> HashSet<String> {
    let Some(metadata) = function.invocation_metadata_context(registry) else {
        return OriginalReadFootprint {
            opaque: true,
            ..Default::default()
        }
        .possible_names(function);
    };
    let mut scanner =
        VarReferenceScanner::with_config(VarScanOptions::default(), function.source_lexer_config());
    let mut out = original_hidden_reads(function, registry, Some(metadata));
    for (block_id, block) in &function.cfg.blocks {
        for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
            let Some(tokens) = function.cfg.source_tokens_at(*block_id, index) else {
                continue;
            };
            let Some(binding) = tokens.source_binding.as_ref() else {
                out.opaque = true;
                continue;
            };
            let Some(site) = binding.invocation_site() else {
                out.opaque = true;
                continue;
            };
            if site.source.source_image().try_text().ok() != Some(source) {
                out.opaque = true;
                continue;
            }
            for word in tokens.words() {
                if let Some(text) = source.get(word.source().span.as_range()) {
                    out.names.extend(scanner.scan_script(text, registry));
                } else {
                    out.opaque = true;
                }
            }
        }
    }
    out.possible_names(function)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;
    use tcl_registry::model::ContextRegistry;

    fn unit(
        source: &str,
        context: &Arc<ContextRegistry>,
        config: tcl_lexer::LexerConfig,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    fn reads(unit: &CompilationUnit, registry: &CommandRegistry) -> OriginalReadFootprint {
        original_hidden_reads(
            &unit.top_level,
            registry,
            unit.top_level
                .invocation_metadata_context_for_module(registry, &unit.ir_module),
        )
    }

    #[test]
    fn original_hidden_read_names_keep_alias_horizons_and_literal_roots() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source names only, never a current read/frame/removal grant.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        for (source, expected) in [
            (
                "interp alias {} get {} set {café(open}; puts [get]",
                Some("café(open"),
            ),
            (
                "interp alias {} get {} set {$literal}; puts [get]",
                Some("$literal"),
            ),
            (
                "rename set moved; interp alias {} get {} moved {a b(k)}; puts [get]",
                Some("a b"),
            ),
            ("proc set args {}; puts [set hidden]", None),
            (
                "interp alias {} get {} set hidden; rename set moved; puts [get]",
                None,
            ),
        ] {
            let selected = unit(source, &context, config);
            let footprint = reads(&selected, context.commands());
            if let Some(expected) = expected {
                assert!(
                    footprint.names.contains(expected),
                    "{source}: {:?}",
                    footprint.names
                );
                assert!(!footprint.names.contains("literal"));
            } else {
                assert!(
                    !footprint.names.contains("hidden"),
                    "known target barrier: {source}"
                );
            }
        }
    }

    #[test]
    fn original_hidden_reads_keep_actual_availability_and_variable_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "metadata_read",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("set").unwrap().clone()
        });
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let base = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        for (context, expected) in [(&current, true), (&older, false)] {
            let selected = unit("puts [metadata_read {café(open}]", context, base);
            let footprint = reads(&selected, &registry);
            assert_eq!(footprint.names.contains("café(open"), expected);
            if !expected {
                assert!(footprint.opaque);
            }
        }
        for (style, expected, other) in [
            (tcl_dialect::BracedVarStyle::FirstClose, "a{b", "a{b}c"),
            (tcl_dialect::BracedVarStyle::Tcl9Nesting, "a{b}c", "a{b"),
        ] {
            let mut config = base;
            config.braced_var = style;
            let selected = unit("puts [expr {${a{b}c}}]", &current, config);
            let footprint = reads(&selected, &registry);
            assert!(
                footprint.names.contains(expected),
                "{style:?}: {:?}",
                footprint.names
            );
            assert!(!footprint.names.contains(other));
        }
    }

    #[test]
    fn original_hidden_reads_refuse_missing_foreign_and_stale_inputs() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        let selected = unit("set saved OLD; puts [set saved]", &context, config);
        assert!(reads(&selected, context.commands()).names.contains("saved"));
        let missing = original_hidden_reads(&selected.top_level, context.commands(), None);
        assert!(missing.opaque);
        assert!(missing.names.is_empty());
        assert!(
            missing
                .possible_names(&selected.top_level)
                .contains("saved")
        );
        let mut foreign_store = CommandRegistry::build_default();
        foreign_store.insert(tcl_registry::CommandSpec {
            name: "foreign_read_axis",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let foreign = Arc::new(context.with_command_store(Arc::new(foreign_store)));
        assert_ne!(
            context.commands().snapshot().semantic_key(),
            foreign.commands().snapshot().semantic_key(),
            "the negative control changes the actual command store"
        );
        let refused = reads(&selected, foreign.commands());
        assert!(refused.opaque && refused.names.is_empty());
        let mut stale = selected.ir_module.clone();
        stale.lexer_config.strict_quoting = !stale.lexer_config.strict_quoting;
        assert!(
            selected
                .top_level
                .invocation_metadata_context_for_module(context.commands(), &stale)
                .is_none()
        );
        stale = selected.ir_module.clone();
        stale.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
        assert!(
            selected
                .top_level
                .invocation_metadata_context_for_module(context.commands(), &stale)
                .is_none()
        );
    }
}
