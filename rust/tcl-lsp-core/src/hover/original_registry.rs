// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hover documentation from retained original declaration and source schemas.

use super::Hover;
use std::fmt::Write;
use std::ops::ControlFlow;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::registry_invocation::source_structure::{
    OriginalRegistrySource, OriginalRegistryWords,
};
use tcl_lexer::{NativeWord, SourceImage, Span};
use tcl_registry::model::ResolvedContext;
use tcl_registry::{CommandSpec, ResolvedInvocation, SubCommand};

pub(super) fn hover(source: &str, analysis: &AnalysisResult, cursor: u32) -> Option<Hover> {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    if let ControlFlow::Break(Some(identity)) =
        crate::original_declaration::select_at_offset("hover://document", source, analysis, cursor)
    {
        if let Some(declaration) = identity.procedure_metadata(analysis) {
            return Some(Hover::markdown(super::proc_hover_text(
                declaration.metadata(),
            )));
        }
        if let Some(declaration) = identity.class_metadata(analysis) {
            return Some(Hover::markdown(super::class_hover_text(
                analysis,
                declaration.metadata(),
            )));
        }
    }
    let config = analysis.body_lexer_config?;
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    let command = structure
        .commands
        .iter()
        .filter(|command| contains(command.span, cursor))
        .min_by_key(|command| command.span.end() - command.span.start())?;
    if let Some(text) = scoped_hover(source, analysis, command.span.start(), cursor) {
        return Some(Hover::markdown(text));
    }
    let words = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, command,
    )?;
    if let Some(head) = words.head_source()
        && contains(head.span(), cursor)
        && let Some(word) = head.word()
        && let Ok(name) = word.try_text()
        && let ControlFlow::Break(answer) = super::alias_hover_at(source, analysis, cursor, name)
    {
        return answer.map(Hover::markdown);
    }
    let text = schema_hover(analysis, &words, cursor)?;
    Some(Hover::markdown(text))
}

fn contains(span: Span, cursor: u32) -> bool {
    span.start() <= cursor && cursor < span.end()
}

fn schema_hover(
    analysis: &AnalysisResult,
    words: &OriginalRegistryWords,
    cursor: u32,
) -> Option<String> {
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let head = words
        .head_source()
        .is_some_and(|head| contains(head.span(), cursor));
    let mut operands = words
        .operands()
        .iter()
        .enumerate()
        .filter_map(|(ordinal, operand)| {
            contains(operand.as_ref()?.span(), cursor).then_some(ordinal)
        });
    let argument = operands.next();
    if operands.next().is_some() {
        return None;
    }
    let mut text = words
        .with_source_schema(&context, |schema| {
            let spec = schema.authored_source_descriptors().command;
            if head {
                return command_text(
                    spec,
                    context.context(),
                    schema.semantics.options.availability.package_version,
                );
            }
            operand_text(schema, spec, context.context(), argument?)
        })
        .flatten()?;
    match words.source() {
        OriginalRegistrySource::Selected => {}
        OriginalRegistrySource::Scoped(_) => {
            text.push_str("\n\n_Scoped source documentation from the retained original body; runtime command availability is unresolved._");
        }
        OriginalRegistrySource::SourceTransitions(_) => {
            text.push_str(
                "\n\n_Source transition documentation; command availability is unresolved._",
            );
        }
        OriginalRegistrySource::ProducedPrefix(_) => {
            text.push_str("\n\n_Produced prefix source documentation; future command availability is unresolved._");
        }
        OriginalRegistrySource::Conditional(_) => {
            text.push_str("\n\n_Source documentation; command availability is unresolved._");
        }
        OriginalRegistrySource::Vendor(_) => {
            text.push_str("\n\n_Hosted source documentation; command availability is unresolved._");
        }
    }
    Some(text)
}

fn command_text(
    spec: &CommandSpec,
    context: &ResolvedContext,
    package_version: Option<&str>,
) -> Option<String> {
    let hover = spec.hover.as_ref()?;
    let mut out = format!("**`{}`** — command documentation\n", spec.name);
    if !hover.summary.is_empty() {
        let _ = write!(out, "\n{}\n", hover.summary);
    }
    if let Some(synopsis) = hover.synopsis.first() {
        let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
    }
    let mut names = context
        .available_subcommands(spec)
        .into_iter()
        .filter(|sub| sub.available_for_version(package_version))
        .map(|sub| sub.name)
        .collect::<Vec<_>>();
    if !names.is_empty() {
        names.sort_unstable();
        names.dedup();
        let _ = write!(out, "\nSubcommands: {}\n", names.join(", "));
    }
    if let Some(package) = spec.required_package {
        let _ = write!(out, "\nDeclared provider: `{package}`.\n");
    }
    if let Some(requires) = &spec.event_requires {
        super::append_valid_events(
            &mut out,
            &super::effective_event_requires(spec.name, requires),
        );
    }
    Some(out)
}

fn operand_text(
    schema: &ResolvedInvocation<'_, '_>,
    spec: &CommandSpec,
    context: &ResolvedContext,
    argument: usize,
) -> Option<String> {
    let sub = schema.authored_source_descriptors().subcommand;
    if argument == 0
        && let Some(sub) = sub
    {
        return Some(subcommand_text(spec.name, sub));
    }
    if argument == 1
        && let Some(sub) = sub.filter(|sub| !sub.sub_subcommands.is_empty())
    {
        let word = schema.words.arguments().literal_at(argument)?;
        let availability = schema.semantics.options.availability;
        let operation = sub.resolve_sub_subcommand_gated(
            word,
            availability.query,
            availability.package_version,
        )?;
        if !context.sub_subcommand_available(spec, sub, operation, availability.package_version) {
            return None;
        }
        let mut out = format!(
            "**`{} {} {}`** — subcommand\n",
            spec.name, sub.name, operation.name
        );
        if !operation.detail.is_empty() {
            let _ = write!(out, "\n{}\n", operation.detail);
        }
        if !operation.synopsis.is_empty() {
            let _ = write!(out, "\n```tcl\n{}\n```\n", operation.synopsis);
        }
        return Some(out);
    }
    option_text(schema, spec, argument)
}

fn subcommand_text(head: &str, sub: &SubCommand) -> String {
    let mut out = format!("**`{head} {}`** — subcommand\n", sub.name);
    if let Some(hover) = &sub.hover {
        if !hover.summary.is_empty() {
            let _ = write!(out, "\n{}\n", hover.summary);
        }
        if let Some(synopsis) = hover.synopsis.first() {
            let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
        }
    } else {
        if !sub.detail.is_empty() {
            let _ = write!(out, "\n{}\n", sub.detail);
        }
        if !sub.synopsis.is_empty() {
            let _ = write!(out, "\n```tcl\n{}\n```\n", sub.synopsis);
        }
    }
    out
}

fn option_text(
    schema: &ResolvedInvocation<'_, '_>,
    spec: &CommandSpec,
    argument: usize,
) -> Option<String> {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let scan = schema.authored_source_diagnostic_options()?;
    let selected = scan
        .options
        .iter()
        .find(|option| option.argument == argument)?;
    let option = selected.option;
    let owner = std::iter::once(spec.name)
        .chain(scan.subcommands.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = format!("**`{}`** — option of `{owner}`\n", option.name);
    if !option.detail.is_empty() {
        let _ = write!(out, "\n{}\n", option.detail);
    }
    if option.takes_value() && !option.value_hint().is_empty() {
        let _ = write!(out, "\nTakes a `{}` value.\n", option.value_hint());
    }
    if option.value_is_boolean() {
        let vocabulary = tcl_registry::abbrev::BOOLEAN_KEYWORDS
            .iter()
            .map(|word| format!("`{word}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, "\nAccepts any boolean spelling: {vocabulary}.\n");
    }
    if !selected.available {
        out.push_str("\n_Not available in the active dialect._\n");
    }
    Some(out)
}

fn scoped_hover(
    source: &str,
    analysis: &AnalysisResult,
    command: u32,
    cursor: u32,
) -> Option<String> {
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    let body = analysis.original_scoped_body_in_source(&image, config, cursor)?;
    let plan = tcl_lexer::native_script_words_in(image, body.content_span(), config).ok()?;
    let words = &plan
        .commands
        .iter()
        .find(|row| {
            row.words
                .first()
                .is_some_and(|head| head.span().start() == command)
        })?
        .words;
    let head = words.first()?;
    let selected = body.environment().command(&static_ascii_word(head)?)?;
    let mut out = if contains(head.span(), cursor) {
        let mut out = format!(
            "**`{}`** — {} command\n",
            selected.name,
            body.environment().name
        );
        if let Some(hover) = &selected.hover {
            if !hover.summary.is_empty() {
                let _ = write!(out, "\n{}\n", hover.summary);
            }
            if !hover.snippet.is_empty() {
                let _ = write!(out, "\n```tcl\n{}\n```\n", hover.snippet);
            }
        } else if !selected.detail.is_empty() {
            let _ = write!(out, "\n{}\n", selected.detail);
        }
        out
    } else {
        let word = words.get(1)?;
        if !contains(word.span(), cursor) {
            return None;
        }
        let sub = selected.subcommand(&static_ascii_word(word)?)?;
        subcommand_text(selected.name, sub)
    };
    out.push_str("\n\n_Source vocabulary for this body._");
    Some(out)
}

fn static_ascii_word(word: &NativeWord) -> Option<String> {
    String::from_utf8(tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn at(source: &str, analysis: &AnalysisResult, needle: &str) -> Option<Hover> {
        let offset = u32::try_from(source.rfind(needle)? + 1).ok()?;
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
        super::super::hover_with_profile(
            source,
            position.line,
            position.character.get(),
            analysis,
            Some(crate::registry_for_dialect("jim")),
            crate::profile_for_dialect("jim"),
        )
    }

    #[test]
    fn original_registry_hover_uses_actual_heads_subcommands_options_and_origins() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        for (source, needle, expected) in [
            ("puts hello", "puts", "command documentation"),
            ("string length value", "length", "`string length`"),
            (
                "regexp -nocase pattern text",
                "-nocase",
                "option of `regexp`",
            ),
            ("info object class ::C", "class", "`info object class`"),
            (
                "namespace ensemble configure ::E -namespace",
                "-namespace",
                "option of `namespace ensemble configure`",
            ),
            ("string {*}{length value}", "length", "`string length`"),
            (
                "interp alias {} rx {} regexp\nrx -nocase pattern text",
                "-nocase",
                "option of `regexp`",
            ),
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            analysis.all_procs.clear();
            analysis.all_classes.clear();
            analysis.command_invocations.clear();
            let selected = at(source, &analysis, needle).expect(source);
            assert!(
                selected.value.contains(expected),
                "{source}: {}",
                selected.value
            );
        }
        for source in [
            "puts string",
            "regexp -nocase pattern string",
            "interp alias {} size {} string length\nsize length",
            "puts {-nocase}",
            "# string length value",
            "set data {string length value}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let needle = if source.contains("length") {
                "length"
            } else if source.contains("string") {
                "string"
            } else {
                "-nocase"
            };
            assert!(at(source, &analysis, needle).is_none(), "{source}");
        }
    }

    #[test]
    fn original_registry_hover_keeps_current_context_and_conditional_shadow_barriers() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let source = "string length value";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(at(source, &analysis, "length").is_some());
        assert!(at("string length other", &analysis, "length").is_none());
        let mut changed = analysis.clone();
        let config = changed.body_lexer_config.unwrap();
        changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        });
        assert!(at(source, &changed, "length").is_none());
        let mut foreign = analysis.clone();
        let jim = crate::profile_for_dialect("jim");
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            jim,
            jim,
            tcl_registry::model::ingress::context_for_profile(jim),
            config,
        ));
        assert!(at(source, &foreign, "length").is_none());
        let source_data = "puts string";
        let mut data = Analyser::new().analyse(source_data, "tcl8.6");
        let donor = Analyser::new().analyse("proc string args {return forged}", "tcl8.6");
        data.all_procs = donor.all_procs;
        data.global_scope.procs = donor.global_scope.procs;
        data.dialect = "copied reporting label".to_owned();
        assert!(at(source_data, &data, "string").is_none());
        for source in [
            "proc string args {return custom}\nstring length value",
            "rename string {}\nstring length value",
            "proc regexp args {}\nregexp -nocase p text",
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            analysis.all_procs.clear();
            analysis.command_invocations.clear();
            let needle = if source.contains("length") {
                "length"
            } else {
                "-nocase"
            };
            assert!(at(source, &analysis, needle).is_none(), "{source}");
        }
        let source = "opaque\nstring length value";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let advice = at(source, &analysis, "length").expect("conditional source documentation");
        assert!(
            advice.value.contains("Source documentation"),
            "{}",
            advice.value
        );
    }

    #[test]
    fn original_registry_head_hover_lists_only_retained_version_subcommands() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let source = "info patchlevel";
        for (dialect, frame, coroutine, object, cmdtype) in [
            ("tcl8.4", false, false, false, false),
            ("tcl8.5", true, false, false, false),
            ("tcl8.6", true, true, true, false),
            ("tcl9.0", true, true, true, true),
            ("tcl9.1", true, true, true, true),
        ] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.dialect = "display-only".to_owned();
            analysis.command_invocations.clear();
            let hover = at(source, &analysis, "info").expect(dialect);
            let names = hover
                .value
                .lines()
                .find_map(|line| line.strip_prefix("Subcommands: "))
                .expect("available info subcommands")
                .split(", ")
                .collect::<Vec<_>>();
            assert!(names.contains(&"exists"), "{dialect}: {}", hover.value);
            for (name, expected) in [
                ("frame", frame),
                ("coroutine", coroutine),
                ("object", object),
                ("cmdtype", cmdtype),
                ("version", false),
                ("stacktrace", false),
            ] {
                assert_eq!(names.contains(&name), expected, "{dialect}: {name}");
            }
        }
    }

    #[test]
    fn original_registry_operand_hover_keeps_parent_nested_and_option_availability() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        for (dialect, tcloo, later_info) in [
            ("tcl8.4", false, false),
            ("tcl8.5", false, false),
            ("tcl8.6", true, false),
            ("tcl9.0", true, true),
            ("tcl9.1", true, true),
        ] {
            for (source, needle, expected) in [
                ("info coroutine", "coroutine", tcloo),
                ("info object class ::C", "class", tcloo),
                ("info object creationid ::C", "creationid", later_info),
            ] {
                let mut analysis = Analyser::new().analyse(source, dialect);
                analysis.command_invocations.clear();
                assert_eq!(
                    at(source, &analysis, needle).is_some(),
                    expected,
                    "{dialect}: {source}"
                );
            }
        }
        for (dialect, admitted) in [
            ("tcl8.4", false),
            ("tcl8.5", false),
            ("tcl8.6", true),
            ("tcl9.0", true),
            ("tcl9.1", true),
        ] {
            let source = "lsort -stride 2 {a b}";
            let analysis = Analyser::new().analyse(source, dialect);
            let option = at(source, &analysis, "-stride").expect("exact source option metadata");
            assert_eq!(
                option.value.contains("Not available in the active dialect"),
                !admitted,
                "{dialect}: {}",
                option.value
            );
        }
        for (dialect, jim) in [("jim", true), ("tcl8.6", false)] {
            let source = "info version";
            let analysis = Analyser::new().analyse(source, dialect);
            assert_eq!(at(source, &analysis, "version").is_some(), jim);
            let source = "regexp -about {(a)}";
            let analysis = Analyser::new().analyse(source, dialect);
            // Jim's selected descriptor omits this C option entirely. The
            // retained excluded rows tested above are a separate source case.
            let option = at(source, &analysis, "-about");
            if jim {
                assert!(option.is_none(), "Jim must not borrow the C option row");
            } else {
                let option = option.expect("selected C source option metadata");
                assert!(!option.value.contains("Not available in the active dialect"));
            }
        }
        let source = "namespace ensemble configure -namespace ::example";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(at(source, &analysis, "-namespace").is_none());
    }

    #[test]
    fn original_scoped_hover_keeps_body_receipts_and_ignores_copied_reporting_regions() {
        // naming.core.original-registry-source-hover
        // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
        let source = "::report::defstyle st {} {\n top set header\n}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.scoped_command_regions.clear();
        analysis.global_scope.defined_symbols.clear();
        analysis.command_invocations.clear();
        let head = at(source, &analysis, "top").expect("retained scoped head");
        assert!(head.value.contains("report style definition"));
        let operation = at(source, &analysis, "set").expect("retained scoped operation");
        assert!(operation.value.contains("`top set`"));
        assert!(at(source, &analysis, "header").is_none());
        let source = "namespace eval ::report {}\nproc ::report::defstyle args {}\n::report::defstyle st {} {top set header}";
        let mut blocked = Analyser::new().analyse(source, "tcl8.6");
        blocked
            .scoped_command_regions
            .push(tcl_compiler::analyser::types::ScopedBodyRegion {
                span: Span::new(0, u32::try_from(source.len()).unwrap()),
                env: &tcl_registry::scoped::REPORT_DEFSTYLE_ENV,
            });
        assert!(at(source, &blocked, "top").is_none());
    }
}
