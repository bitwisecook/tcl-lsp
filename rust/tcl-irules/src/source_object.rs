// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Object operands selected from actual source schema and the shared ref table.

use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::{InvocationArguments, ResolvedInvocation};

/// Possible attachment purpose selected separately from general side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesSourceAttachmentKind {
    /// An actual pool-reference template and possible pool selection write.
    Pool,
    /// An actual node-reference template and possible node selection write.
    Node,
    /// An actual SNAT-pool operand; a general SNAT write alone is insufficient.
    SnatPool,
}

/// Readonly original source operand in the existing object-reference schema.
#[derive(Debug, Clone)]
pub struct IrulesSourceObjectOperand {
    argument: usize,
    kinds: Vec<&'static str>,
    attachment: Option<IrulesSourceAttachmentKind>,
}
impl IrulesSourceObjectOperand {
    /// Actual post-head ordinal, never a positional text placeholder.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.argument
    }
    /// Configuration object-kind candidates from the shared declarative table.
    #[must_use]
    pub fn kinds(&self) -> &[&'static str] {
        &self.kinds
    }
    /// Possible attachment category, without selection or availability proof.
    #[must_use]
    pub const fn attachment(&self) -> Option<IrulesSourceAttachmentKind> {
        self.attachment
    }
}

fn literal(arguments: InvocationArguments<'_>, index: usize) -> Option<&str> {
    let word = arguments.get(index)?;
    word.literal()
        .or_else(|| std::str::from_utf8(word.native_bytes()?).ok())
}

fn ordinal(position: &crate::ObjectRefArg, arguments: InvocationArguments<'_>) -> Option<usize> {
    let count = arguments.exact_argv_len()?;
    if position.last {
        return count.checked_sub(1);
    }
    if let Some(index) = position.index {
        let index = usize::try_from(index).ok()?;
        return (index < count).then_some(index);
    }
    let keyword = position.after_keyword.as_deref()?;
    (0..count).find_map(|index| {
        (literal(arguments, index)?.eq_ignore_ascii_case(keyword) && index + 1 < count)
            .then_some(index + 1)
    })
}

fn source_object_attachment(
    selected: &ResolvedInvocation<'_, '_>,
    kinds: &[&str],
) -> Option<IrulesSourceAttachmentKind> {
    let writes = |target| {
        selected
            .semantics
            .side_effects
            .iter()
            .any(|effect| effect.writes && effect.target == target)
    };
    if kinds
        .iter()
        .any(|kind| kind.starts_with("ltm_pool") || kind.starts_with("gtm_pool"))
        && writes(SideEffectTarget::PoolSelection)
    {
        Some(IrulesSourceAttachmentKind::Pool)
    } else if kinds.contains(&"ltm_node") && writes(SideEffectTarget::NodeSelection) {
        Some(IrulesSourceAttachmentKind::Node)
    } else if kinds.contains(&"ltm_snatpool") && writes(SideEffectTarget::SnatSelection) {
        Some(IrulesSourceAttachmentKind::SnatPool)
    } else {
        None
    }
}

/// Project object-operand advice from a genuinely selected authored invocation.
/// The same existing table supplies object kinds and positions. Unknown values
/// retain exact ordinary slots; unknown expansion cardinality withdraws advice.
/// This metadata grants no object identity, traffic effect or execution.
#[must_use]
pub fn original_source_object_operands(
    selected: &ResolvedInvocation<'_, '_>,
    rule_module: Option<&str>,
) -> Vec<IrulesSourceObjectOperand> {
    // Implementation contract: naming.consumer.original-source-attachment-candidates
    // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
    let arguments = selected.words.arguments();
    let Some(count) = arguments.exact_argv_len() else {
        return Vec::new();
    };
    if !selected
        .argument_count_for_arity()
        .is_some_and(|count| selected.semantics.arity.accepts(count))
    {
        return Vec::new();
    }
    let table = crate::tables();
    let lower = selected.canonical_command.to_ascii_lowercase();
    let mut choices = Vec::new();
    for spec in &table.base_specs {
        if spec.command != lower
            || spec
                .when_module
                .as_deref()
                .is_some_and(|module| rule_module.is_some_and(|actual| actual != module))
            || spec.sub_command.as_deref().is_some_and(|sub| {
                literal(arguments, 0).is_none_or(|value| !value.eq_ignore_ascii_case(sub))
            })
        {
            continue;
        }
        if let Some(index) = ordinal(&spec.position, arguments) {
            choices.push((
                index,
                spec.kinds.iter().map(String::as_str).collect::<Vec<_>>(),
            ));
        }
    }
    for template in &table.pool_templates {
        if template.command != lower {
            continue;
        }
        if let Some(index) = ordinal(&template.position, arguments) {
            let kinds = if rule_module.is_none() {
                table
                    .ltm_pool_kinds
                    .iter()
                    .chain(&table.gtm_pool_kinds)
                    .map(String::as_str)
                    .collect()
            } else {
                crate::pool_kinds_for_module(rule_module)
            };
            choices.push((index, kinds));
        }
    }
    // The shared class/persist resolvers consume only genuine complete literal
    // values. Missing values never become arbitrary placeholder strings.
    if matches!(lower.as_str(), "class" | "persist")
        && let Some(values) = (0..count)
            .map(|index| literal(arguments, index))
            .collect::<Option<Vec<_>>>()
    {
        choices.extend(crate::resolve_object_ref_args(
            selected.canonical_command,
            &values,
            rule_module,
        ));
    }
    let mut seen = std::collections::HashSet::new();
    choices
        .into_iter()
        .filter_map(|(argument, kinds)| {
            if !seen.insert(argument)
                || literal(arguments, argument)
                    .is_some_and(|value| table.falsey.contains(&value.to_ascii_lowercase()))
            {
                return None;
            }
            if table
                .pool_templates
                .iter()
                .any(|template| template.command == lower)
                && literal(arguments, argument).is_some_and(|value| {
                    matches!(value.to_ascii_lowercase().as_str(), "member" | "members")
                })
            {
                return None;
            }
            let attachment = source_object_attachment(selected, &kinds);
            Some(IrulesSourceObjectOperand {
                argument,
                kinds,
                attachment,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::InvocationWord;

    #[test]
    fn original_attachment_operands_keep_snat_pool_purpose_and_original_cardinality() {
        // Implementation contract: naming.consumer.original-source-attachment-candidates
        // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
        let context = tcl_registry::model::ingress::static_context_for("f5-irules");
        let select = |head, args: &[InvocationWord<'_>]| {
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(InvocationWord::Literal(head), args),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .map_or_else(Vec::new, |selected| {
                original_source_object_operands(&selected, None)
            })
        };
        for (head, kind) in [
            ("pool", IrulesSourceAttachmentKind::Pool),
            ("node", IrulesSourceAttachmentKind::Node),
            ("snatpool", IrulesSourceAttachmentKind::SnatPool),
        ] {
            let operands = select(head, &[InvocationWord::Dynamic]);
            assert_eq!(operands.len(), 1);
            assert_eq!(operands[0].argument(), 0);
            assert_eq!(operands[0].attachment(), Some(kind));
            assert!(select(head, &[InvocationWord::Expanded]).is_empty());
        }
        assert!(select("snat", &[InvocationWord::Literal("10.0.0.1")]).is_empty());
        let operands = select(
            "snat",
            &[InvocationWord::Literal("pool"), InvocationWord::Dynamic],
        );
        assert_eq!(operands.len(), 1);
        assert_eq!(operands[0].argument(), 1);
        assert_eq!(
            operands[0].attachment(),
            Some(IrulesSourceAttachmentKind::SnatPool)
        );
        assert!(select("pool", &[InvocationWord::Literal("none")]).is_empty());
    }
}
