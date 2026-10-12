// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Closed dictionary mutation values at the same captured physical receiver.

use super::{
    Arc, ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
};
use crate::var_resolve::{ContentsOrigin, ContentsPresence};
use tcl_registry::{InvocationFacts, TraceOperation};
use tcl_syntax::word_rules::WordValueRules;

pub(super) fn retain_dictionary_store(
    outcomes: &mut SourceOutcomes,
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    before: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) {
    if !native.target.registry_backed
        || facts.operation
            != tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::DictSet)
        || !native.target.prepended.is_empty()
    {
        return;
    }
    let arguments = native.invocation.arguments();
    let Some(count) = arguments.exact_argv_len() else {
        return;
    };
    let from = facts.argument_offset;
    if count < from + 3 {
        return;
    }
    let Some(name) = arguments.literal_at(from) else {
        return;
    };
    let receiver = crate::var_resolve::resolve_literal_access(
        name,
        &before.source_variables,
        true,
        context.registry,
        TraceOperation::Read,
    );
    let writer = crate::var_resolve::resolve_literal_access(
        name,
        &before.source_variables,
        true,
        context.registry,
        TraceOperation::Write,
    );
    if receiver.dynamic || receiver.observed || writer.observed || receiver.cell != writer.cell {
        return;
    }
    let Some(initial) = before
        .source_variables
        .literal_contents_at(&receiver, context.registry)
    else {
        return;
    };
    let Some(dialect) = before.baseline.dialect else {
        return;
    };
    let keys = (from + 1..count - 1)
        .map(|index| arguments.literal_at(index))
        .collect::<Option<Vec<_>>>();
    let (Some(keys), Some(value)) = (keys, arguments.literal_at(count - 1)) else {
        return;
    };
    let rules = WordValueRules::from_grammar(&dialect.lexer_grammar);
    let Some(updated) = dictionary_set(initial, &keys, value, rules) else {
        return;
    };
    let Some(normal) = outcomes.normal.as_mut() else {
        return;
    };
    let after = crate::var_resolve::resolve_literal_access(
        name,
        &normal.source_variables,
        true,
        context.registry,
        TraceOperation::Write,
    );
    if after.dynamic
        || after.observed
        || after.cell != writer.cell
        || after.index != writer.index
        || normal.source_variables.contents_presence(&after) != ContentsPresence::Defined
        || normal.source_variables.contents_origin(&after)
            != ContentsOrigin::WrittenAt(context.invocation_offset)
        || normal
            .current_source_origin
            .as_ref()
            .is_none_or(|origin| !normal.source_variables.contents_have_source(&after, origin))
    {
        return;
    }
    Arc::make_mut(&mut normal.source_variables).publish_captured_store(
        &after,
        Some(&updated),
        context.invocation_offset,
    );
}

fn dictionary_set(
    initial: &str,
    keys: &[&str],
    value: &str,
    rules: WordValueRules,
) -> Option<String> {
    let mut current = initial.to_owned();
    let mut parents = Vec::with_capacity(keys.len());
    for key in keys {
        let elements = rules.split_list(&current).ok()?;
        if !elements.len().is_multiple_of(2) {
            return None;
        }
        let slots =
            tcl_syntax::value::canonical_dict_slots(elements.iter().step_by(2).map(AsRef::as_ref));
        let pairs = slots
            .into_iter()
            .map(|(key, value)| {
                (
                    elements[key * 2].to_string(),
                    elements[value * 2 + 1].to_string(),
                )
            })
            .collect::<Vec<_>>();
        current = pairs
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map_or_else(String::new, |(_, value)| value.clone());
        parents.push((pairs, *key));
    }
    value.clone_into(&mut current);
    for (mut pairs, key) in parents.into_iter().rev() {
        if let Some((_, value)) = pairs.iter_mut().find(|(candidate, _)| candidate == key) {
            *value = current;
        } else {
            pairs.push((key.to_owned(), current));
        }
        current = tcl_syntax::list::join_list(
            pairs
                .iter()
                .flat_map(|(key, value)| [key.as_str(), value.as_str()]),
        );
    }
    Some(current)
}

#[cfg(test)]
mod tests {

    #[test]
    fn captured_dictionary_updates_preserve_only_closed_nested_values() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile)
            .commands()
            .clone();
        for (prefix, expected) in [
            ("", Some("UPDATED")),
            ("trace add variable ops write unknown_callback; ", None),
        ] {
            let source = format!(
                "set ops {{x old nested {{y old}}}}; {prefix}dict set ops x NEW; dict set ops nested y UPDATED; set value [dict get $ops nested y]; list READY"
            );
            let analysed = super::super::SourceCommandBindings::analyse_with_options(&source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar), &registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                        mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                        ..Default::default()
                    }, ..Default::default()
                });
            let binding = analysed.invocation_at_source(
                "set",
                u32::try_from(source.rfind("set value").unwrap()).unwrap(),
            );
            assert_eq!(
                binding.evaluated_written_argument_value(1),
                expected,
                "{source}"
            );
        }
    }
}
