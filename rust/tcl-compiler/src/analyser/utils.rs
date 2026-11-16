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

//! Pure analyser helpers.
//!
//! Pure free functions used by handlers and diagnostic emitters
//! across the analyser. `parse_param_list` is re-exported from
//! `signature_scan::params` so the analyser can keep its imports
//! flat.

use std::collections::{HashMap, HashSet};
use tcl_dialect::model::SurfaceQuery;

use tcl_lexer::{SourceMap, Span, Token, TokenType};
use tcl_registry::definer::DefinitionBodyGrammar;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::ir::Statement;

pub use crate::signature_scan::params::{
    param_name_spans, param_name_spans_for_token, parse_param_list,
};

/// Cap on how many leading lines the file-suppression scanner
/// inspects. Pathological all-comment files stop scanning past
/// this.
const FILE_DIRECTIVE_SCAN_LINES: usize = 100;

/// Sentinel line key for a file-wide suppression directive: a
/// `# tcl-lsp: disable=…` at the top of the file is recorded against
/// line `-1` in `suppressed_lines`, alongside the real 0-based lines
/// an inline `# noqa` covers.
pub const FILE_SUPPRESS_KEY: i32 = -1;

/// The one suppression contract every diagnostic surface obeys: is
/// `code` silenced at 0-based `line` by an inline `# noqa` or a
/// top-of-file `# tcl-lsp: disable=…` directive?
///
/// `suppressed` is the analyser's `suppressed_lines` map (see
/// [`parse_noqa_line_suppressions_for_dialect`] and
/// [`parse_file_suppression`], whose results
/// `Analyser::analyse` merges into it). A `"*"` entry silences every
/// code, and the file-level [`FILE_SUPPRESS_KEY`] bucket applies
/// document-wide.
///
/// The analyser records the map but never filters with it — the
/// consumer that renders diagnostics does, because only it knows
/// which line a finding lands on. Both consumers (the language
/// server's publish path and the `diag` / `lint` / `validate` CLI
/// verbs) call this, so a `# noqa` means the same thing in the
/// editor and on the command line; see
/// `docs/kcs/kcs-howto-suppress-diagnostics.md`.
///
/// Generic over both hashers so a caller holding an `FxHashMap` /
/// `FxHashSet` need not rebuild it as a `std` map to ask.
#[must_use]
pub fn line_suppressed<S, T>(
    code: &str,
    line: i32,
    suppressed: &HashMap<i32, HashSet<String, T>, S>,
) -> bool
where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher,
{
    let hit = |key: i32| {
        suppressed
            .get(&key)
            .is_some_and(|codes| codes.contains("*") || codes.contains(code))
    };
    hit(FILE_SUPPRESS_KEY) || hit(line)
}

/// The codes a comment's `# noqa` directive silences, or `None` when the
/// comment carries no directive.
///
/// Two spellings, the ones `docs/kcs/kcs-howto-suppress-diagnostics.md`
/// documents:
///
/// * `noqa:` at a word boundary, anywhere in the comment, silences the codes
///   listed after the colon — so an explanatory prefix (`# see #123, noqa:
///   W210`) still carries the directive. An empty list means every code.
/// * A comment line whose whole body is `noqa` silences every code (the `"*"`
///   sentinel).
///
/// Prose that merely mentions the word is not a directive: `# do not use noqa
/// here` would otherwise silence every finding on the command below it. A
/// suppression nobody asked for is invisible by construction — the finding it
/// hides is the only evidence it happened — so only the shapes above count.
///
/// `comment` is the comment text with or without its leading `#`, one physical
/// line or several.
#[must_use]
pub fn parse_noqa_marker(comment: &str) -> Option<HashSet<String>> {
    if let Some(codes) = coded_noqa_codes(comment) {
        return Some(codes);
    }
    comment
        .lines()
        .any(|line| {
            let body = line.trim().trim_start_matches('#').trim();
            body.eq_ignore_ascii_case("noqa")
        })
        .then(|| std::iter::once("*".to_owned()).collect())
}

/// The code list of the first `noqa:` marker in `comment`, or `None` when it
/// carries none.
///
/// A marker must start a word — `renoqa: W210` is a word, not a directive —
/// and the colon must follow the token immediately.
fn coded_noqa_codes(comment: &str) -> Option<HashSet<String>> {
    // `to_ascii_lowercase` maps only A-Z, so every byte offset it reports is
    // equally valid in `comment`, non-ASCII text included.
    let lower = comment.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(offset) = lower[from..].find("noqa") {
        let start = from + offset;
        let end = start + "noqa".len();
        let starts_a_word = !lower[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if starts_a_word && let Some(list) = comment[end..].strip_prefix(':') {
            let codes: HashSet<String> = list
                .split([',', ' ', '\t', '\r', '\n'])
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(str::to_owned)
                .collect();
            // `# noqa:` naming nothing is still a directive, and the only
            // reading left is the one the bare form has.
            return Some(if codes.is_empty() {
                std::iter::once("*".to_owned()).collect()
            } else {
                codes
            });
        }
        from = end;
    }
    None
}

/// Extract file-wide diagnostic suppression from top-of-file
/// directives.
///
/// Scans leading comment / blank lines for
/// `# tcl-lsp: disable=CODE1,CODE2` (or `=*` for "all codes");
/// stops at the first line that is neither blank nor a `#`
/// comment. Multiple directives accumulate into the returned set.
///
/// Codes are split on commas and any whitespace. Empty tokens
/// after splitting are discarded. The keyword `tcl-lsp:disable=`
/// matches case-insensitively, but the codes themselves are not
/// lowercased.
#[must_use]
pub fn parse_file_suppression(source: &str) -> HashSet<String> {
    let mut codes: HashSet<String> = HashSet::new();
    for (idx, line) in source.lines().enumerate() {
        if idx >= FILE_DIRECTIVE_SCAN_LINES {
            break;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let Some(rest) = parse_disable_directive(line) else {
            continue;
        };
        for token in rest.split([',', ' ', '\t', '\r', '\n']) {
            let token = token.trim();
            if !token.is_empty() {
                codes.insert(token.to_string());
            }
        }
    }
    codes
}

/// Extract §5.4 version-target declarations from top-of-file
/// ``# tcl-lsp: supports NAME RANGE…`` directives (centralisation ruling
/// R6 — the resolver-invisible targets declaration).
///
/// Same scan window as [`parse_file_suppression`]: leading comment /
/// blank lines only, capped at [`FILE_DIRECTIVE_SCAN_LINES`]. `NAME` is
/// the provider — `tcl` for the core, otherwise a package name (`Tk`,
/// `struct`) — and `RANGE` is one or more whitespace-separated clauses in
/// the targets grammar (`8.5-9.0`, `8.5-`, a bare `8.5` naming that
/// release line only). Returns `(provider, clauses)` pairs in file
/// order; a later directive for the same provider replaces the earlier
/// one, and clause validation happens at resolution, not here.
#[must_use]
pub fn parse_supports_directives(source: &str) -> Vec<(String, String)> {
    let mut declarations: Vec<(String, String)> = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        if idx >= FILE_DIRECTIVE_SCAN_LINES {
            break;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let Some((name, clauses)) = parse_supports_directive(line) else {
            continue;
        };
        declarations.retain(|(existing, _)| existing != name);
        declarations.push((name.to_owned(), clauses.to_owned()));
    }
    declarations
}

/// Match ``# tcl-lsp: supports NAME RANGE…`` (case-insensitive on the
/// keywords), returning the provider name and the raw clause text.
/// `None` when the line is not that directive — including a `supports`
/// with no range, which declares nothing.
fn parse_supports_directive(line: &str) -> Option<(&str, &str)> {
    let mut s = line.trim_start();
    s = s.strip_prefix('#')?.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if !s.get(..kw_end)?.eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    s = s[kw_end..].trim_start();
    s = s.strip_prefix(':')?.trim_start();
    let supports_kw = "supports";
    let sup_end = supports_kw.len();
    if !s.get(..sup_end)?.eq_ignore_ascii_case(supports_kw) {
        return None;
    }
    let rest = s.get(sup_end..)?;
    // The keyword must end at a word boundary (`supportsx` is not it).
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim();
    let (name, clauses) = rest.split_once([' ', '\t'])?;
    let clauses = clauses.trim();
    (!name.is_empty() && !clauses.is_empty()).then_some((name, clauses))
}

/// Scan *source* for ``# noqa`` comment lines and record next-line
/// suppressions.
///
/// For each ``# noqa`` / ``# noqa: CODE,…`` comment on line *N*
/// (0-based), records the suppression codes against line *N+1*.
/// This covers two cases the command-attached
/// `apply_preceding_noqa` mechanism cannot reach:
///
/// * A ``# noqa`` comment at the tail of a brace body (orphaned —
///   no following command in that scope), where the diagnostic
///   fires on the immediately following ``} elseif …`` or similar
///   line.
/// * A ``# noqa`` comment placed immediately before another comment
///   line that itself generates a diagnostic (e.g. W115 on a
///   ``\\``-continued comment).
///
/// The scan is additive — callers are expected to merge the
/// returned map into ``result.suppressed_lines`` alongside the
/// existing command-level (`apply_preceding_noqa`) and file-level
/// (`parse_file_suppression`) passes.
///
/// [`parse_noqa_marker`] decides what each comment carries: bare
/// ``# noqa`` is recorded with the `"*"` sentinel the suppression
/// filter treats as "every code", a ``noqa:`` marker with the codes
/// it names, and prose that merely mentions the word with nothing.
#[cfg(test)]
#[must_use]
pub fn parse_noqa_line_suppressions(
    source: &str,
) -> std::collections::HashMap<i32, HashSet<String>> {
    parse_noqa_line_suppressions_for_dialect(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
}

/// Dialect-aware [`parse_noqa_line_suppressions`].
#[must_use]
pub fn parse_noqa_line_suppressions_for_dialect(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> std::collections::HashMap<i32, HashSet<String>> {
    let generation = crate::environment_ingress::context_for_profile(dialect);
    parse_noqa_line_suppressions_with_registry(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        generation.commands(),
    )
}

fn parse_noqa_line_suppressions_with_registry(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> std::collections::HashMap<i32, HashSet<String>> {
    noqa_lines_from_comments(script_comment_facts(source, config, registry))
}

/// Next-line suppression facts from the same complete retained analysis.
/// Missing source/config/context correspondence remains unavailable; source
/// directives cannot reconstruct a nominal Registry or entered body.
#[must_use]
pub fn parse_noqa_line_suppressions_from_analysis(
    source: &str,
    analysis: &super::AnalysisResult,
) -> Option<std::collections::HashMap<i32, HashSet<String>>> {
    let comments = script_comment_facts_from_analysis(source, analysis)?;
    Some(noqa_lines_from_comments(comments))
}

fn noqa_lines_from_comments(
    comments: impl IntoIterator<Item = ScriptCommentFact>,
) -> std::collections::HashMap<i32, HashSet<String>> {
    let mut result: std::collections::HashMap<i32, HashSet<String>> =
        std::collections::HashMap::new();
    for fact in comments {
        let Some(codes) = parse_noqa_marker(&fact.text) else {
            continue;
        };
        let next_line = i32::try_from(fact.line)
            .unwrap_or(i32::MAX)
            .saturating_add(1);
        result.entry(next_line).or_default().extend(codes);
    }
    result
}

/// Physical source comments under the actual complete analysis and Registry.
/// Authored Body, member/case/lambda geometry stays source syntax only; this
/// does not enter a frame, choose runtime dispatch or grant an edit.
#[must_use]
pub fn script_comment_facts_from_analysis(
    source: &str,
    analysis: &super::AnalysisResult,
) -> Option<Vec<ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    CommentLineWalker::from_analysis(source, analysis)?.collect_original()
}

/// Lexer-confirmed comments at every original command position, including
/// inline `;#` comments, under the actual complete analysis and Registry.
/// Source-body selection remains conditional syntax; this supplies no entered
/// body, execution, complete command census or source-edit authority.
#[must_use]
pub fn script_command_comment_facts_from_analysis(
    source: &str,
    analysis: &super::AnalysisResult,
) -> Option<Vec<ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let mut walker = CommentLineWalker::from_analysis(source, analysis)?;
    walker.comment_position = ScriptCommentPosition::CommandPosition;
    walker.collect_original()
}

/// One lexer-confirmed command-position comment in a registry-reachable script.
///
/// `span` is an absolute byte range in the whole document and includes the
/// leading `#`. Physical-line queries retain only markers preceded by horizontal
/// whitespace on that line; the command-position query also retains inline
/// comments. The exact slice carries the selected lexical purpose and never
/// turns a quoted or braced data argument into script.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptCommentFact {
    /// Absolute lexer-confirmed comment-token span, including `#`.
    pub span: Span,
    /// Zero-based physical line on which the `#` begins.
    pub line: usize,
    /// Exact lexer-confirmed comment text, including `#` and any continued
    /// physical lines. Consumers must inspect this instead of rescanning the
    /// source line that happens to contain the comment.
    pub text: String,
}

/// Lexer-confirmed physical comments in the document's top-level script or a
/// registry-declared script body. Braced data arguments are never recursively
/// tokenised as scripts. Callers must pass the document's resolved lexer
/// configuration and registry; there is deliberately no implicit Tcl profile.
#[must_use]
pub fn script_comment_facts(
    source: &str,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<ScriptCommentFact> {
    // Every retained physical comment includes its original marker byte.
    // An absent marker closes this inventory without selecting body roles.
    if !source.as_bytes().contains(&b'#') {
        return Vec::new();
    }
    CommentLineWalker::new(source, config, registry).collect()
}

/// Physical comment tokens in explicitly supplied original script regions.
///
/// The caller owns each region's source-syntax applicability. This lexical
/// projection does not select command roles, reconstruct an analysis, prove
/// execution or authorise an edit. Regions and comment slices remain in the
/// supplied image's original byte coordinates; cooked values have no mapping.
/// Unknown text, geometry or lexing returns `None`, never an empty inventory.
#[must_use]
pub fn original_script_comment_facts(
    image: &tcl_lexer::SourceImage,
    scripts: &[(Span, u32)],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<ScriptCommentFact>> {
    original_script_comment_facts_for_position(
        image,
        scripts,
        config,
        ScriptCommentPosition::PhysicalLine,
    )
}

/// Command-position comment tokens, inline `;#` comments included, in
/// explicitly supplied original script regions. Otherwise exactly
/// [`original_script_comment_facts`]: the caller owns each region's
/// applicability and nothing is interpreted.
#[must_use]
pub fn original_script_command_comment_facts(
    image: &tcl_lexer::SourceImage,
    scripts: &[(Span, u32)],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<ScriptCommentFact>> {
    original_script_comment_facts_for_position(
        image,
        scripts,
        config,
        ScriptCommentPosition::CommandPosition,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScriptCommentPosition {
    PhysicalLine,
    CommandPosition,
}

fn original_script_comment_facts_for_position(
    image: &tcl_lexer::SourceImage,
    scripts: &[(Span, u32)],
    config: tcl_lexer::LexerConfig,
    position: ScriptCommentPosition,
) -> Option<Vec<ScriptCommentFact>> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = image.try_text().ok()?;
    let source_map = image.source_map();
    let mut facts = Vec::new();
    for &(region, depth) in scripts {
        let script = source.get(region.as_range())?;
        let tokens = tcl_lexer::Lexer::with_config(script, config.normalized().at_depth(depth))
            .tokenise_all()
            .ok()?;
        for token in tokens
            .into_iter()
            .filter(|token| token.kind == TokenType::Comment)
        {
            let span = Span::new(
                region.start().checked_add(token.span.start())?,
                region.start().checked_add(token.span.end())?,
            );
            let start = usize::try_from(span.start()).ok()?;
            let before = source.get(..start)?;
            let line_start = before.rfind('\n').map_or(0, |offset| offset + 1);
            if position == ScriptCommentPosition::PhysicalLine
                && !source
                    .get(line_start..start)?
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c))
            {
                continue;
            }
            facts.push(ScriptCommentFact {
                span,
                line: usize::try_from(source_map.line_index().line_at(span.start())).ok()?,
                text: source.get(span.as_range())?.to_owned(),
            });
        }
    }
    facts.sort_unstable_by_key(|fact| (fact.span.start(), fact.span.end()));
    facts.dedup_by_key(|fact| fact.span);
    Some(facts)
}

/// Lexer-fact walk over every registry-declared script shape.  This is shared
/// by `# noqa`, W115, and code actions so neither feature can accidentally
/// invent a separate idea of where executable comment lines may occur.
struct CommentLineWalker<'a> {
    whole: &'a str,
    config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    identities: std::borrow::Cow<'a, crate::realm::CommandBindingRealm>,
    source_context: Option<
        std::sync::Arc<crate::registry_invocation::source_structure::OriginalSourceRegistryContext>,
    >,
    analysis: Option<&'a super::AnalysisResult>,
    availability: Option<SurfaceQuery<'a>>,
    visited: HashSet<(u32, u32)>,
    facts: Vec<ScriptCommentFact>,
    comment_position: ScriptCommentPosition,
}

impl<'a> CommentLineWalker<'a> {
    fn new(whole: &'a str, config: tcl_lexer::LexerConfig, registry: &'a CommandRegistry) -> Self {
        let source_context = registry.profile().filter(|profile| {
            tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy().is_some()
        }).map(|profile| {
            let generation = crate::environment_ingress::context_for_profile(profile);
            let context = std::sync::Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
            let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
            std::sync::Arc::new(crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(whole, input))
        });
        let identities = source_context.as_ref().map_or_else(
            || crate::realm::document_realm_bindings_with_config(whole, config, registry),
            |source| source.retained_source_realm().clone(),
        );
        Self {
            whole,
            config,
            registry,
            identities: std::borrow::Cow::Owned(identities),
            source_context,
            analysis: None,
            availability: registry
                .profile()
                .map(tcl_dialect::DialectProfile::surface_query),
            visited: HashSet::new(),
            facts: Vec::new(),
            comment_position: ScriptCommentPosition::PhysicalLine,
        }
    }

    fn from_analysis(whole: &'a str, analysis: &'a super::AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let input = analysis.resolved_input.as_ref()?;
        let image = tcl_lexer::SourceImage::document(whole);
        analysis
            .matches_original_source_image(&image, config)
            .then_some(())?;
        Some(Self {
            whole,
            config,
            registry: analysis.resolved_registry()?,
            identities: std::borrow::Cow::Borrowed(analysis.retained_command_realm()?),
            source_context: None,
            analysis: Some(analysis),
            availability: Some(input.availability_context().authoring_query()),
            visited: HashSet::new(),
            facts: Vec::new(),
            comment_position: ScriptCommentPosition::PhysicalLine,
        })
    }

    fn collect_original(mut self) -> Option<Vec<ScriptCommentFact>> {
        if !self.whole.as_bytes().contains(&b'#') {
            return Some(Vec::new());
        }
        let end = u32::try_from(self.whole.len()).ok()?;
        self.visit_original(Span::new(0, end), 0, None)?;
        self.facts
            .sort_unstable_by_key(|fact| (fact.span.start(), fact.span.end()));
        self.facts.dedup_by_key(|fact| fact.span);
        Some(self.facts)
    }

    fn visit_original(
        &mut self,
        region: Span,
        depth: u32,
        definition_parent: Option<&crate::registry_invocation::OriginalSourceScriptBody>,
    ) -> Option<()> {
        if depth > 256 || !self.visited.insert((region.start(), region.end())) {
            return Some(());
        }
        let analysis = self.analysis?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let image = tcl_lexer::SourceImage::document(self.whole);
        self.facts
            .extend(original_script_comment_facts_for_position(
                &image,
                &[(region, depth)],
                self.config,
                self.comment_position,
            )?);
        let script = self.whole.get(region.as_range())?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            script,
            region.start(),
            self.config.at_depth(depth),
        );
        let plan = tcl_lexer::native_script_words_in(image, region, self.config).ok()?;
        let definition_members =
            definition_parent.and_then(|parent| parent.definition_member_region(&context, region));
        for native in &plan.commands {
            for word in &native.words {
                for part in word.executable_parts().all_parts() {
                    if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                        self.visit_original(body, depth + 1, definition_parent)?;
                    }
                }
            }
            if let Some(members) = &definition_members
                && let Some(bodies) = members.script_bodies(&native.words)
            {
                for body in bodies {
                    self.visit_original(body.content_span(), depth + 1, body.definition_parent())?;
                }
                continue;
            }
            let Some(command) = commands.iter().find(|command| {
                command.argv.first().map(|token| token.span.start())
                    == native.words.first().map(|word| word.span().start())
            }) else {
                continue;
            };
            if let Some(declared) =
                crate::registry_invocation::source_structure::source_declared_command_words(
                    self.whole, analysis, command,
                )
            {
                for body in declared.source_script_bodies_for(
                    crate::registry_invocation::OriginalSourceScriptPurpose::Syntax,
                ) {
                    self.visit_original(body.content_span(), depth + 1, None)?;
                }
                continue;
            }
            let Some(words) = crate::registry_invocation::source_structure::source_registry_words(
                self.whole, analysis, command,
            ) else {
                continue;
            };
            for body in words.source_script_bodies(&context) {
                let parent = body.definition_parent_for(&context, definition_parent);
                self.visit_original(body.content_span(), depth + 1, parent.as_ref())?;
            }
            if let Some(bodies) =
                words.source_expression_script_bodies(analysis.resolved_input.as_ref()?)
            {
                for body in bodies {
                    self.visit_original(body.content_span(), depth + 1, definition_parent)?;
                }
            }
            for &(ordinal, role) in words.roles().unwrap_or_default() {
                if role != ArgRole::LambdaLiteral {
                    continue;
                }
                let Some(word) = words
                    .operands()
                    .get(ordinal)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                else {
                    continue;
                };
                if let Some(body) = crate::lambda_literal::split_original_lambda_literal(word)
                    .and_then(|lambda| lambda.braced_body())
                {
                    self.visit_original(body, depth + 1, None)?;
                }
            }
        }
        Some(())
    }

    fn collect(mut self) -> Vec<ScriptCommentFact> {
        self.visit(self.whole, 0, 0, None);
        self.facts.sort_unstable_by_key(|fact| fact.span.start());
        self.facts.dedup_by_key(|fact| fact.span);
        self.facts
    }

    fn visit(
        &mut self,
        script: &'a str,
        base_offset: u32,
        depth: u32,
        definition_grammar: Option<&'static DefinitionBodyGrammar>,
    ) {
        if depth > 64
            || !self
                .visited
                .insert((base_offset, u32::try_from(script.len()).unwrap_or(u32::MAX)))
        {
            return;
        }
        self.record_comments(script, base_offset, depth);
        for command in crate::segmenter::segment_commands_with_offset_and_config(
            script,
            0,
            self.config.at_depth(depth),
        ) {
            if self.source_context.is_some() && definition_grammar.is_none() {
                self.visit_source_schema(&command, base_offset, depth);
                self.visit_command_substitutions(script, &command, base_offset, depth);
                continue;
            }
            let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
            // Member keywords are syntax within an already selected definition
            // grammar. They do not establish a Tcl command implementation.
            let name = command.name();
            let literal_head = command.single_token_word.first() == Some(&true)
                && command
                    .argv
                    .first()
                    .is_some_and(|token| matches!(token.kind, TokenType::Str | TokenType::Esc));
            let surface = tcl_registry::model::DocumentCommandSurface::new(self.registry, None);
            let view = crate::registry_invocation::segmented_body_assistance(
                &surface,
                self.identities.source_bindings_ref(),
                script,
                self.config.at_depth(depth),
                &command,
                base_offset,
                self.identities.source_metadata_context().as_deref(),
            );
            let roles = view
                .as_ref()
                .map_or(&[][..], |view| view.possible_roles.as_slice());
            let member = definition_grammar
                .filter(|_| literal_head)
                .and_then(|grammar| grammar.member(name));
            let body_indices = member.map_or_else(
                || {
                    roles
                        .iter()
                        .filter_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
                        .collect()
                },
                |_| {
                    definition_grammar
                        .expect("member has grammar")
                        .member_body_indices_in(name, &args, self.availability)
                },
            );
            let next_grammar = next_definition_grammar(
                view.as_ref()
                    .map_or(&[], |view| view.possible_definition_grammars.as_slice()),
                &args,
                definition_grammar,
                member,
            );
            let case_lists = view
                .as_ref()
                .map_or(&[][..], |view| view.possible_case_lists.as_slice());
            for &(spec, index) in case_lists {
                if let Some(&token) = command.arg_tokens().get(index)
                    && token.kind == tcl_lexer::TokenType::Str
                {
                    self.visit_case_list(script, token, &spec, base_offset, depth);
                }
            }

            for index in body_indices {
                let Some(&token) = command.arg_tokens().get(index) else {
                    continue;
                };
                if token.kind != tcl_lexer::TokenType::Str {
                    continue;
                }
                if case_lists
                    .iter()
                    .any(|&(_, case_index)| case_index == index)
                {
                    continue;
                }
                self.visit_braced_word(script, token, base_offset, depth, next_grammar);
            }
            self.visit_lambda_literals(script, &command, roles, base_offset, depth);
            self.visit_command_substitutions(script, &command, base_offset, depth);
        }
    }

    /// Readonly source roles select physical script geometry. Neither unknown
    /// argument values nor an unavailable execution entry turn a body into data.
    fn visit_source_schema(
        &mut self,
        command: &crate::segmenter::SegmentedCommand,
        base_offset: u32,
        depth: u32,
    ) {
        let Some(source) = &self.source_context else {
            return;
        };
        let command = command.clone().shifted_by(base_offset);
        let Some(words) = source
            .words(&command)
            .or_else(|| source.conditional_native_case_words(&command))
        else {
            return;
        };
        let context = source.editing_input().context_registry();
        let conditional_native = matches!(
            words.source(),
            crate::registry_invocation::source_structure::OriginalRegistrySource::ConditionalNativeSource(
                _
            )
        );
        let grammar = (!conditional_native)
            .then(|| {
                words
                    .with_source_schema(&context, |schema| {
                        context
                            .context()
                            .resolve_spec(context.commands(), schema.canonical_command)
                            .and_then(|spec| spec.definition_body)
                    })
                    .flatten()
            })
            .flatten();
        let mut children = if conditional_native {
            Vec::new()
        } else {
            words
                .source_script_bodies(&context)
                .into_iter()
                .map(|body| body.content_span())
                .collect::<Vec<_>>()
        };
        // Case candidates are syntax-only even when the subject/option
        // selection is unknown; retain that residual while visiting their
        // original spans, without treating any candidate as entered.
        if let Some(candidates) = words.conditional_source_case_bodies(&context)
            && candidates.matches_source(source.source_image(), self.config)
        {
            let _selection_unknown = candidates.selection_unknown();
            children.extend(
                candidates
                    .bodies()
                    .iter()
                    .map(super::super::registry_invocation::OriginalSourceScriptBody::content_span),
            );
        }
        for &(ordinal, role) in words.roles().unwrap_or_default() {
            if role != ArgRole::LambdaLiteral {
                continue;
            }
            let Some(word) = words
                .operands()
                .get(ordinal)
                .and_then(Option::as_ref)
                .and_then(|operand| operand.word())
            else {
                continue;
            };
            if let Some(body) = crate::lambda_literal::split_original_lambda_literal(word)
                .and_then(|lambda| lambda.braced_body())
            {
                children.push(body);
            }
        }
        children.sort_unstable_by_key(|span| (span.start(), span.end()));
        children.dedup();
        for child in children {
            if let Some(script) = self.whole.get(child.as_range()) {
                self.visit(script, child.start(), depth.saturating_add(1), grammar);
            }
        }
    }

    fn record_comments(&mut self, script: &str, base_offset: u32, depth: u32) {
        let Some(end) = u32::try_from(script.len())
            .ok()
            .and_then(|length| base_offset.checked_add(length))
        else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(self.whole);
        if let Some(facts) = original_script_comment_facts(
            &image,
            &[(Span::new(base_offset, end), depth)],
            self.config,
        ) {
            self.facts.extend(facts);
        }
    }

    fn visit_braced_word(
        &mut self,
        script: &'a str,
        token: tcl_lexer::Token,
        base_offset: u32,
        depth: u32,
        grammar: Option<&'static DefinitionBodyGrammar>,
    ) {
        let whole_word = tcl_lexer::word_span_at(script, token.span);
        let start = usize::try_from(whole_word.start()).unwrap_or(script.len());
        let end = usize::try_from(whole_word.end()).unwrap_or(script.len());
        if end <= start + 1 || script.as_bytes().get(start) != Some(&b'{') {
            return;
        }
        let Some(body) = script.get(start + 1..end - 1) else {
            return;
        };
        self.visit(
            body,
            base_offset.saturating_add(u32::try_from(start + 1).unwrap_or(u32::MAX)),
            depth.saturating_add(1),
            grammar,
        );
    }

    fn visit_case_list(
        &mut self,
        script: &'a str,
        token: tcl_lexer::Token,
        spec: &tcl_registry::CaseListSpec,
        base_offset: u32,
        depth: u32,
    ) {
        let content_start = token.span.start() as usize + token.content_offset as usize;
        let content_end = token.span.end() as usize;
        let Some(list) = script.get(content_start..content_end) else {
            return;
        };
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: spec.clause_flags,
            clause_value_flags: spec.clause_value_flags,
        };
        for clause in tcl_syntax::case_list::split_case_list(list, &shape) {
            let Some(body) = clause.body.filter(|body| body.braced) else {
                continue;
            };
            let range = body.content_range();
            let start = content_start + range.start;
            let end = content_start + range.end;
            let Some(arm) = script.get(start..end) else {
                continue;
            };
            self.visit(
                arm,
                base_offset.saturating_add(u32::try_from(start).unwrap_or(u32::MAX)),
                depth.saturating_add(1),
                None,
            );
        }
    }

    fn visit_lambda_literals(
        &mut self,
        script: &'a str,
        command: &crate::segmenter::SegmentedCommand,
        roles: &[(usize, ArgRole)],
        base_offset: u32,
        depth: u32,
    ) {
        if let Some(source) = self.source_context.clone() {
            let command = command.clone().shifted_by(base_offset);
            let Some(words) = source.words(&command) else {
                return;
            };
            if !words.matches_source(source.source_image(), self.config) {
                return;
            }
            for &(ordinal, role) in words.roles().unwrap_or_default() {
                if role != ArgRole::LambdaLiteral {
                    continue;
                }
                let Some(word) = words
                    .operands()
                    .get(ordinal)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                else {
                    continue;
                };
                if word.image() != source.source_image() || word.config() != self.config {
                    continue;
                }
                let Some(body) = crate::lambda_literal::split_original_lambda_literal(word)
                    .and_then(|lambda| lambda.braced_body())
                else {
                    continue;
                };
                if let Some(lambda) = self.whole.get(body.as_range()) {
                    self.visit(lambda, body.start(), depth.saturating_add(1), None);
                }
            }
            return;
        }
        for index in roles
            .iter()
            .filter_map(|&(index, role)| (role == ArgRole::LambdaLiteral).then_some(index))
        {
            let Some(&token) = command.arg_tokens().get(index) else {
                continue;
            };
            let Some(body) = crate::lambda_literal::split_lambda_literal(script, token)
                .and_then(|literal| literal.braced_body())
            else {
                continue;
            };
            let start = body.start() as usize;
            let end = body.end() as usize;
            let Some(lambda) = script.get(start..end) else {
                continue;
            };
            self.visit(
                lambda,
                base_offset.saturating_add(body.start()),
                depth.saturating_add(1),
                None,
            );
        }
    }

    /// Descend command-substitution tokens exactly as the CST segmented them.
    /// A braced word has no command-substitution token, so inert data remains
    /// opaque; bare and quoted words retain every executable bracketed span
    /// without a second source scan.
    fn visit_command_substitutions(
        &mut self,
        script: &'a str,
        command: &crate::segmenter::SegmentedCommand,
        base_offset: u32,
        depth: u32,
    ) {
        for &token in &command.all_tokens {
            if token.kind != tcl_lexer::TokenType::Cmd {
                continue;
            }
            let start = token.span.start() as usize + token.content_offset as usize;
            let end = token.span.end() as usize;
            let Some(inner) = script.get(start..end) else {
                continue;
            };
            self.visit(
                inner,
                base_offset.saturating_add(
                    token
                        .span
                        .start()
                        .saturating_add(u32::from(token.content_offset)),
                ),
                depth.saturating_add(1),
                None,
            );
        }
    }
}

fn next_definition_grammar(
    possible: &[&'static DefinitionBodyGrammar],
    args: &[&str],
    current: Option<&'static DefinitionBodyGrammar>,
    member: Option<&'static tcl_registry::definer::MemberSpec>,
) -> Option<&'static DefinitionBodyGrammar> {
    if let [grammar] = possible {
        return Some(*grammar);
    }
    if !possible.is_empty() {
        return None;
    }
    let Some(member) = member else {
        return current;
    };
    if member.wrapper_block_body
        && args
            .first()
            .is_some_and(|inner| current.is_some_and(|grammar| grammar.member(inner).is_none()))
    {
        current
    } else {
        None
    }
}

/// Scan `source` for inline ``# tcl-lsp: stub <name> ...``
/// declarations bounded by ``# tcl-lsp: stubs-begin`` /
/// ``# tcl-lsp: stubs-end`` markers, returning the set of
/// declared command names.
///
/// Only the command names are extracted (not arg-roles or
/// flags). Adding the names to the W123 candidate set is the
/// load-bearing use case — the role/flag data only matters for
/// downstream diagnostic emitters.
///
/// Stubs declared *outside* the begin/end markers are ignored.
/// ``expr-func`` / ``expr-op`` lines are skipped — they declare
/// expression functions, not commands.
#[must_use]
pub fn scan_stub_command_names(source: &str) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    let mut in_block = false;
    for line in source.lines() {
        let stripped = line.trim();
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        // Markers — both case-insensitive on the keyword.
        if matches_marker(body, "stubs-begin") {
            in_block = true;
            continue;
        }
        if matches_marker(body, "stubs-end") {
            in_block = false;
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(name) = parse_stub_name(body) {
            // Skip ``expr-func`` / ``expr-op`` — those
            // declare expression-language symbols, not
            // commands.
            if name.starts_with("expr-") {
                continue;
            }
            names.insert(name.to_string());
        }
    }
    names
}

/// Build the "known command" name universe consulted by unclosed-delimiter
/// recovery: the segmenter's scan-to-next heuristic
/// ([`crate::segmenter::segment_commands_with_recovery_and_config`]) and the
/// E100/E201/E202/E203 recovery diagnostics (`syntax_checks`) both use "does
/// the next line start with a known command?" as their strongest signal that
/// a missing delimiter belongs right before it.
///
/// Restricting "known" to the active dialect's [`CommandRegistry`]
/// under-recognises real code: almost every non-trivial Tcl file calls its
/// own `proc`s, so a break just before a call to one of them falls back to
/// the imprecise generic diagnostic (or, worse, the recovery point is never
/// found at all and the rest of the document goes unanalysed). This unions
/// the registry names with every proc / class / command-alias /
/// rename-target name the document itself defines — qualified and
/// unqualified tail — using the same fast, non-lowering scan
/// [`crate::signature_scan::extract_signatures`] already uses for
/// background-indexed files, so recovery recognises a document's own procs
/// as readily as a builtin.
///
/// `extra` is unioned in unconditionally alongside the registry — the same
/// user-declared / LSP-layer-resolved name set [`super::state::Analyser::extra_commands`]
/// already carries for the W123 unresolved-command check (`tclLsp.extraCommands`,
/// widened one layer up in `tcl-lsp-server` with workspace- and package-resolved
/// names on the recovery path — see `widen_recovery_extra_commands` there). It is
/// a plain in-memory set by the time it reaches here, so folding it in costs
/// nothing extra beyond the registry union already does.
///
/// The in-file scan runs only when configured Document completeness proves
/// the source incomplete. Both queries share the supplied whole-file grammar;
/// an unavailable parser retains only registry and caller-supplied candidates.
#[must_use]
pub fn recovery_known_commands(
    source: &str,
    registry: &CommandRegistry,
    extra: &SharedNameSet,
    config: tcl_lexer::LexerConfig,
) -> RecoveryKnownCommands {
    let mut names: HashSet<String> = registry.command_names().map(str::to_owned).collect();
    if tcl_lexer::script_image_is_complete(&tcl_lexer::SourceImage::document(source), config)
        == Ok(false)
    {
        let sig = crate::signature_scan::extract_signatures_with_config(source, config, registry);
        for declaration in &sig.procedure_declarations {
            if let Some(spelling) = declaration.source_spelling() {
                insert_qualified_and_tail(&mut names, &spelling);
                names.insert(declaration.name.clone());
            }
        }
        for declaration in &sig.class_declarations {
            if let Some(spelling) = declaration.source_spelling() {
                insert_qualified_and_tail(&mut names, &spelling);
                names.insert(declaration.name.clone());
            }
        }
        for spelling in sig
            .command_aliases
            .values()
            .filter_map(super::super::signature_scan::types::SignatureCommandAlias::source_spelling)
            .chain(
                sig.renames.values().filter_map(
                    super::super::signature_scan::types::SignatureRename::source_spelling,
                ),
            )
        {
            insert_qualified_and_tail(&mut names, &spelling);
        }
    }
    RecoveryKnownCommands {
        local: names,
        extra: std::sync::Arc::clone(extra),
    }
}

/// The recovery known-command universe [`recovery_known_commands`] builds,
/// held as **two layers** rather than one merged set.
///
/// `local` is derived from the document being analysed (the active registry's
/// names plus the document's own proc / class / alias / rename names), so it is
/// rebuilt on every edit. `extra` is the caller-supplied name set
/// ([`super::state::Analyser::extra_commands`]), which on the LSP's
/// unclosed-delimiter recovery path is the *widened* set — every
/// workspace-indexed proc and class, plus every auto-loadable command name:
/// tens of thousands of entries on a large workspace, and unchanged between
/// keystrokes. Merging the two would deep-copy that set on every analysis of a
/// document with an open delimiter — which is the *normal* mid-typing state, so
/// it would happen on every debounced run. Sharing it behind an `Arc` makes the
/// layering free.
///
/// Every consumer only ever asks "is this name known?" or enumerates the
/// universe once, both of which the two layers answer directly.
#[derive(Debug, Default, Clone)]
pub struct RecoveryKnownCommands {
    local: HashSet<String>,
    extra: SharedNameSet,
}

/// A command-name set shared between the analyser and its caller — see
/// [`RecoveryKnownCommands`] and [`super::state::Analyser::extra_commands`].
pub type SharedNameSet = std::sync::Arc<HashSet<String>>;

impl RecoveryKnownCommands {
    /// Whether `name` is a known command in either layer.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.local.contains(name) || self.extra.contains(name)
    }

    /// Every known name, `local` first. A name present in both layers is
    /// yielded twice; every consumer collects into a set, so that is harmless
    /// and avoids a de-duplication pass over the (large) shared layer.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.local
            .iter()
            .chain(self.extra.iter())
            .map(String::as_str)
    }

    /// Drop the per-document layer and release the shared one — the
    /// between-runs reset ([`super::state::Analyser::clear_run_state`]).
    pub fn clear(&mut self) {
        self.local.clear();
        self.extra = std::sync::Arc::default();
    }
}

impl FromIterator<String> for RecoveryKnownCommands {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self {
            local: iter.into_iter().collect(),
            extra: std::sync::Arc::default(),
        }
    }
}

/// True when an earlier *unconditional* user `proc` named `qualified_name`
/// lexically precedes `call_start` — Tcl resolves a proc over a same-named
/// builtin/keyword at that call site, so the diagnostic that would fire
/// against the builtin/keyword must be suppressed. Shared by W002's inline
/// disabled-command shadow check and the EXPR-role emitters'
/// (W100/W110/W003/W114) control-flow-keyword shadow check — both are the
/// exact same position comparison against [`ProcDef::name_span`], just
/// triggered from different call sites. W002's *per-item* flush path needs
/// the fuller `UserResolutionFacts::resolves_to_user` instead (it also
/// covers aliases/renames/classes/ensembles and namespace-qualified procs),
/// so this simpler check isn't a fit there.
#[must_use]
pub fn proc_shadows_call<S: std::hash::BuildHasher>(
    all_procs: &HashMap<String, super::types::ProcDef, S>,
    qualified_name: &str,
    call_start: u32,
) -> bool {
    all_procs
        .get(qualified_name)
        .is_some_and(|def| def.name_span.start() < call_start)
}

/// Insert `qname` (e.g. `::ns::foo`) — as-is, with the leading `::`
/// stripped, and as its unqualified tail (`foo`) — into `names`. Mirrors the
/// tail-matching the W123 unresolved-command pass already uses for the same
/// proc/class/alias collections (`build_w123_known_names`), plus the
/// original fully-qualified form: `signature_scan::qualify` always prepends
/// `::`, and a recovery-point call can be written either way
/// (`::ns::foo` or `ns::foo`), so both must be recognised.
///
/// `pub` (not just `pub(super)`) because `tcl-lsp-server` reuses this exact
/// three-form insertion when widening the recovery known-command set with
/// workspace-indexed and package-resolved names one layer up — those names
/// carry the same `::`-qualified shape and need the same recognition rules,
/// so this is the single place that logic lives rather than a second,
/// independently-maintained copy.
pub fn insert_qualified_and_tail<S: std::hash::BuildHasher>(
    names: &mut HashSet<String, S>,
    qname: &str,
) {
    if !qname.is_empty() {
        names.insert(qname.to_string());
    }
    let absolute = crate::naming::unroot_rooted_key(qname).unwrap_or(qname);
    if !absolute.is_empty() {
        names.insert(absolute.to_string());
    }
    let tail = crate::naming::key_tail(qname);
    if !tail.is_empty() {
        names.insert(tail.to_string());
    }
}

/// Match a ``# tcl-lsp: <marker>`` line, where `marker` is
/// e.g. ``"stubs-begin"`` / ``"stubs-end"``.  Case-insensitive
/// on the keyword run; whitespace flexible.
fn matches_marker(body: &str, marker: &str) -> bool {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    // `str::get` returns `None` when `kw_end` is past the end *or* falls inside
    // a multi-byte char, so it subsumes the length check and never panics on
    // non-ASCII source (the marker run itself is pure ASCII).
    let Some(prefix) = s.get(..kw_end) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case(lower_prefix) {
        return false;
    }
    let rest = s[kw_end..].trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    let rest = rest.trim_start();
    let m_end = marker.len();
    let Some(head) = rest.get(..m_end) else {
        return false;
    };
    head.eq_ignore_ascii_case(marker)
}

/// Scan `source` for ``# tcl-lsp: package NAME provides PKG …`` directives,
/// returning each declared edge as `(loading package, packages it also
/// loads)` in source order.
///
/// The directive states what a package's loader pulls in behind the
/// analyser's back — a binary extension whose C `Init` calls
/// `Tcl_PkgRequire` or `Tk_InitStubs`. It declares the same
/// edge `[packages.provides]` configures, for one file, and it names the
/// loading package rather than relying on where it sits: a document gains
/// the packages only once it actually requires the one named, and the
/// directive may live anywhere, including the top-of-file comment block.
///
/// `package` is the keyword, as in every other member of the family
/// (`supports`, `stub`, `disable`), and it deliberately echoes the Tcl
/// commands the line is about.
///
/// The scan covers the whole file on the same terms as
/// [`scan_source_for_stubs`] — any line whose first non-whitespace character
/// is `#`. One directive may name several loaded packages, and several
/// directives may name the same loader; the edges merge, keeping
/// first-declaration order.
#[must_use]
pub fn parse_provides_directives(source: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in source.lines() {
        let stripped = line.trim();
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        if !matches_marker(body, "package") {
            continue;
        }
        let Some((from, loaded)) = parse_provides_directive(body) else {
            continue;
        };
        let position = out.iter().position(|(existing, _)| existing == from);
        let index = position.unwrap_or_else(|| {
            out.push((from.to_owned(), Vec::new()));
            out.len() - 1
        });
        for name in loaded.split_whitespace() {
            if !out[index].1.iter().any(|existing| existing == name) {
                out[index].1.push(name.to_owned());
            }
        }
    }
    out.retain(|(_, loaded)| !loaded.is_empty());
    out
}

/// Match ``# tcl-lsp: package NAME provides PKG …`` on a comment body whose
/// `package` marker [`matches_marker`] already confirmed, returning the
/// loading package and the raw list of packages it loads.
///
/// `None` when the line is not that shape — a `package` with no name, no
/// `provides`, or nothing after it declares no edge.
fn parse_provides_directive(body: &str) -> Option<(&str, &str)> {
    let s = body.trim_start();
    let rest = s
        .get("tcl-lsp".len()..)?
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let after_kw = rest.get("package".len()..)?;
    // The keyword must end at a word boundary (`packages` is not it).
    if !after_kw.starts_with([' ', '\t']) {
        return None;
    }
    let (from, rest) = after_kw.trim_start().split_once([' ', '\t'])?;
    let rest = rest.trim_start();
    let verb = "provides";
    if !rest.get(..verb.len())?.eq_ignore_ascii_case(verb) {
        return None;
    }
    let loaded = rest.get(verb.len()..)?;
    if !loaded.starts_with([' ', '\t']) {
        return None;
    }
    let loaded = loaded.trim();
    (!from.is_empty() && !loaded.is_empty()).then_some((from, loaded))
}

/// Extract the command name from a ``# tcl-lsp: stub NAME …``
/// line.  Returns the raw name as a borrowed slice or `None`
/// when the line doesn't match the stub shape.
fn parse_stub_name(body: &str) -> Option<&str> {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if s.len() < kw_end || !s[..kw_end].eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    let rest = s[kw_end..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let stub_kw = "stub";
    let stub_end = stub_kw.len();
    if rest.len() < stub_end || !rest[..stub_end].eq_ignore_ascii_case(stub_kw) {
        return None;
    }
    let after = rest[stub_end..].trim_start();
    if after.is_empty() {
        return None;
    }
    // First whitespace-delimited token is the command name.
    let end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    if end == 0 { None } else { Some(&after[..end]) }
}

/// Scan `source` for inline ``# tcl-lsp: stub NAME {ARGS} ?FLAGS?`` and
/// ``# tcl-lsp: stub expr-func NAME ?ARITY?`` (also ``expr-op``)
/// declarations bounded by the ``stubs-begin`` / ``stubs-end``
/// markers, returning ``(commands, expr_defs)``.
///
/// Parses args and flags for the command form and extracts arity
/// for the expression form. Command stubs *require* the
/// ``{ARGS}`` brace block; bare ``stub NAME`` is rejected. Stubs
/// whose ``:role`` annotation is unrecognised are silently
/// dropped.
#[must_use]
pub fn scan_source_for_stubs(
    source: &str,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let (commands, expressions, _) =
        // dialect-drift-ok: sidecar stub declarations (and tests); a document's
        // stubs are scanned by `scan_source_for_stubs_with_outside` under its config
        scan_source_for_stubs_with_outside(source, tcl_lexer::LexerConfig::default());
    (commands, expressions)
}

/// Parse inline stubs and retain valid directives discarded outside a block.
/// Comment positions come from the lexer, not from the directive grammar.
#[must_use]
pub fn scan_source_for_stubs_with_outside(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
    Vec<Span>,
) {
    let mut cmds = Vec::new();
    let mut exprs = Vec::new();
    let mut outside_candidates = Vec::new();
    let mut in_block = false;
    let mut offset: u32 = 0;
    for (line_number, line) in source.split_inclusive('\n').enumerate() {
        let trimmed_line: &str = line.trim_end_matches('\n');
        let line_byte_len =
            u32::try_from(line.len()).expect("line length fits in u32 for in-memory source");
        let stripped = trimmed_line.trim();
        let trimmed_len = u32::try_from(trimmed_line.len())
            .expect("line length fits in u32 for in-memory source");
        let span = tcl_lexer::Span::new(offset, offset + trimmed_len);
        offset += line_byte_len;
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        if matches_marker(body, "stubs-begin") {
            in_block = true;
            continue;
        }
        if matches_marker(body, "stubs-end") {
            in_block = false;
            continue;
        }
        if !in_block {
            let after_prefix = strip_tcl_lsp_prefix(body);
            if after_prefix != body
                && (parse_expr_stub(after_prefix).is_some()
                    || parse_command_stub(after_prefix, span).is_some())
                && let Ok(line) = u32::try_from(line_number)
            {
                outside_candidates.push((line, span));
            }
            continue;
        }
        // ``# tcl-lsp:`` prefix is optional inside a stubs block —
        // strip it once if present.
        let after_prefix = strip_tcl_lsp_prefix(body);
        // Try the expr-func / expr-op shape first because ``stub
        // expr-func NAME`` would otherwise match the command shape
        // with NAME == "expr-func".
        if let Some((kind, name, arity)) = parse_expr_stub(after_prefix) {
            exprs.push(super::types::StubExprDef {
                name: name.to_string(),
                kind: kind.to_string(),
                arity,
                range: span,
                from_sidecar: false,
            });
            continue;
        }
        if let Some(stub) = parse_command_stub(after_prefix, span) {
            cmds.push(stub);
        }
    }
    let outside = if outside_candidates.is_empty() {
        Vec::new()
    } else {
        let comments = tcl_lexer::comment_line_starts(source, config);
        outside_candidates
            .into_iter()
            .filter_map(|(line, span)| comments.binary_search(&line).is_ok().then_some(span))
            .collect()
    };
    (cmds, exprs, outside)
}

/// Report an authored declaration that the stub grammar parsed but could not
/// install because no stub block owned its comment line.
pub fn outside_stub_diagnostics(
    spans: Vec<Span>,
) -> impl Iterator<Item = super::types::Diagnostic> {
    spans.into_iter().map(|span| {
        super::types::Diagnostic::new(
            tcl_core_types::DiagCode::W153,
            span,
            "Stub directive outside a stubs-begin/stubs-end block has no effect",
            super::types::Severity::Warning,
        )
    })
}

/// Load the nearest workspace sidecar named `<dialect>.tcl.stubs` and parse
/// its declarations using the same grammar as inline stubs.  Sidecars are
/// deliberately searched from the source directory upwards: a package or
/// workspace can provide a broad bundle at its root while a nested project
/// can override it with a nearer file.  The source itself is never modified,
/// so all source spans remain stable; sidecar declaration spans are synthetic
/// and are only retained for the normal stub metadata surface.
#[must_use]
pub fn scan_sidecar_stubs(
    file_path: Option<&str>,
    // The dialect *name*, not a resolved profile: this is a filename on disk
    // (`<dialect>.tcl.stubs`), so it must keep the spelling the workspace
    // configured. Canonicalising an alias here would stop finding a sidecar a
    // user named after it.
    dialect: &str,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let Some(file_path) = file_path else {
        return (Vec::new(), Vec::new());
    };
    let mut dir = std::path::Path::new(file_path).parent();
    while let Some(current) = dir {
        let sidecar = current.join(format!("{dialect}.tcl.stubs"));
        if let Ok(content) = std::fs::read_to_string(&sidecar) {
            return scan_sidecar_lines(&content);
        }
        dir = current.parent();
    }
    (Vec::new(), Vec::new())
}

/// Parse sidecar syntax: one declaration per line, without the inline
/// comment prefix or begin/end markers. It is adapted into the canonical
/// scanner rather than maintaining a second parser; declaration spans are
/// synthetic.
fn scan_sidecar_lines(
    content: &str,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let mut adapted = String::from("# tcl-lsp: stubs-begin\n");
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            adapted.push_str(trimmed);
        } else {
            adapted.push_str("# tcl-lsp: ");
            adapted.push_str(line);
        }
        adapted.push('\n');
    }
    adapted.push_str("# tcl-lsp: stubs-end\n");
    let (mut cmds, mut exprs) = scan_source_for_stubs(&adapted);
    for stub in &mut cmds {
        stub.from_sidecar = true;
    }
    for stub in &mut exprs {
        stub.from_sidecar = true;
    }
    (cmds, exprs)
}

/// The procedures sibling workspace documents define, declared from their
/// sidecar-syntax `stub` lines through the same grammar as inline and sidecar
/// stubs. Each is [`Provenance::WorkspaceTrusted`]: it stands for a source
/// procedure in another document, not an assistance-only stub.
///
/// [`Provenance::WorkspaceTrusted`]: tcl_dialect::model::Provenance::WorkspaceTrusted
#[must_use]
pub fn workspace_procedure_declarations(content: &str) -> tcl_registry::model::DeclaredSurface {
    let (cmds, _) = scan_sidecar_lines(content);
    let stubbed = super::types::build_declared_surface(&cmds);
    let mut surface = tcl_registry::model::DeclaredSurface::new();
    for (_, command) in stubbed.iter() {
        let mut command = command.clone().inferred_from_procedure();
        command.declaration.provenance = tcl_dialect::model::Provenance::WorkspaceTrusted;
        surface.declare(command);
    }
    surface
}

/// Whether the source's nearest ancestor sidecar is readable. The incremental
/// analyser uses this as a correctness gate: sidecar signatures affect
/// isolated proc bodies, so their presence selects the full analyser until
/// those signatures are part of every per-body memo key.
#[must_use]
pub fn has_sidecar_stubs(file_path: Option<&str>, profile: &tcl_dialect::DialectProfile) -> bool {
    let Some(file_path) = file_path else {
        return false;
    };
    let mut dir = std::path::Path::new(file_path).parent();
    while let Some(current) = dir {
        if current
            .join(format!("{}.tcl.stubs", profile.name))
            .is_file()
        {
            return true;
        }
        dir = current.parent();
    }
    false
}

/// Strip a leading ``tcl-lsp:`` keyword (case-insensitive) from
/// the comment body, if present.  Whitespace flexible.
fn strip_tcl_lsp_prefix(body: &str) -> &str {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    // `s.get(..kw_end)` (unlike `s[..kw_end]`) returns `None` rather than
    // panicking when `kw_end` falls inside a multi-byte char instead of on a
    // boundary — a non-ASCII byte in the comment before that offset would
    // otherwise crash the scan of a stubs block.
    let Some(prefix) = s.get(..kw_end) else {
        return s;
    };
    if !prefix.eq_ignore_ascii_case(lower_prefix) {
        return s;
    }
    let rest = s[kw_end..].trim_start();
    rest.strip_prefix(':').map_or(s, str::trim_start)
}

/// Parse a ``stub NAME {ARGS} ?FLAGS?`` line (case-insensitive on
/// the ``stub`` keyword).  Returns a fully-populated
/// `StubCommandDef` (name, args, flags, range) on match, or
/// ``None`` when the brace block is missing or an argument's
/// ``:role`` annotation is unrecognised.
fn parse_command_stub(line: &str, range: tcl_lexer::Span) -> Option<super::types::StubCommandDef> {
    let s = line.trim_start();
    let stub_kw = "stub";
    if !s
        .get(..stub_kw.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(stub_kw))
    {
        return None;
    }
    let after = s[stub_kw.len()..].trim_start();
    if after.is_empty() {
        return None;
    }
    // First whitespace-delimited token is the command name.
    let name_end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    if name_end == 0 {
        return None;
    }
    let name = &after[..name_end];
    // Reject ``expr-func`` / ``expr-op`` here — those are handled
    // by ``parse_expr_stub``.
    if name.eq_ignore_ascii_case("expr-func") || name.eq_ignore_ascii_case("expr-op") {
        return None;
    }
    let after_name = after[name_end..].trim_start();
    // ``stub NAME {ARGS}`` must have a closing ``}`` — find the
    // matching close, capture the args body, then parse the
    // trailing flag run.
    let after_open = after_name.strip_prefix('{')?;
    let close_rel = after_open.find('}')?;
    let args_body = &after_open[..close_rel];
    let after_close = after_open[close_rel + 1..].trim_start();
    let args = parse_stub_args(args_body)?;
    let flags = parse_stub_flags(after_close);
    let frame = tcl_registry::model::DeclaredFrameEffect::from_stub_flags(after_close);
    Some(super::types::StubCommandDef {
        name: name.to_string(),
        args,
        range,
        flags,
        frame,
        from_sidecar: false,
    })
}

/// Parse the inside of ``stub NAME { … }``.  Returns the
/// argument list, or `None` if any token uses an unrecognised
/// ``:role`` annotation or has an empty name (which drops the
/// whole stub).  Empty input yields an empty list.
fn parse_stub_args(args_str: &str) -> Option<Vec<super::types::StubArgDef>> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for token in trimmed.split_whitespace() {
        let mut name = token;
        let optional = name.len() >= 2 && name.starts_with('?') && name.ends_with('?');
        if optional {
            name = &name[1..name.len() - 1];
        }
        let (arg_name, role) = if let Some(idx) = name.find(':') {
            let arg_name = &name[..idx];
            let role_lower = name[idx + 1..].to_ascii_lowercase();
            // The registry owns the role vocabulary, so a word it does not
            // know is a typo rather than a silent `Value`: reject the whole
            // declaration and let the unresolved-command hint say so.
            tcl_registry::model::role_for_word_checked(&role_lower)?;
            (arg_name, role_lower)
        } else {
            (name, "value".to_string())
        };
        if arg_name.is_empty() {
            return None;
        }
        result.push(super::types::StubArgDef {
            name: arg_name.to_string(),
            role,
            optional,
        });
    }
    Some(result)
}

/// Parse the trailing ``?-flag…?`` run after the ``{ARGS}`` block.
/// Unrecognised tokens are ignored.
fn parse_stub_flags(flags_str: &str) -> super::types::StubFlags {
    let mut flags = super::types::StubFlags::empty();
    for token in flags_str.split_whitespace() {
        match token {
            "-barrier" => flags |= super::types::StubFlags::BARRIER,
            "-loop" => flags |= super::types::StubFlags::LOOP,
            "-pure" => flags |= super::types::StubFlags::PURE,
            "-mutator" => flags |= super::types::StubFlags::MUTATOR,
            "-unsafe" => flags |= super::types::StubFlags::UNSAFE,
            "-scope_alias" => flags |= super::types::StubFlags::SCOPE_ALIAS,
            "-extension" => flags |= super::types::StubFlags::EXTENSION,
            _ => {}
        }
    }
    flags
}

/// Parse a ``stub expr-func NAME ?ARITY?`` / ``stub expr-op NAME
/// ?ARITY?`` line.  Returns ``(kind, name, arity)`` on match.
/// ``arity`` defaults to 1 for functions and 2 for operators when
/// the trailing arity word is absent.
fn parse_expr_stub(line: &str) -> Option<(&'static str, &str, u32)> {
    let s = line.trim_start();
    let stub_kw = "stub";
    if !s
        .get(..stub_kw.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(stub_kw))
    {
        return None;
    }
    let after = s[stub_kw.len()..].trim_start();
    let kind: &'static str;
    let default_arity: u32;
    let rest;
    if after
        .get(.."expr-func".len())
        .is_some_and(|p| p.eq_ignore_ascii_case("expr-func"))
    {
        kind = "function";
        default_arity = 1;
        rest = after["expr-func".len()..].trim_start();
    } else if after
        .get(.."expr-op".len())
        .is_some_and(|p| p.eq_ignore_ascii_case("expr-op"))
    {
        kind = "operator";
        default_arity = 2;
        rest = after["expr-op".len()..].trim_start();
    } else {
        return None;
    }
    if rest.is_empty() {
        return None;
    }
    let name_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    let name = &rest[..name_end];
    let after_name = rest[name_end..].trim_start();
    let arity = if after_name.is_empty() {
        default_arity
    } else {
        let arity_end = after_name
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_name.len());
        after_name[..arity_end]
            .parse::<u32>()
            .unwrap_or(default_arity)
    };
    Some((kind, name, arity))
}

/// Decoration-character set for the body-docstring scanner
/// (``.-=*~#``).
const DECORATION_CHARS: &[char] = &['.', '-', '=', '*', '~', '#'];

/// Extract a leading comment block from a proc body — the
/// fallback that fires when there's no preceding-comment harvest
/// from the segmenter.
///
/// Lines containing only decoration characters
/// (``.-=*~#``) are skipped; remaining ``#``-prefixed lines have
/// the leading hash + whitespace stripped, then accumulated.
/// The first non-comment / non-blank line ends the block.
#[must_use]
pub fn extract_body_docstring(body: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in body.lines() {
        let stripped = raw.trim();
        if stripped.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let text = stripped.trim_start_matches('#').trim().to_string();
        // Skip pure-hash decoration lines (``####`` etc.).
        if text.is_empty() && stripped.chars().all(|c| c == '#') {
            continue;
        }
        // Skip lines made entirely of decoration characters.
        if !text.is_empty() && text.chars().all(|c| DECORATION_CHARS.contains(&c)) {
            continue;
        }
        lines.push(text);
    }
    lines.join("\n")
}

/// Per-line ``# noqa: CODE`` suppression scanner.
///
/// For each ``# noqa`` (with or without a ``: CODE`` list),
/// records the codes against the 0-based line range read from the
/// ``preceding_comment`` field of the segmented command. The
/// segmented-command dispatch loop calls this on each command to
/// attach the noqa codes to the *following* command's line range.
///
/// The directive shapes come from [`parse_noqa_marker`]: bare
/// ``# noqa`` (no ``: CODE`` list) suppresses every code (`"*"`
/// sentinel); ``# noqa: A, B`` suppresses the named codes. The
/// text searched is the segmenter's ``preceding_comment`` field,
/// which carries comment text only, so a ``#`` inside a Tcl
/// string is never read as a directive.
pub fn apply_preceding_noqa<S, T>(
    cmd: &crate::segmenter::SegmentedCommand,
    line_offsets: &[usize],
    suppressed_lines: &mut std::collections::HashMap<i32, std::collections::HashSet<String, T>, S>,
) where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher + Default,
{
    let Some(comment) = cmd.preceding_comment.as_deref() else {
        return;
    };
    let Some(codes) = parse_noqa_marker(comment) else {
        return;
    };
    // Attribute to every line spanned by the command.
    // ``SegmentedCommand.span`` is byte offsets; convert each via
    // the precomputed ``line_offsets`` index in ``O(log N)``
    // instead of a linear scan per call (the helper runs once per
    // segmented command).
    let span_start = cmd.span.start() as usize;
    let span_end = cmd.span.end() as usize;
    let start_line = super::state::line_at_offset(line_offsets, span_start);
    let end_line = super::state::line_at_offset(line_offsets, span_end);
    for line in start_line..=end_line {
        suppressed_lines
            .entry(line)
            .or_default()
            .extend(codes.iter().cloned());
    }
}

/// Match `# tcl-lsp: disable=…` (case-insensitive on the keyword),
/// returning the trailing CODE list as a borrowed slice. Returns
/// `None` if the line doesn't match the directive shape.
///
/// Uses [`str::get`] rather than byte-index slicing for the keyword
/// checks: a fixed ASCII-length prefix like `s[..kw_end]` panics when a
/// multi-byte UTF-8 character (an em dash, accented letter, …) straddles
/// that byte offset — `s.get(..kw_end)` instead returns `None` for a
/// non-boundary offset exactly as it does for a too-short `s`, so a
/// leading comment with early non-ASCII text is simply "not a directive"
/// rather than a panic.
fn parse_disable_directive(line: &str) -> Option<&str> {
    let mut s = line.trim_start();
    s = s.strip_prefix('#')?.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if !s.get(..kw_end)?.eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    s = s[kw_end..].trim_start();
    s = s.strip_prefix(':')?.trim_start();
    let disable_kw = "disable";
    let dis_end = disable_kw.len();
    if !s.get(..dis_end)?.eq_ignore_ascii_case(disable_kw) {
        return None;
    }
    s = s[dis_end..].trim_start();
    let s = s.strip_prefix('=')?.trim();
    Some(s)
}

/// Format a literal value for inclusion in a diagnostic message.
///
/// Replaces newlines with `\n` (literal two characters), and
/// truncates strings longer than 40 characters to `…[37 chars]…`.
#[must_use]
pub fn format_literal_for_message(value: &str) -> String {
    let display = value.replace('\n', "\\n");
    if display.chars().count() > 40 {
        // Slice at character (not byte) boundaries — Tcl source is
        // usually ASCII but multibyte diagnostic messages are
        // valid.
        let truncated: String = display.chars().take(37).collect();
        format!("{truncated}...")
    } else {
        display
    }
}

/// Return `(variable, static_value)` for assignments worth the
/// W214 paste-error heuristic check.
///
/// Only [`Statement::AssignConst`] and [`Statement::AssignValue`]
/// shapes qualify; `AssignValue` rejects values containing
/// `$` / `[` / `]` (interpolation / command substitution
/// disqualify the fingerprint).
#[must_use]
pub fn possible_paste_fingerprint(stmt: &Statement) -> Option<(String, String)> {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            Some((name.clone(), value.trim().to_string()))
        }
        Statement::AssignValue { name, value, .. } => {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            if value.contains('$') || value.contains('[') || value.contains(']') {
                return None;
            }
            Some((name.clone(), value.to_string()))
        }
        _ => None,
    }
}

/// Return argv tokens widened to each Tcl word's full token span.
///
/// The segmenter already returns argv tokens with word-wide spans
/// (see `rust/tcl-compiler/src/segmenter.rs`), so this is the
/// identity function. The helper is kept as a transparent
/// passthrough so the indirection layer at the call sites stays
/// intact.
#[must_use]
pub fn argv_with_word_spans(argv: Vec<Token>, _all_tokens: &[Token]) -> Vec<Token> {
    argv
}

/// Full source span of one Tcl word, as defined by the lexer.
///
/// [`tcl_lexer::word_span`] is the authoritative delimiter policy: it handles
/// braces, brackets, quotes, empty words, escaped closers, and recovery
/// tokens without each analyser diagnostic reimplementing byte arithmetic.
///
/// Builds a [`SourceMap`] per call, and [`SourceMap::new`] rescans the whole
/// document to construct its line index. That is fine for a one-off caller,
/// but a caller that runs **per command or per word** must hoist the map and
/// use [`full_word_span_in`] instead — see its doc comment.
#[must_use]
pub fn full_word_span(token: Token, source: &str) -> Span {
    full_word_span_in(&SourceMap::new(source), token)
}

/// [`full_word_span`] against an already-built [`SourceMap`].
///
/// Same answer, without the rebuild. `SourceMap::new` scans the entire
/// document to populate its line index, so calling [`full_word_span`] once per
/// *word* makes span resolution `O(document size × word count)` — the same
/// quadratic that [`crate::analyser::state::Analyser::cached_line_index`] was
/// introduced to remove for the per-command `SourceMap::new(&self.source)`
/// call sites. On a ~5k-line file those rebuilds dominate whole-file analysis.
///
/// Callers inside the analyser should build the map once with
/// `Analyser::source_map(&self.source, &self.cached_line_index,
/// self.cached_line_index_source_len)`, hoisted out of any loop over words.
#[must_use]
pub fn full_word_span_in(source_map: &SourceMap<'_>, token: Token) -> Span {
    tcl_lexer::word_span(source_map, token)
}

/// The set of iRules commands that may only appear at the top
/// level of an event handler, computed fresh from the supplied
/// `registry`.
///
/// Reads the `IRULES_TOP_LEVEL_ONLY` trait from `registry` and
/// returns the matching command names.
///
/// **Not cached.** Caching the first-call result in a static
/// `OnceLock` silently returns stale data once a non-default
/// `CommandRegistry` is passed on a later call. The trait scan is `O(n)` over the registry's
/// command specs (~150 entries) and the registry itself is
/// already cached at the call sites that need this — so per-call
/// recomputation is cheap in practice and removes a correctness
/// trap. Callers that want caching should cache at their own
/// layer, keyed by whatever identity makes sense for them.
#[must_use]
pub fn irules_top_level_only(registry: &CommandRegistry) -> HashSet<String> {
    registry
        .commands_with_trait(Traits::IRULES_TOP_LEVEL_ONLY)
        .into_iter()
        .map(String::from)
        .collect()
}

/// The exact source window a [`Traits::SCRIPT_CONCATENATES_ARGS`] tail
/// evaluates — every content byte at its true offset, every structural
/// byte blanked to a space — or `None` when the script is not statically
/// knowable.
///
/// `Tcl_ConcatObj` (`generic/tclUtil.c`) is the one rule the whole eval
/// family shares: take each word's **string representation**, trim ASCII
/// whitespace from both ends of it, drop the ones that end up empty, and
/// join the rest with a single space. A braced word contributes its
/// *contents* — Tcl's word parsing has already removed the outer braces
/// before `Tcl_ConcatObj` ever sees the value — so braced and bare words
/// concatenate identically and a command may span word boundaries.
///
/// Joining the trimmed texts into a fresh string reproduces those bytes
/// but loses their *positions*: stripped delimiters, dropped words, and
/// collapsed whitespace shift every byte after the first divergence, so a
/// span recorded while walking the joined text can land on the wrong
/// source bytes (in `eval {} foo` the reconstructed `foo` starts at the
/// empty word's closing brace, and a rename would edit that delimiter
/// instead of the command). So this builds the equivalent script *in
/// place* instead: a buffer covering the words' whole span in `source`,
/// holding each word's content bytes at their real offsets and a space
/// everywhere else (delimiters, inter-word gaps). Script parsing treats
/// any whitespace run as one separator, so the window parses into exactly
/// the words the trimmed-and-joined text would — `Tcl_ConcatObj`'s trim,
/// drop, and single-space steps all degenerate to "more whitespace" —
/// while every span the walk records maps to the source bytes it
/// describes.
///
/// Pinned against tclsh8.6.14 and tclsh9.0.4:
///
/// ```text
/// eval set l2 hello        ≡ set l2 hello
/// eval {set l2} hello      ≡ set l2 hello   (braces gone, contents kept)
/// eval "set q" 7           ≡ set q 7        (quotes gone; a space inside
///                                            a quoted word separates —
///                                            the re-parse splits it just
///                                            as the join would)
/// eval {set l2} {$value}   ≡ set l2 $value  (braces blocked the outer
///                                            substitution, so the script
///                                            text is static; `eval`
///                                            itself resolves `$value`
///                                            when the script runs)
/// eval {} foo              ≡ foo            (an empty word is just more
///                                            whitespace)
/// ```
///
/// Returns `None` when any word is not statically-known script text
/// ([`super::diagnostics::helpers::word_is_static_script_text`]: a
/// substitution or a decode runs before concatenation, so the real script
/// is unknowable), when a word's text does not sit verbatim at its
/// token's content offset (bytes that cannot be mapped must not be
/// analysed), or when the window holds no content at all. Callers must
/// then consume the command without walking it rather than guess.
///
/// This is **not** the rule for `namespace inscope`, whose tail is appended
/// as list elements — see [`Traits::SCRIPT_APPENDS_LIST_ARGS`].
#[must_use]
pub fn concat_script_window(
    words: &[String],
    tokens: &[Token],
    source: &str,
) -> Option<(String, tcl_lexer::Span)> {
    let (first, last) = (tokens.first()?, tokens.last()?);
    if words.len() != tokens.len() {
        return None;
    }
    let start = first.span.start() as usize;
    let end = last.span.end() as usize;
    if end <= start || source.len() < end {
        return None;
    }
    let mut window = vec![b' '; end - start];
    for (word, tok) in words.iter().zip(tokens) {
        if !crate::analyser::diagnostics::helpers::word_is_static_script_text(word, tok.kind) {
            return None;
        }
        let content_start = tok.span.start() as usize + usize::from(tok.content_offset);
        // The word's text must sit verbatim at its content offset — any
        // divergence (an escape decode, a token shape this fn does not
        // model) makes the bytes unmappable, and unmappable bytes must
        // not be walked.
        if source.get(content_start..content_start + word.len()) != Some(word.as_str()) {
            return None;
        }
        let off = content_start.checked_sub(start)?;
        window
            .get_mut(off..off + word.len())?
            .copy_from_slice(word.as_bytes());
    }
    if window.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    let window = String::from_utf8(window).ok()?;
    Some((
        window,
        tcl_lexer::Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?),
    ))
}

#[cfg(test)]
mod concat_script_window_tests;

/// Every stub declaration in force for one document: the nearest
/// `<dialect>.tcl.stubs` sidecar, overridden by the document's own inline
/// `# tcl-lsp: stubs-begin` block.
///
/// The one ingestion path. The analyser calls it to build its
/// [`AnalysisResult::stub_commands`](super::types::AnalysisResult::stub_commands)
/// and its [`DeclaredSurface`](tcl_registry::model::DeclaredSurface); every
/// host that builds a [`CompilationUnit`](crate::compilation_unit::CompilationUnit)
/// through the `cu_override` seam calls it (via
/// [`document_declared_surface`]) so the unit it supplies declares exactly
/// what the analyser's own unit would.
#[must_use]
pub fn document_stub_declarations(
    source: &str,
    file_path: Option<&str>,
    dialect: &str,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let (commands, expressions, _) = document_stub_declarations_with_outside(
        source,
        file_path,
        dialect,
        tcl_lexer::LexerConfig::default(),
    );
    (commands, expressions)
}

/// The same declaration scan, including discarded authored directive spans.
#[must_use]
pub fn document_stub_declarations_with_outside(
    source: &str,
    file_path: Option<&str>,
    dialect: &str,
    config: tcl_lexer::LexerConfig,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
    Vec<Span>,
) {
    let (inline_cmds, inline_exprs, outside) = scan_source_for_stubs_with_outside(source, config);
    let (mut cmds, mut exprs) = scan_sidecar_stubs(file_path, dialect);
    // The document-local declaration is nearest in scope and wins over a
    // workspace sidecar with the same name — `DeclaredSurface::declare`
    // keeps the last ingested one.
    cmds.extend(inline_cmds);
    exprs.extend(inline_exprs);
    (cmds, exprs, outside)
}

/// The document's [`DeclaredSurface`](tcl_registry::model::DeclaredSurface) —
/// [`document_stub_declarations`] ingested through
/// [`build_declared_surface`](super::types::build_declared_surface).
#[must_use]
pub fn document_declared_surface(
    source: &str,
    file_path: Option<&str>,
    dialect: &str,
) -> tcl_registry::model::DeclaredSurface {
    let (cmds, _) = document_stub_declarations(source, file_path, dialect);
    super::types::build_declared_surface(&cmds)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_configured_recovery_tests;
