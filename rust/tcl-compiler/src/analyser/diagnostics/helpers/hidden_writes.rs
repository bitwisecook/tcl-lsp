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

//! Names written where the SSA form does not look: unseen calls, command substitutions in expressions and concatenated scripts.

use super::undef_suppression::UndefSuppressionSemantics;
use crate::cfg::BlockId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;

/// Each `(block, statement)` in the blocks `considered` where code the module
/// cannot see runs that may set any name the function reads: at the top level
/// any such code, which can set a global by name; in a procedure a sourced
/// file, which runs in the procedure's own frame. A callee the module cannot
/// see sets a procedure's local through `upvar 1` under a name it is handed,
/// which the per-name abstention answers
/// ([`crate::interprocedural::collect_opaque_callee_name_args`]), so its marker
/// is no site here.
pub(super) fn collect_unseen_call_sites(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    initial_global: bool,
    semantics: UndefSuppressionSemantics<'_>,
) -> Vec<(BlockId, usize)> {
    let registry = semantics.context.commands();
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return Vec::new();
    };
    let sources = |statement: &crate::ir::Statement| {
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            Some(metadata),
            statement,
        )
        .is_some_and(|call| {
            call.facts
                .traits
                .contains(tcl_registry::Traits::SOURCES_FILE)
        })
    };
    let mut out = Vec::new();
    for &block_id in considered {
        let Some(block) = fu.cfg.blocks.get(&block_id) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            if crate::ssa::is_unseen_call_marker(statement)
                && (initial_global
                    || block
                        .statements
                        .iter()
                        .filter(|other| {
                            other.span() == statement.span() && other.synthetic_marker().is_none()
                        })
                        .any(sources))
            {
                out.push((block_id, index));
            }
        }
    }
    out
}

pub(super) fn collect_expr_cmd_sub_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    semantics: UndefSuppressionSemantics<'_>,
) -> FxHashMap<String, Vec<(BlockId, usize)>> {
    let mut out = FxHashMap::<String, Vec<(BlockId, usize)>>::default();
    let registry = semantics.context.commands();
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return out;
    };
    let Some(input) = metadata.source_analysis_input() else {
        return out;
    };
    if semantics.analysis.resolved_input.as_ref() != Some(input)
        || input.lexer_config().normalized() != semantics.lexer_config.normalized()
    {
        return out;
    }
    for &block_id in considered {
        let Some(block) = fu.ssa.blocks.get(&block_id) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            let crate::ir::Statement::AssignExpr {
                expr_base: Some(base),
                ..
            } = &statement.statement
            else {
                continue;
            };
            let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, index).source_tokens()
            else {
                continue;
            };
            let mut names = FxHashSet::default();
            collect_original_expression_writes(tokens, *base, semantics, metadata, &mut names);
            for name in names {
                out.entry(name).or_default().push((block_id, index));
            }
        }
        if let Some(crate::cfg::Terminator::Branch {
            condition_base: Some(base),
            ..
        }) = fu
            .cfg
            .blocks
            .get(&block_id)
            .and_then(|block| block.terminator.as_ref())
            && let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, usize::MAX)
                    .source_tokens()
        {
            let mut names = FxHashSet::default();
            collect_original_expression_writes(tokens, *base, semantics, metadata, &mut names);
            for name in names {
                out.entry(name).or_default().push((block_id, usize::MAX));
            }
        }
    }
    out
}

/// Select the authentic Expr operand at its own original source site. An
/// outer assignment's post-substitution lookup cannot replace a child lookup.
fn collect_original_expression_writes(
    tokens: &crate::ir::CommandTokens,
    expression_base: u32,
    semantics: UndefSuppressionSemantics<'_>,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    out: &mut FxHashSet<String>,
) {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    let Some(input) = metadata.source_analysis_input() else {
        return;
    };
    let positions = tokens
        .source_binding
        .iter()
        .filter_map(|binding| binding.invocation_site().map(|site| site.offset))
        .chain(tokens.nested_bindings.iter().map(|(offset, _)| *offset));
    let mut visited = FxHashSet::default();
    for offset in positions.filter(|offset| visited.insert(*offset)) {
        let Some(words) = crate::registry_invocation::source_structure::source_registry_words_at(
            semantics.source,
            semantics.analysis,
            offset,
        ) else {
            continue;
        };
        let Some(bodies) = words.source_expression_script_bodies_at(input, expression_base) else {
            continue;
        };
        for body in bodies {
            collect_original_expression_body_writes(&body, semantics, metadata, out);
        }
    }
}

fn collect_original_expression_body_writes(
    body: &crate::registry_invocation::OriginalSourceScriptBody,
    semantics: UndefSuppressionSemantics<'_>,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    out: &mut FxHashSet<String>,
) {
    let Some(realm) = semantics.analysis.retained_command_realm() else {
        return;
    };
    let Some(text) = semantics.source.get(body.content_span().as_range()) else {
        return;
    };
    if !body.matches_context(semantics.context)
        || !body.matches_source(
            &tcl_lexer::SourceImage::document(semantics.source),
            semantics.lexer_config,
        )
    {
        return;
    }
    let source = tcl_lexer::SourceMap::new(semantics.source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        text,
        body.content_span().start(),
        semantics.lexer_config.nested(),
    );
    for segment in segments {
        let tokens =
            crate::ir::CommandTokens::from_segmented(&source, semantics.lexer_config, &segment);
        let Some(head) = tokens.argv.first() else {
            continue;
        };
        let binding = realm.invocation_at_source("", head.start());
        let Some((_, original)) = binding.original_recorded_command() else {
            continue;
        };
        // The lifted geometry is genuine original syntax. Its own source
        // issuer below validates the complete vector and lookup horizon.
        for nested in crate::word_subst::checked_lifted_calls(&original, semantics.lexer_config)
            .unwrap_or_default()
        {
            if let Some(nested) = nested.tokens
                && let Some(head) = nested.argv.first()
            {
                collect_original_command_writes(head.start(), semantics, metadata, out);
            }
        }
        collect_original_command_writes(head.start(), semantics, metadata, out);
    }
}

fn collect_original_command_writes(
    offset: u32,
    semantics: UndefSuppressionSemantics<'_>,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    out: &mut FxHashSet<String>,
) {
    let Some(realm) = semantics.analysis.retained_command_realm() else {
        return;
    };
    let binding = realm.invocation_at_source("", offset);
    let Some((_, tokens)) = binding.original_recorded_command() else {
        return;
    };
    let Some(footprint) = binding.original_materialized_footprint(
        &tokens,
        semantics.source,
        semantics.context.commands(),
        Some(metadata),
    ) else {
        return;
    };
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_tokens(
            semantics.source,
            semantics.analysis,
            &tokens,
        )
    else {
        return;
    };
    for name in footprint.invocation_writes(&words).names {
        let root = tcl_syntax::naming::split_array_name_braced(&name, true).0;
        if !root.is_empty() {
            out.insert(root.to_owned());
        }
    }
}

/// Names written by a [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`]
/// command whose trailing words are all static literals.
///
/// The lowering only inlines the single-word `eval {script}` shape; a
/// multi-word `eval set l2 hello` stays a `Statement::Barrier`, so SSA sees
/// no definition of `l2` and a later `puts $l2` reads a version-0 origin.
/// Reconstructing the `Tcl_ConcatObj` join here recovers the write without
/// claiming anything the barrier does not already guarantee — name-level and
/// suppress-only, exactly like [`collect_expr_cmd_sub_writes`].
///
/// Gated on [`tcl_registry::BodyKind::Plain`], which is the registry's own
/// record of *whose frame the body runs in*.  `eval` is `Plain` — its script
/// runs in the caller's own frame, so its writes are this function's writes.
/// `uplevel`, `namespace eval`, `namespace inscope`, and `interp eval` are
/// all `Structural`: their scripts write somewhere else entirely, and
/// tclsh8.6.14/9.0.4 confirm the difference —
/// `proc p {} {uplevel 1 set x 5; puts $x}` errors `can't read "x"` because
/// the `set` landed in the *caller's* frame. Suppressing on those would hide
/// a real read-before-set.
pub(super) fn collect_script_concat_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    semantics: UndefSuppressionSemantics<'_>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.ssa.blocks.get(&bn) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            if !matches!(statement.statement, crate::ir::Statement::Barrier { .. }) {
                continue;
            }
            let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, bn, index).source_tokens()
            else {
                continue;
            };
            let Some(words) =
                crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                    semantics.source,
                    semantics.analysis,
                    tokens,
                )
            else {
                continue;
            };
            let same_frame_concat = words.with_source_schema(semantics.context, |schema| {
                schema
                    .semantics
                    .traits
                    .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
                    && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                    && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            }) == Some(true);
            if !same_frame_concat {
                continue;
            }
            let Some(first) = words.roles().and_then(|roles| {
                roles.iter().find_map(|&(index, role)| {
                    (role == tcl_registry::ArgRole::Body).then_some(index)
                })
            }) else {
                continue;
            };
            if words.with_source_schema(semantics.context, |schema| {
                schema.authored_source_script_timing_at(first)
            }) != Some(Some(tcl_registry::ScriptTiming::SameInvocation))
            {
                continue;
            }
            let Some(metadata) = fu.invocation_metadata_context(semantics.context.commands())
            else {
                continue;
            };
            let Some(input) = metadata.source_analysis_input() else {
                continue;
            };
            if semantics.analysis.resolved_input.as_ref() != Some(input)
                || input.lexer_config().normalized() != semantics.lexer_config.normalized()
            {
                continue;
            }
            let Some(binding) = tokens.source_binding.as_ref() else {
                continue;
            };
            let Some(footprint) = binding.original_materialized_footprint(
                tokens,
                semantics.source,
                semantics.context.commands(),
                Some(metadata),
            ) else {
                continue;
            };
            let Some(script) = concatenated_script_value(&words, first) else {
                continue;
            };
            let Ok(script) = std::str::from_utf8(&script) else {
                continue;
            };
            out.extend(footprint.script_writes(script).names);
        }
    }
    out
}

/// Form the effective script bytes from the same selected body suffix.
/// Returns `None` whenever the source spelling cannot establish those bytes.
fn concatenated_script_value(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    first: usize,
) -> Option<Vec<u8>> {
    let tail = &words.arguments()[first..];
    let values: Option<Vec<_>> = tail
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        .collect();
    let script = if let Some(values) = values {
        match words.dialect().and_then(|dialect| dialect.native_family) {
            Some(tcl_dialect::model::Family::Jim) => tcl_syntax::list::concat_bytes_jim(values),
            Some(
                tcl_dialect::model::Family::Tcl
                | tcl_dialect::model::Family::F5Tcl
                | tcl_dialect::model::Family::F5Irules,
            ) => tcl_syntax::list::concat_bytes(values),
            None => return None,
        }
    } else {
        // Only the genuine braced first value supplies unchanged prefix text
        // when later values are unknown; this is lexical suppression only.
        let word = words
            .operands()
            .get(first)
            .and_then(Option::as_ref)
            .and_then(crate::registry_invocation::source_structure::OriginalOperandSource::word)
            .filter(|word| word.group().kind == tcl_lexer::WordKind::Braced)?;
        let value = tail
            .first()
            .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)?;
        if word.group().expand {
            return None;
        }
        value.to_vec()
    };
    Some(script)
}
