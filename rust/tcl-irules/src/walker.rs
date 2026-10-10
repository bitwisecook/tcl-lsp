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

//! Readonly object-reference candidates from the shared original source context.
//! Literal source values remain candidates; no set-value propagation, live
//! handler, evaluated target or deletion-safety proof is issued here.

use std::collections::HashSet;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

fn category_for_kinds(kinds: &[&str]) -> IrulesObjectReferenceCategory {
    let tables = crate::tables();
    if kinds.iter().any(|kind| {
        tables
            .ltm_pool_kinds
            .iter()
            .chain(&tables.gtm_pool_kinds)
            .any(|candidate| candidate == kind)
    }) {
        IrulesObjectReferenceCategory::Pool
    } else if kinds.iter().any(|kind| {
        tables
            .data_group_kinds
            .iter()
            .any(|candidate| candidate == kind)
    }) {
        IrulesObjectReferenceCategory::DataGroup
    } else {
        IrulesObjectReferenceCategory::Other
    }
}

/// Configuration-reference presentation category; no object identity is issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesObjectReferenceCategory {
    Pool,
    DataGroup,
    Other,
}

/// One readonly literal source-reference candidate, with original geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesObjectReference {
    /// Actual supported literal source value, not a propagated runtime value.
    pub name: String,
    /// Shared declarative object-kind candidates.
    pub kinds: Vec<&'static str>,
    /// Actual selected source-schema label for presentation.
    pub command: String,
    /// Source-schema label; this field supplies no live command identity.
    pub effective_command: String,
    /// Category derived from the exact shared object-kind template.
    pub category: IrulesObjectReferenceCategory,
    /// Original post-head argument ordinal.
    pub argument_index: usize,
    /// Genuine original operand source extent.
    pub range: Span,
}

/// Literal source-reference candidates from an independently retained context.
#[must_use]
pub fn extract_original_irules_source_object_references(
    source: &str,
    context: &crate::OriginalIrulesSourceContext,
    rule_module: Option<&str>,
) -> Vec<IrulesObjectReference> {
    // Implementation contract: naming.consumer.original-source-attachment-candidates
    // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
    if !context.matches_source(source) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for command in context.commands() {
        let Some(operands) = command
            .words()
            .with_source_schema(context.context_registry(), |schema| {
                crate::original_source_object_operands(schema, rule_module)
            })
        else {
            continue;
        };
        for operand in operands {
            let ordinal = operand.argument();
            let Some(name) = command
                .words()
                .arguments()
                .get(ordinal)
                .and_then(|value| value.literal_bytes())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .filter(|name| !name.contains('\0'))
            else {
                continue;
            };
            let Some(range) = command.words().operands().get(ordinal).and_then(Option::as_ref).map(tcl_compiler::registry_invocation::source_structure::OriginalOperandSource::span) else { continue; };
            let label = command.words().command().to_owned();
            out.push(IrulesObjectReference {
                name: name.to_owned(),
                kinds: operand.kinds().to_vec(),
                command: label.clone(),
                effective_command: label,
                category: category_for_kinds(operand.kinds()),
                argument_index: ordinal,
                range,
            });
        }
    }
    out.sort_by(|a, b| {
        (a.range.start(), a.range.end(), &a.name).cmp(&(b.range.start(), b.range.end(), &b.name))
    });
    out.dedup();
    out
}

/// Source-name ranges for presentation; no runtime object lookup is performed.
#[must_use]
pub fn object_ref_spans(source: &str, registry: &CommandRegistry) -> Vec<Span> {
    let mut spans = extract_irules_object_references(source, None, registry)
        .into_iter()
        .map(|reference| reference.range)
        .collect::<Vec<_>>();
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

/// Current event-associated literal source-reference candidates.
#[must_use]
pub fn extract_irules_object_references(
    source: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
) -> Vec<IrulesObjectReference> {
    crate::OriginalIrulesSourceContext::capture(source, registry).map_or_else(Vec::new, |context| {
        extract_original_irules_source_object_references(source, &context, rule_module)
    })
}

/// Literal candidates associated with an actual current event source descriptor.
#[must_use]
pub fn extract_irules_event_object_references(
    source: &str,
    event: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
) -> Vec<IrulesObjectReference> {
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    let spans = context
        .commands()
        .iter()
        .filter(|command| command.event_source().event().eq_ignore_ascii_case(event))
        .map(crate::source_context::OriginalIrulesSourceCommand::span)
        .collect::<Vec<_>>();
    extract_original_irules_source_object_references(source, &context, rule_module)
        .into_iter()
        .filter(|reference| {
            spans.iter().any(|span| {
                span.start() <= reference.range.start() && reference.range.end() <= span.end()
            })
        })
        .collect()
}

/// Compatibility source-range filter. DTO labels never select source schema;
/// every accepted extent is revalidated against the genuine current inventory.
#[must_use]
pub fn extract_irules_object_references_in_closure(
    source: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
    executable: &[crate::IrulesExecutableCommand],
) -> Vec<IrulesObjectReference> {
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    let selected = executable
        .iter()
        .map(|command| command.span)
        .collect::<HashSet<_>>();
    let spans = context
        .commands()
        .iter()
        .filter(|command| selected.contains(&command.span()))
        .map(crate::source_context::OriginalIrulesSourceCommand::span)
        .collect::<Vec<_>>();
    extract_original_irules_source_object_references(source, &context, rule_module)
        .into_iter()
        .filter(|reference| {
            spans.iter().any(|span| {
                span.start() <= reference.range.start() && reference.range.end() <= span.end()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extract object references from `source` against the profile-stamped
    /// iRules registry (`pool` / `snatpool` / `class` are dialect commands).
    fn refs(source: &str) -> Vec<IrulesObjectReference> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(
            tcl_dialect::DialectProfile::irules(),
        )
        .commands();
        extract_irules_object_references(source, None, registry)
    }

    #[test]
    fn object_refs_follow_only_reachable_valid_execution_regions() {
        let source = concat!(
            "pool /Common/top\n",
            "proc dormant {} { pool /Common/dormant }\n",
            "proc leaf {} { pool /Common/leaf }\n",
            "proc malformed {} { pool /Common/malformed } extra\n",
            "when BOGUS_EVENT { pool /Common/unknown }\n",
            "when HTTP_REQUEST { call leaf; pool /Common/live }\n",
        );
        let names: Vec<_> = refs(source)
            .into_iter()
            .map(|reference| reference.name)
            .collect();
        assert_eq!(names, ["/Common/leaf", "/Common/live"]);
    }

    #[test]
    fn extracts_pool_snatpool_and_datagroup_refs() {
        let source = "\n\
            when HTTP_REQUEST {\n\
            \x20   if [class match -- [HTTP::host] equals /Common/host_dg] {\n\
            \x20       snatpool /Common/sp1\n\
            \x20       pool /Common/web_pool\n\
            \x20   }\n\
            }\n";
        let by_name: Vec<(String, String)> = refs(source)
            .iter()
            .map(|r| (r.command.clone(), r.name.clone()))
            .collect();
        assert!(by_name.contains(&("class".to_owned(), "/Common/host_dg".to_owned())));
        assert!(by_name.contains(&("snatpool".to_owned(), "/Common/sp1".to_owned())));
        assert!(by_name.contains(&("pool".to_owned(), "/Common/web_pool".to_owned())));
    }

    /// Return readonly candidates; configuration object selection is independent.
    fn ref_names(source: &str) -> Vec<String> {
        refs(source).into_iter().map(|r| r.name).collect()
    }

    /// A `when` body invoking `caller /Common/web_pool`, after `prelude`.
    fn pool_rule(prelude: &str, caller: &str) -> String {
        format!("{prelude}when HTTP_REQUEST {{\n    {caller} /Common/web_pool\n}}\n")
    }

    #[test]
    fn unavailable_irules_alias_does_not_create_object_ref_command() {
        // F5 K36322151 disables `interp`, so this cannot create `p`.
        assert!(ref_names(&pool_rule("interp alias {} p {} pool\n", "p")).is_empty());
        assert!(ref_names(&pool_rule("set y 1\n", "p")).is_empty());
        assert_eq!(
            ref_names(&pool_rule("interp alias {} p {} pool\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
            "a failed alias cannot take the real pool command away"
        );
    }

    #[test]
    fn categories_follow_registry_returned_object_kinds() {
        let source = concat!(
            "when HTTP_REQUEST {\n",
            " set n [::active_members /Common/active]\n",
            " LB::reselect pool /Common/fallback\n",
            " set h [HSL::open -proto UDP -pool /Common/logging]\n",
            " class match x equals /Common/hosts\n",
            "}\n",
        );
        let refs = refs(source);
        for name in ["/Common/active", "/Common/fallback", "/Common/logging"] {
            let reference = refs
                .iter()
                .find(|reference| reference.name == name)
                .unwrap();
            assert_eq!(
                reference.category,
                IrulesObjectReferenceCategory::Pool,
                "{reference:?}"
            );
        }
        let datagroup = refs
            .iter()
            .find(|reference| reference.name == "/Common/hosts")
            .unwrap();
        assert_eq!(datagroup.category, IrulesObjectReferenceCategory::DataGroup);
    }

    #[test]
    fn authored_nominal_tombstone_withdraws_object_source_schema() {
        assert!(ref_names(&pool_rule("rename pool p\n", "p")).is_empty());
        assert!(
            ref_names(&pool_rule("rename pool p\n", "pool")).is_empty(),
            "source metadata cannot donate the old nominal schema after an explicit source cell barrier"
        );
    }

    #[test]
    fn object_refs_abstain_for_a_command_shadowed_by_a_user_proc() {
        assert!(
            ref_names(&pool_rule("proc pool {args} { return 1 }\n", "pool")).is_empty(),
            "a user `proc pool` takes the name over; its argument is not a pool name"
        );
        // Guard: the unshadowed command still resolves the reference.
        assert_eq!(
            ref_names(&pool_rule("set y 1\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
        );
    }

    #[test]
    fn object_refs_abstain_for_a_dynamic_binding() {
        assert!(
            ref_names(&pool_rule("rename $old p\n", "p")).is_empty(),
            "a dynamic rename must not make `p` an object-reference command"
        );
        assert_eq!(
            ref_names(&pool_rule("rename $old p\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
            "a dynamic rename must not take `pool`'s grammar away either"
        );
    }

    /// An unavailable `interp alias` cannot make `assign` a `set` command.
    #[test]
    fn unavailable_irules_alias_does_not_change_constant_propagation() {
        let source = "interp alias {} assign {} set\n\
                      when HTTP_REQUEST {\n\
                      assign p /Common/web_pool\n\
                      pool $p\n\
                      }\n";
        assert!(ref_names(source).is_empty());
    }

    #[test]
    fn command_substitution_set_effects_stay_in_the_live_scope() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      puts [set p /Common/new]\n\
                      pool $p\n\
                      }\n";
        assert!(
            ref_names(source).is_empty(),
            "source values do not certify runtime set/read continuity"
        );
    }

    #[test]
    fn ordered_sibling_command_substitutions_share_effects() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      puts [set p /Common/first] [set p /Common/second]\n\
                      pool $p\n\
                      }\n";
        assert!(
            ref_names(source).is_empty(),
            "source values do not certify sibling observer/mutation ordering"
        );
    }

    #[test]
    fn body_role_command_substitution_runs_once_in_the_live_scope() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      catch [set p /Common/new]\n\
                      pool $p\n\
                      }\n";
        assert!(
            ref_names(source).is_empty(),
            "source values do not certify runtime set/read continuity"
        );
    }

    #[test]
    fn expr_role_substitutions_run_live_without_duplicate_references() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      if [string length [set p /Common/new]] { set seen 1 }\n\
                      if [string length [active_members /Common/expr_pool]] { set seen 1 }\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), vec!["/Common/expr_pool".to_owned()]);
    }

    #[test]
    fn braced_expr_text_does_not_invent_object_references() {
        let source = "when HTTP_REQUEST {\n\
                      if {class match [HTTP::host] equals /Common/inert_dg} { set seen 1 }\n\
                      if [class match [HTTP::host] equals /Common/live_dg] { set seen 1 }\n\
                      }\n";
        let names = ref_names(source);
        assert_eq!(names, ["/Common/live_dg"]);
    }

    #[test]
    fn braced_expr_command_substitution_keeps_its_data_group_reference() {
        let source = "when HTTP_REQUEST {\n\
                      if {[class match [HTTP::host] equals /Common/braced_dg]} { set seen 1 }\n\
                      }\n";
        assert_eq!(ref_names(source), ["/Common/braced_dg"]);
    }

    /// Regression coverage: `walk`/`recurse_token`'s mutual recursion over
    /// nested command-substitution bodies is capped at `MAX_WALK_DEPTH`
    /// (128). 300 nested `[…]` command substitutions is
    /// comfortably past the cap; the assertion is that extraction returns
    /// at all, not what it returns.
    #[test]
    fn deeply_nested_command_substitution_does_not_crash() {
        const DEPTH: usize = 300;
        let mut source = "when HTTP_REQUEST {\n    set x ".to_owned();
        for _ in 0..DEPTH {
            source.push('[');
        }
        source.push_str("pool /Common/p");
        for _ in 0..DEPTH {
            source.push(']');
        }
        source.push_str("\n}\n");
        let _ = refs(&source);
    }

    #[test]
    fn extracts_refs_nested_in_body_and_command_substitution() {
        let source = "\n\
            when HTTP_REQUEST {\n\
            \x20   set count [active_members /Common/app_pool]\n\
            \x20   LB::reselect pool /Common/fallback_pool\n\
            }\n";
        let names: Vec<String> = refs(source).iter().map(|r| r.name.clone()).collect();
        assert!(names.contains(&"/Common/app_pool".to_owned()));
        assert!(names.contains(&"/Common/fallback_pool".to_owned()));
    }
}

#[cfg(test)]
mod case_list_tests {
    use super::extract_irules_object_references;
    use crate::{
        extract_irules_object_references_in_closure,
        extract_original_irules_source_object_references,
    };
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules())
    }

    /// An object referenced only from a `switch` arm must still be found.
    ///
    /// A `switch` case list is not an `ArgRole::Body`, so the walker never
    /// descended into it: a pool used only inside a `switch` arm — an entirely
    /// ordinary iRule — looked **unreferenced**.  For highlighting that meant the
    /// name read as a plain string; for the reference graph it meant
    /// `bigip-cleanup` would have seen a live pool as an orphan.
    #[test]
    fn objects_referenced_from_a_switch_arm_are_found() {
        let registry = reg();
        let src = "when HTTP_REQUEST {\n\
                   \x20   pool /Common/top\n\
                   \x20   switch -glob [HTTP::uri] {\n\
                   \x20       \"/api/*\" { pool /Common/in_switch }\n\
                   \x20       default  { pool /Common/fallback }\n\
                   \x20   }\n\
                   }\n";
        let names: Vec<String> = extract_irules_object_references(src, None, &registry)
            .into_iter()
            .map(|r| r.name)
            .collect();
        for want in ["/Common/top", "/Common/in_switch", "/Common/fallback"] {
            assert!(
                names.iter().any(|n| n == want),
                "`{want}` must be a resolved reference; got {names:?}"
            );
        }
    }
    #[test]
    fn original_object_reference_consumers_ignore_dto_labels_and_runtime_value_guesses() {
        // Implementation contract: naming.consumer.original-source-attachment-candidates
        // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
        let source = "when HTTP_REQUEST {pool /Common/literal; set p /Common/guess; pool $p}";
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let context = crate::OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let original = extract_original_irules_source_object_references(source, &context, None);
        assert_eq!(
            original
                .iter()
                .map(|reference| reference.name.as_str())
                .collect::<Vec<_>>(),
            ["/Common/literal"]
        );
        let mut displayed = context.presentation_commands(source);
        for command in &mut displayed {
            command.command = "counterfactual".to_owned();
            command.args = vec!["fake".to_owned()];
        }
        assert_eq!(
            extract_irules_object_references_in_closure(source, None, registry, &displayed),
            original
        );
        assert!(
            extract_original_irules_source_object_references(
                &format!("#changed\n{source}"),
                &context,
                None
            )
            .is_empty()
        );
    }
}
