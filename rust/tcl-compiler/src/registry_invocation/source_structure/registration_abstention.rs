// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Negative source-schema evidence for an unchanged standalone structural walk.

use super::{OriginalSourceRegistryContext, original_single_source_command_substitution_in_plan};
use tcl_lexer::{NativeScriptCommandWords, NativeScriptWordsPlan, NativeWord, Span, TokenType};

const MAX_COMMANDS: usize = 4096;
const MAX_DEPTH: usize = 32;

/// Complete original source and input shared with the ordinary standalone
/// `Analyser::new().with_resolved_input(input).structure_only().analyse(...)`.
/// Entered, hosted and supplied body frames retain their independent owners.
#[derive(Debug)]
pub struct OriginalStandaloneSchemaAbstentions<'a> {
    source: &'a OriginalSourceRegistryContext,
    plan: NativeScriptWordsPlan,
}

/// The structural walk cannot enrich this complete original command's source
/// schema. This is not absence of a runtime command, callback, variable read,
/// lookup alternative, entered frame or source effect.
#[derive(Debug)]
pub struct OriginalStandaloneSchemaAbstention<'a> {
    source: &'a OriginalSourceRegistryContext,
    original: Vec<NativeWord>,
}

impl OriginalSourceRegistryContext {
    /// Authenticate the complete ordinary C Tcl entry. Directives, declarations,
    /// scoped vocabularies and incomplete root roles retain the general path.
    #[must_use]
    pub fn standalone_schema_abstentions(&self) -> Option<OriginalStandaloneSchemaAbstentions<'_>> {
        let input = self.editing_input();
        let source = self.image.try_text().ok()?;
        if self.purpose != super::OriginalRegistryCapturePurpose::StandaloneStructure
            || input.has_hosted_source_name_context()
            || input.has_logical_source_name_context()
            || input.vendor_source_policy().is_some()
            || !std::ptr::eq(input.analyser_profile(), input.unit_profile())
            || !self.realm.matches_resolved_analysis_input(input)
            || self
                .realm
                .source_bindings_ref()
                .source_metadata_owner()
                .source_analysis_input()
                != Some(input)
        {
            return None;
        }
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()?;
        if !matches!(
            policy.recipe(),
            tcl_syntax::naming::NativeNameProtocol::C(_)
        ) {
            return None;
        }
        let (commands, expressions) = crate::analyser::utils::document_stub_declarations(
            source,
            None,
            input.analyser_profile().name,
        );
        if !commands.is_empty()
            || !expressions.is_empty()
            || !crate::analyser::utils::parse_supports_directives(source).is_empty()
            || !crate::analyser::utils::parse_provides_directives(source).is_empty()
        {
            return None;
        }
        let config = input.lexer_config();
        let plan = tcl_lexer::native_script_words_in(
            self.image.clone(),
            Span::new(0, u32::try_from(source.len()).ok()?),
            config,
        )
        .ok()?;
        if plan.fatal_tail.is_some()
            || plan.commands.len() > MAX_COMMANDS
            || plan
                .commands
                .iter()
                .flat_map(|command| &command.words)
                .any(|word| word.group().expand)
        {
            return None;
        }
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        if segments.len() != plan.commands.len() {
            return None;
        }
        let context = input.borrowed_context_registry();
        let survey = OriginalStandaloneSchemaAbstentions { source: self, plan };
        for segment in segments {
            let Some(schema) = self.registration_words(&segment) else {
                survey.at(segment.argv.first()?.span.start())?;
                continue;
            };
            let closed = schema.with_source_schema(context, |selected| {
                selected.authored_source_argument_roles().1
                    && selected.authored_source_retains_callback() == Some(false)
                    && selected.semantics.procedure_definition.is_none()
                    && selected.authored_source_procedure_arguments().is_none()
                    && selected.authored_source_definition_body_grammar().is_none()
                    && selected
                        .authored_source_descriptors()
                        .command
                        .body_scope
                        .is_none()
            })?;
            if !closed {
                return None;
            }
        }
        Some(survey)
    }
}

impl<'a> OriginalStandaloneSchemaAbstentions<'a> {
    /// The full consumer's additional metadata requires original catalogue
    /// observations even if its walker supplies a source namespace separately.
    #[must_use]
    pub fn at(&self, offset: u32) -> Option<OriginalStandaloneSchemaAbstention<'a>> {
        let mut remaining = MAX_COMMANDS;
        let command = substitution_command_at(&self.plan, offset, 0, &mut remaining)?;
        let config = self.source.input.lexer_config();
        let source = self.source.image.try_text().ok()?;
        let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(command.span.as_range())?,
            command.span.start(),
            config.nested(),
        );
        if segments.len() != 1 {
            return None;
        }
        let segment = segments.pop()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&self.source.image),
            config,
            &segment,
        );
        self.source.realm.stamp_original_tokens(&mut tokens);
        let original = crate::registry_invocation::original_native_compiler_words(
            &self.source.image,
            tokens.words(),
            offset,
            config,
        )?;
        if original != command.words
            || !tokens
                .source_binding
                .as_ref()?
                .original_catalogue_source_layout_is_absent()
        {
            return None;
        }
        let context = self.source.input.borrowed_context_registry();
        if self
            .source
            .realm
            .original_source_registry_barrier(context, &tokens)
            || self.source.registration_words_at(offset).is_some()
            || self.source.registration_words(&segment).is_some()
            || self
                .source
                .realm
                .original_source_prefix_catalogue_advice(&self.source.input, &tokens)
                .is_some()
            || self
                .source
                .realm
                .original_member_source_catalogue_advice(&self.source.input, &tokens)
                .is_some()
        {
            return None;
        }
        // Full source_registry_words_at additionally consults scoped bodies,
        // retained conditional metadata and prefix/member advice. Metadata
        // requires the absent observations; prefix/member advice use this same
        // immutable realm and input. Scoped bodies retain one original Str word.
        // Every ancestor here is a whole Command substitution with a non-Str
        // token, so no scoped body can contain the query. A root command is
        // outside every body word; sibling extents are disjoint.
        Some(OriginalStandaloneSchemaAbstention {
            source: self.source,
            original,
        })
    }
}

impl OriginalStandaloneSchemaAbstention<'_> {
    /// Exact immutable whole source, input generation and original issuer.
    #[must_use]
    pub fn matches_source_context(
        &self,
        source: &str,
        context: &OriginalSourceRegistryContext,
    ) -> bool {
        std::ptr::eq(self.source, context)
            && context.source_image().channel() == tcl_lexer::SourceChannel::Document
            && context.source_image().bytes() == source.as_bytes()
            && self.original.iter().all(|word| {
                word.image() == context.source_image()
                    && word.config() == context.editing_input().lexer_config()
            })
    }

    /// Original schema-query geometry, without source lookup or native entry.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
}

fn substitution_command_at(
    plan: &NativeScriptWordsPlan,
    offset: u32,
    depth: usize,
    remaining: &mut usize,
) -> Option<NativeScriptCommandWords> {
    if depth > MAX_DEPTH || plan.fatal_tail.is_some() {
        return None;
    }
    // Root command spans are ordered and disjoint. Child plans are already
    // required below to contain exactly one command.
    let ordinal = plan
        .commands
        .partition_point(|command| command.span.start() <= offset)
        .checked_sub(1)?;
    let command = plan.commands.get(ordinal)?;
    *remaining = remaining.checked_sub(1)?;
    if command.words.iter().any(|word| word.group().expand) {
        return None;
    }
    if command.words.first()?.span().start() == offset {
        return Some(command.clone());
    }
    if offset < command.span.start() || offset >= command.span.end() {
        return None;
    }
    for word in &command.words {
        if offset <= word.span().start() || offset >= word.span().end() {
            continue;
        }
        let child = original_single_source_command_substitution_in_plan(plan, word)?;
        if word.tokens().len() != 1 || word.tokens()[0].kind == TokenType::Str {
            return None;
        }
        let body = word
            .executable_parts()
            .all_parts()
            .find_map(|part| match part.part {
                tcl_lexer::ExecutablePart::Command { body } => Some(body),
                _ => None,
            })?;
        let child_plan = plan.reborrow_command_substitution_plan(word, body).ok()?;
        if child_plan.commands.as_slice() != std::slice::from_ref(child.command()) {
            return None;
        }
        return substitution_command_at(&child_plan, offset, depth + 1, remaining);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, ResolvedAnalysisInput};
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words_at,
    };

    const TWO_ROWS: &str = "set auto_index(a) [list source [file join $dir a.tcl]]\nset auto_index(b) [list source [file join $dir b.tcl]]\n";

    fn context(source: &str, version: tcl_dialect::TclVersion) -> OriginalSourceRegistryContext {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let generation = tcl_registry::model::ingress::static_context_for_profile(profile);
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                generation.with_command_store(generation.commands().snapshot().shared_registry()),
            ),
            config,
        );
        OriginalSourceRegistryContext::capture_standalone_structure(source, input)
    }

    #[test]
    fn standalone_abstention_matches_full_structure_after_original_variable_read() {
        for version in tcl_dialect::TclVersion::ALL {
            let owner = context(TWO_ROWS, version);
            let survey = owner
                .standalone_schema_abstentions()
                .expect("complete standalone source prerequisite");
            let first = u32::try_from(TWO_ROWS.find("[list").unwrap() + 1).unwrap();
            let second = u32::try_from(TWO_ROWS.rfind("[list").unwrap() + 1).unwrap();
            assert!(
                survey.at(first).is_none(),
                "existing source schema is not an abstention: {version:?}"
            );
            let certificate = survey
                .at(second)
                .expect("genuine original catalogue absence prerequisite");
            assert!(certificate.matches_source_context(TWO_ROWS, &owner));
            assert!(survey.at(second + 1).is_none(), "interior head offset");
            assert!(
                survey.at(u32::try_from(TWO_ROWS.len()).unwrap()).is_none(),
                "source end is not a command"
            );
            assert!(!certificate.matches_source_context(&format!("{TWO_ROWS}# changed"), &owner));
            let other = context(TWO_ROWS, version);
            assert!(!certificate.matches_source_context(TWO_ROWS, &other));
            let input = owner.editing_input().clone();
            let full = Analyser::new()
                .with_resolved_input(input.clone())
                .structure_only()
                .analyse(TWO_ROWS, input.unit_profile().name);
            assert_eq!(full.resolved_input.as_ref(), Some(owner.editing_input()));
            assert!(
                source_registry_words_at(TWO_ROWS, &full, second).is_none(),
                "{version:?}"
            );
            let before = owner.registration_words_at(first).unwrap();
            let old = source_registry_words_at(TWO_ROWS, &full, first).unwrap();
            assert!(!before.operands_preserve_source_lookup());
            assert_eq!(
                before.operands_preserve_source_lookup(),
                old.operands_preserve_source_lookup()
            );
            assert_eq!(before.origins(), old.origins());
            let (
                OriginalRegistrySource::Conditional(before),
                OriginalRegistrySource::Conditional(old),
            ) = (before.source(), old.source())
            else {
                panic!("conditional source prerequisite: {version:?}")
            };
            assert_eq!(before.obligations(), old.obligations(), "{version:?}");
        }
    }

    #[test]
    fn standalone_abstention_rejects_body_directive_shadow_and_malformed_inputs() {
        let version = tcl_dialect::TclVersion::V8_6;
        let prepared = context(TWO_ROWS, version);
        let general =
            OriginalSourceRegistryContext::capture(TWO_ROWS, prepared.editing_input().clone());
        assert!(
            general.standalone_schema_abstentions().is_none(),
            "general settings cannot claim structural settings"
        );
        let package = OriginalSourceRegistryContext::capture_package_index_survey(
            TWO_ROWS,
            prepared.editing_input().clone(),
        );
        assert!(
            package.standalone_schema_abstentions().is_none(),
            "a specialised source realm cannot claim the ordinary structural consumer"
        );
        for prefix in [
            "# tcl-lsp: supports tcl 8.6-9.0\n",
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub list {args}\n# tcl-lsp: stubs-end\n",
            "# tcl-lsp: package custom provides Tk\n",
            "proc list args {return ignored}; ",
            "rename list saved; ",
            "unavailable; ",
            "trace add variable dir read callback; ",
        ] {
            let source = format!("{prefix}{TWO_ROWS}");
            assert!(
                context(&source, version)
                    .standalone_schema_abstentions()
                    .is_none(),
                "{prefix}"
            );
        }
        let malformed = format!("{TWO_ROWS}set broken [");
        assert!(
            context(&malformed, version)
                .standalone_schema_abstentions()
                .is_none()
        );
        let expanded = format!("{{*}}$unknown\n{TWO_ROWS}");
        assert!(
            context(&expanded, version)
                .standalone_schema_abstentions()
                .is_none()
        );
        let source = format!("set body {{{TWO_ROWS}}}");
        let owner = context(&source, version);
        let survey = owner.standalone_schema_abstentions().unwrap();
        let offset = u32::try_from(source.rfind("[list").unwrap() + 1).unwrap();
        assert!(
            survey.at(offset).is_none(),
            "inert body text is not command-substitution ancestry"
        );
    }
}
