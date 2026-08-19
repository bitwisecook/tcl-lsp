// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-driven traversal of statically executable Tcl source regions.
//!
//! Consumers that need to inspect a command embedded in another command must
//! not each invent a partial list of body-bearing commands.  This walker keeps
//! the source-coordinate and command-identity rules in one place: it visits
//! normal body arguments, clause-list arm bodies, lambda bodies, definition
//! members, and live command substitutions.

use tcl_compiler::head_identity::HeadIdentityMap;
use tcl_compiler::lambda_literal::split_lambda_literal;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Token, TokenType};
use tcl_registry::definer::DefinitionBodyGrammar;
use tcl_registry::{ArgRole, CommandRegistry};

use crate::oo_body::{HeadWords, is_member, member_body_indices_in, next_definition_grammar};

/// Defensive recursion limit shared by executable-source walkers.
const MAX_EXECUTABLE_REGION_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Visit every statically locatable command that Tcl can execute from
/// `source`, preserving each command's absolute source spans.
///
/// `visitor` returns `true` to stop early.  A callback receives the head in
/// its written and registry-resolved forms; the latter is empty for a proven
/// rebound command, so registry grammar is never applied to a shadowed or
/// mutated builtin.  Definition members deliberately use their written head:
/// they are lexical keywords, not global command bindings.
pub(crate) fn visit_executable_commands(
    source: &str,
    config: LexerConfig,
    registry: &CommandRegistry,
    availability: tcl_dialect::DialectSet,
    identities: &HeadIdentityMap,
    visitor: &mut impl FnMut(&SegmentedCommand, HeadWords<'_>) -> bool,
) {
    let mut walk = ExecutableWalker {
        source,
        config,
        registry,
        availability,
        identities,
        visitor,
    };
    let _ = walk.region(0, source.len(), 0, None);
}

struct ExecutableWalker<'a, F> {
    source: &'a str,
    config: LexerConfig,
    registry: &'a CommandRegistry,
    availability: tcl_dialect::DialectSet,
    identities: &'a HeadIdentityMap,
    visitor: &'a mut F,
}

impl<F: FnMut(&SegmentedCommand, HeadWords<'_>) -> bool> ExecutableWalker<'_, F> {
    fn region(
        &mut self,
        start: usize,
        end: usize,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
    ) -> bool {
        if MAX_EXECUTABLE_REGION_DEPTH.exceeded(depth) || start >= end || end > self.source.len() {
            return false;
        }
        let commands = segment_commands_with_offset_and_config(
            &self.source[start..end],
            u32::try_from(start).unwrap_or(0),
            self.config.at_depth(depth),
        );
        for command in &commands {
            let Some(head_tok) = command.argv.first() else {
                continue;
            };
            let head = self
                .identities
                .head_words(command.name(), head_tok.span.start());
            if (self.visitor)(command, head) {
                return true;
            }

            // A command substitution is live wherever it appears in a bare
            // or quoted word, including a command's own head.  Braced words
            // are intentionally excluded by `command_substitution_regions`.
            for token in &command.argv {
                for (inner_start, inner_end) in
                    command_substitution_regions(self.source, self.config, *token)
                {
                    if self.region(inner_start, inner_end, depth + 1, grammar) {
                        return true;
                    }
                }
            }

            let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
            let member = grammar.filter(|g| is_member(g, head.written));
            let body_indices = member.map_or_else(
                || {
                    self.registry
                        .arg_indices_for_role(head.resolved, &args, ArgRole::Body)
                },
                |g| member_body_indices_in(g, head.written, &args, self.availability),
            );
            let next_grammar = next_definition_grammar(head, &args, grammar, self.registry);
            let case_list = self
                .registry
                .case_invocation(head.resolved, &args, self.availability)
                .and_then(|(spec, invocation)| invocation.clause_list_index.map(|i| (spec, i)));

            // A case-list descriptor owns its nested scripts even when the
            // outer list word has no generic `ArgRole::Body`.  In particular,
            // Expect's descriptor supplies its clause bodies directly rather
            // than duplicating that grammar as a flat argument role.
            if let Some((spec, case_index)) = case_list
                && let (Some(&token), Some(list)) =
                    (command.arg_tokens().get(case_index), args.get(case_index))
                && token.kind == TokenType::Str
                && self.case_list_regions(token, list, &spec, depth, next_grammar)
            {
                return true;
            }

            for index in body_indices {
                let Some(&token) = command.arg_tokens().get(index) else {
                    continue;
                };
                if token.kind != TokenType::Str {
                    continue;
                }
                if case_list.is_some_and(|(_, case_index)| case_index == index) {
                    continue;
                }
                if let Some((body_start, body_end)) = braced_body_region(self.source, token)
                    && self.region(body_start, body_end, depth + 1, next_grammar)
                {
                    return true;
                }
            }

            for index in
                self.registry
                    .arg_indices_for_role(head.resolved, &args, ArgRole::LambdaLiteral)
            {
                let Some(&token) = command.arg_tokens().get(index) else {
                    continue;
                };
                let Some(body) = split_lambda_literal(self.source, token)
                    .and_then(|elements| elements.braced_body())
                else {
                    continue;
                };
                if self.region(body.start() as usize, body.end() as usize, depth + 1, None) {
                    return true;
                }
            }
        }
        false
    }

    fn case_list_regions(
        &mut self,
        token: Token,
        list: &str,
        spec: &tcl_registry::CaseListSpec,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
    ) -> bool {
        // The compiler-owned flattener is the source-coordinate bridge between
        // a Tcl list element and a source token.  Besides braced actions, it
        // marks a substitution-free quoted action as `Str`; dynamic and
        // backslash-built actions remain opaque, so this traversal cannot
        // invent a statically executable body.
        for (_, (body_text, body_token)) in
            tcl_compiler::segmenter::flatten_case_list_clauses(self.source, list, token, spec)
        {
            if spec.fallthrough_body == Some(body_text.as_str()) {
                continue;
            }
            let Some((start, end)) = case_action_region(self.source, body_token) else {
                continue;
            };
            if self.region(start, end, depth + 1, grammar) {
                return true;
            }
        }
        false
    }
}

/// Return a statically executable case action's verbatim script region.
///
/// [`tcl_compiler::segmenter::flatten_case_list_clauses`] only gives a
/// `TokenType::Str` body token to an action that it proved literal.  List
/// elements may still be braced, quoted, or bare, so strip their opening
/// delimiter here while retaining the segmenter's absolute spans.  The list
/// parser's token end is already the byte at the closing delimiter.
fn case_action_region(source: &str, token: Token) -> Option<(usize, usize)> {
    if token.kind != TokenType::Str {
        return None;
    }
    let start = token.span.start() as usize;
    let end = token.span.end() as usize;
    if start >= end || end > source.len() {
        return None;
    }
    let bytes = source.as_bytes();
    let start = start + usize::from(matches!(bytes.get(start), Some(b'{' | b'"')));
    (start < end).then_some((start, end))
}

/// Return a braced body's verbatim source content, excluding delimiters.
fn braced_body_region(source: &str, token: Token) -> Option<(usize, usize)> {
    let start = token.span.start() as usize + token.content_offset as usize;
    let raw_end = token.span.end() as usize;
    let bytes = source.as_bytes();
    let end = if raw_end > start && raw_end - start == 1 && bytes.get(raw_end - 1) == Some(&b'}') {
        start
    } else {
        raw_end
    };
    (start < end && end <= source.len()).then_some((start, end))
}

/// Locate active bracket substitutions inside one token, preserving absolute
/// offsets.  The segmenter coalesces compound bare/quoted words into `Esc`,
/// so those are re-lexed to recover every embedded `Cmd` fragment.
fn command_substitution_regions(
    source: &str,
    config: LexerConfig,
    token: Token,
) -> Vec<(usize, usize)> {
    let start = token.span.start() as usize;
    let end = token.span.end() as usize;
    if start >= end || end > source.len() {
        return Vec::new();
    }
    let strip = |fragment_start: usize, fragment_end: usize| {
        let inner_start =
            fragment_start + usize::from(source.as_bytes().get(fragment_start) == Some(&b'['));
        let inner_end = fragment_end
            - usize::from(
                fragment_end > inner_start
                    && source.as_bytes().get(fragment_end - 1) == Some(&b']'),
            );
        (inner_start, inner_end)
    };
    match token.kind {
        TokenType::Cmd => vec![strip(start, end)],
        TokenType::Esc if source.as_bytes()[start..end].contains(&b'[') => {
            let Ok(tokens) =
                Lexer::with_source_map(SourceMap::new(&source[start..end]), config).tokenise_all()
            else {
                return Vec::new();
            };
            tokens
                .into_iter()
                .filter(|token| token.kind == TokenType::Cmd)
                .map(|token| {
                    strip(
                        start + token.span.start() as usize,
                        start + token.span.end() as usize,
                    )
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Whether `cursor` is inside an active command substitution within `token`.
///
/// Consumers that inspect the containing word must defer to the recursively
/// executable command in this case: a widened quoted or compound token is not
/// a literal fragment at a position Tcl evaluates as `[...]`.
pub(crate) fn cursor_in_command_substitution(
    source: &str,
    config: LexerConfig,
    token: Token,
    cursor: u32,
) -> bool {
    command_substitution_regions(source, config, token)
        .into_iter()
        .any(|(start, end)| cursor as usize >= start && (cursor as usize) < end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visited_format_heads(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> Vec<(String, String, u32)> {
        let profile = dialect;
        let registry = tcl_registry::registry_for_profile(profile);
        let config = LexerConfig::for_file_dialect(profile.name);
        let identities = tcl_compiler::head_identity::command_head_identities_with_config(
            source, config, registry,
        );
        let mut heads = Vec::new();
        visit_executable_commands(
            source,
            config,
            registry,
            profile.availability_mask,
            &identities,
            &mut |command, identity| {
                if command.name() == "format" || command.name() == "fmt" {
                    heads.push((
                        identity.written.to_owned(),
                        identity.resolved.to_owned(),
                        command.span.start(),
                    ));
                }
                false
            },
        );
        heads
    }

    #[test]
    fn quoted_case_actions_retain_absolute_spans_for_each_registry_descriptor() {
        for (source, dialect, written) in [
            ("switch $x {a \"format {%d} 1\"}", "tcl8.6", "format"),
            ("expect {-re {ready} \"format {%d} 1\"}", "expect", "format"),
        ] {
            let heads = visited_format_heads(source, tcl_dialect::DialectProfile::by_name(dialect));
            assert_eq!(
                heads,
                vec![(
                    written.to_owned(),
                    "format".to_owned(),
                    u32::try_from(source.rfind(written).expect("nested head")).unwrap(),
                ),],
                "quoted case action must preserve its whole-document command span: {source}"
            );
        }
    }

    #[test]
    fn quoted_case_actions_preserve_identity_and_abstain_for_dynamic_or_malformed_lists() {
        let aliased = "interp alias {} fmt {} format\nswitch $x {a \"fmt {%d} 1\"}";
        assert_eq!(
            visited_format_heads(aliased, tcl_dialect::DialectProfile::by_name("tcl8.6")),
            vec![(
                "fmt".to_owned(),
                "format".to_owned(),
                u32::try_from(aliased.rfind("fmt").expect("nested alias")).unwrap(),
            )]
        );

        let renamed = "rename format saved\nswitch $x {a \"format {%d} 1\"}";
        assert_eq!(
            visited_format_heads(renamed, tcl_dialect::DialectProfile::by_name("tcl8.6")),
            vec![(
                "format".to_owned(),
                String::new(),
                u32::try_from(renamed.rfind("format").expect("nested head")).unwrap(),
            )],
            "the nested call remains executable but must not reclaim a renamed builtin"
        );

        for source in [
            "set actions {a \"format {%d} 1\"}\nswitch $x $actions",
            "switch $x {a \"format {%d} 1\" orphan}",
        ] {
            assert!(
                visited_format_heads(source, tcl_dialect::DialectProfile::by_name("tcl8.6")).is_empty(),
                "dynamic and malformed lists must not expose nested actions: {source}"
            );
        }
    }
}
