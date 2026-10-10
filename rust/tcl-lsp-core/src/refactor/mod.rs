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

//! Mechanical refactoring transforms.
//!
//! Each transform accepts source text plus a cursor (or selection)
//! and returns a [`Refactoring`] describing a titled set of text
//! edits, or `None` when the transform does not apply.  The
//! [`code_actions`](crate::code_actions) provider lifts each
//! [`Refactoring`] into a `CodeAction`.
//!
//! Transforms:
//!
//! * [`extract_proc`] — move a run of commands into a new proc and call
//!   it, carrying caller-frame writes through with `upvar`.
//! * [`extract_variable`] — replace a selected expression with a named
//!   `set` assignment.
//! * [`inline_variable`] — inline a single-use `set` variable.
//! * [`inline_proc`] — replace a call with the proc's body, with its
//!   parameters bound to the call's argument *values*.
//! * [`if_to_switch`] — convert an `if`/`elseif` equality chain to a
//!   `switch`.
//! * [`switch_to_dict`] — convert a constant-mapping `switch` to a
//!   `dict` lookup.
//! * [`extract_to_datagroup`] — convert inline if/switch membership /
//!   mapping patterns to iRules `class match` / `class lookup` against
//!   a generated data-group.
//!
//! ## Coordinate convention
//!
//! Cursors and edit ranges use LSP `(line, character)` coordinates with
//! UTF-16 character columns, matching the existing `extract_proc` /
//! `inline_proc` transforms in [`crate::code_actions`].  Internally a
//! transform works in **byte offsets** (so it can slice `&str` directly)
//! and converts to UTF-16 [`LspRange`] only when building the final
//! edits, via the document's [`LineIndex`].

mod brace_expr;
mod datagroup;
mod extract_proc;
mod extract_variable;
mod if_to_switch;
mod inline_proc;
mod inline_variable;
mod source_command;
mod source_rewrite;
mod switch_to_dict;

pub use brace_expr::brace_expr;
pub use datagroup::{
    DataGroupDefinition, OriginalExactCaseSource, OriginalExactSwitchSource,
    OriginalScalarVariableSubject, ScalarVariableSourceSyntax, data_group_tcl,
    extract_to_datagroup, extract_to_datagroup_from_if, extract_to_datagroup_from_switch,
    extract_to_datagroup_with_analysis, original_exact_case_source_at_analysis,
    original_exact_switch_source_at_analysis, scalar_variable_source_syntax,
};
pub use extract_proc::{extract_proc, extract_proc_rename_command};
pub use extract_variable::extract_variable;
pub use if_to_switch::if_to_switch;
pub use inline_proc::{inline_proc, inline_proc_in_program};
pub use inline_variable::inline_variable;
pub use source_command::find_original_command_at;
pub use switch_to_dict::switch_to_dict;

use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_dialect::BracedVarStyle;
use tcl_lexer::{LexerConfig, LineIndex, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::definition::LspRange;

/// The `${…}` close rule the document's dialect uses.
///
/// A refactor rewrites *text*, so reading a variable name by one release's
/// rule while the document is another's changes what the edit means: on a
/// Tcl 9 document `${a{b}c}` is one variable named `a{b}c`, but the 8.x
/// first-`}` rule reads `a{b` and leaves `c}` looking like ordinary word
/// text.
pub(crate) fn braced_var_style(
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> BracedVarStyle {
    crate::profile_for_analysis(analysis).grammar.braced_var
}

/// Every `$name` / `${name}` reference in `text`, as
/// `(name, start, end)` byte offsets covering the whole reference
/// (`$` through the closing `}`).
///
/// The braced form is delimited by [`tcl_lexer::braced_var_name_end`] — the
/// one release-aware `Tcl_ParseVarName` port — rather than a `find('}')`,
/// so a brace-bearing name is read the way the document's own release reads
/// it. An unterminated `${` is not a reference: it is a parse error in the
/// document, and inventing a name for it would put that name into a
/// generated parameter list.
///
/// Array elements are **not** reduced here; a caller that wants the array's
/// own name (extract-proc's captured set) trims at the `(`.
pub(crate) fn variable_reference_spans(
    text: &str,
    style: BracedVarStyle,
) -> Vec<(String, usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'{') {
            let name_start = index + 2;
            if let tcl_lexer::BracedVarEnd::Closed(close) =
                tcl_lexer::braced_var_name_end(bytes, name_start, style)
            {
                out.push((text[name_start..close].to_string(), index, close + 1));
                index = close + 1;
                continue;
            }
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b':')
        {
            end += 1;
        }
        if end > index + 1 {
            out.push((text[index + 1..end].to_string(), index, end));
            index = end;
        } else {
            index += 1;
        }
    }
    out
}

/// A single text replacement, in byte-offset space.
///
/// The transform layer composes edits in byte offsets and only converts
/// to LSP [`LspRange`] coordinates at the boundary, via
/// [`RefactorEdit::to_lsp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactorEdit {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
    /// Replacement text.
    pub new_text: String,
}

impl RefactorEdit {
    /// Convert this byte-offset edit to an LSP `(line, character)` edit
    /// with UTF-16 character columns.
    #[must_use]
    pub fn to_lsp(&self, source: &str, line_index: &LineIndex) -> crate::rename::TextEdit {
        let start = line_index.position_at_utf16(self.start, source);
        let end = line_index.position_at_utf16(self.end, source);
        crate::rename::TextEdit {
            range: LspRange {
                start_line: start.line,
                start_character: start.character.get(),
                end_line: end.line,
                end_character: end.character.get(),
            },
            new_text: self.new_text.clone(),
        }
    }
}

/// The outcome of a refactoring transform — a titled set of edits plus
/// the LSP code-action kind.
///
/// `data_group` carries the rendered tmsh definition for the
/// extract-to-datagroup transform and is `None` for every other
/// transform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refactoring {
    /// Title shown in the editor.
    pub title: String,
    /// Edits, in byte-offset space.  Empty when [`Self::disabled`] is set —
    /// a refused refactoring has nothing to apply.
    pub edits: Vec<RefactorEdit>,
    /// LSP code-action kind dotted string (`refactor.extract`, …).
    pub kind: crate::code_actions::ActionKind,
    /// Generated data-group definition, when this is an
    /// extract-to-datagroup result.
    pub data_group: Option<DataGroupDefinition>,
    /// Why this refactoring cannot be applied here, when it cannot.
    ///
    /// A transform that finds its subject but *cannot preserve behaviour* says
    /// so rather than vanishing: the reason is surfaced as the code action's
    /// LSP `disabled.reason`, so the editor greys the entry out and explains
    /// itself.  Silently offering nothing leaves a user unable to tell "this
    /// refactoring does not apply here" from "this refactoring is broken".
    pub disabled: Option<String>,
}

impl Refactoring {
    /// Apply the edits to `source` bottom-to-top, returning the
    /// rewritten text.  Edits are sorted by descending start offset so
    /// an earlier edit never shifts a later one's coordinates.
    ///
    /// The sort key is `(start, end)`, not `start` alone: an insertion and a
    /// replacement can legitimately share a start offset — extract-proc puts
    /// the new definition at the line the selection begins on and replaces
    /// that selection with a call — and ordering by start alone leaves which
    /// of the two lands first up to the sort's stability, interleaving the two
    /// texts.  Applying the wider edit first is the only order that composes.
    #[must_use]
    pub fn apply(&self, source: &str) -> String {
        let mut sorted: Vec<&RefactorEdit> = self.edits.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse((e.start, e.end)));
        let mut result = source.to_owned();
        for edit in sorted {
            let start = (edit.start as usize).min(result.len());
            let end = (edit.end as usize).min(result.len()).max(start);
            result.replace_range(start..end, &edit.new_text);
        }
        result
    }
}

/// Exclusive end byte offset for `tok`, including its closing delimiter.
///
/// Lexer token spans follow the
/// inner-end convention — a non-empty `{…}` / `[…]` / `"…"` word's
/// `span.end()` is the exclusive offset of the closer, so the closer
/// sits one byte past the end and a whole-word span must widen by one.
/// An empty `{}` / `[]` / `""` already has its span extended over the
/// closer, so widening would overshoot; the discriminator is whether the
/// token text is empty (the closer is already covered), exactly as the
/// segmenter's `widen_word_end`.
#[must_use]
pub fn token_end_offset(source: &str, tok: Token) -> u32 {
    let bytes = source.as_bytes();
    let start = tok.span.start() as usize;
    let end = tok.span.end();
    // The closer is derived from the *opening* delimiter the token's
    // source slice begins with — a braced (`STR`) word opens with `{`, a
    // command substitution (`CMD`) with `[`, and a quoted word (an `ESC`
    // word whose first source byte is `"`) with `"`.  `in_quote` marks an
    // *interpolated* fragment inside quotes, not a whole quoted word, so
    // the opening byte is the reliable discriminator.
    let closer = match tok.kind {
        TokenType::Str => b'}',
        TokenType::Cmd => b']',
        TokenType::Esc if bytes.get(start) == Some(&b'"') => b'"',
        _ => return end.min(byte_len(source)),
    };
    // Detect the degenerate empty word (`{}` / `[]` / `""`), whose span
    // the lexer already extended over its closer — widening again would
    // overshoot (`a {}}`).  Mirror `SourceMap::token_text`'s
    // emptiness rule: strip the opener via `content_offset`, then treat a
    // remainder of `""`, `"}"`, or `"]"` as the empty case.
    let raw = source.get(start..end as usize).unwrap_or("");
    let stripped = raw.get(tok.content_offset as usize..).unwrap_or("");
    if stripped.is_empty() || stripped == "}" || stripped == "]" {
        return end.min(byte_len(source));
    }
    if bytes.get(end as usize) == Some(&closer) {
        (end + 1).min(byte_len(source))
    } else {
        end.min(byte_len(source))
    }
}

/// Refactoring traversal over the actual retained document configuration and
/// positioned command owner. Conditional structure cannot certify execution.
pub(crate) struct FrameWalk<'a> {
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    dialect: &'static tcl_dialect::DialectProfile,
    nesting: &'a CommandRegistry,
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
    lexical: bool,
    complete: std::cell::Cell<bool>,
    config: LexerConfig,
}

impl<'a> FrameWalk<'a> {
    pub(crate) fn new(
        source: &str,
        analysis: &'a tcl_compiler::analyser::AnalysisResult,
    ) -> Option<Self> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let config = current.config();
        let dialect = current.profile();
        Some(Self {
            analysis,
            dialect,
            nesting: current.registry(),
            identities: analysis.retained_command_realm()?,
            lexical: analysis.allows_lexical_declaration_advice(),
            complete: std::cell::Cell::new(true),
            config,
        })
    }

    pub(crate) fn complete(&self) -> bool {
        self.complete.get()
    }

    pub(crate) fn tokens(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> tcl_compiler::ir::CommandTokens {
        let mut tokens = tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            self.config,
            command,
        );
        self.identities.stamp_original_tokens(&mut tokens);
        tokens
    }

    pub(crate) fn structure(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<tcl_compiler::registry_invocation::ResolvedStatementInvocation> {
        tcl_compiler::registry_invocation::original_structured_invocation_with_metadata_context(
            self.nesting,
            self.metadata_context()?,
            &self.tokens(source, command),
        )
    }

    /// Borrow complete metadata from this actual document owner. Missing,
    /// foreign or changed source policy cannot reopen standalone assistance.
    pub(crate) fn metadata_context(
        &self,
    ) -> Option<tcl_compiler::registry_invocation::InvocationMetadataContext<'a>> {
        tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
            self.nesting,
            self.analysis.resolved_input.as_ref()?,
            self.config,
            Some(self.dialect),
        )
    }

    /// Selected conditional source roles and captured values at this actual
    /// command point. No Native dispatch or editing permission follows.
    pub(crate) fn source_words(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        tcl_compiler::registry_invocation::source_structure::source_registry_words(
            source,
            self.analysis,
            command,
        )
    }

    pub(crate) fn source_context(&self) -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
        self.analysis
            .resolved_input
            .as_ref()
            .expect("captured actual input")
            .context_registry()
    }

    pub(crate) fn source_traits(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<tcl_registry::Traits> {
        self.source_words(source, command)?.with_source_schema(
            &self.analysis.resolved_input.as_ref()?.context_registry(),
            |schema| schema.semantics.traits,
        )
    }

    /// Unknown selected options withdraw the template query; a command that
    /// performs no template substitution retains that separate answer.
    pub(crate) fn substitution_kinds(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<Option<tcl_registry::substitution::SubstitutionKinds>> {
        self.source_words(source, command)?.with_source_schema(
            &self.analysis.resolved_input.as_ref()?.context_registry(),
            |schema| {
                if schema
                    .semantics
                    .traits
                    .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
                {
                    schema
                        .authored_source_substitution_proposal(schema.words.arguments())
                        .map(Some)
                } else {
                    Some(None)
                }
            },
        )?
    }

    pub(crate) fn native_words(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        tcl_compiler::registry_invocation::original_native_compiler_words(
            &tcl_lexer::SourceImage::document(source),
            self.tokens(source, command).words(),
            command.argv.first()?.span.start(),
            self.config,
        )
    }

    /// Exact word and expression components under the selected conditional
    /// grammar. Braced data is never scanned as a variable or script.
    pub(crate) fn components(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Option<Vec<tcl_lexer::ExecutablePartArena>> {
        let words = self.native_words(source, command)?;
        let selected = self.source_words(source, command)?;
        selected.roles()?;
        let mut arenas = words
            .iter()
            .map(|word| word.executable_parts().clone())
            .collect::<Vec<_>>();
        for (written, role) in selected.written_argument_roles() {
            if role != ArgRole::Expr {
                continue;
            }
            let word = words.get(written.checked_add(1)?)?;
            if word.group().kind != tcl_lexer::WordKind::Braced {
                continue;
            }
            let span = word.content_span().ok()?;
            selected.source_expression_script_bodies_at(
                self.analysis.resolved_input.as_ref()?,
                span.start(),
            )?;
            arenas.push(
                tcl_lexer::ExecutablePartArena::decompose(
                    word.image().clone(),
                    span,
                    tcl_lexer::word_parts::SubstFlags::default(),
                    self.config,
                )
                .ok()?,
            );
        }
        if let Some(kinds) = self.substitution_kinds(source, command)? {
            let variables = match selected.dialect()?.family()? {
                tcl_dialect::model::Family::Tcl => {
                    tcl_lexer::word_parts::TemplateVariableSyntax::CTcl
                }
                tcl_dialect::model::Family::Jim => {
                    tcl_lexer::word_parts::TemplateVariableSyntax::Jim084
                }
                _ => return None,
            };
            for word in words
                .iter()
                .skip(1)
                .filter(|word| word.group().kind == tcl_lexer::WordKind::Braced)
            {
                arenas.push(
                    tcl_lexer::ExecutablePartArena::decompose_template(
                        word.image().clone(),
                        word.content_span().ok()?,
                        tcl_lexer::word_parts::SubstFlags {
                            vars: kinds.variables,
                            cmds: kinds.commands,
                            backslashes: kinds.backslashes,
                            ..Default::default()
                        },
                        self.config,
                        variables,
                    )
                    .ok()?,
                );
            }
        }
        Some(arenas)
    }

    fn original_regions(
        &self,
        source: &str,
        command: &SegmentedCommand,
        structural: bool,
    ) -> Option<Vec<(usize, usize)>> {
        let selected = self.structure(source, command)?;
        let tokens = self.tokens(source, command);
        let words = self.native_words(source, command)?;
        let mut regions = Vec::new();
        if !structural {
            for arena in self.components(source, command)? {
                for part in arena.all_parts() {
                    match part.part {
                        tcl_lexer::ExecutablePart::Command { body } => {
                            regions.push((body.start() as usize, body.end() as usize))
                        }
                        tcl_lexer::ExecutablePart::ParseError(_)
                        | tcl_lexer::ExecutablePart::Expression { .. } => return None,
                        _ => {}
                    }
                }
            }
        }
        let body_kind_matches = if structural {
            selected.facts.body_kind == tcl_registry::BodyKind::Structural
        } else {
            selected.facts.body_kind == tcl_registry::BodyKind::Plain
        };
        let body_span = |argument: usize, element: Option<usize>| -> Option<tcl_lexer::Span> {
            let origin = selected.effective.origins.get(argument.checked_add(1)?)?;
            let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) = origin
            else {
                return None;
            };
            let word = words.get(*written)?;
            if let Some(element) = element {
                let parent = tokens
                    .source_binding
                    .as_ref()?
                    .original_written_name_input(&tokens, *written)?;
                let children = parent.original_list_elements_with_source_spans()?;
                let (child, span) = children.get(element)?;
                let span = (*span)?;
                let raw = word.image().bytes().get(span.as_range())?;
                (tcl_syntax::backslash::native_source_literal_bytes(
                    raw,
                    word.image().channel(),
                    child.policy().string_protocol(),
                )
                .ok()?
                .as_ref()
                    == child.bytes())
                .then_some(span)
            } else {
                (word.group().kind == tcl_lexer::WordKind::Braced && !word.group().expand)
                    .then_some(())?;
                word.content_span().ok()
            }
        };
        if body_kind_matches {
            let flow = selected.with_argument_words(|arguments| {
                tcl_registry::case_bodies::script_body_flow_in_registry(
                    self.nesting,
                    &selected.facts,
                    arguments.arguments(),
                )
            });
            if let tcl_registry::script_body_flow::ScriptBodyFlow::CaseBodies(cases) = flow {
                if cases.selection_unknown {
                    return None;
                }
                for body in cases.bodies {
                    let span = body_span(body.argument, body.list_element)?;
                    regions.push((span.start() as usize, span.end() as usize));
                }
            } else {
                for &(index, role) in &selected.facts.arg_roles {
                    if role != ArgRole::Body {
                        continue;
                    }
                    let span =
                        body_span(selected.facts.argument_offset + usize::from(index), None)?;
                    regions.push((span.start() as usize, span.end() as usize));
                }
            }
        }
        if structural {
            for &(index, role) in &selected.facts.arg_roles {
                if role != ArgRole::LambdaLiteral {
                    continue;
                }
                let span = body_span(selected.facts.argument_offset + usize::from(index), Some(1))?;
                regions.push((span.start() as usize, span.end() as usize));
            }
        }
        regions.sort_unstable();
        regions.dedup();
        Some(regions)
    }

    /// Every command nested inside `command` that still runs in `command`'s
    /// own variable frame, appended to `out` innermost-first.
    ///
    /// The nesting is [`crate::references::nested_dispatch_regions`]'s answer
    /// — the same walker Find All References and the caller-frame scan use for
    /// "which nested scripts run in this frame": an [`ArgRole::Body`] argument
    /// with a `Plain` body kind (`if`, `while`, `foreach`, `try`, `catch`, …),
    /// a `switch`-style clause list flattened through the registry's own
    /// `CaseListSpec`, and every `[…]` command substitution.  A `Structural`
    /// body — `proc`, `namespace eval`, `uplevel`, `oo::define` — and `apply`'s
    /// lambda are deliberately *not* descended: what they read and write are
    /// their own frame's variables, so counting them would put a stranger's
    /// name in a generated parameter list or an `upvar` against a variable the
    /// moved code never touches.
    pub(crate) fn nested_same_frame_commands(
        &self,
        source: &str,
        command: &SegmentedCommand,
        out: &mut Vec<SegmentedCommand>,
    ) {
        self.walk_same_frame(source, command, 0, out);
    }

    fn walk_same_frame(
        &self,
        source: &str,
        command: &SegmentedCommand,
        depth: u32,
        out: &mut Vec<SegmentedCommand>,
    ) {
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
            self.complete.set(false);
            return;
        }
        for (start, end) in self.same_frame_regions(source, command) {
            let Some(text) = source.get(start..end) else {
                continue;
            };
            for nested in segment_commands_with_offset_and_config(
                text,
                u32::try_from(start).unwrap_or(0),
                self.config,
            ) {
                if nested.name().is_empty() {
                    continue;
                }
                self.walk_same_frame(source, &nested, depth + 1, out);
                out.push(nested);
            }
        }
    }

    /// `text`, segmented at its real offset in the walked source.
    pub(crate) fn segment(&self, text: &str, offset: u32) -> Vec<SegmentedCommand> {
        segment_commands_with_offset_and_config(text, offset, self.config)
    }

    /// The regions of `command` that run in `command`'s own variable frame.
    ///
    /// Genuine selected source bodies and original word/expression/template
    /// components own these regions under the complete current input. Unknown
    /// schema, grammar or template switches mark the traversal incomplete.
    pub(crate) fn same_frame_regions(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Vec<(usize, usize)> {
        if !self.lexical {
            return self
                .original_regions(source, command, false)
                .unwrap_or_else(|| {
                    self.complete.set(false);
                    Vec::new()
                });
        }
        self.source_regions(source, command, false)
            .unwrap_or_else(|| {
                self.complete.set(false);
                Vec::new()
            })
    }

    fn source_regions(
        &self,
        source: &str,
        command: &SegmentedCommand,
        structural: bool,
    ) -> Option<Vec<(usize, usize)>> {
        let words = self.source_words(source, command)?;
        words.roles()?;
        let context = self.analysis.resolved_input.as_ref()?.context_registry();
        let kind = words.with_source_schema(&context, |schema| schema.semantics.body_kind)?;
        let mut regions = Vec::new();
        if !structural {
            for arena in self.components(source, command)? {
                for part in arena.all_parts() {
                    match part.part {
                        tcl_lexer::ExecutablePart::Command { body } => {
                            regions.push((body.start() as usize, body.end() as usize));
                        }
                        tcl_lexer::ExecutablePart::ParseError(_)
                        | tcl_lexer::ExecutablePart::Expression { .. } => return None,
                        _ => {}
                    }
                }
            }
        }
        if kind
            == if structural {
                tcl_registry::BodyKind::Structural
            } else {
                tcl_registry::BodyKind::Plain
            }
        {
            regions.extend(
                words
                    .source_script_bodies_for_mutation_coverage(&context)?
                    .into_iter()
                    .map(|body| {
                        let span = body.content_span();
                        (span.start() as usize, span.end() as usize)
                    }),
            );
        }
        if structural {
            regions.extend(crate::references::frame_shifted_dispatch_regions(
                source,
                self.analysis,
                self.dialect,
                command,
            ));
        }
        regions.sort_unstable();
        regions.dedup();
        Some(regions)
    }

    /// The regions of `command` that open a variable frame of their own — the
    /// exact complement of [`Self::same_frame_regions`].
    pub(crate) fn frame_shifted_regions(
        &self,
        source: &str,
        command: &SegmentedCommand,
    ) -> Vec<(usize, usize)> {
        if !self.lexical {
            return self
                .original_regions(source, command, true)
                .unwrap_or_else(|| {
                    self.complete.set(false);
                    Vec::new()
                });
        }
        self.source_regions(source, command, true)
            .unwrap_or_else(|| {
                self.complete.set(false);
                Vec::new()
            })
    }

    /// Every region inside `command`, at any same-frame depth, that runs in a
    /// variable frame of its own, appended to `out`.
    ///
    /// A `proc` nested in an `if` branch is found as readily as one at
    /// `command`'s own top level.
    pub(crate) fn frame_shifted_regions_within(
        &self,
        source: &str,
        command: &SegmentedCommand,
        out: &mut Vec<(u32, u32)>,
    ) {
        let mut push = |regions: Vec<(usize, usize)>| {
            for (start, end) in regions {
                let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
                    continue;
                };
                out.push((start, end));
            }
        };
        push(self.frame_shifted_regions(source, command));
        let mut nested = Vec::new();
        self.nested_same_frame_commands(source, command, &mut nested);
        for inner in &nested {
            push(self.frame_shifted_regions(source, inner));
        }
    }
}

fn byte_len(source: &str) -> u32 {
    u32::try_from(source.len()).unwrap_or(u32::MAX)
}

/// `(start, end)` byte offsets covering the full command text.
///
/// The segmenter's `span` already widens for a
/// trailing brace / bracket but deliberately **not** for a trailing
/// quote (the segmenter keeps the inner-end for `"…"` words), so widen
/// the end here to the final argv word's [`token_end_offset`] — this
/// covers the closing `"` of a command like `set name "hello"`.
#[must_use]
pub fn command_span_offsets(source: &str, cmd: &SegmentedCommand) -> (u32, u32) {
    let len = byte_len(source);
    let mut end = cmd.span.end();
    if let Some(&last) = cmd.argv.last() {
        end = end.max(token_end_offset(source, last));
    }
    (cmd.span.start().min(len), end.min(len))
}

/// Recursively find the innermost command at byte offset `cursor`.
///
/// Walks into registry-resolved
/// `ArgRole::Body` arguments (proc / when / if / while / foreach
/// bodies, …) so transforms apply inside nested bodies; the innermost
/// match wins.  When `predicate` is `Some(name)`, only a command whose
/// first word equals `name` is returned.
///
/// Body words are re-segmented at their absolute offset
/// ([`segment_commands_with_offset_and_config`]) so the returned command's
/// spans are always in the outer source buffer's offset space — the caller
/// can slice `source` with them directly.
///
/// `config` is the document's [`LexerConfig`] — every re-segmentation this
/// walk performs, at any nesting depth, lexes under it rather than the
/// default grammar.
#[must_use]
pub fn find_command_at(
    source: &str,
    cursor: u32,
    predicate: Option<&str>,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<SegmentedCommand> {
    find_command_at_inner(source, cursor, predicate, registry, config, 0)
}

/// Recursively walk every command in `source`, including those nested inside
/// registry-resolved body arguments (proc / when / if / while / foreach
/// bodies, …). Returns `(texts, line, character)` per command — `texts` is the
/// command's words (name first), `(line, character)` is the 0-based
/// (UTF-16-counted) position of its start.
///
/// `config` is the document's [`LexerConfig`], threaded into every
/// re-segmentation the walk performs.
#[must_use]
pub fn walk_commands(
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Vec<(Vec<String>, u32, u32)> {
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    walk_commands_inner(
        &WalkCtx {
            full: source,
            registry,
            config,
            line_index: &line_index,
        },
        source,
        0,
        0,
        &mut out,
    );
    out
}

/// What a nested-command walk carries unchanged through its recursion:
/// the whole document, the registry that classifies body words, the
/// document's lexer config and its line index.
struct WalkCtx<'a> {
    full: &'a str,
    registry: &'a CommandRegistry,
    config: LexerConfig,
    line_index: &'a LineIndex,
}

fn walk_commands_inner(
    ctx: &WalkCtx<'_>,
    slice: &str,
    offset: u32,
    depth: u32,
    out: &mut Vec<(Vec<String>, u32, u32)>,
) {
    if MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
        return;
    }
    for cmd in segment_commands_with_offset_and_config(slice, offset, ctx.config) {
        let pos = ctx.line_index.position_at_utf16(cmd.span.start(), ctx.full);
        out.push((cmd.texts.clone(), pos.line, pos.character.get()));
        // Spans here are absolute into `full` (the segmenter was given
        // `offset`), so the lambda splitter reads from `full` too.
        for body in body_words(ctx.full, &cmd, ctx.registry) {
            let Some(body_slice) = ctx
                .full
                .get(body.inner_start as usize..body.inner_end as usize)
            else {
                continue;
            };
            walk_commands_inner(ctx, body_slice, body.inner_start, depth + 1, out);
        }
    }
}

/// Defensive recursion bound for the nested-command search, set to match the
/// compiler analyser's `MAX_BODY_DEPTH` so deeply (but validly) nested code
/// still resolves the command under the cursor. Real source never nests
/// anywhere near this.
pub(super) const MAX_COMMAND_SEARCH_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

fn find_command_at_inner(
    source: &str,
    cursor: u32,
    predicate: Option<&str>,
    registry: &CommandRegistry,
    config: LexerConfig,
    depth: u32,
) -> Option<SegmentedCommand> {
    if MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
        return None;
    }
    for cmd in segment_commands_with_offset_and_config(source, 0, config) {
        let (cmd_start, cmd_end) = command_span_offsets(source, &cmd);
        if cursor < cmd_start || cursor > cmd_end {
            continue;
        }
        // Recurse into body arguments first — the innermost match wins.
        for body in body_words(source, &cmd, registry) {
            if let Some(inner) = find_command_at_inner(
                &source[body.inner_start as usize..body.inner_end as usize],
                cursor.saturating_sub(body.inner_start),
                predicate,
                registry,
                config,
                depth + 1,
            ) {
                // Relocate the inner command's spans back into `source`.
                return Some(inner.shifted_by(body.inner_start));
            }
        }
        if predicate.is_none_or(|name| cmd.name() == name) {
            return Some(cmd);
        }
    }
    None
}

/// A registry-resolved body word and the byte range of its interior
/// (delimiters excluded) in the outer source buffer.
struct BodyWord {
    inner_start: u32,
    inner_end: u32,
}

/// Yield the script-bearing words of `cmd` that [`find_command_at_inner`]
/// may descend into, as interior byte ranges (delimiters excluded) in the
/// outer source buffer.
///
/// Two registry roles qualify, both resolved from the spec — never from a
/// command name:
///
/// * [`ArgRole::Body`] — a plain braced script; the whole interior is code.
/// * [`ArgRole::LambdaLiteral`] — `apply`'s `{argList body ?ns?}` two- or
///   three-element list, where only element 1 is code.  It is split with
///   [`tcl_compiler::lambda_literal::split_lambda_literal`], the same
///   splitter every other consumer uses.  Treating
///   the whole literal as one body instead reads `argList` as a command
///   name and swallows the real body, which is why no refactor code action
///   fired inside an `apply` lambda.  Only a `{braced}` body element is
///   descended
///   ([`LambdaLiteralElements::braced_body`](tcl_compiler::lambda_literal::LambdaLiteralElements::braced_body)):
///   a bare / double-quoted one is backslash-decoded before `apply`
///   evaluates it, so its source slice is not the script that runs and the
///   spans a code action derived from it would edit the wrong bytes.
fn body_words(source: &str, cmd: &SegmentedCommand, registry: &CommandRegistry) -> Vec<BodyWord> {
    let name = cmd.name();
    if name.is_empty() {
        return Vec::new();
    }
    let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for idx in registry.arg_indices_for_role(name, &args, ArgRole::Body) {
        // `arg_indices_for_role` is 0-based over the post-name args; the
        // representative argv token sits at `idx + 1`.
        let Some(&tok) = cmd.argv.get(idx + 1) else {
            continue;
        };
        if tok.kind != TokenType::Str {
            continue;
        }
        // The STR token's span starts on the opening `{` and ends before
        // the closing `}` (inner-end convention).  The descended interior
        // is one byte past the `{` up to the span end.
        let inner_start = tok.span.start() + u32::from(tok.content_offset);
        let inner_end = tok.span.end();
        push_body_word(&mut out, inner_start, inner_end);
    }
    for idx in registry.arg_indices_for_role(name, &args, ArgRole::LambdaLiteral) {
        let Some(&tok) = cmd.argv.get(idx + 1) else {
            continue;
        };
        // A `$var` / `[cmd]`-computed lambda can't be split statically —
        // the guard every `LambdaLiteral` consumer applies.
        if tok.kind != TokenType::Str {
            continue;
        }
        let Some(body) = tcl_compiler::lambda_literal::split_lambda_literal(source, tok)
            .and_then(|elems| elems.braced_body())
        else {
            continue;
        };
        push_body_word(&mut out, body.start(), body.end());
    }
    out
}

/// Record a descendable interior range, skipping an empty one (nothing to
/// descend into).
fn push_body_word(out: &mut Vec<BodyWord>, inner_start: u32, inner_end: u32) {
    if inner_end > inner_start {
        out.push(BodyWord {
            inner_start,
            inner_end,
        });
    }
}

/// Leading-whitespace indent of `line`.
#[must_use]
pub fn line_indent(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Re-indent a (possibly braced) body to `target_indent`.
///
/// Used by `if_to_switch` and `extract_datagroup`: strips one layer
/// of surrounding braces, computes
/// the body's minimum indent, and re-emits each non-blank line at
/// `target_indent` (preserving interior blank lines).  An empty body
/// becomes `"{target_indent}# empty"`.
#[must_use]
pub fn reindent_body(body: &str, target_indent: &str) -> String {
    let mut text = body.trim();
    if text.starts_with('{') && text.ends_with('}') && text.len() >= 2 {
        text = text[1..text.len() - 1].trim();
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min();
    let Some(min_indent) = min_indent else {
        return format!("{target_indent}# empty");
    };
    let mut result: Vec<String> = Vec::new();
    for ln in &lines {
        if ln.trim().is_empty() {
            if !result.is_empty() {
                result.push(String::new());
            }
        } else {
            result.push(format!("{target_indent}{}", &ln[min_indent..]));
        }
    }
    result.join("\n")
}

/// Strip one layer of surrounding double quotes, if present.
#[must_use]
pub fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
pub(crate) fn test_registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_is_bottom_to_top() {
        let r = Refactoring {
            title: "t".into(),
            edits: vec![
                RefactorEdit {
                    start: 6,
                    end: 8,
                    new_text: "99".into(),
                },
                RefactorEdit {
                    start: 0,
                    end: 3,
                    new_text: "let".into(),
                },
            ],
            kind: crate::code_actions::ActionKind::RefactorRewrite,
            data_group: None,
            disabled: None,
        };
        assert_eq!(r.apply("set x 42"), "let x 99");
    }

    #[test]
    fn find_command_descends_into_proc_body() {
        let reg = test_registry();
        let src = "proc handler {m} {\n    if {$m eq \"GET\"} { go }\n}";
        // Cursor on the inner `if` (line 1).
        let cursor = u32::try_from(src.find("if {").unwrap()).unwrap() + 1;
        let cmd = find_command_at(src, cursor, Some("if"), &reg, LexerConfig::default())
            .expect("inner if");
        assert_eq!(cmd.name(), "if");
        // The returned span is in the outer buffer's offsets.
        let (s, _e) = command_span_offsets(src, &cmd);
        assert!(src[s as usize..].starts_with("if {$m"));
    }

    /// Parity with [`find_command_descends_into_proc_body`] for `apply`'s
    /// lambda literal.  `apply`'s first argument is
    /// `ArgRole::LambdaLiteral`, not `Body`: the whole `{argList body}`
    /// blob is not a script, so descending into it verbatim reads
    /// `argList` as a command name and swallows the real body.  Splitting
    /// it reaches the body element, and the `if` inside is found exactly
    /// as it is inside a `proc`.  tclsh8.6/9.0-verified that this lambda
    /// really does run that `if` (`get` / `post` / `other`).
    #[test]
    fn find_command_descends_into_apply_lambda_body() {
        let reg = test_registry();
        let src = "proc handler {} {\n    apply {{m} {\n        if {$m eq \"GET\"} {\n            puts get\n        } elseif {$m eq \"POST\"} {\n            puts post\n        } else {\n            puts other\n        }\n    }} $x\n}\n";
        // Cursor on the `if` inside the lambda body.
        let cursor = u32::try_from(src.find("if {$m").unwrap()).unwrap() + 1;
        let cmd = find_command_at(src, cursor, Some("if"), &reg, LexerConfig::default())
            .expect("if inside apply lambda");
        assert_eq!(cmd.name(), "if");
        // The returned span is in the outer buffer's offsets.
        let (start, _end) = command_span_offsets(src, &cmd);
        assert!(
            src[start as usize..].starts_with("if {$m"),
            "span must relocate back into the outer buffer: {:?}",
            &src[start as usize..]
        );
    }

    /// TN: the lambda's *argument-list* element is a plain word list, not
    /// code.  A cursor on it must not produce a command match — descending
    /// into it is exactly the mis-read the split avoids.
    #[test]
    fn find_command_does_not_match_inside_the_lambda_arg_list() {
        let reg = test_registry();
        let src = "apply {{if} {\n    puts $if\n}} 1\n";
        // Cursor on the `if` *parameter name* in the argument list.
        let cursor = u32::try_from(src.find("{if}").unwrap()).unwrap() + 1;
        assert!(
            find_command_at(src, cursor, Some("if"), &reg, LexerConfig::default()).is_none(),
            "a parameter word named `if` is not an `if` command"
        );
    }

    #[test]
    fn reindent_strips_braces_and_normalises() {
        let out = reindent_body("{\n        puts hi\n    }", "    ");
        assert_eq!(out, "    puts hi");
    }
}

#[cfg(test)]
mod original_frame_walk_tests {
    use super::*;
    #[test]
    fn original_frame_walk_keeps_full_document_context_and_absolute_components() {
        // Implementation contract: naming.refactor.original-frame-traversal
        // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
        let source = "proc p {x} {expr {$x + [string length VALUE]}}\np 2\n";
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let mut analysis = analyser.analyse(source, "tcl8.6").clone();
        let selected_registry = analysis.resolved_registry().unwrap() as *const CommandRegistry;
        analysis.dialect = "f5-irules".to_owned();
        let walk = FrameWalk::new(source, &analysis).unwrap();
        assert_eq!(walk.nesting as *const CommandRegistry, selected_registry);
        assert_eq!(walk.config, analysis.body_lexer_config.unwrap());
        assert!(std::ptr::eq(
            walk.identities,
            analysis.retained_command_realm().unwrap()
        ));
        let start = u32::try_from(source.find("expr").unwrap()).unwrap();
        let end = source.find("}\np").unwrap();
        let command = walk.segment(&source[start as usize..end], start).remove(0);
        let words = walk.native_words(source, &command).unwrap();
        assert_eq!(words[0].span().start(), start);
        assert!(
            words
                .iter()
                .all(|word| word.image() == &tcl_lexer::SourceImage::document(source))
        );
        assert!(FrameWalk::new(&format!("{source}# stale"), &analysis).is_none());
        assert!(FrameWalk::new("expr {$x + [string length VALUE]}", &analysis).is_none());
    }
}

#[cfg(test)]
pub(crate) fn test_logical_analysis(
    source: &str,
    environment: &str,
    registry: std::sync::Arc<CommandRegistry>,
) -> tcl_compiler::analyser::AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = LexerConfig::from_grammar(profile.grammar);
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for(environment).with_command_store(registry),
    );
    tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

#[cfg(test)]
mod selected_frame_context_tests {
    use super::*;

    #[test]
    fn original_frame_roles_keep_actual_availability_captures_and_source_currency() {
        // naming.refactor.original-frame-traversal
        // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
        let mut registry = CommandRegistry::build_default();
        let mut setter = registry.get("set").unwrap().clone();
        setter.name = "selected_write";
        setter.surface = Some(tcl_registry::model::SpecSurface::TCL90_PLUS);
        registry.insert(setter);
        let registry = std::sync::Arc::new(registry);
        let source = "interp alias {} write {} selected_write held
write 2";
        let analysis = test_logical_analysis(source, "tcl9.0", registry.clone());
        let walk = FrameWalk::new(source, &analysis).unwrap();
        let command = walk.segment(source, 0).pop().unwrap();
        let words = walk.source_words(source, &command).unwrap();
        assert!(words.roles().unwrap().contains(&(0, ArgRole::VarWrite)));
        assert_eq!(
            words.arguments()[0].literal_bytes(),
            Some(b"held".as_slice())
        );
        assert!(matches!(
            words.origins()[1],
            tcl_compiler::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
        ));
        let old = test_logical_analysis(source, "tcl8.6", registry);
        assert!(
            FrameWalk::new(source, &old)
                .unwrap()
                .source_words(source, &command)
                .is_none()
        );
        assert!(FrameWalk::new(&format!("#{source}"), &analysis).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting = !walk.config.strict_quoting;
        assert!(FrameWalk::new(source, &changed).is_none());
        changed.resolved_input = None;
        assert!(FrameWalk::new(source, &changed).is_none());
    }

    #[test]
    fn original_frame_regions_keep_selected_expression_template_and_replacement_horizon() {
        // naming.refactor.original-frame-traversal
        // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
        let source = "rename expr evaluate
evaluate {$x + [string length VALUE]}
subst -novariables {[puts $x]}
proc evaluate args {}
evaluate {[set hidden 1]}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let walk = FrameWalk::new(source, &analysis).unwrap();
        let commands = walk.segment(source, 0);
        let expression = walk.same_frame_regions(source, &commands[1]);
        assert_eq!(expression.len(), 1);
        assert_eq!(
            &source[expression[0].0..expression[0].1],
            "string length VALUE"
        );
        let template = walk.same_frame_regions(source, &commands[2]);
        assert_eq!(template.len(), 1);
        assert_eq!(&source[template[0].0..template[0].1], "puts $x");
        assert!(walk.complete());
        assert!(walk.source_words(source, &commands[4]).is_none());
        walk.same_frame_regions(source, &commands[4]);
        assert!(!walk.complete());
        let dynamic = "eval $script";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(dynamic, "tcl");
        let walk = FrameWalk::new(dynamic, &analysis).unwrap();
        assert!(
            walk.same_frame_regions(dynamic, &walk.segment(dynamic, 0)[0])
                .is_empty()
        );
        assert!(
            !walk.complete(),
            "a partial body inventory cannot prove coverage"
        );
    }
}
