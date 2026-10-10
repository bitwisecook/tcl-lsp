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

//! Constant-`$cmd` dispatch settlement.
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
//! span marks its indirect invocation `rename_safe: false`, so the rename
//! providers abstain for that symbol rather than emit an edit set that
//! leaves the variable still holding the old name.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::analyser::state::{Analyser, ConstDispatchSite};
use crate::command_binding::SourceCommandReference;
use crate::signature_scan::types::SignatureCommandInvocation;
use crate::value_provenance::{ValueContributor, const_contributors};

/// Select the actual first list value for an expanded command prefix.
/// The native list owner parses under the selected grammar. A source span is
/// retained only when the first element is byte-exact in the contributor.
fn command_component(
    contributor: &ValueContributor,
    expanded: bool,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<ValueContributor> {
    if !expanded {
        return Some(contributor.clone());
    }
    let values = rules.split_list(&contributor.value).ok()?;
    let value = values.first()?.to_string();
    let literal_span =
        tcl_syntax::list::find_element_with_syntax(&contributor.value, 0, rules.list)
            .ok()
            .flatten()
            .filter(|element| element.literal)
            .and_then(|element| {
                let span = contributor.literal_span?;
                let start = span
                    .start()
                    .checked_add(u32::try_from(element.value.start).ok()?)?;
                let end = span
                    .start()
                    .checked_add(u32::try_from(element.value.end).ok()?)?;
                (end <= span.end() && contributor.value.get(element.value)? == value)
                    .then(|| tcl_lexer::Span::new(start, end))
            });
    Some(ValueContributor {
        value,
        literal_span,
    })
}

/// Settle one pending dispatch site against `cu`'s flow-sensitive value
/// model, appending its settled invocations to `settled`.  Extracted from
/// [`Analyser::settle_const_dispatches`] to keep that function within the
/// line budget; see its doc comment for the two-invocation-kinds contract
/// this builds.
fn settle_one_site(
    cu: &crate::compilation_unit::CompilationUnit,
    site: &ConstDispatchSite,
    analyser: &Analyser,
    settled: &mut Vec<SignatureCommandInvocation>,
) {
    // A write trace can mutate the variable at any read — the
    // reaching-definition walk cannot see the trace callback's writes, so
    // a traced head (or any dynamic variable trace in the module) abstains.
    if cu.ir_module.has_dynamic_variable_trace
        || cu.ir_module.traced_variables.contains(&site.var_name)
    {
        return;
    }
    let fu = cu.function_unit_at(site.span.start());
    let call_off = site.span.start();
    let config = analyser.lexer_config();
    let Some(contributors) = const_contributors(fu, call_off, &site.var_name, config) else {
        return;
    };
    let lookup = analyser.head_identities.invocation_at_source("", call_off);
    // Group the contributors by the user command their value resolves to
    // at *this* site's namespace context.  A value resolving to a builtin
    // or to nothing contributes no reference (a builtin carries no
    // navigable definition; an unknown value abstains) — and, being a
    // separate may-path, it does not block the other targets.
    // With an expanded head (`{*}$cmd args…`) the value is a command
    // *prefix*: its first whitespace-delimited element names the command
    // and the writable provenance narrows to that element's sub-span
    // within the defining literal.  A plain `$cmd` head uses the whole
    // value as the name.
    let mut by_target: HashMap<SourceCommandReference, Vec<ValueContributor>> = HashMap::new();
    let mut order: Vec<SourceCommandReference> = Vec::new();
    for c in &contributors {
        let Some(c) = command_component(
            c,
            site.head_expanded,
            tcl_syntax::word_rules::WordValueRules::of_profile(Some(analyser.profile)),
        ) else {
            continue;
        };
        let Some(reference) = lookup.command_reference(&c.value) else {
            continue;
        };
        if !reference.is_user_command() {
            continue;
        }
        if !by_target.contains_key(&reference) {
            order.push(reference.clone());
        }
        by_target.entry(reference).or_default().push(c);
    }
    for winner in order {
        let group = &by_target[&winner];
        let rename_safe = group.iter().all(|c| c.literal_span.is_some());
        let written = &group[0].value;
        let reference = &winner;
        let mut invocation = SignatureCommandInvocation::written(written.clone(), site.span, None);
        invocation.retain_reference(reference);
        invocation.indirect = true;
        invocation.rename_safe = rename_safe;
        settled.push(invocation);
        for contributor in group {
            let Some(span) = contributor.literal_span else {
                continue;
            };
            let mut invocation =
                SignatureCommandInvocation::written(contributor.value.clone(), span, None);
            invocation.lookup =
                crate::signature_scan::types::SignatureCommandLookup::DeferredReference;
            invocation.retain_reference(reference);
            settled.push(invocation);
        }
    }
}

/// Contributor edit proof supplements an existing exact executed-head receipt.
/// It cannot replace another temporal reference or introduce a second row for
/// that same execution point. Literal contributor records remain separate.
fn reconcile_positioned_dispatches(
    existing: &mut [SignatureCommandInvocation],
    settled: &mut Vec<SignatureCommandInvocation>,
) {
    settled.retain(|candidate| {
        if !candidate.indirect || candidate.resolved_command_reference.is_none() {
            return true;
        }
        let Some(original) = existing.iter_mut().find(|original| {
            original.lookup.is_execution_site()
                && original.indirect
                && original.range == candidate.range
                && original.resolved_command_reference == candidate.resolved_command_reference
        }) else {
            return true;
        };
        // The source point selected this reference already. Only its complete
        // source-exact contributor proof adds rename safety for the value words.
        original.rename_safe = candidate.rename_safe;
        false
    });
}

impl Analyser {
    /// Settle the pending `$cmd`-head dispatch sites against `cu`'s
    /// flow-sensitive value model, appending the settled invocations to
    /// `result.command_invocations`.  Runs inside the CFG/SSA phase —
    /// after positioned source lookup has been retained and before result-order
    /// canonicalisation.
    pub(in crate::analyser) fn settle_const_dispatches(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.pending_const_dispatches.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.pending_const_dispatches);

        let mut settled: Vec<SignatureCommandInvocation> = Vec::new();
        for site in &sites {
            settle_one_site(cu, site, self, &mut settled);
        }
        // One editable literal cannot claim a unique implementation when its
        // consuming worlds disagree. Keep every navigation alternative, but
        // withhold rename authority at that shared literal and its indirect sites.
        let mut literal_definitions = HashMap::new();
        let mut ambiguous = HashSet::new();
        for invocation in settled.iter().filter(|invocation| !invocation.indirect) {
            let span = (invocation.range.start(), invocation.range.end());
            let target = invocation.resolved_command_reference.clone();
            if literal_definitions
                .insert(span, target.clone())
                .is_some_and(|previous| previous != target)
            {
                ambiguous.insert(span);
            }
        }
        let unsafe_targets: HashSet<_> = settled
            .iter()
            .filter(|invocation| {
                !invocation.indirect
                    && ambiguous.contains(&(invocation.range.start(), invocation.range.end()))
            })
            .map(|invocation| invocation.resolved_command_reference.clone())
            .collect();
        for invocation in &mut settled {
            if (invocation.indirect
                && unsafe_targets.contains(&invocation.resolved_command_reference))
                || ambiguous.contains(&(invocation.range.start(), invocation.range.end()))
            {
                invocation.rename_safe = false;
            }
        }
        // A literal feeding several dispatch sites (or several sites
        // resolving the same head) settles once per distinct
        // (span, target, kind).
        let mut seen = HashSet::new();
        settled.retain(|inv| {
            seen.insert((
                inv.range.start(),
                inv.range.end(),
                inv.indirect,
                inv.resolved_command_reference.clone(),
            ))
        });
        reconcile_positioned_dispatches(&mut self.result.command_invocations, &mut settled);
        self.result.command_invocations.extend(settled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::word_rules::WordValueRules;

    fn contributor(value: &str) -> ValueContributor {
        ValueContributor {
            value: value.to_owned(),
            literal_span: Some(tcl_lexer::Span::new(
                100,
                100 + u32::try_from(value.len()).unwrap(),
            )),
        }
    }

    #[test]
    fn expanded_prefix_uses_native_list_values_and_exact_element_spans() {
        for rules in [WordValueRules::TCL, WordValueRules::JIM] {
            let selected = command_component(&contributor("{has space} arg"), true, rules).unwrap();
            assert_eq!(selected.value, "has space");
            assert_eq!(selected.literal_span, Some(tcl_lexer::Span::new(101, 110)));
            let selected = command_component(&contributor("has\\ space arg"), true, rules).unwrap();
            assert_eq!(selected.value, "has space");
            assert!(selected.literal_span.is_none());
        }
    }

    #[test]
    fn expanded_prefix_preserves_jim_leniency_and_native_source_spans() {
        let value = contributor("{target");
        assert!(command_component(&value, true, WordValueRules::TCL).is_none());
        let selected = command_component(&value, true, WordValueRules::JIM).unwrap();
        assert_eq!(selected.value, "target");
        assert_eq!(selected.literal_span, Some(tcl_lexer::Span::new(101, 107)));
        assert!(command_component(&contributor(""), true, WordValueRules::JIM).is_none());
    }
}
