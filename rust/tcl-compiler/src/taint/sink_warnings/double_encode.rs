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

//! T106: a tainted value encoded twice by the same transform.

use super::super::{
    DiagCode, FxHashSet, HashMap, HashSet, Span, Symbol, TaintCtx, TaintLattice, TaintWarning,
    ValueKey, double_encode_label, reg_colour, transform_colour, word_taint,
};
use super::source_reads::reference_root_with_config;

/// **T106.** Emit a double-encoding warning when a tainted value that
/// already carries a command's `taint_double_encode_colour` is passed
/// through that command again (e.g. `uri::encode [uri::encode $x]`).
/// Emit T106 (double-encode) warnings — one warning per variable.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn emit_double_encode_warnings<S: std::hash::BuildHasher>(
    command: &str,
    call_args: &[String],
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    span: Span,
    warnings: &mut Vec<TaintWarning>,
    ctx: TaintCtx<'_>,
    quoted_uses: &HashSet<Symbol>,
) {
    let Some(spec) = ctx.registry.get(command) else {
        return;
    };
    let Some(dup_colour) = spec.taint_double_encode_colour.map(reg_colour) else {
        return;
    };
    // A conditional transform encodes only on the calls that satisfy it, so
    // only those can encode twice (`regsub` escapes only in its quoting form).
    if spec.taint_transform_when.is_some() {
        let args: Vec<&str> = call_args.iter().map(String::as_str).collect();
        if transform_colour(ctx.registry, command, &args).is_none() {
            return;
        }
    }
    let label = double_encode_label(dup_colour);
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    // Named-variable double-encode: `set e [enc $x]; enc $e`.
    for (&sym, &ver) in uses {
        let name = ctx.ssa.var_name(sym);
        if taints
            .get(&(sym, ver))
            .is_some_and(|value| value.is_conservative_global_seed_only())
            || emitted.contains(&sym)
            || quoted_uses.contains(&sym)
        {
            continue;
        }
        let t = taints
            .get(&(sym, ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if t.is_tainted() && t.colours.intersects(dup_colour) {
            warnings.push(TaintWarning {
                source_sink: None,
                span,
                variable: name.to_owned(),
                sink_command: command.to_owned(),
                code: DiagCode::T106,
                message: format!(
                    "Variable ${name} is already {label}; passing through {command} \
                     double-encodes the value"
                ),
                replacement: None,
                fixes: Vec::new(),
            });
            emitted.insert(sym);
        }
    }
    // Nested double-encode with no named intermediate: `enc [enc $x]`. The
    // inner encoder's result colour never lands on a named variable, so scan
    // each argument *word* — a command substitution whose value already carries
    // the colour is the same hazard. Emit once for the statement, and skip pure
    // `$var` words (already covered by the named-variable loop above).
    if emitted.is_empty() {
        for arg in call_args {
            if reference_root_with_config(arg.trim(), ctx.source.config).is_some() {
                continue;
            }
            let t = word_taint(arg, uses, taints, ctx);
            if t.is_tainted() && t.colours.intersects(dup_colour) {
                warnings.push(TaintWarning {
                    source_sink: None,
                    span,
                    variable: String::new(),
                    sink_command: command.to_owned(),
                    code: DiagCode::T106,
                    message: format!(
                        "The value passed to {command} is already {label}; passing it \
                         through {command} again double-encodes it"
                    ),
                    replacement: None,
                    fixes: Vec::new(),
                });
                break;
            }
        }
    }
}
