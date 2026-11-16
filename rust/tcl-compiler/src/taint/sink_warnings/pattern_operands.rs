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

//! T103: tainted values in regular-expression and glob pattern operands.

use super::super::{
    CommandRegistry, CommandTokens, DiagCode, HashSet, Span, TaintColour, TaintCtx, TaintLattice,
    TaintScan, TaintSourceContext, TaintWarning, scan_argument_names, word_taint,
};
use super::sink_wrappers::var_wrapper_colours;
use super::source_reads::operand_read_taint;

/// Emit `T103` (regex injection / `ReDoS`) for a tainted variable sitting in
/// the regex-pattern argument of a regex command (`regexp` / `regsub` —
/// gated on `pattern_type == Regex`). Suppressed when the
/// value carries the `REGEX_LITERAL` colour.
pub(in super::super) fn emit_regexp_pattern_warnings<S: std::hash::BuildHasher>(
    env: &TaintScan<'_, S>,
    span: Span,
    registry: &CommandRegistry,
    warnings: &mut Vec<TaintWarning>,
    tokens: Option<&CommandTokens>,
    source: TaintSourceContext<'_>,
) {
    let Some(source_tokens) = tokens else { return };
    let metadata = match source.metadata {
        Some(context) => Some(context),
        None if source.allows_nominal_metadata() => None,
        None => return,
    };
    let Some(normal) =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry,
            metadata,
            source_tokens,
        )
    else {
        return;
    };
    let Some(pattern_indices) = normal.possible_pattern_source_argument_indices(registry) else {
        return;
    };
    let display_command = normal.diagnostic_command();
    let mut possible_names = HashSet::new();
    for pattern_idx in pattern_indices {
        let first_warning = warnings.len();
        emit_pattern_operand_warnings(
            env,
            span,
            registry,
            warnings,
            source_tokens,
            display_command,
            pattern_idx,
        );
        let mut index = first_warning;
        while index < warnings.len() {
            if possible_names.insert(warnings[index].variable.clone()) {
                index += 1;
            } else {
                warnings.remove(index);
            }
        }
    }
}

fn emit_pattern_operand_warnings<S: std::hash::BuildHasher>(
    env: &TaintScan<'_, S>,
    span: Span,
    registry: &CommandRegistry,
    warnings: &mut Vec<TaintWarning>,
    source_tokens: &CommandTokens,
    display_command: &str,
    pattern_idx: usize,
) {
    let args = source_tokens.argv_texts.get(1..).unwrap_or(&[]);
    let Some(arg) = args.get(pattern_idx) else {
        return;
    };
    let mut names: Vec<String> = scan_argument_names(env, arg);
    names.sort_unstable();
    names.dedup();
    for var in names {
        if env.ssa.var_symbol(&var).is_some_and(|symbol| {
            env.quoted_uses
                .is_some_and(|quoted| quoted.contains(&symbol))
        }) {
            continue;
        }
        let Some(t) = operand_read_taint(env, &var, source_tokens.words().get(pattern_idx + 1))
        else {
            continue;
        };
        if !t.is_tainted() {
            continue;
        }
        // A literal-regex colour proves the pattern is trusted, whether the
        // value carries it already or is quoted inline in the pattern slot.
        // Scoped to that slot: the same variable used elsewhere in the call is
        // not in a pattern position and has no bearing on this hazard.
        let wrapped = var_wrapper_colours(
            registry,
            env.source,
            &args[pattern_idx..=pattern_idx],
            &var,
            Some(source_tokens),
            pattern_idx,
        );
        let t = wrapped.map_or(t, |colours| t.shape_unproven().with(colours));
        if t.colours.intersects(TaintColour::REGEX_LITERAL) {
            continue;
        }
        if wrapped.is_some()
            && summarised_pattern_taint(env, registry, source_tokens, pattern_idx, arg).is_some_and(
                |word| !word.is_tainted() || word.colours.intersects(TaintColour::REGEX_LITERAL),
            )
        {
            continue;
        }
        warnings.push(TaintWarning {
            source_sink: None,
            span,
            variable: var.clone(),
            sink_command: display_command.to_owned(),
            code: DiagCode::T103,
            message: format!(
                "Tainted variable ${var} in regexp pattern position ({display_command}); \
                 risk of regex injection or ReDoS"
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// The pattern word's taint under the solved procedure return summaries, so
/// an inline call to a quoting procedure proves what its assigned result would.
fn summarised_pattern_taint<S: std::hash::BuildHasher>(
    env: &TaintScan<'_, S>,
    registry: &CommandRegistry,
    source_tokens: &CommandTokens,
    pattern_idx: usize,
    word: &str,
) -> Option<TaintLattice> {
    let summaries = env.source.procedure_summaries()?;
    let known: HashSet<crate::ir::ProcedureId> = summaries.keys().copied().collect();
    let ctx = TaintCtx {
        registry,
        ssa: env.ssa,
        interproc: None,
        known_procs: Some(&known),
        dialect: env.source.dialect(),
        source: env.source,
        taint_summaries: Some(summaries),
        instance_classes: None,
        source_position: None,
        source_tokens: Some(source_tokens),
        word_offset: source_tokens
            .argv
            .get(pattern_idx + 1)
            .map(|span| span.start()),
        source_word: source_tokens.word_exprs.get(pattern_idx + 1),
    };
    Some(word_taint(word, env.uses, env.taints, ctx))
}
