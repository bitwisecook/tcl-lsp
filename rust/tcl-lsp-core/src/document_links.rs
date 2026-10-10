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

//! Document-links provider.
//!
//! Detects `source <path>` invocations in the document and
//! surfaces each path argument as a clickable link.  When a
//! `workspace_root` is provided, relative paths resolve
//! against it; absolute paths surface as-is.
//!
//! Limitations:
//!
//! * `package require <pkg>` resolution — needs a package
//!   index (Tcl's `auto_path` / pkgIndex.tcl scan) — is not
//!   done.
//! * Native links use the complete current analysis, selected Registry source
//!   handler and exact original effective path value. Unavailable values and
//!   filesystem units decline. The logical compatibility path resolves through
//!   the **source graph's own**
//!   path evaluator
//!   ([`tcl_compiler::auto_path_eval::evaluate_auto_path_expr`]) — the
//!   `[file dirname [info script]]` / `[file join …]` idioms — plus the
//!   shared single-assignment `set` constant map
//!   ([`tcl_compiler::auto_path_eval::constant_path_vars`]), which folds
//!   chains, so a directory reached through an intermediate resolves as
//!   readily as a direct one.  Anything outside that subset produces
//!   **no link at all**:
//!   a `source` argument whose reconstructed text still carries `$` or
//!   `[` is never treated as a literal path — doing so percent-encodes the
//!   raw Tcl text into a syntactically valid but semantically bogus
//!   `file://` URI.
//! * Workspace-folder enumeration that lets a `source` link
//!   resolve across multiple roots is not done; the single
//!   `workspace_root` parameter is sufficient for the
//!   common single-root case.
//!
//! A resolved target is only half the link: the **range** it is offered on
//! must not span code.  A link is painted as one flat run, so a range
//! covering a whole `[file join $dir x.tcl]` hides every token boundary
//! inside it and the substitution stops looking like the command sequence
//! it is.  [`link_anchor_with_config`] is the rule — a literal word links
//! whole, a substitution links on its trailing literal word alone — and
//! `tests/e2e/semantic_tokens.rs` pins the invariant it exists to keep: no
//! link range covers more than one semantic token.
//!
//! Also handled:
//!
//! * Tilde expansion (`~/path/to/file`) — via the
//!   `home` argument plumbed in by the server from the env.
//! * Literal `[file join a b c]` — recognised when every
//!   sub-arg is a simple bareword / quoted string; the joined
//!   path resolves against `workspace_root` like any other
//!   relative arg.
//! * A spec pack's `include NAME` row — see [`pack_include_links`].

use tcl_compiler::auto_path_eval::carries_substitution;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LineIndex, Token, TokenType};
use tcl_registry::definer::DefinerFamily;

/// One link in a document — target URI plus the source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    /// Source-range start line.
    pub start_line: u32,
    /// Source-range start character.
    pub start_character: u32,
    /// Source-range end line.
    pub end_line: u32,
    /// Source-range end character.
    pub end_character: u32,
    /// Target URI string.  Empty when the link has no navigable target
    /// (e.g. a `package require` whose package isn't on disk) — the
    /// `tooltip` still surfaces.
    pub target: String,
    /// Hover tooltip (the raw path / package name).  `None` for links
    /// with no extra text.
    pub tooltip: Option<String>,
}

/// Everything a link needs to turn a `source` argument into a filesystem
/// path: where relative paths anchor, what `~` expands to, and what the
/// document's own `[info script]` would answer.
///
/// A struct rather than three more positional parameters so adding the next
/// contextual fact doesn't change every call site (and doesn't trip
/// `clippy::too_many_arguments`).
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkContext<'a> {
    /// The document's **imported** path constants — values source-graph
    /// ancestors establish before it runs
    /// (`WorkspaceIndex::imported_path_constants_for`).  `None`
    /// means no import view is available (single-file callers), which only
    /// costs coverage, never correctness.
    pub imported_constants: Option<&'a tcl_compiler::auto_path_eval::FoldedPathConstants>,
    /// Directory relative paths resolve against — typically the document's
    /// own enclosing directory.  `None` leaves relative paths unresolvable,
    /// so only absolute ones produce links.
    pub workspace_root: Option<&'a str>,
    /// `$HOME`, for `~/…` expansion.  `None` leaves `~` paths unresolved.
    pub home: Option<&'a str>,
    /// The document's own filesystem path — what `[info script]` answers
    /// while it is being sourced.  `None` makes every `[info script]`-based
    /// computed path abstain rather than guess.
    pub script_path: Option<&'a str>,
}

/// Compute document links for a document.
///
/// `workspace_root`, when `Some`, is the directory used to
/// resolve relative paths.  Typically the document's enclosing
/// directory.  When `None`, only absolute paths produce links.
///
/// `~/...` paths expand against `$HOME`; the helper reads
/// the env var at call-time so server tests can stub it (see
/// `document_links_with_home`).
#[must_use]
pub fn document_links(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    workspace_root: Option<&str>,
) -> Vec<DocumentLink> {
    let home = std::env::var("HOME").ok();
    document_links_with_home(source, dialect, workspace_root, home.as_deref())
}

/// Same as [`document_links`] but lets callers pass an
/// explicit `home` directory string (for testability under
/// `#![forbid(unsafe_code)]`, where we can't mutate
/// `std::env`).  Production callers should use the
/// zero-argument [`document_links`].
#[must_use]
pub fn document_links_with_home(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    workspace_root: Option<&str>,
    home: Option<&str>,
) -> Vec<DocumentLink> {
    document_links_in_context(
        source,
        dialect,
        &LinkContext {
            imported_constants: None,
            workspace_root,
            home,
            script_path: None,
        },
    )
}

/// Fresh-source compatibility entry point. Retain the supplied profile and
/// its actual Registry context before delegating to the analysis-based owner.
#[must_use]
pub fn document_links_in_context(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    ctx: &LinkContext<'_>,
) -> Vec<DocumentLink> {
    let mut analyser = tcl_compiler::analyser::Analyser::new();
    let generation = tcl_registry::model::ingress::context_for_profile(dialect);
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        dialect,
        dialect,
        generation,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
    );
    analyser = analyser.with_resolved_input(input);
    let analysis = analyser.analyse(source, dialect.name);
    document_links_from_analysis(source, &analysis, ctx)
}

/// Readonly path/package links from the complete current analysis source and
/// full configuration. Registry handler advice and exact original effective
/// operands remain independent of filesystem availability or file execution.
/// Known shadowed commands, unavailable values and stale images produce no
/// links. Native paths never use the logical String path evaluator. Hosted
/// paths use their independently selected source templates and supported
/// source units, without borrowing a native loader or filesystem recipe.
#[must_use]
pub fn document_links_from_analysis(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    ctx: &LinkContext<'_>,
) -> Vec<DocumentLink> {
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return Vec::new();
    }
    if analysis.resolved_profile().is_none() {
        return Vec::new();
    }
    if analysis.allows_lexical_declaration_advice() {
        return lexical_document_links_in_context(source, analysis, ctx);
    }
    let line_index = LineIndex::new(source);
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let mut links = pack_include_links(source, registry, config, &line_index, ctx.script_path);
    let selection = OriginalLinkSelection {
        source,
        analysis,
        ctx,
        line_index: &line_index,
    };
    if analysis.has_original_vendor_source_names() {
        links.extend(selection.vendor_links());
    } else if let Some(commands) =
        crate::original_invocation::registry_commands_in_source(source, analysis)
    {
        links.extend(
            commands
                .into_iter()
                .filter_map(|(_, words)| selection.navigation_link(&words)),
        );
    }
    links.extend(
        analysis
            .package_requires
            .iter()
            .filter_map(|requirement| selection.package_link(requirement)),
    );
    links.sort_by_key(|link| {
        (
            link.start_line,
            link.start_character,
            link.end_line,
            link.end_character,
        )
    });
    links.dedup();
    links
}

struct OriginalLinkSelection<'a> {
    source: &'a str,
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    ctx: &'a LinkContext<'a>,
    line_index: &'a LineIndex,
}
impl OriginalLinkSelection<'_> {
    fn vendor_links(&self) -> Vec<DocumentLink> {
        use tcl_registry::source_navigation::SourceNavigationOperand;
        use tcl_syntax::naming::VendorSourceNamePurpose;
        let mut seen = std::collections::HashSet::new();
        self.analysis
            .original_vendor_source_names()
            .filter_map(|occurrence| {
                let words = occurrence.original_words();
                if words.first()? != occurrence.name_input().original_word()
                    || !seen.insert(occurrence.site().clone())
                {
                    return None;
                }
                let (metadata, _) = crate::original_invocation::selected_vendor_registry_words_at(
                    self.source,
                    self.analysis,
                    words.first()?.span().start(),
                )?;
                let shape = metadata.shape();
                let (argument, purpose) = match shape.source_navigation_operand()? {
                    SourceNavigationOperand::File { argument } => {
                        (argument, VendorSourceNamePurpose::SourcePath)
                    }
                    SourceNavigationOperand::Package { argument } => {
                        (argument, VendorSourceNamePurpose::PackageName)
                    }
                };
                let word = shape.original_words().get(argument.checked_add(1)?)?;
                let units = tcl_syntax::naming::vendor_source_literal_units(
                    shape.original_head().policy(),
                    word,
                    purpose,
                )?;
                let value = std::str::from_utf8(units).ok()?;
                let (target, tooltip) = match shape.source_navigation_operand()? {
                    SourceNavigationOperand::File { .. } => (
                        resolve_path(value, self.ctx.workspace_root, self.ctx.home)?,
                        format!("Source path candidate: {value}"),
                    ),
                    SourceNavigationOperand::Package { .. } => (
                        String::new(),
                        format!("Package requirement candidate: {value}"),
                    ),
                };
                Some(self.link(word.content_span().ok()?, target, tooltip))
            })
            .collect()
    }

    fn source_operands(
        &self,
        words: &crate::original_invocation::OriginalRegistryWords,
    ) -> Option<tcl_registry::source_file::SourceFileOperands> {
        let context = self.analysis.resolved_input.as_ref()?.context_registry();
        let source_handler = words.with_source_schema(&context, |selected| {
            selected.semantics.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::Source)
        })?;
        if !source_handler {
            return None;
        }
        let grammar = words.dialect?.source_file_grammar()?;
        let selection = grammar.select_original(
            words.arguments.len(),
            words
                .arguments
                .first()
                .and_then(|word| word.literal_bytes()),
        );
        let tcl_registry::source_file::SourceFileSelection::Selected(operands) = selection else {
            return None;
        };
        Some(operands)
    }
    fn navigation_link(
        &self,
        words: &crate::original_invocation::OriginalRegistryWords,
    ) -> Option<DocumentLink> {
        if let Some(link) = self.source_link(words) {
            return Some(link);
        }
        let context = self.analysis.resolved_input.as_ref()?.context_registry();
        let argument = match words.with_source_schema(&context, |schema| {
            schema.authored_source_navigation_operand()
        })?? {
            tcl_registry::source_navigation::SourceNavigationOperand::Package { argument } => {
                argument
            }
            tcl_registry::source_navigation::SourceNavigationOperand::File { .. } => return None,
        };
        let operand = words.operands.get(argument)?.as_ref()?;
        let input = operand.input.as_ref()?;
        (words.arguments.get(argument)?.literal_bytes()? == input.bytes()).then_some(())?;
        let name = tcl_registry::native_package::NativePackageNameKey::from_native_units(
            input.bytes(),
            input.policy(),
        );
        let span = match input.original_word_key() {
            Some(key) if operand.word.as_ref() == Some(key.original_word()) => {
                key.original_word().content_span().ok()?
            }
            Some(_) => return None,
            None => operand.span,
        };
        Some(self.link(
            span,
            String::new(),
            format!(
                "package require {}",
                tcl_syntax::native_string::resident_name_label(name.bytes())
            ),
        ))
    }

    fn source_link(
        &self,
        words: &crate::original_invocation::OriginalRegistryWords,
    ) -> Option<DocumentLink> {
        let operands = self.source_operands(words)?;
        let value = words.arguments.get(operands.path_at)?.literal_bytes()?;
        // This is a readonly file candidate. Unrepresentable native filesystem
        // units and zero-terminated path boundaries require a separate owner.
        let path = std::str::from_utf8(value).ok()?;
        if path.contains('\0') {
            return None;
        }
        let target = resolve_path(path, self.ctx.workspace_root, self.ctx.home)?;
        let operand = words.operands.get(operands.path_at)?.as_ref()?;
        let input = operand.input.as_ref()?;
        if input.bytes() != value {
            return None;
        }
        let span = if let Some(key) = input.original_word_key() {
            // The sealed schema already owns this whole original vector and
            // static value; a missing runtime input issuer cannot erase it.
            (operand.word.as_ref() == Some(key.original_word())).then_some(())?;
            key.original_word().content_span().ok()?
        } else {
            let word = operand.word.as_ref()?;
            // An evaluated word has value authority but no replacement Key.
            // Link only a single command substitution's final literal anchor.
            if !crate::original_name_edit::original_input_matches_source(
                self.source,
                self.analysis,
                input,
                operand.span,
            ) || word.tokens().len() != 1
                || word.tokens()[0].kind != TokenType::Cmd
            {
                return None;
            }
            let (start, end) = link_anchor_with_config(
                self.source,
                self.analysis.body_lexer_config?,
                &word.tokens()[0],
            )?;
            tcl_lexer::Span::new(start, end)
        };
        Some(self.link(span, target, path.to_owned()))
    }
    fn package_link(
        &self,
        requirement: &tcl_compiler::signature_scan::types::SignaturePackageRequire,
    ) -> Option<DocumentLink> {
        let name = requirement.original_name.as_ref()?;
        let input = name.input();
        let word = input.original_word_key()?.original_word();
        if !crate::original_name_edit::original_input_matches_source(
            self.source,
            self.analysis,
            input,
            word.span(),
        ) {
            return None;
        }
        Some(self.link(
            word.content_span().ok()?,
            String::new(),
            format!(
                "package require {}",
                tcl_syntax::native_string::resident_name_label(name.key().bytes())
            ),
        ))
    }
    fn link(&self, span: tcl_lexer::Span, target: String, tooltip: String) -> DocumentLink {
        let start = self.line_index.position_at_utf16(span.start(), self.source);
        let end = self.line_index.position_at_utf16(span.end(), self.source);
        DocumentLink {
            start_line: start.line,
            start_character: start.character.get(),
            end_line: end.line,
            end_character: end.character.get(),
            target,
            tooltip: Some(tooltip),
        }
    }
}

/// The full-context entry point — the one the server calls, since only it
/// knows the document's own filesystem path (needed to evaluate the
/// `[file dirname [info script]]` idiom).
#[must_use]
fn lexical_document_links_in_context(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    ctx: &LinkContext<'_>,
) -> Vec<DocumentLink> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let script_path = ctx.script_path;
    let line_index = LineIndex::new(source);
    let mut links = Vec::new();
    // Constant single-assignment `set` map for the `set dir [file dirname
    // [info script]] … source [file join $dir x.tcl]` idiom, built once per
    // request.  Chained assignments fold too, so a
    // directory reached through an intermediate resolves, and
    // an import view from the host makes values sourced-in from ancestor
    // documents resolve exactly as they do for navigation.
    let no_imports = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
    let Some(assignments) =
        tcl_compiler::auto_path_eval::constant_path_assignments_from_analysis(source, analysis)
    else {
        return Vec::new();
    };
    let constants = tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
        &assignments,
        script_path,
        ctx.imported_constants.unwrap_or(&no_imports),
    );
    links.extend(pack_include_links(
        source,
        registry,
        config,
        &line_index,
        script_path,
    ));
    let mut commands = segment_commands_with_offset_and_config(source, 0, config);
    for span in assignments.namespace_body_spans() {
        if let Some(body) = source.get(span.start() as usize..span.end() as usize) {
            commands.extend(segment_commands_with_offset_and_config(
                body,
                span.start(),
                config,
            ));
        }
    }
    commands.sort_by_key(|command| command.span.start());
    for segment in commands {
        let Some(words) =
            tcl_compiler::registry_invocation::source_structure::source_registry_words(
                source, analysis, &segment,
            )
        else {
            continue;
        };
        let Some(navigation) = words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                schema.authored_source_navigation_operand()
            })
            .flatten()
        else {
            continue;
        };
        if let Some(link) = lexical_navigation_link(
            &segment,
            &words,
            navigation,
            &LexicalNavigationInputs {
                source,
                analysis,
                constants: &constants,
                line_index: &line_index,
                link_context: ctx,
            },
        ) {
            links.push(link);
        }
    }
    links
}

struct LexicalNavigationInputs<'a> {
    source: &'a str,
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    constants: &'a tcl_compiler::auto_path_eval::FoldedPathConstants,
    line_index: &'a LineIndex,
    link_context: &'a LinkContext<'a>,
}

fn lexical_navigation_link(
    segment: &tcl_compiler::segmenter::SegmentedCommand,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    navigation: tcl_registry::source_navigation::SourceNavigationOperand,
    inputs: &LexicalNavigationInputs<'_>,
) -> Option<DocumentLink> {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    use tcl_registry::source_navigation::SourceNavigationOperand;
    let (SourceNavigationOperand::File { argument }
    | SourceNavigationOperand::Package { argument }) = navigation;
    let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) =
        *words.origins().get(argument + 1)?
    else {
        return None;
    };
    let word = words.operands().get(argument)?.as_ref()?.word()?;
    let token = segment.argv.get(written)?;
    let content = word.content_span().ok()?;
    let config = inputs.analysis.body_lexer_config?;
    let (target, tooltip, anchor) = match navigation {
        SourceNavigationOperand::Package { .. } => {
            let name =
                std::str::from_utf8(words.arguments().get(argument)?.literal_bytes()?).ok()?;
            (
                String::new(),
                format!("package require {name}"),
                (content.start(), content.end()),
            )
        }
        SourceNavigationOperand::File { .. } => {
            let value = if let Some(bytes) = words.arguments().get(argument)?.literal_bytes() {
                std::str::from_utf8(bytes).ok()?.to_owned()
            } else {
                let expression = tcl_compiler::auto_path_eval::capture_source_path_expression(
                    inputs.source,
                    inputs.analysis,
                    word,
                )?;
                expression.evaluate(inputs.link_context.script_path, &|name| {
                    tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
                        &inputs.constants.at(segment.span.start()),
                        name,
                    )
                })?
            };
            let target = resolve_path(
                &value,
                inputs.link_context.workspace_root,
                inputs.link_context.home,
            )?;
            (
                target,
                value,
                link_anchor_with_config(inputs.source, config, token)?,
            )
        }
    };
    let start = inputs.line_index.position_at_utf16(anchor.0, inputs.source);
    let end = inputs.line_index.position_at_utf16(anchor.1, inputs.source);
    Some(DocumentLink {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
        target,
        tooltip: Some(tooltip),
    })
}

/// Every `include NAME` row of a spec pack, as a link to the sibling file it
/// names.
///
/// A pack's `include` row is the one statement in the `SpecTcl` vocabulary
/// that names another file, and the link has to land where the *loader* reads
/// — anywhere else it navigates to a file the pack never included. So this
/// spells out `IncludeContext::for_file`'s rule and only that rule: the name is
/// joined to the including document's own directory, with no tilde expansion
/// and no `workspace_root` fallback. The general `source <path>` resolver does
/// both, and `include ~` under it linked to the user's home directory while the
/// loader read a sibling literally named `~`.
///
/// A name carrying path structure gets no link either, because it names no
/// file: `loader::include_name` drops such a row before the resolver is ever
/// asked, so the declarations behind it are not loaded and there is nothing to
/// navigate to.
///
/// The gate is the registry, not the dialect's name: a document whose command
/// surface declares a [`DefinerFamily::SpecTcl`] document grammar *is* a pack,
/// however it reached that surface (the `.tclspec` extension, an editor
/// language id, a `# tcl-dialect:` pin). An ordinary Tcl script that happens to
/// call a proc of its own named `include` has no such grammar and gets no link.
///
/// Only the pack scope is scanned, because that is the only scope a file
/// include is legal in — the loader drops one written anywhere else — and only
/// the two-word file form: `include from SOURCE into TARGET { … }` composes a
/// command surface out of packs already loaded and resolves no file at all.
fn pack_include_links(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    line_index: &LineIndex,
    script_path: Option<&str>,
) -> Vec<DocumentLink> {
    if registry
        .document_grammar()
        .is_none_or(|grammar| grammar.family != DefinerFamily::SpecTcl)
    {
        return Vec::new();
    }
    // The loader's anchor is `path.parent()` of the including pack; a document
    // with no path of its own has no such directory and so no include to link.
    let Some(dir) = script_path
        .map(std::path::Path::new)
        .and_then(std::path::Path::parent)
    else {
        return Vec::new();
    };
    let mut links = Vec::new();
    for pack in segment_commands_with_offset_and_config(source, 0, config) {
        // `speclib NAME VERSION { … }` — the pack body is the fourth word, and
        // the document grammar admits no other statement at the root.
        if pack.texts.first().is_none_or(|head| head != "speclib") {
            continue;
        }
        let Some(body) = pack.argv.get(3) else {
            continue;
        };
        let body_start = body.span.start() + u32::from(body.content_offset);
        let Some(body_text) = source
            .get(body_start as usize..)
            .and_then(|rest| rest.get(..body.span.end().saturating_sub(body_start) as usize))
        else {
            continue;
        };
        for row in segment_commands_with_offset_and_config(body_text, body_start, config) {
            if row.texts.first().is_none_or(|head| head != "include") || row.texts.len() != 2 {
                continue;
            }
            let name = &row.texts[1];
            if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
                continue;
            }
            let target = file_uri_for_path(&dir.join(name).to_string_lossy());
            let Some(tok) = row.argv.get(1) else { continue };
            let Some((anchor_start, anchor_end)) = link_anchor_with_config(source, config, tok)
            else {
                continue;
            };
            let start = line_index.position_at_utf16(anchor_start, source);
            let end = line_index.position_at_utf16(anchor_end, source);
            links.push(DocumentLink {
                start_line: start.line,
                start_character: start.character.get(),
                end_line: end.line,
                end_character: end.character.get(),
                target,
                tooltip: Some(name.clone()),
            });
        }
    }
    links
}

/// The byte range a `source` link should underline inside its path argument,
/// or `None` when no range can carry the link and the provider must emit none.
///
/// A literal path word *is* the link, so the whole word (past any opening
/// delimiter) is the range.  A computed one — `source [file join $dir x.tcl]`
/// — is not: it is a command sequence with highlighting of its own, and an
/// editor paints a link range in one flat link colour plus an underline.
/// Spanning the whole substitution therefore erases that highlighting —
/// `file`, `join`, `$currentDir`, and the
/// file name all collapsing into one link-coloured run — and it also asserts
/// something untrue, that the *code* is the link rather than the file it
/// names.
///
/// So a substitution anchors on its **last** word, and only when that word is
/// a plain literal: the file name, which is both the part a reader clicks and
/// the only part that survives evaluation as itself.  Every idiom this
/// provider resolves ends in one — `[file join $dir x.tcl]`, `[file join
/// [file dirname [info script]] util.tcl]`, `[file join lib helper.tcl]`.
///
/// Strictly the *last* word, never the last literal one: scanning backwards
/// for any literal makes `[file join $dir $name]` anchor on the subcommand
/// word `join`, underlining a piece of the call rather than a path.  A
/// substitution whose last word is not a literal yields `None` and so no link
/// at all, the same abstention the unresolvable paths take — a missing link
/// costs a click, a link over code costs the highlighting of the whole line.
///
/// A single-word substitution (`[pwd]`) has only its command name to offer
/// and so abstains too, hence the `1..` lower bound.
fn link_anchor_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
    arg: &Token,
) -> Option<(u32, u32)> {
    let content_start = arg.span.start() + u32::from(arg.content_offset);
    if arg.kind != TokenType::Cmd {
        return Some((content_start, arg.span.end()));
    }
    // A `Cmd` token's span runs from the `[` to the closing `]`, and
    // `content_offset` steps past the `[`, so this slice is exactly the
    // substitution's inner text — byte-identical to `source` at
    // `content_start`, which is what makes rebasing the inner spans truthful.
    let inner = source.get(content_start as usize..arg.span.end() as usize)?;
    let seg = segment_commands_with_offset_and_config(inner, content_start, config).pop()?;
    let last = seg.texts.len().checked_sub(1).filter(|i| *i >= 1)?;
    let text = seg.texts.get(last)?;
    if text.is_empty()
        || carries_substitution(text)
        || seg.single_token_word.get(last) != Some(&true)
    {
        return None;
    }
    let tok = seg.argv.get(last)?;
    Some((
        tok.span.start() + u32::from(tok.content_offset),
        tok.span.end(),
    ))
}

/// Resolve `path` against `workspace_root` (when provided).
///
/// Returns a `file://`-prefixed URI string.  Absolute paths
/// (starting with `/` on POSIX, drive letter on Windows) pass
/// through; relative paths are joined to `workspace_root`.
/// When `workspace_root` is `None`, relative paths return
/// `None` (no anchor to resolve against).
fn resolve_path(path: &str, workspace_root: Option<&str>, home: Option<&str>) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    // Tilde expansion: `~/...` → `$HOME/...`; `~user/...` is
    // not supported (would need /etc/passwd parsing).
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        let home = home?;
        let home_trimmed = home.trim_end_matches('/');
        format!("{home_trimmed}/{rest}")
    } else if path == "~" {
        home?.to_string()
    } else {
        path.to_string()
    };
    let resolved = if std::path::Path::new(&expanded).is_absolute() {
        expanded
    } else {
        let root = workspace_root?;
        let root_trimmed = root.trim_end_matches('/');
        format!("{root_trimmed}/{expanded}")
    };
    Some(file_uri_for_path(&resolved))
}

/// Build a `file://` URI for a resolved filesystem path.
///
/// Normalises Windows backslash separators to forward slashes
/// and ensures the URI's path component starts with `/` so
/// drive-letter paths (`C:/foo`) become `file:///C:/foo`
/// rather than `file://C:/foo`.
/// The path bytes are percent-encoded per RFC 3986 so special
/// characters (spaces, `#`, `%`, non-ASCII) survive parsing
/// as a URI.
fn file_uri_for_path(path: &str) -> String {
    let normalised: String = path
        .chars()
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    let encoded = percent_encode_path(&normalised);
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

/// Percent-encode the path component of a `file://` URI per
/// RFC 3986.  Characters that don't form valid path-segment
/// bytes (space, `#`, `?`, `%`, `[`, `]`, non-ASCII, …) get
/// `%HH`-escaped so editors can parse the link target as a URI.
///
/// Per RFC 3986 the path-component reserves:
///
/// * unreserved: `A-Z` / `a-z` / `0-9` / `-` / `.` / `_` / `~`
/// * sub-delims: `!` / `$` / `&` / `'` / `(` / `)` / `*` / `+`
///   / `,` / `;` / `=`
/// * pchar adds `:` / `@`, and the path layer adds `/`.
///
/// Everything else is encoded.  Keeps the encoded path
/// conservative enough to round-trip through a `file://` URI
/// parser.
fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for &b in path.as_bytes() {
        if is_path_safe_byte(b) {
            out.push(b as char);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

const fn is_path_safe_byte(b: u8) -> bool {
    matches!(
        b,
        b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b':'
            | b'@'
            | b'/'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_links_for_non_source_commands() {
        assert!(
            document_links(
                "set x 1\n",
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                None
            )
            .is_empty()
        );
        assert!(
            document_links(
                "puts hello\n",
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                None
            )
            .is_empty()
        );
    }

    /// Links for a pack at `/home/user/project/.tcl-lsp/mylib.tclspec`, given
    /// the full context the server supplies — the include anchor is the
    /// document's own path, so `document_links` alone cannot express one.
    ///
    /// `workspace_root` and `home` are populated deliberately: an include that
    /// reached for either would resolve to a *different* file, so the tests
    /// below can tell the loader's rule from the `source` resolver's.
    fn pack_links(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> Vec<DocumentLink> {
        document_links_in_context(
            src,
            dialect,
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/home/user/project/.tcl-lsp"),
                home: Some("/home/user"),
                script_path: Some("/home/user/project/.tcl-lsp/mylib.tclspec"),
            },
        )
    }

    /// A pack's `include NAME` row links to the sibling file the loader would
    /// read, anchored on the name alone.
    #[test]
    fn a_pack_include_row_links_to_its_sibling_file() {
        let src = "speclib mylib 1 {\n    include shared.tclspec\n}\n";
        let links = pack_links(src, crate::profile_for_dialect("spectcl"));
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(
            links[0].target,
            "file:///home/user/project/.tcl-lsp/shared.tclspec"
        );
        assert_eq!(links[0].start_line, 1);
        assert_eq!(links[0].tooltip.as_deref(), Some("shared.tclspec"));
    }

    /// The link goes where the loader reads, and the loader does not expand
    /// `~`: `include ~` makes it read a sibling literally named `~`, so that
    /// is the file the link has to name — not the user's home directory.
    #[test]
    fn a_pack_include_does_not_expand_a_tilde() {
        let links = pack_links(
            "speclib mylib 1 {\n    include ~\n}\n",
            crate::profile_for_dialect("spectcl"),
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///home/user/project/.tcl-lsp/~");
    }

    /// A name carrying path structure is dropped by `loader::include_name`
    /// before the resolver sees it, so it names no file to navigate to.
    #[test]
    fn a_pack_include_with_path_structure_is_not_a_link() {
        for row in ["include ../outside.tclspec", "include sub/shared.tclspec"] {
            let src = format!("speclib mylib 1 {{\n    {row}\n}}\n");
            assert!(
                pack_links(&src, crate::profile_for_dialect("spectcl")).is_empty(),
                "{row}"
            );
        }
    }

    /// `include from … into …` composes a surface out of loaded packs and
    /// names no file, so it must not be offered as one.
    #[test]
    fn a_surface_roster_include_is_not_a_file_link() {
        let src = "speclib mylib 1 {\n    include from vendor into mylib { a b }\n}\n";
        assert!(pack_links(src, crate::profile_for_dialect("spectcl")).is_empty());
    }

    /// The gate is the document's own grammar: an ordinary Tcl script calling
    /// a proc of its own named `include` is not a pack.
    #[test]
    fn an_ordinary_tcl_include_call_is_not_a_link() {
        let src = "speclib mylib 1 {\n    include shared.tclspec\n}\n";
        assert!(
            pack_links(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            )
            .is_empty()
        );
    }

    #[test]
    fn absolute_path_surfaces_as_link() {
        let src = "source /usr/lib/tcl/init.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///usr/lib/tcl/init.tcl");
    }

    fn path_analysis(
        source: &str,
        profile: &'static tcl_dialect::DialectProfile,
        config: tcl_lexer::LexerConfig,
    ) -> tcl_compiler::analyser::AnalysisResult {
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            config,
        );
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn original_namespace_source_links_use_typed_jim_local_scope_without_export() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let source = "namespace eval N {set dir /FIRST; source $dir/a.tcl}; namespace eval N {source $dir/b.tcl}; source $dir/c.tcl";
        let profile = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
        assert_eq!(
            tcl_registry::InvocationDialect::of_profile(profile)
                .authored_name_policy()
                .unwrap()
                .recipe(),
            tcl_syntax::naming::NativeNameProtocol::Jim084
        );
        let analysis = path_analysis(
            source,
            profile,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let context = LinkContext {
            workspace_root: Some("/workspace"),
            ..LinkContext::default()
        };
        // Public Native links need current values independently of source advice.
        assert!(document_links_from_analysis(source, &analysis, &context).is_empty());
        let inventory = tcl_compiler::auto_path_eval::constant_path_assignments_from_analysis(
            source, &analysis,
        )
        .unwrap();
        let constants = tcl_compiler::auto_path_eval::fold_constant_assignments(&inventory, None);
        assert!(constants.exported().is_empty());
        // This direct helper is explicit source simulation, not public Native lookup.
        let links = lexical_document_links_in_context(source, &analysis, &context);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///FIRST/a.tcl");
        let unknown = tcl_dialect::DialectProfile::projected_from_point(
            "source-path-jim079",
            &[],
            "Unknown Jim source path recipe",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        let unavailable = path_analysis(
            source,
            unknown,
            tcl_lexer::LexerConfig::for_profile(Some(unknown)),
        );
        assert!(
            tcl_compiler::auto_path_eval::constant_path_assignments_from_analysis(
                source,
                &unavailable
            )
            .is_none()
        );
        assert!(lexical_document_links_in_context(source, &unavailable, &context).is_empty());
        assert!(document_links_from_analysis(source, &unavailable, &context).is_empty());
    }

    #[test]
    fn original_source_links_use_actual_selected_navigation_and_decline_shadows_or_stale_input() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let context = LinkContext {
            workspace_root: Some("/workspace"),
            ..LinkContext::default()
        };
        for source in [
            "::source /tmp/é.tcl",
            "rename source load; load /tmp/é.tcl",
            "interp alias {} load {} source; load /tmp/é.tcl",
        ] {
            let analysis = path_analysis(source, profile, config);
            let links = lexical_document_links_in_context(source, &analysis, &context);
            assert_eq!(links.len(), 1, "{source}: {links:?}");
            assert_eq!(links[0].target, "file:///tmp/%C3%A9.tcl");
            assert_eq!(links[0].tooltip.as_deref(), Some("/tmp/é.tcl"));
            assert!(
                lexical_document_links_in_context(&source.replace("é", "x"), &analysis, &context)
                    .is_empty()
            );
            let mut stale = analysis.clone();
            stale.body_lexer_config = Some(tcl_lexer::LexerConfig {
                braced_var: tcl_dialect::BracedVarStyle::FirstClose,
                ..config
            });
            assert!(lexical_document_links_in_context(source, &stale, &context).is_empty());
        }
        for source in [
            "proc source {path} {}; source /tmp/a.tcl",
            "proc file {args} {return /BAD}; set d [file join /tmp sub]; source $d/a.tcl",
            "interp alias {} load {} source /tmp/fixed.tcl; load",
        ] {
            let analysis = path_analysis(source, profile, config);
            assert!(
                lexical_document_links_in_context(source, &analysis, &context).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn relative_path_resolves_against_workspace_root() {
        let src = "source helper.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some("/home/user/project"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///home/user/project/helper.tcl");
    }

    #[test]
    fn relative_path_without_root_produces_no_link() {
        let src = "source helper.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn encoding_flag_skipped_before_path() {
        let src = "source -encoding utf-8 /tmp/foo.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///tmp/foo.tcl");
    }

    #[test]
    fn link_range_anchors_at_path_argument() {
        // `source ` is 7 chars; the path starts at col 7.
        let src = "source /tmp/foo.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].start_character, 7);
        // End col covers the path (12 chars: `/tmp/foo.tcl`).
        assert_eq!(links[0].end_character, 19);
    }

    #[test]
    fn braced_and_quoted_path_range_excludes_delimiter() {
        // `source {/tmp/foo.tcl}` must underline the path, not
        // the opening `{` — the range starts at the content, past the delimiter.
        for (src, want_start) in [
            ("source {/tmp/foo.tcl}\n", 8u32), // `source ` = 7, then `{` at 7, content at 8
            ("source \"/tmp/foo.tcl\"\n", 8u32),
        ] {
            let links = document_links(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                None,
            );
            assert_eq!(links.len(), 1, "{src:?} → {links:?}");
            assert_eq!(links[0].target, "file:///tmp/foo.tcl");
            assert_eq!(
                links[0].start_character, want_start,
                "range must start at the path content for {src:?}",
            );
        }
    }

    #[test]
    fn dynamic_path_produces_no_link() {
        // `source $somevar` — variable substitution, not a
        // literal path.  Skipped.
        let src = "source $somevar\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert!(links.is_empty(), "{links:?}");
    }

    // Computed `source` paths, exercised through
    // the code path a real editor takes: `workspace_root = Some(...)` *and*
    // `script_path = Some(...)`.  The `dynamic_path_produces_no
    // _link` test above passes `None` for both, which short-circuits
    // `resolve_path` on `let root = workspace_root?` before the
    // `carries_substitution`
    // gate is reached at all — so it never covered the bug.

    /// TP — the positive side nothing pinned before: the corpus idiom
    /// `set dir [file dirname [info script]]; source [file join $dir x.tcl]`
    /// resolves to the **correct** target, not merely to no bogus one.
    ///
    /// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/test/caller.tcl` loads
    /// `/proj/test/helper.tcl`, since `[info script]` is the sourcing file.
    #[test]
    fn a_computed_source_path_resolves_to_its_real_target_923_idx41() {
        let src = "set dir [file dirname [info script]]\nsource [file join $dir helper.tcl]\n";
        let links = document_links_in_context(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/proj"),
                home: None,
                script_path: Some("/proj/test/caller.tcl"),
            },
        );
        assert_eq!(links.len(), 1, "expected exactly one link: {links:?}");
        assert_eq!(
            links[0].target, "file:///proj/test/helper.tcl",
            "the computed path must resolve against `[info script]`'s own \
             directory: {links:?}",
        );
    }

    /// TN — a shape the evaluator cannot fold, so `dir` cannot be
    /// constant-propagated.  The whole expression must abstain rather than
    /// fall through to percent-encoding its own raw text into a `file://`
    /// URI (`file:///proj/%5Bfile%20join%20$dir%20helper.tcl%5D`).
    ///
    /// `file readlink` is the unmodellable command here: the subset does not
    /// model it, and cannot without touching the filesystem.  A `file
    /// normalize` wrapper does fold, and
    /// `a_normalized_computed_source_path_resolves_775` below pins its
    /// target.
    #[test]
    fn an_unfoldable_computed_source_path_produces_no_link_923_idx41() {
        for src in [
            "set dir [file readlink [file dirname [info script]]]\n\
             source [file join $dir helper.tcl]\n",
            "source [file join $undeclared .. pkgfile.tcl]\n",
            "source $other\n",
        ] {
            let links = document_links_in_context(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                &LinkContext {
                    imported_constants: None,
                    workspace_root: Some("/proj"),
                    home: None,
                    script_path: Some("/proj/test/caller.tcl"),
                },
            );
            assert!(
                links.is_empty(),
                "an unresolvable computed path must produce no link at all, \
                 got {links:?} for {src:?}",
            );
        }
    }

    /// A computed path that folds to a *relative* result is anchored by this
    /// provider on `workspace_root`, exactly as the literal path beside it —
    /// never on the language server process's own working directory.
    ///
    /// Tcl resolves such a path against the interpreter's run-time working
    /// directory (tclsh 8.6.16 / 9.0.4: `cd other && tclsh ../sub/main.tcl`
    /// with `set v helper.tcl; source $v` loads `other/helper.tcl`), which is
    /// unknowable statically; what must never happen is the two spellings
    /// disagreeing, or the answer depending on how the editor was launched.
    ///
    /// The computed spelling is `[file join $dir helper.tcl]` rather than
    /// `[file join $v]` (with `set v helper.tcl`) because a substitution must
    /// end in a literal word to have anything to anchor its link on, and
    /// `[file join $v]` ends in a variable.  The claim under test — a
    /// *relative* computed result anchoring on `workspace_root` like the
    /// literal beside it — is unaffected, since `dir` folds to `.`.
    #[test]
    fn a_relative_computed_source_path_anchors_like_the_literal_beside_it_923_idx41() {
        let ctx = LinkContext {
            imported_constants: None,
            workspace_root: Some("/proj"),
            home: None,
            script_path: Some("/proj/test/caller.tcl"),
        };
        let literal = document_links_in_context(
            "source helper.tcl\n",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &ctx,
        );
        let computed = document_links_in_context(
            "set dir .\nsource [file join $dir helper.tcl]\n",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &ctx,
        );
        assert_eq!(literal.len(), 1, "{literal:?}");
        assert_eq!(computed.len(), 1, "{computed:?}");
        assert_eq!(
            computed[0].target, literal[0].target,
            "a computed relative path must anchor exactly like the literal one",
        );
        assert_eq!(literal[0].target, "file:///proj/helper.tcl");
    }

    // A `source` link must never span the code inside a
    // command substitution.  The reporter's file is
    // georgtree/SpiceGenTcl's `test/arbitaryTest.tcl` shape: `set
    // currentDir [file … [info script]]` then `source [file join
    // $currentDir <name>.tcl]`.  The argument's own semantic tokens were
    // always correct (that is what the earlier fix pinned, in
    // `tests/e2e/semantic_tokens.rs`); what the reporter actually sees is
    // this link, painted flat over `file join $currentDir <name>.tcl`.

    /// The link on a computed path anchors on the file name alone, leaving
    /// the rest of the substitution — `file`, `join`, `$currentDir` — free to
    /// carry its own highlighting.
    #[test]
    fn a_computed_source_link_anchors_on_the_file_name_only_775() {
        let src = "set currentDir [file dirname [info script]]\n\
                   source [file join $currentDir esd_pulse_circuit.tcl]\n";
        let links = document_links_in_context(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/proj/test"),
                home: None,
                script_path: Some("/proj/test/main.tcl"),
            },
        );
        assert_eq!(links.len(), 1, "{links:?}");
        let line = src.lines().nth(1).expect("the source line");
        let covered = &line[links[0].start_character as usize..links[0].end_character as usize];
        assert_eq!(
            covered, "esd_pulse_circuit.tcl",
            "the link must cover the file name, not the substitution: {links:?}",
        );
        // Navigation is kept, not traded away for the highlighting.
        assert_eq!(links[0].target, "file:///proj/test/esd_pulse_circuit.tcl");
    }

    /// The reporter's file, verbatim in shape: georgtree/SpiceGenTcl's
    /// `test/arbitaryTest.tcl` wraps its directory in `file normalize`, which
    /// the evaluator now folds, so the link both appears and stays off the
    /// code.  Every `source` in that project is spelled this way.
    ///
    /// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/test/arbitaryTest.tcl`
    /// loads `/proj/test/testUtilities.tcl`.
    #[test]
    fn a_normalized_computed_source_path_resolves_775() {
        let src = "set currentDir [file normalize [file dirname [info script]]]\n\
                   source [file join $currentDir testUtilities.tcl]\n";
        let links = document_links_in_context(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/proj"),
                home: None,
                script_path: Some("/proj/test/arbitaryTest.tcl"),
            },
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///proj/test/testUtilities.tcl");
        let line = src.lines().nth(1).expect("the source line");
        assert_eq!(
            &line[links[0].start_character as usize..links[0].end_character as usize],
            "testUtilities.tcl",
            "{links:?}",
        );
    }

    /// A directory reached through an intermediate resolves like a direct
    /// one — georgtree/SpiceGenTcl's own `SpiceGenTcl.tcl` shape, where every
    /// one of seventeen `source` lines goes through `$sourceDir`.
    ///
    /// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/SpiceGenTcl.tcl` loads
    /// `/proj/src/generalClasses.tcl`.
    #[test]
    fn a_chained_computed_source_path_resolves_775() {
        let src = "set dir [file dirname [file normalize [info script]]]\n\
                   set sourceDir [file join $dir src]\n\
                   source [file join $sourceDir generalClasses.tcl]\n\
                   source [file join $sourceDir ngspice netlistParserClassNgspice.tcl]\n";
        let links = document_links_in_context(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/proj"),
                home: None,
                script_path: Some("/proj/SpiceGenTcl.tcl"),
            },
        );
        let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(
            targets,
            vec![
                "file:///proj/src/generalClasses.tcl",
                "file:///proj/src/ngspice/netlistParserClassNgspice.tcl",
            ],
            "{links:?}",
        );
        // Still anchored on the file name, chained or not.
        let line = src.lines().nth(2).expect("the first source line");
        assert_eq!(
            &line[links[0].start_character as usize..links[0].end_character as usize],
            "generalClasses.tcl",
        );
    }

    /// The same narrowing for a fully literal `[file join …]`: the joined
    /// path's *name* is the link, not the `file join` call that builds it.
    #[test]
    fn a_literal_file_join_link_anchors_on_the_file_name_only_775() {
        let src = "source [file join lib helper.tcl]\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some("/proj"),
        );
        assert_eq!(links.len(), 1, "{links:?}");
        let covered = &src[links[0].start_character as usize..links[0].end_character as usize];
        assert_eq!(covered, "helper.tcl", "{links:?}");
        assert_eq!(links[0].target, "file:///proj/lib/helper.tcl");
    }

    /// A substitution with no literal word to anchor on emits no link at all,
    /// rather than falling back to a range that spans code.
    #[test]
    fn a_computed_source_path_with_no_literal_word_produces_no_link_775() {
        let src = "set dir [file dirname [info script]]\n\
                   set name helper.tcl\n\
                   source [file join $dir $name]\n";
        let links = document_links_in_context(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/proj"),
                home: None,
                script_path: Some("/proj/main.tcl"),
            },
        );
        assert!(
            links.is_empty(),
            "no anchor means no link, never a link over code: {links:?}",
        );
    }

    /// A plain literal path is unaffected by the narrowing — the whole word
    /// stays the link, since it is the file name.
    #[test]
    fn a_literal_source_link_still_covers_the_whole_word_775() {
        let src = "source lib/helper.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some("/proj"),
        );
        assert_eq!(links.len(), 1, "{links:?}");
        let covered = &src[links[0].start_character as usize..links[0].end_character as usize];
        assert_eq!(covered, "lib/helper.tcl", "{links:?}");
    }

    #[test]
    fn double_dash_terminator_skipped() {
        let src = "source -- /tmp/x.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///tmp/x.tcl");
    }

    #[test]
    fn trailing_slash_on_workspace_root_handled() {
        let src = "source helper.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some("/home/user/"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///home/user/helper.tcl");
    }

    #[test]
    fn tilde_expansion_uses_supplied_home() {
        let src = "source ~/lib/init.tcl\n";
        let links = document_links_with_home(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
            Some("/test-home"),
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///test-home/lib/init.tcl");
    }

    #[test]
    fn tilde_without_home_produces_no_link() {
        let src = "source ~/lib/init.tcl\n";
        let links = document_links_with_home(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
            None,
        );
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        let src = "source ~\n";
        let links = document_links_with_home(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
            Some("/test-home"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///test-home");
    }

    // URI escaping

    #[test]
    fn percent_encode_path_passes_unreserved_through() {
        // Letters, digits, and the unreserved punctuation set
        // pass through unchanged, as does the path separator.
        assert_eq!(
            percent_encode_path("/usr/local/lib/init.tcl"),
            "/usr/local/lib/init.tcl",
        );
    }

    #[test]
    fn percent_encode_path_escapes_spaces_and_specials() {
        assert_eq!(
            percent_encode_path("/path/with spaces/file.tcl"),
            "/path/with%20spaces/file.tcl",
        );
        // `#` is a fragment delimiter in URIs.
        assert_eq!(
            percent_encode_path("/has/#hash/file.tcl"),
            "/has/%23hash/file.tcl",
        );
        // `%` is the escape introducer — it itself needs escaping.
        assert_eq!(
            percent_encode_path("/has/50%off/file.tcl"),
            "/has/50%25off/file.tcl",
        );
    }

    #[test]
    fn percent_encode_path_escapes_non_ascii() {
        // UTF-8 bytes for `é` are 0xC3 0xA9.
        assert_eq!(
            percent_encode_path("/caf\u{00E9}/x.tcl"),
            "/caf%C3%A9/x.tcl"
        );
    }

    #[test]
    fn file_uri_for_unix_absolute_path() {
        // `/foo/bar.tcl` → `file:///foo/bar.tcl` (one slash
        // from `file://` + the leading `/` of the path).
        assert_eq!(file_uri_for_path("/foo/bar.tcl"), "file:///foo/bar.tcl");
    }

    #[test]
    fn file_uri_for_windows_drive_letter_path() {
        // `C:/Users/me/file.tcl` → `file:///C:/Users/me/file.tcl`
        // — `file_uri_for_path` must insert the third slash
        // so the host component is empty and the path starts
        // at the drive letter.
        assert_eq!(
            file_uri_for_path("C:/Users/me/file.tcl"),
            "file:///C:/Users/me/file.tcl",
        );
    }

    #[test]
    fn file_uri_normalises_windows_backslashes() {
        assert_eq!(
            file_uri_for_path(r"C:\Users\me\file.tcl"),
            "file:///C:/Users/me/file.tcl",
        );
    }

    #[test]
    fn source_with_spaces_in_path_produces_escaped_uri() {
        // End-to-end: a literal path arg containing spaces or
        // hashes surfaces as a properly percent-encoded
        // `file://` URI rather than a raw concatenation.
        let src = "source /path/with spaces.tcl\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        // The literal-path arg may or may not parse cleanly
        // through the segmenter; if it does, the URI must be
        // escaped.  If not, the test is informational only.
        if let Some(link) = links.first() {
            assert!(
                !link.target.contains(' '),
                "URI target must not contain raw spaces: {target}",
                target = link.target,
            );
        }
    }

    // literal `[file join …]`

    #[test]
    fn file_join_joins_literal_segments() {
        // `[file join lib core init.tcl]` → `lib/core/init.tcl`.
        assert_eq!(
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr(
                "[file join lib core init.tcl]",
                None
            ),
            Some("lib/core/init.tcl".to_owned()),
        );
    }

    #[test]
    fn file_join_handles_quoted_and_braced_segments() {
        assert_eq!(
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr(
                r#"[file join "lib" {core} init.tcl]"#,
                None
            ),
            Some("lib/core/init.tcl".to_owned()),
        );
    }

    #[test]
    fn file_join_absolute_segment_resets_accumulator() {
        // Per Tcl's `file join` semantics, an absolute path
        // resets the joined accumulator.
        assert_eq!(
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr(
                "[file join /etc /opt/foo bar]",
                None
            ),
            Some("/opt/foo/bar".to_owned()),
        );
    }

    #[test]
    fn file_join_returns_none_for_variable_segments() {
        assert!(
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[file join $dir foo]", None)
                .is_none()
        );
        assert!(
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[file join [pwd] foo]", None)
                .is_none()
        );
    }

    #[test]
    fn file_join_returns_none_for_non_file_join_subst() {
        assert!(tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[exec ls]", None).is_none());
        assert_eq!(
            tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[file dirname /foo]", None)
                .as_deref(),
            Some("/")
        );
    }

    #[test]
    fn source_with_literal_file_join_surfaces_link() {
        let src = "source [file join lib helper.tcl]\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some("/home/user/project"),
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///home/user/project/lib/helper.tcl",);
    }

    #[test]
    fn source_with_absolute_file_join_segment_surfaces_link() {
        let src = "source [file join /usr/local/lib tcl init.tcl]\n";
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "file:///usr/local/lib/tcl/init.tcl");
    }
}

#[cfg(test)]
mod original_document_link_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    // Implementation contract: naming.core.original-document-link-selection
    // docs/design/analysis/name-resolution-proofs/original-document-link-selection.md
    fn original_document_links_use_actual_handlers_values_and_current_source() {
        let source = "source {a $b [c].tcl}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let ctx = LinkContext {
            workspace_root: Some("/project"),
            ..LinkContext::default()
        };
        let links = document_links_from_analysis(source, &analysis, &ctx);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "file:///project/a%20$b%20%5Bc%5D.tcl");
        analysis.source_targets.clear();
        analysis.command_invocations.clear();
        analysis.dialect = "f5-irules".into();
        assert_eq!(document_links_from_analysis(source, &analysis, &ctx), links);
        assert!(document_links_from_analysis(&format!("#{source}"), &analysis, &ctx).is_empty());
        for source in [
            "proc source args {}; source {a.tcl}",
            "source [unknown]",
            "source {-encoding} utf-8 a.tcl extra",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                document_links_from_analysis(source, &analysis, &ctx).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    // Implementation contract: naming.core.original-document-link-selection
    // docs/design/analysis/name-resolution-proofs/original-document-link-selection.md
    fn original_document_links_retain_package_inputs_and_versioned_source_grammar() {
        let ctx = LinkContext {
            workspace_root: Some("/project"),
            ..LinkContext::default()
        };
        for (dialect, source, expected) in [
            ("tcl8.4", "source -encoding utf-8 a.tcl", 0),
            ("tcl8.6", "source -encoding utf-8 a.tcl", 1),
            ("tcl9.0", "source -nopkg a.tcl", 1),
        ] {
            let analysis = Analyser::new().analyse(source, dialect);
            assert_eq!(
                document_links_from_analysis(source, &analysis, &ctx).len(),
                expected,
                "{dialect}"
            );
        }
        let source = "package require {p$[x]}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(analysis.package_requires.len(), 1);
        analysis.package_requires[0].name = "counterfactual".into();
        let links = document_links_from_analysis(source, &analysis, &ctx);
        assert_eq!(links.len(), 1);
        assert!(links[0].tooltip.as_ref().unwrap().contains("p$[x]"));
        assert!(document_links_from_analysis("package require other", &analysis, &ctx).is_empty());
    }
}

#[cfg(test)]
mod original_hosted_document_link_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_hosted_links_keep_source_schema_and_name_purposes_independent() {
        // Implementation contract: naming.vendor.original-source-navigation
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-navigation.md
        let context = LinkContext {
            workspace_root: Some("/work"),
            ..LinkContext::default()
        };
        for dialect in ["f5-iapps", "f5-tmsh"] {
            let source = "source {a $b [c].tcl}\npackage require -exact {pkg $[x]} 1.0";
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.source_targets.clear();
            analysis.package_requires.clear();
            let links = document_links_from_analysis(source, &analysis, &context);
            assert_eq!(links.len(), 2, "{dialect}: {links:?}");
            assert!(
                links
                    .iter()
                    .any(|link| link.target == "file:///work/a%20$b%20%5Bc%5D.tcl")
            );
            assert!(
                links.iter().any(|link| link.tooltip.as_deref()
                    == Some("Package requirement candidate: pkg $[x]"))
            );
            assert!(
                document_links_from_analysis(&source.replace("a $b", "z $b"), &analysis, &context)
                    .is_empty()
            );
            for source in [
                "source {f\\uD800.tcl}",
                "source $path",
                "source -encoding utf-8 a.tcl",
                "proc source args {}; source a.tcl",
                "rename source {}; source a.tcl",
                "proc package args {}; package require P",
            ] {
                let analysis = Analyser::new().analyse(source, dialect);
                assert!(
                    document_links_from_analysis(source, &analysis, &context).is_empty(),
                    "{dialect}: {source}"
                );
            }
        }
        let source = "source a.tcl\npackage require P";
        let analysis = Analyser::new().analyse(source, "f5-irules");
        assert!(document_links_from_analysis(source, &analysis, &context).is_empty());
    }
    #[test]
    fn original_source_path_links_use_selected_operations_and_authentic_bound_values() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let context = LinkContext {
            workspace_root: Some("/workspace"),
            ..LinkContext::default()
        };
        let source =
            "interp alias {} build {} file join {/ROOT with space}; source [build {literal$é.tcl}]";
        let analysis = path_analysis(source, profile, config);
        let links = lexical_document_links_in_context(source, &analysis, &context);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(
            links[0].target,
            file_uri_for_path("/ROOT with space/literal$é.tcl")
        );
        for source in [
            "rename file moved; interp alias {} file {} list; source [file join /ROOT leaf]",
            "proc file {args} {}; source [file join /ROOT leaf]",
        ] {
            let analysis = path_analysis(source, profile, config);
            assert!(lexical_document_links_in_context(source, &analysis, &context).is_empty());
        }
    }
}
