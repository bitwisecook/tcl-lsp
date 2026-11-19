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

//! The complete census of one class member's source occurrences, and how a command names or relates the member.

use super::OriginalDeclarationIdentity;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::Span;
use tcl_syntax::native_string::NativeStringProtocol;

/// One classified command of a readonly member census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberCensusCommand {
    /// An immediate self-dispatch whose selector the typed tape must record.
    SelfDispatch,
    /// A command whose proven layout dispatches and captures no member.
    Other,
}

/// Inputs shared by every command of one readonly member census.
struct MemberCensus<'a> {
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    syntax: crate::source_structure::SourceSyntaxStructure,
    pub(super) name: &'a [u8],
    protocol: NativeStringProtocol,
    /// Literal operand text naming the member, with its whole word extent.
    mentions: Vec<(Span, Span)>,
}

/// A complete readonly census of one member's source occurrences. The
/// checked source walk must observe every command of every reachable region,
/// and every command must be classified with a proven layout:
/// - a definition member routes by its selected effect, and forwards,
///   filters, visibility, retraction, mixin relations, method references and
///   command-prefix, command-name or opaque-script roles stay incomplete;
/// - an immediate self-dispatch keeps the Registry selector layout and must
///   correspond one-to-one with the compiler's typed method/property tape;
/// - any other command needs complete selected roles, no retained callback
///   (stored script or command prefix), no substitution template, no
///   unrepresented definer form, no descriptor declaring an implementation
///   namespace (qualified `oo::Helpers` spellings and every `dict` call) and
///   no method-context helper without a declared dispatch kind;
/// - no other literal operand text may name the member outside a walked
///   nested script, since a captured selector (`[list my m]`, `callback m`,
///   an object command call) can dispatch without a self-dispatch site.
///
/// Unknown or dynamic heads, unavailable member geometry and dynamic
/// selectors stay incomplete. The census covers this one document only.
pub(super) fn complete_member_source_census(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    methods: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfMethodReference],
    properties: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference],
) -> bool {
    identity.input.as_ref().is_some_and(|input| {
        complete_member_name_census(
            input.bytes(),
            input.policy().string_protocol(),
            source,
            analysis,
            methods,
            properties,
        )
    })
}

/// [`complete_member_source_census`] for one member name, independently of
/// the declaration that spells it.
pub(crate) fn complete_member_name_census(
    name: &[u8],
    protocol: NativeStringProtocol,
    source: &str,
    analysis: &AnalysisResult,
    methods: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfMethodReference],
    properties: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference],
) -> bool {
    if name.is_empty() || !name.is_ascii() {
        return false;
    }
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    let Some(syntax) = crate::source_structure::SourceSyntaxStructure::capture(source, analysis)
    else {
        return false;
    };
    let mut census = MemberCensus {
        source,
        analysis,
        context: input.context_registry(),
        syntax,
        name,
        protocol,
        mentions: Vec::new(),
    };
    let mut walked = Vec::new();
    let mut uncertain = false;
    let mut regions = Vec::new();
    let status = crate::executable_regions::visit_checked_analysis_executable_commands_with_regions(
        source,
        analysis,
        &mut |command, _head, execution_context| {
            match census.classify(command) {
                Some(MemberCensusCommand::Other) => {}
                Some(MemberCensusCommand::SelfDispatch)
                    if execution_context
                        == crate::executable_regions::ExecutableContext::PotentialBody =>
                {
                    walked.push(command.span);
                }
                Some(MemberCensusCommand::SelfDispatch) | None => uncertain = true,
            }
            false
        },
        &mut regions,
    );
    if status != crate::executable_regions::PotentialSourceVisit::Complete || uncertain {
        return false;
    }
    // A mention is excused only inside a walked script nested in its own
    // word, whose commands were classified one by one.
    if census.mentions.iter().any(|&(mention, word)| {
        !regions.iter().any(|&(region, _)| {
            word.start() <= region.start()
                && region.end() <= word.end()
                && region.start() <= mention.start()
                && mention.end() <= region.end()
        })
    }) {
        return false;
    }
    walked.sort_by_key(|span| (span.start(), span.end()));
    let Some(mut recorded) = methods
        .iter()
        .map(tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfMethodReference::command_span)
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    recorded.extend(properties.iter().map(tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference::command_span));
    recorded.sort_by_key(|span| (span.start(), span.end()));
    walked == recorded && !recorded.windows(2).any(|pair| pair[0] == pair[1])
}

impl MemberCensus<'_> {
    fn classify(
        &mut self,
        command: &tcl_compiler::segmenter::SegmentedCommand,
    ) -> Option<MemberCensusCommand> {
        use tcl_registry::{AuthoredSourceSelfDispatch, Traits};
        let head = command.argv.first()?;
        // Selected definition members have their own scoped vocabulary; a
        // global name lookup cannot classify them, and unavailable member
        // geometry has no global fallback.
        if let Some(member) = self.syntax.definition_member_at(head.span.start()).ok()? {
            return (census_member_matches(&member, command) && census_admits_member(&member))
                .then_some(MemberCensusCommand::Other);
        }
        let words =
            crate::original_invocation::source_registry_words(self.source, self.analysis, command)?;
        let selected = words
            .with_source_schema(&self.context, |schema| {
                if let Some(dispatch) = schema.authored_source_self_dispatch() {
                    return (dispatch != AuthoredSourceSelfDispatch::Unproven)
                        .then_some(MemberCensusCommand::SelfDispatch);
                }
                // `next`, `link`, `callback`/`mymethod` and any other
                // method-context helper without a declared dispatch kind can
                // reach a member without a self-dispatch site.
                let traits = schema.semantics.traits;
                if traits
                    .intersects(Traits::TCLOO_NEXT_CHAIN.union(Traits::TCLOO_BINDS_METHOD_ALIAS))
                    || (traits.contains(Traits::TCLOO_METHOD_CONTEXT)
                        && !traits.contains(Traits::TCLOO_INTROSPECTION))
                {
                    return None;
                }
                // A substitution template (`subst`) evaluates command
                // substitutions that neither the walker nor syntax coverage
                // enters, so an escaped or computed selector stays unseen.
                if traits.contains(Traits::PERFORMS_SUBSTITUTION)
                    || matches!(
                        schema.semantics.body_execution,
                        Some(tcl_registry::body_execution::BodyExecutionSpec::SubstitutionTemplate)
                    )
                {
                    return None;
                }
                // A descriptor that declares an implementation namespace is
                // not classified here: the qualified `::oo::Helpers::next` and
                // `::oo::Helpers::link` spellings drop the dispatch traits of
                // their contextual twins. Every `dict` call is also excluded,
                // because the bare `dict` descriptor declares `::tcl::dict`.
                if schema
                    .authored_source_descriptors()
                    .command
                    .implementation_namespace
                    .is_some()
                {
                    return None;
                }
                // A definer form without a selected definition body (an inline
                // `oo::define C forward …`) installs members no region shows.
                if schema
                    .authored_source_descriptors()
                    .command
                    .definition_body
                    .is_some()
                    && schema.authored_source_definition_body_grammar().is_none()
                {
                    return None;
                }
                (schema.authored_source_retains_callback() == Some(false))
                    .then_some(MemberCensusCommand::Other)
            })
            .flatten()?;
        if selected == MemberCensusCommand::SelfDispatch {
            // The selector is reconciled with the typed tape; passed values
            // may still carry a captured selector.
            return self
                .record_mentions(&words.operands, |position| position == 0)
                .then_some(selected);
        }
        let roles = words.roles.as_deref()?;
        if roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::OpaqueScript)
        {
            return None;
        }
        // Variable-name operands cannot carry a selector.
        let exempt = roles
            .iter()
            .filter(|(_, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::VarRead
                        | tcl_registry::ArgRole::VarWrite
                        | tcl_registry::ArgRole::LoopVarList
                        | tcl_registry::ArgRole::StaticVarList
                        | tcl_registry::ArgRole::ParamList
                )
            })
            .map(|(position, _)| *position)
            .collect::<Vec<_>>();
        self.record_mentions(&words.operands, |position| exempt.contains(&position))
            .then_some(selected)
    }

    /// Record literal operand text naming the member. This is a conservative
    /// lexical token scan of literal parts, not a list or script parse;
    /// command substitutions are classified as commands of their own. An
    /// operand without an original word (a captured or expanded argument)
    /// cannot be scanned, so the command stays unclassified.
    fn record_mentions(
        &mut self,
        operands: &[Option<crate::original_invocation::OriginalOperandSource>],
        skip: impl Fn(usize) -> bool,
    ) -> bool {
        for (position, operand) in operands.iter().enumerate() {
            if skip(position) {
                continue;
            }
            let Some(word) = operand.as_ref().and_then(|operand| operand.word.as_ref()) else {
                return false;
            };
            let Some(mentions) = word_member_mentions(self.source, word, self.name, self.protocol)
            else {
                return false;
            };
            self.mentions
                .extend(mentions.into_iter().map(|mention| (mention, word.span())));
        }
        true
    }
}

/// Literal text of one original word that may name the member, by the
/// census's conservative token scan. Substituted values are not scanned;
/// command substitutions are separate commands. `None` when a raw part has no
/// source text.
pub(crate) fn word_member_mentions(
    source: &str,
    word: &tcl_lexer::NativeWord,
    name: &[u8],
    protocol: NativeStringProtocol,
) -> Option<Vec<Span>> {
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(word),
        protocol,
    )
    .ok()?;
    let mut mentions = Vec::new();
    if let Some(value) = captured.literal(0) {
        // A selector is a complete native value. Whitespace, semicolons and
        // parentheses inside its name remain data, including inside a stored
        // command prefix. A formed value can span several source fragments.
        if census_names_member(value, name) {
            return Some(vec![word.span()]);
        }
        if let Ok(elements) = tcl_syntax::list::split_native_list_elements(value, protocol) {
            for element in elements {
                if census_names_member(&element.value, name) {
                    mentions.push(
                        captured
                            .original_literal_extent(0, element.source.value)
                            .unwrap_or(word.span()),
                    );
                }
            }
        }
    }
    for part in word.executable_parts().all_parts() {
        let tcl_lexer::ExecutablePart::Text(text) = &part.part else {
            continue;
        };
        if let tcl_lexer::ExecutableText::Decoded(bytes) = text {
            if census_tokens(bytes).any(|(_, token)| census_names_member(token, name)) {
                mentions.push(part.span);
            }
            continue;
        }
        let raw = source.get(part.span.as_range())?;
        for (offset, token) in census_tokens(raw.as_bytes()) {
            if !census_names_member(token, name) {
                continue;
            }
            let mention = u32::try_from(offset)
                .ok()
                .and_then(|offset| part.span.start().checked_add(offset))
                .zip(u32::try_from(token.len()).ok())
                .and_then(|(start, len)| Some(Span::new(start, start.checked_add(len)?)))
                .unwrap_or(part.span);
            mentions.push(mention);
        }
    }
    Some(mentions)
}

/// The selected member vector is exactly the walked command: equal extents
/// and word tokens, without the separators a segmented command also keeps.
/// An expanded member keeps its `Expand` token and stays unmatched.
fn census_member_matches(
    member: &crate::source_structure::SourceSyntaxDefinitionMember<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> bool {
    let words = member.original_words();
    words
        .first()
        .is_some_and(|word| word.span().start() == command.span.start())
        && words
            .last()
            .is_some_and(|word| word.span().end() == command.span.end())
        && words
            .iter()
            .flat_map(|word| word.tokens().iter())
            .copied()
            .eq(command.all_tokens.iter().copied().filter(|token| {
                !matches!(
                    token.kind,
                    tcl_lexer::TokenType::Sep | tcl_lexer::TokenType::Eol
                )
            }))
}

/// A member admitted by the census declares or relates only: its effect
/// cannot dispatch or name a method later, and it carries no method
/// reference, command prefix, command name or opaque script. Routing uses
/// the selected member row's effect, never its keyword.
fn census_admits_member(
    member: &crate::source_structure::SourceSyntaxDefinitionMember<'_>,
) -> bool {
    use tcl_registry::ArgRole;
    use tcl_registry::definer::{MemberEffect, MemberRefKind, RelationSlot};
    let region = member.region();
    let words = member.original_words();
    region.checked_member_row(words).is_some_and(|row| {
        matches!(
            row.effect,
            MemberEffect::Callable { .. }
                | MemberEffect::StateDeclaration { .. }
                | MemberEffect::InitScript { .. }
                | MemberEffect::Configuration
                | MemberEffect::Relation {
                    slot: RelationSlot::Superclass
                }
        )
    }) && region.checked_references(words).is_some_and(|references| {
        references
            .iter()
            .all(|reference| reference.kind() == MemberRefKind::Class)
    }) && [
        ArgRole::CommandPrefix,
        ArgRole::CommandName,
        ArgRole::OpaqueScript,
    ]
    .into_iter()
    .all(|role| {
        region
            .argument_words_for_role(words, role)
            .is_some_and(|words| words.is_empty())
    })
}

/// Whether `command` is exactly one selected superclass or mixin relation
/// member naming classes only. It runs no script and dispatches no member.
pub(crate) fn class_relation_member(
    syntax: &crate::source_structure::SourceSyntaxStructure,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> bool {
    use tcl_registry::ArgRole;
    use tcl_registry::definer::{MemberEffect, MemberRefKind, RelationSlot};
    let Some(head) = command.argv.first() else {
        return false;
    };
    let Ok(Some(member)) = syntax.definition_member_at(head.span.start()) else {
        return false;
    };
    let region = member.region();
    let words = member.original_words();
    census_member_matches(&member, command)
        && region.checked_member_row(words).is_some_and(|row| {
            matches!(
                row.effect,
                MemberEffect::Relation {
                    slot: RelationSlot::Superclass | RelationSlot::Mixin
                }
            )
        })
        && region.checked_references(words).is_some_and(|references| {
            references
                .iter()
                .all(|reference| reference.kind() == MemberRefKind::Class)
        })
        && [
            ArgRole::CommandPrefix,
            ArgRole::CommandName,
            ArgRole::OpaqueScript,
        ]
        .into_iter()
        .all(|role| {
            region
                .argument_words_for_role(words, role)
                .is_some_and(|words| words.is_empty())
        })
}

/// An `export`/`unexport` definition member naming methods only.
pub(crate) fn class_visibility_member(
    syntax: &crate::source_structure::SourceSyntaxStructure,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> bool {
    use tcl_registry::definer::{MemberEffect, MemberRefKind};
    let Some(head) = command.argv.first() else {
        return false;
    };
    let Ok(Some(member)) = syntax.definition_member_at(head.span.start()) else {
        return false;
    };
    let region = member.region();
    let words = member.original_words();
    census_member_matches(&member, command)
        && region
            .checked_member_row(words)
            .is_some_and(|row| matches!(row.effect, MemberEffect::Visibility))
        && region.checked_references(words).is_some_and(|references| {
            references
                .iter()
                .all(|reference| reference.kind() == MemberRefKind::Method)
        })
}

/// How a command rename covers one selected definition member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandRenameMember {
    /// A callable or wrapper row, whose declared body the caller locates.
    Declaration,
    /// State, visibility, retraction or a superclass or mixin relation that
    /// names none of the checked commands.
    Inert,
}

/// Classify the selected definition member at this command for a command
/// rename. `Ok(None)` when the command is not a definition member. `Err`
/// when the member may name or run a command the rename cannot follow: a
/// forward, filter, init script or unknown row, a command name, prefix or
/// opaque script operand, a command substitution in any word, or a relation
/// operand that is substituted or whose written command tail is one of
/// `simple_names`.
pub(crate) fn command_rename_member(
    syntax: &crate::source_structure::SourceSyntaxStructure,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    simple_names: &[&[u8]],
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Result<Option<CommandRenameMember>, ()> {
    use tcl_registry::ArgRole;
    use tcl_registry::definer::{MemberEffect, RelationSlot};
    let Some(head) = command.argv.first() else {
        return Ok(None);
    };
    let Some(member) = syntax
        .definition_member_at(head.span.start())
        .map_err(|_| ())?
    else {
        return Ok(None);
    };
    let region = member.region();
    let words = member.original_words();
    if !census_member_matches(&member, command)
        || words.iter().any(|word| {
            word.group().expand
                || word.executable_parts().all_parts().any(|part| {
                    !matches!(
                        part.part,
                        tcl_lexer::ExecutablePart::Text(_)
                            | tcl_lexer::ExecutablePart::Variable { .. }
                    )
                })
        })
        || ![
            ArgRole::CommandPrefix,
            ArgRole::CommandName,
            ArgRole::OpaqueScript,
        ]
        .into_iter()
        .all(|role| {
            region
                .argument_words_for_role(words, role)
                .is_some_and(|words| words.is_empty())
        })
    {
        return Err(());
    }
    let row = region.checked_member_row(words).ok_or(())?;
    match row.effect {
        MemberEffect::Callable { .. } | MemberEffect::Configuration => {
            Ok(Some(CommandRenameMember::Declaration))
        }
        MemberEffect::StateDeclaration { .. }
        | MemberEffect::Visibility
        | MemberEffect::Retraction => Ok(Some(CommandRenameMember::Inert)),
        MemberEffect::Relation {
            slot: RelationSlot::Superclass | RelationSlot::Mixin,
        } => {
            let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                words,
                policy.string_protocol(),
            )
            .map_err(|_| ())?;
            // Each superclass/mixin operand is one native class command name,
            // even when it contains whitespace or script punctuation.
            for index in 1..words.len() {
                let value = captured.literal(index).ok_or(())?;
                let tail = policy.recipe().command_tail_extent(value).ok_or(())?;
                if simple_names.contains(&&value[tail]) {
                    return Err(());
                }
            }
            Ok(Some(CommandRenameMember::Inert))
        }
        _ => Err(()),
    }
}

/// Tokens of literal text split at whitespace and Tcl word, script, list and
/// substitution punctuation, with their byte offsets.
fn census_tokens(bytes: &[u8]) -> impl Iterator<Item = (usize, &[u8])> {
    let mut start = 0;
    bytes
        .split(|byte| byte.is_ascii_whitespace() || b"[]{}\";$\\()".contains(byte))
        .map(move |token| {
            let offset = start;
            start += token.len() + 1;
            (offset, token)
        })
        .filter(|(_, token)| !token.is_empty())
}

/// Whether a literal token may name the member: the name itself, a unique
/// `-` option prefix of it (property `configure`/`cget`), or an internal
/// accessor method name such as `<ReadProp-name>`.
fn census_names_member(token: &[u8], name: &[u8]) -> bool {
    token == name
        || token
            .strip_prefix(b"-")
            .is_some_and(|prefix| !prefix.is_empty() && name.starts_with(prefix))
        || (token.contains(&b'<') && token.windows(name.len()).any(|window| window == name))
}
