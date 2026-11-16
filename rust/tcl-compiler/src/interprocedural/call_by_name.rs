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

//! Caller-frame names passed by name: to a user procedure that reads or writes them through `upvar`, or to an opaque callee.

use super::{InterproceduralAnalysis, ProcArgTrait, ProcIndex, ProcParameterSummary};
use std::collections::HashSet;

/// Build a protective MAY index without choosing one colliding procedure.
#[must_use]
pub fn build_proc_index_from_summaries(ia: &InterproceduralAnalysis) -> ProcIndex {
    let mut index = ProcIndex::new();
    for summary in ia.procedures.values() {
        let entry = ProcParameterSummary {
            params: summary.params.clone(),
            param_traits: summary.param_traits.clone(),
            param_defaults: summary.param_defaults.clone(),
        };
        for key in tcl_syntax::naming::key_suffixes(&summary.qualified_name) {
            index.entry(key).or_default().push(entry.clone());
        }
    }
    index
}

/// Exact original call/formal/registry caller-link advice for diagnostics.
/// This projection never publishes a runtime read or a store-removal licence.
pub(crate) fn collect_positioned_call_by_name_reads(
    cfg: &crate::cfg::Function,
    registry: &tcl_registry::CommandRegistry,
    identities: &crate::realm::CommandBindingRealm,
) -> HashSet<String> {
    let bindings = identities.source_bindings_ref();
    let mut output = HashSet::new();
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        let Some(tokens) = statement
            .tokens()
            .filter(|tokens| tokens.synthetic.is_none())
        else {
            continue;
        };
        output.extend(
            identities
                .declaration_caller_alias_arguments(tokens, registry)
                .unwrap_or_default(),
        );
        let Some(binding) = tokens.source_binding.as_ref() else {
            continue;
        };
        if binding
            .original_invocation_metadata_for_owner(
                tokens,
                bindings.source_metadata_owner(),
                registry,
            )
            .is_err()
        {
            continue;
        }
        let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
            continue;
        };
        let Some(calls) = crate::word_subst::checked_lifted_calls(tokens, config) else {
            continue;
        };
        for call in calls {
            if let Some(tokens) = call.tokens {
                output.extend(
                    identities
                        .declaration_caller_alias_arguments(&tokens, registry)
                        .unwrap_or_default(),
                );
            }
        }
    }
    output
}

/// Record any literal-name argument landing on a callee param that
/// carries `VarRead` / `VarWrite` (a call-by-name read/write), and the
/// default such a param binds where the call omits it.
fn add_call_by_name(cmd: &str, args: &[String], index: &ProcIndex, out: &mut HashSet<String>) {
    let candidates = (!cmd.is_empty() && !cmd.contains(['$', '[']))
        .then(|| index.get(cmd))
        .flatten();
    let Some(candidates) = candidates.filter(|candidates| !candidates.is_empty()) else {
        // A missing key supplies no effect-free callee. These literal passed
        // names may be observed; opaque hidden reads remain the CFG owner's
        // separate barrier, not an absence conclusion from this projection.
        out.extend(args.iter().filter(|arg| is_literal_var_name(arg)).cloned());
        return;
    };
    for candidate in candidates {
        for (i, pname) in candidate.params.iter().enumerate() {
            let is_var = candidate.param_traits.get(pname).is_some_and(|traits| {
                traits.contains(&ProcArgTrait::VarRead) || traits.contains(&ProcArgTrait::VarWrite)
            });
            let Some(arg) = args.get(i).or_else(|| candidate.param_defaults.get(pname)) else {
                break;
            };
            if is_var && is_literal_var_name(arg) {
                out.insert(arg.clone());
            }
        }
    }
}

/// Scan a whole-value `[cmd …]` command substitution (the dominant
/// tcllib call-by-name shape, `set len [asnPeekTag data tag type
/// dummy]`).  A simple whitespace split suffices — the call-by-name
/// args are literal names, and [`add_call_by_name`] rejects anything
/// that isn't.
fn scan_value_cmd_subst(text: &str, index: &ProcIndex, out: &mut HashSet<String>) {
    let Some(inner) = text
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
    else {
        return;
    };
    let mut words = inner.split_whitespace();
    if let Some(cmd) = words.next() {
        let args: Vec<String> = words.map(str::to_owned).collect();
        add_call_by_name(cmd, &args, index, out);
    }
}

/// Caller-local variable names passed *by name* to a user proc that
/// consumes them via `upvar` (call-by-name) — these must NOT be flagged
/// dead / unused (W211 / W220 / O109 / O126).
///
/// Scans direct `Call` / `Barrier` statements plus a `[cmd …]`
/// substitution that is the whole value of an `AssignValue` /
/// `AssignExpr` / `Return`.
#[must_use]
pub fn collect_call_by_name_reads(
    cfg: &crate::cfg::Function,
    index: &ProcIndex,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut out = HashSet::new();
    if index.is_empty() {
        return out;
    }
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if !stmt.is_executable_invocation() {
                continue;
            }
            match stmt {
                Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. } => {
                    add_call_by_name(command, args, index, &mut out);
                }
                Statement::AssignValue { value, .. } => {
                    scan_value_cmd_subst(value, index, &mut out);
                }
                Statement::Return { value: Some(v), .. } => {
                    scan_value_cmd_subst(v, index, &mut out);
                }
                Statement::AssignExpr {
                    expr: crate::expr_ast::ExprNode::Command { text, .. },
                    ..
                } => {
                    scan_value_cmd_subst(text, index, &mut out);
                }
                _ => {}
            }
        }
    }
    out
}

/// Whether `word` is a plain, literal variable *name* — the only argument
/// shape whose call-by-name meaning is knowable.
///
/// The same test [`add_call_by_name`] applies to a resolvable callee's
/// name-role argument, factored out so the opaque-callee scan below cannot
/// drift from it.
fn is_literal_var_name(word: &str) -> bool {
    !word.is_empty()
        && !word.contains(['$', '['])
        && word
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
}

/// Caller-frame names a call to an **opaque callee** — a command this
/// compilation unit can resolve neither in the registry nor to any
/// definition of its own — may create in the calling frame.
///
/// A Tcl procedure's `upvar 1 $param local` reaches the frame of whoever
/// called it, so a helper defined in *another file* creates the caller's
/// variable just as one defined here does.  C Tcl, tclsh 9.0.4 and 8.6.16
/// (identical), for a ticklecharts-style layout — `setdef` in `utils.tcl`,
/// the caller in `options.tcl`, tied together by a `pkgIndex.tcl`:
///
/// ```text
/// proc demo::setdef {d key args} { upvar 1 $d _dict; … dict set _dict … }
/// proc demo::build {items} { setdef options name …; dict get $options name }
/// demo::build {a b c}   → {nothing str no} …   — `options` exists, unassigned here
/// ```
///
/// The per-proc frame-effect summary ([`crate::cfg_builder::upvar_info`])
/// is built from one module, so it holds nothing at all for such a callee
/// and the read looked like `W210` read-before-set.  The honest answer is
/// that this unit cannot tell: the callee's body is not here.  So each
/// **literal, name-shaped argument word** of the call — the only words that
/// could name a caller-frame variable — is reported as possibly-defined,
/// and the read-before-set emitters abstain for exactly those names.
///
/// Deliberately narrow in three ways, so this is an abstention and not a
/// blanket silence:
///
/// * only names the call **spells** are covered — an unrelated local read
///   in the same frame still reports (its true-positive control lives in
///   `tests/caller_frame_effects.rs`);
/// * only *literal* words qualify — a `$x` / `[f]` argument names a
///   variable nothing here can identify, the same abstention direction
///   `is_plain_var_name` takes on the navigation side;
/// * only *opaque* callees qualify — a registry command has a declared
///   frame effect, and a procedure defined in this unit has a real summary
///   which is trusted per-name (so a same-file helper without an `upvar`
///   keeps its reads reporting). A head this unit can resolve is opaque all
///   the same where the flow graph marks the call as one to code the module
///   cannot see ([`crate::ir::SyntheticMarker::UnseenCall`]): an alias of
///   `upvar`, a rename's target, the unresolved-command handler reached by
///   another name.
///
/// `resolvable` answers "can this unit resolve that command head?", which
/// the analyser supplies from the dialect profile plus its own definition
/// tables — no command name appears here.
#[must_use]
pub fn collect_opaque_callee_name_args(
    cfg: &crate::cfg::Function,
    resolvable: &dyn Fn(&str) -> bool,
) -> HashSet<String> {
    collect_positioned_opaque_callee_name_args(cfg, &|statement| match statement {
        crate::ir::Statement::Call { command, .. }
        | crate::ir::Statement::Barrier { command, .. } => resolvable(command),
        _ => false,
    })
}

/// Opaque-callee source arguments at each original executable invocation.
/// The consumer retains its actual statement/tokens and independently decides
/// body availability; a final display-name set cannot supply positioned lookup.
#[must_use]
pub fn collect_positioned_opaque_callee_name_args(
    cfg: &crate::cfg::Function,
    resolvable: &dyn Fn(&crate::ir::Statement) -> bool,
) -> HashSet<String> {
    collect_executable_opaque_callee_name_args(cfg, resolvable, &|_| true)
}

/// [`collect_positioned_opaque_callee_name_args`] over the blocks `executable`
/// admits: a call in a block that never runs binds nothing.
#[must_use]
pub fn collect_executable_opaque_callee_name_args(
    cfg: &crate::cfg::Function,
    resolvable: &dyn Fn(&crate::ir::Statement) -> bool,
    executable: &dyn Fn(&crate::cfg::BlockId) -> bool,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut out = HashSet::new();
    for (id, block) in &cfg.blocks {
        if !executable(id) {
            continue;
        }
        for (index, stmt) in block.statements.iter().enumerate() {
            if !stmt.is_executable_invocation()
                || stmt
                    .tokens()
                    .is_some_and(|tokens| !tokens.may_dispatch_source_command())
            {
                continue;
            }
            let (Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. }) =
                stmt
            else {
                continue;
            };
            // A dynamic head (`$cmd …`) is handled by the dynamic-name
            // barrier, not here; an empty head is not a call.
            if command.is_empty() || command.contains(['$', '[']) {
                continue;
            }
            let marked_unseen = block.statements[index + 1..]
                .iter()
                .take_while(|next| next.span() == stmt.span() && next.synthetic_marker().is_some())
                .any(crate::ssa::is_unseen_call_marker);
            if resolvable(stmt) && !marked_unseen {
                continue;
            }
            out.extend(args.iter().filter(|a| is_literal_var_name(a)).cloned());
        }
    }
    out
}
