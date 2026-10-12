// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Completion from original effective argv or independent hosted source schema.

use super::{CompletionItem, package_version_floor};
use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::{CommandSpec, Traits};

struct SelectedContext {
    spec: CommandSpec,
    arguments: Vec<Option<String>>,
    active: usize,
}

pub(super) fn items(
    source: &str,
    cursor: u32,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let selected = select(source, analysis, cursor)?;
    let context = current.context();
    let spec = &selected.spec;
    let profile = analysis.resolved_profile()?;
    let floor = package_version_floor(analysis, spec, profile);
    let query = Some(context.context().authoring_query());
    let first = selected.arguments.first().and_then(Option::as_deref);
    let sub = (selected.active >= 1)
        .then(|| first.and_then(|name| spec.resolve_subcommand_for_dialect(name, query)))
        .flatten();
    let switches = SwitchAdvice {
        spec,
        context: context.context(),
        floor,
        query,
    };
    if let Some(items) = switches.items(source, line, character, partial, &selected) {
        return Some(items);
    }
    if selected.active == 0 {
        if spec.traits.contains(Traits::IS_EVENT_HANDLER) {
            return Some(super::event_name_completions(partial));
        }
        if spec.traits.contains(Traits::INVOKES_USER_PROC) {
            return Some(super::invoked_proc_completions(analysis, partial));
        }
        if !spec.subcommands.is_empty() {
            let subs = context
                .context()
                .available_subcommands(spec)
                .into_iter()
                .filter(|sub| sub.available_for_version(floor))
                .collect::<Vec<_>>();
            return Some(super::arg_value_completions_from_subcommands(subs, partial));
        }
    }
    if let Some(sub) = sub {
        if selected.active == 1 && !sub.sub_subcommands.is_empty() {
            return Some(super::sub_subcommand_completions(
                context
                    .context()
                    .available_sub_subcommands(spec, sub, floor),
                partial,
            ));
        }
        if let Some(index) = selected
            .active
            .checked_sub(1)
            .and_then(|index| u8::try_from(index).ok())
        {
            let values = sub.available_arg_values_at(index, floor);
            if !values.is_empty() {
                return Some(super::arg_value_completions_from(values, partial));
            }
        }
    }
    if let Some(items) = option_values(spec, &selected, floor, partial) {
        return Some(items);
    }
    if spec.traits.contains(Traits::IS_EVENT_HANDLER)
        && selected.active >= 2
        && selected
            .arguments
            .get(selected.active - 1)
            .and_then(Option::as_deref)
            != Some("timing")
    {
        return None;
    }
    let index = u8::try_from(selected.active).ok()?;
    let values = spec.available_arg_values_at(index, floor);
    (!values.is_empty()).then(|| super::arg_value_completions_from(values, partial))
}

struct SwitchAdvice<'a> {
    spec: &'a CommandSpec,
    context: &'a tcl_registry::model::ResolvedContext,
    floor: Option<&'a str>,
    query: Option<tcl_dialect::model::SurfaceQuery<'a>>,
}
impl SwitchAdvice<'_> {
    fn items(
        &self,
        source: &str,
        line: u32,
        character: u32,
        partial: &str,
        selected: &SelectedContext,
    ) -> Option<Vec<CompletionItem>> {
        let first = selected.arguments.first().and_then(Option::as_deref);
        let sub = (selected.active >= 1)
            .then(|| {
                first.and_then(|name| self.spec.resolve_subcommand_for_dialect(name, self.query))
            })
            .flatten();
        if let Some(switch_partial) =
            super::switch_partial_at_position(source, line, character, partial)
        {
            let (options, surface) = if let Some(sub) = sub {
                let next = (selected.active >= 2)
                    .then(|| selected.arguments.get(1).and_then(Option::as_deref))
                    .flatten();
                let scope = sub.option_scope(next, self.query, self.floor, self.spec.surface);
                (scope.options, scope.surface)
            } else {
                (self.spec.options, self.spec.surface)
            };
            if !options.is_empty() {
                let text = source.split('\n').nth(line as usize)?;
                let end = super::utf16_col_to_char_col(text, character).min(text.chars().count());
                let start = end.saturating_sub(switch_partial.chars().count());
                return Some(super::switch_completions(
                    options,
                    self.context,
                    surface,
                    &switch_partial,
                    (
                        super::char_col_to_utf16(text, start),
                        super::char_col_to_utf16(text, end),
                    ),
                    self.floor,
                ));
            }
        }
        None
    }
}

fn option_values(
    spec: &CommandSpec,
    selected: &SelectedContext,
    floor: Option<&str>,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let prev = selected
        .arguments
        .get(selected.active.checked_sub(1)?)?
        .as_deref()?;
    if !prev.starts_with('-') {
        return None;
    }
    let option = spec
        .options
        .iter()
        .chain(
            spec.command_forms
                .iter()
                .flat_map(|form| form.options.iter()),
        )
        .find(|option| option.matches(prev))?;
    let values = option
        .value_values()
        .iter()
        .filter(|value| value.available_for_version(floor))
        .collect::<Vec<_>>();
    if !values.is_empty() {
        return Some(super::arg_value_completions_from(values, partial));
    }
    option
        .value_is_boolean()
        .then(|| super::boolean_value_completions(partial))
}

fn select(source: &str, analysis: &AnalysisResult, cursor: u32) -> Option<SelectedContext> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    if analysis.has_original_vendor_source_names() {
        let (metadata, _) = crate::original_invocation::selected_vendor_registry_words_at(
            source, analysis, cursor,
        )?;
        let shape = metadata.shape();
        if shape
            .original_words()
            .iter()
            .any(|word| word.group().expand)
        {
            return None;
        }
        let active = shape
            .original_words()
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, word)| word.span().start() <= cursor && cursor <= word.span().end())
            .map(|(index, _)| index - 1)
            .or_else(|| {
                only_trailing_space(source, shape.original_words().last()?.span().end(), cursor)
                    .then_some(shape.original_words().len().checked_sub(1)?)
            })?;
        let arguments = shape
            .original_words()
            .iter()
            .skip(1)
            .map(|word| {
                tcl_syntax::naming::vendor_source_literal_units(
                    shape.original_head().policy(),
                    word,
                    tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                )
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
            })
            .collect();
        return Some(SelectedContext {
            spec: shape
                .context()
                .resolve_spec(current.registry(), shape.command())?
                .clone(),
            arguments,
            active,
        });
    }
    let commands = crate::original_invocation::registry_commands_in_source(source, analysis)?;
    let (command, words) = commands
        .into_iter()
        .filter(|(command, _)| {
            command.span.start() <= cursor
                && (cursor <= command.span.end()
                    || only_trailing_space(source, command.span.end(), cursor))
        })
        .min_by_key(|(command, _)| command.span.end().saturating_sub(command.span.start()))?;
    let active = words
        .operands
        .iter()
        .enumerate()
        .find_map(|(index, operand)| {
            let operand = operand.as_ref()?;
            let span = operand
                .word
                .as_ref()
                .map_or(operand.span, |word| word.span());
            (span.start() <= cursor && cursor <= span.end()).then_some(index)
        })
        .or_else(|| {
            only_trailing_space(source, command.span.end(), cursor).then_some(words.arguments.len())
        })?;
    let arguments = words
        .arguments
        .iter()
        .map(|word| {
            word.literal_bytes()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
        })
        .collect();
    let spec = words.with_source_schema(&current.context(), |schema| {
        schema.authored_source_descriptors().command.clone()
    })?;
    Some(SelectedContext {
        spec,
        arguments,
        active,
    })
}
fn only_trailing_space(source: &str, end: u32, cursor: u32) -> bool {
    cursor > end
        && source
            .get(end as usize..cursor as usize)
            .is_some_and(|text| !text.is_empty() && text.chars().all(|c| c == ' ' || c == '\t'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    // Implementation contract: naming.core.original-registry-context-completion
    // docs/design/analysis/name-resolution-proofs/original-registry-context-completion.md
    fn original_context_completion_preserves_actual_handler_and_effective_ordinals() {
        let source = "string is al";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let result = items(
            source,
            u32::try_from(source.len()).unwrap(),
            0,
            u32::try_from(source.len()).unwrap(),
            &analysis,
            "al",
        )
        .expect("actual Registry context");
        assert!(result.iter().any(|item| item.label == "alnum"));
        let shadow = "proc string args {}; string is al";
        let analysis = Analyser::new().analyse(shadow, "tcl8.6");
        assert!(
            items(
                shadow,
                u32::try_from(shadow.len()).unwrap(),
                0,
                u32::try_from(shadow.len()).unwrap(),
                &analysis,
                "al"
            )
            .is_none()
        );
        for source in [
            "interp alias {} classify {} string is\nclassify al",
            "string {*}{is al}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let cursor = u32::try_from(source.rfind("al").unwrap() + 2).unwrap();
            let (line, character) = if source.contains('\n') {
                (1, 11)
            } else {
                (0, cursor)
            };
            let selected = select(source, &analysis, cursor).expect("same effective origin");
            assert_eq!(selected.active, 1, "{source}");
            let result = items(source, cursor, line, character, &analysis, "al").unwrap();
            assert!(result.iter().any(|item| item.label == "alnum"), "{source}");
        }
    }

    #[test]
    // Implementation contract: naming.core.original-registry-context-completion
    // docs/design/analysis/name-resolution-proofs/original-registry-context-completion.md
    fn original_context_completion_rejects_stale_owners_and_keeps_hosted_schema() {
        let source = "when HTTP_RE";
        let mut analysis = Analyser::new().analyse(source, "f5-irules");
        let result = items(source, 12, 0, 12, &analysis, "HTTP_RE").expect("hosted authored role");
        assert!(result.iter().any(|item| item.label == "HTTP_REQUEST"));
        let blank = "when ";
        let blank_analysis = Analyser::new().analyse(blank, "f5-irules");
        assert!(
            items(blank, 5, 0, 5, &blank_analysis, "HTTP_RE")
                .unwrap()
                .iter()
                .any(|item| item.label == "HTTP_REQUEST")
        );
        analysis.dialect = "tcl8.6".into();
        assert!(items(source, 12, 0, 12, &analysis, "HTTP_RE").is_some());
        assert!(items("when CLIENT_A", 12, 0, 12, &analysis, "HTTP_RE").is_none());
    }
    fn logical_analysis(
        source: &str,
        environment: &str,
        store: std::sync::Arc<tcl_registry::CommandRegistry>,
    ) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for(environment).with_command_store(store),
        );
        Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            ))
            .analyse(source, profile.name)
    }

    #[test]
    fn original_logical_completion_uses_actual_availability_and_captured_ordinals() {
        // naming.core.original-registry-context-completion
        // docs/design/analysis/name-resolution-proofs/original-registry-context-completion.md
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let surface = registry.get("dict").unwrap().surface;
        let subcommands = registry.get("string").unwrap().subcommands;
        registry.insert(tcl_registry::CommandSpec {
            name: "source_classify",
            surface,
            subcommands,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let store = std::sync::Arc::new(registry);
        for source in [
            "source_classify is al",
            "interp alias {} classify {} source_classify is\nclassify al",
        ] {
            let current = logical_analysis(source, "tcl8.6", store.clone());
            assert!(current.allows_lexical_declaration_advice());
            let cursor = u32::try_from(source.len()).unwrap();
            let (line, column) = if source.contains('\n') {
                (1, 11)
            } else {
                (0, cursor)
            };
            let result = items(source, cursor, line, column, &current, "al").unwrap();
            assert!(result.iter().any(|item| item.label == "alnum"), "{source}");
            let old = logical_analysis(source, "tcl8.4", store.clone());
            assert!(items(source, cursor, line, column, &old, "al").is_none());
        }
        let source = "proc source_classify args {}; source_classify is al";
        let current = logical_analysis(source, "tcl8.6", store);
        assert!(
            items(
                source,
                u32::try_from(source.len()).unwrap(),
                0,
                u32::try_from(source.len()).unwrap(),
                &current,
                "al"
            )
            .is_none()
        );
    }

    #[test]
    fn original_completion_switches_use_actual_option_surface_and_current_source() {
        // naming.core.original-registry-context-completion
        // docs/design/analysis/name-resolution-proofs/original-registry-context-completion.md
        let store = std::sync::Arc::new(tcl_registry::CommandRegistry::build_default());
        let source = "chan configure stdin -inputm";
        let cursor = u32::try_from(source.len()).unwrap();
        let current = logical_analysis(source, "tcl9.0", store.clone());
        assert!(
            items(source, cursor, 0, cursor, &current, "inputm")
                .unwrap()
                .iter()
                .any(|item| item.label == "-inputmode")
        );
        let old = logical_analysis(source, "tcl8.6", store);
        assert!(
            !items(source, cursor, 0, cursor, &old, "inputm")
                .unwrap()
                .iter()
                .any(|item| item.label == "-inputmode")
        );
        assert!(
            items(
                "chan configure stdin -inputx",
                cursor,
                0,
                cursor,
                &current,
                "inputm"
            )
            .is_none()
        );
    }
}
