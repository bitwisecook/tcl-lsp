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

//! Constant-`$cmd` dispatch settlement (M7, reworked for issue #945
//! faults 1–2).
//!
//! Settles the `$var`-head dispatch sites the walk recorded against the
//! compiler's **flow-sensitive value model**
//! ([`crate::value_provenance::const_contributors`]): the value facts at
//! each site are the SSA use-version's reaching constant definitions —
//! a branch join yields *every* arm's constant as a may-target (never
//! the lexically last write), and anything unprovable (computed values,
//! `upvar`/trace writes, proc parameters, complexity-guarded bodies)
//! abstains soundly.
//!
//! A settled site emits two kinds of [`SignatureCommandInvocation`]:
//!
//! * an **indirect** invocation at the `$cmd` head per resolved user
//!   command — navigation (references / go-to-definition / call
//!   hierarchy) reaches the dispatched command, but the span carries no
//!   written name so no rename may rewrite it;
//! * a **writable** literal-anchored invocation at each contributing
//!   constant definition whose value word is source-exact — the rename
//!   edit that keeps the dispatch alive (`set cmd target` →
//!   `set cmd renamed`).
//!
//! A resolved target any of whose contributors has *no* exact source
//! span marks its indirect invocation `rename_safe: false`, so the
//! rename providers abstain for that symbol instead of emitting an edit
//! set that leaves the variable holding the old name (fault 1's
//! corruption, inverted).

use std::collections::HashMap;

use crate::analyser::state::Analyser;
use crate::signature_scan::types::SignatureCommandInvocation;
use crate::value_provenance::{ValueContributor, const_contributors};

/// The command-naming component of one contributor: the whole value for
/// a plain `$cmd` head, or — for an expanded `{*}$cmd` head — the first
/// whitespace-delimited list element, with the writable span narrowed to
/// that element inside the defining literal.  `None` abstains: an
/// expanded head whose first element carries substitution syntax or list
/// quoting (braces, quotes, backslashes) has no exact writable
/// component this simple scan can prove.
fn command_component(c: &ValueContributor, head_expanded: bool) -> Option<ValueContributor> {
    if !head_expanded {
        return Some(c.clone());
    }
    let trimmed_start = c.value.len() - c.value.trim_start().len();
    let rest = &c.value[trimmed_start..];
    let first_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let first = &rest[..first_len];
    if first.is_empty() || first.contains(['{', '}', '"', '\\', '$', '[', ']']) {
        return None;
    }
    let literal_span = c.literal_span.and_then(|span| {
        let start = span.start() + u32::try_from(trimmed_start).ok()?;
        let end = start + u32::try_from(first_len).ok()?;
        (end <= span.end()).then(|| tcl_lexer::Span::new(start, end))
    });
    Some(ValueContributor {
        value: first.to_string(),
        literal_span,
    })
}

impl Analyser {
    /// Settle the pending `$cmd`-head dispatch sites against `cu`'s
    /// flow-sensitive value model, appending the settled invocations to
    /// `result.command_invocations`.  Runs inside the CFG/SSA phase —
    /// after `finalise_invocation_resolutions` (whose namespace-path and
    /// definition tables it reads) and before the result-order
    /// canonicalisation.
    pub(in crate::analyser) fn settle_const_dispatches(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.pending_const_dispatches.is_empty() {
            return;
        }
        let _ = self.builtin_command_names();
        let Some(builtins) = self.builtin_names.as_ref() else {
            self.pending_const_dispatches.clear();
            return;
        };
        let sites = std::mem::take(&mut self.pending_const_dispatches);

        let renamed_away: std::collections::HashSet<&String> = self
            .result
            .renamed_commands
            .values()
            .chain(self.deleted_commands.keys())
            .collect();
        let result = &self.result;
        let known = |qualified: &str| {
            result.all_procs.contains_key(qualified)
                || result.all_classes.contains_key(qualified)
                || result.command_aliases.contains_key(qualified)
                || result.renamed_commands.contains_key(qualified)
                || (builtins.contains(qualified.trim_start_matches(':'))
                    && !renamed_away.contains(&qualified.to_string()))
        };
        let user_defined = |qualified: &str| {
            result.all_procs.contains_key(qualified)
                || result.all_classes.contains_key(qualified)
                || result.command_aliases.contains_key(qualified)
                || result.renamed_commands.contains_key(qualified)
        };

        let mut settled: Vec<SignatureCommandInvocation> = Vec::new();
        for site in &sites {
            // A write trace can mutate the variable at any read — the
            // reaching-definition walk cannot see the trace callback's
            // writes, so a traced head (or any dynamic variable trace in
            // the module) abstains (issue #945 fault 2's trace arm).
            if cu.ir_module.has_dynamic_variable_trace
                || cu.ir_module.traced_variables.contains(&site.var_name)
            {
                continue;
            }
            let fu = cu.function_unit_at(site.span.start());
            let Some(contributors) = const_contributors(fu, site.span.start(), &site.var_name)
            else {
                continue;
            };
            let path: &[String] = result
                .namespace_paths
                .get(&site.ns)
                .map_or(&[], Vec::as_slice);
            // Group the contributors by the user command their value
            // resolves to at *this* site's namespace context.  A value
            // resolving to a builtin or to nothing contributes no
            // reference (a builtin carries no navigable definition; an
            // unknown value abstains) — and, being a separate may-path,
            // it does not block the other targets.
            // With an expanded head (`{*}$cmd args…`) the value is a
            // command *prefix*: its first whitespace-delimited element
            // names the command and the writable provenance narrows to
            // that element's sub-span within the defining literal.  A
            // plain `$cmd` head uses the whole value as the name.
            let mut by_target: HashMap<String, Vec<ValueContributor>> = HashMap::new();
            let mut order: Vec<String> = Vec::new();
            for c in &contributors {
                let Some(c) = command_component(c, site.head_expanded) else {
                    continue;
                };
                if c.value.trim().is_empty() || crate::naming::is_dynamic_word(&c.value) {
                    continue;
                }
                let Some(winner) =
                    crate::naming::resolve_command_with(&site.ns, path, &c.value, known)
                else {
                    continue;
                };
                if !user_defined(&winner) {
                    continue;
                }
                if !by_target.contains_key(&winner) {
                    order.push(winner.clone());
                }
                by_target.entry(winner).or_default().push(c);
            }
            for winner in order {
                let group = &by_target[&winner];
                let rename_safe = group.iter().all(|c| c.literal_span.is_some());
                let written = &group[0].value;
                settled.push(SignatureCommandInvocation {
                    name: written.clone(),
                    range: site.span,
                    resolved_qualified_name: Some(winner.clone()),
                    resolution_candidates: crate::naming::command_resolution_candidates(
                        &site.ns, path, written,
                    ),
                    argc: None,
                    callback_arity: None,
                    callback_baked_args: 0,
                    indirect: true,
                    rename_safe,
                    existence_probe: false,
                });
                for c in group {
                    let Some(span) = c.literal_span else { continue };
                    settled.push(SignatureCommandInvocation {
                        name: c.value.clone(),
                        range: span,
                        resolved_qualified_name: Some(winner.clone()),
                        resolution_candidates: crate::naming::command_resolution_candidates(
                            &site.ns, path, &c.value,
                        ),
                        argc: None,
                        callback_arity: None,
                        callback_baked_args: 0,
                        indirect: false,
                        rename_safe: true,
                        existence_probe: false,
                    });
                }
            }
        }
        // A literal feeding several dispatch sites (or several sites
        // resolving the same head) settles once per distinct
        // (span, target, kind).
        let mut seen: std::collections::HashSet<(u32, u32, String, bool)> =
            std::collections::HashSet::new();
        settled.retain(|inv| {
            seen.insert((
                inv.range.start(),
                inv.range.end(),
                inv.resolved_qualified_name.clone().unwrap_or_default(),
                inv.indirect,
            ))
        });
        self.result.command_invocations.extend(settled);
    }
}
