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

//! Signature-help provider.
//!
//! Surfaces a single [`SignatureInformation`] for the command
//! whose name appears as the first word of the active command
//! segment at the cursor. The active parameter is derived from a
//! lexer-aware count of arguments in the innermost registry-declared
//! executable region containing the cursor.
//!
//! Native lookup uses the retained complete source, full configuration and
//! Registry, then selects original procedure allocations or retains the same
//! source schema and its conditional availability. Captured alias prefixes
//! occupy arguments without source anchors;
//! expanded arguments retain their own list-child extents. Reporting names
//! render labels without supplying lookup identity. Explicit lexical advice
//! keeps its independent source declaration lookup.
//!
//! Braced data remains part of its containing command, while registry-declared
//! script bodies, clause actions, lambdas, definition members, and live
//! command substitutions establish nested command contexts. Rich doc-comment
//! rendering is not done — the summary is surfaced verbatim.

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_registry::CommandRegistry;

/// One element in a signature's parameter list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInformation {
    /// Parameter label as shown in the signature
    /// (e.g. `name` or `{count 1}`).
    pub label: String,
}

/// One signature in a signature-help response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    /// Full label of the signature (e.g. `proc ::greet name`).
    pub label: String,
    /// Parameter list in declaration order.
    pub parameters: Vec<ParameterInformation>,
    /// Optional documentation body (markdown).  `None` when no
    /// doc-comment was harvested.
    pub documentation: Option<String>,
}

/// LSP signature-help response — one or more signatures plus
/// the index of the active one and the active parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// Signatures to surface to the editor.
    pub signatures: Vec<SignatureInformation>,
    /// Index of the active signature (0-based).
    pub active_signature: u32,
    /// Index of the active parameter (0-based) within the
    /// active signature.
    pub active_parameter: u32,
}

/// Per-request controls for signature-help rendering.
#[derive(Debug, Clone, Copy, Default)]
pub struct SignatureHelpOptions<'a> {
    /// Canonical qualified registry command names whose built-in signature
    /// should not be shown (for example `::set`). Canonicalisation belongs to
    /// [`tcl_syntax::naming::normalise_qualified_name`].
    /// User-defined procs keep their signatures even when they have the same
    /// written name, because Tcl resolves those before the registry fallback.
    pub disabled_builtin_commands: &'a [String],
}

/// Compute signature help for the command being typed at the
/// cursor.
///
/// Returns `None` when the cursor isn't inside a recognisable
/// command-argument position or no matching user proc / built-in
/// was recorded.
///
/// `registry`, when `Some`, lets the lookup fall through to
/// the built-in command set: user
/// procs win, but if the cursor's command isn't a user proc,
/// the spec's first `hover.synopsis` entry renders as the
/// signature.  When `registry` is `None` the surface degrades
/// cleanly to the user-proc-only behaviour.
#[must_use]
pub fn signature_help(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Option<SignatureHelp> {
    signature_help_in_program(
        source,
        line,
        character,
        analysis,
        crate::definition::CallResolution {
            registry,
            program: None,
        },
    )
}

/// [`signature_help`] with the caller's whole-program export view attached —
/// the entry point a host with a workspace index should call.
///
/// The signature rendered is *the reached proc's*, so a `namespace import
/// -force` whose covering `namespace export` lives in another file changes
/// which parameter list is correct at this call.  Showing
/// the shadowed local proc's signature would mis-describe the call that
/// actually runs.
///
/// `resolution` carries the registry and the oracle together — the same pair
/// [`crate::definition::resolve_called_proc`] consumes.
#[must_use]
pub fn signature_help_in_program(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<SignatureHelp> {
    signature_help_in_program_with_options(
        source,
        line,
        character,
        analysis,
        resolution,
        SignatureHelpOptions::default(),
    )
}

/// [`signature_help_in_program`] with per-request rendering controls.
#[must_use]
pub fn signature_help_in_program_with_options(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    options: SignatureHelpOptions<'_>,
) -> Option<SignatureHelp> {
    let registry = resolution.registry;
    let structural_registry = analysis.resolved_registry()?;
    let context =
        command_context_with_args(source, line, character, analysis, structural_registry)?;
    let command = &context.command;
    let active_param = context.active_parameter;
    // Resolve the command from the namespace the cursor sits in, the way C
    // Tcl's own command resolution would (caller namespace, then global), so a
    // same-named proc in an unrelated namespace never hijacks the signature and
    // a same-named builtin keeps its synopsis unless a proc is actually visible.
    let cursor_off = {
        let line_index = tcl_lexer::LineIndex::new(source);
        crate::definition::byte_offset_at(&line_index, source, line, character)
    };
    let namespace = if analysis.allows_lexical_declaration_advice() {
        crate::definition::namespace_context_at(
            &analysis.global_scope,
            cursor_off,
            &analysis.namespace_overrides,
        )
    } else {
        String::new()
    };
    if let Some(proc_def) = lookup_proc(
        analysis,
        source,
        &namespace,
        command,
        context.head_offset,
        resolution,
    ) {
        return Some(proc_signature_help(proc_def, active_param));
    }
    registry?;
    source_signature_help(analysis, context.source_words.as_ref()?, &context, options)
}

/// Render the same original argv's source schema in its retained availability
/// generation. Conditional advice cannot establish Native target selection.
fn source_signature_help(
    analysis: &AnalysisResult,
    words: &crate::original_invocation::OriginalRegistryWords,
    command: &SignatureCommandContext,
    options: SignatureHelpOptions<'_>,
) -> Option<SignatureHelp> {
    // naming.consumer.original-signature-schema-rendering
    // docs/design/analysis/name-resolution-proofs/original-signature-schema-rendering.md
    let generation = analysis.resolved_input.as_ref()?.context_registry();
    let mut help = words
        .with_source_schema(&generation, |schema| {
            let spec = generation
                .context()
                .resolve_spec(generation.commands(), schema.canonical_command)?;
            if options
                .disabled_builtin_commands
                .contains(&tcl_syntax::naming::normalise_qualified_name(spec.name))
            {
                return None;
            }
            if !spec.subcommands.is_empty() && command.argument_count != 0 {
                let selected = schema.subcommand.resolved()?;
                let sub = generation
                    .context()
                    .available_subcommands(spec)
                    .into_iter()
                    .find(|sub| {
                        sub.name == selected.canonical_name
                            && sub.available_for_version(
                                schema.semantics.options.availability.package_version,
                            )
                    })?;
                return subcommand_signature_help(
                    &command.command,
                    sub,
                    command.active_parameter.saturating_sub(1),
                );
            }
            builtin_signature_help(spec, command.active_parameter)
        })
        .flatten()?;
    if !matches!(
        words.source,
        crate::original_invocation::OriginalRegistrySource::Selected
    ) {
        for signature in &mut help.signatures {
            let documentation = signature.documentation.get_or_insert_with(String::new);
            documentation
                .push_str("\n\n_Source documentation; command availability is unresolved._");
        }
    }
    Some(help)
}

/// Lexer-driven command-context detection.
///
/// Returns `(command_name, args, active_parameter_index)` for
/// the active command segment at `(line, character)`.  The
/// "active segment" is the run of words from the most recent
/// command boundary (start of source, `\n`, `;`, or
/// `{ … }`-body opener) up to the cursor.
///
/// Segmentation rules:
///
/// * Continuation lines (`\<newline>` and unclosed `{…}` /
///   `[…]` bodies) are part of the same segment.
/// * `;` resets the segment so multiple commands on one line
///   each have their own context.
/// * Comments are skipped.
///
/// `args` is the list of already-typed argument tokens
/// (everything after the command head) — used by
/// subcommand-aware signature help to dispatch on `args[0]`.
struct SignatureCommandContext {
    command: String,
    source_words: Option<crate::original_invocation::OriginalRegistryWords>,
    head_offset: u32,
    argument_count: usize,
    active_parameter: u32,
}

fn command_context_with_args(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Option<SignatureCommandContext> {
    use tcl_lexer::{Lexer, LineIndex, TokenType, Utf16Col};

    let cursor_offset = {
        let line_index = LineIndex::new(source);
        let line_count = u32::try_from(line_index.line_count()).unwrap_or(0);
        if line_count <= line {
            return None;
        }
        // `character` is an LSP UTF-16 column; resolve it to a byte offset
        // with `offset_at_utf16` (which snaps to a char boundary and clamps
        // to this line's content end, before its newline). Adding the column
        // to `line_start` directly is a byte+UTF-16 mismatch that selects the
        // wrong active parameter on any line with non-ASCII text before the
        // cursor — the same encoding hazard the rest of the crate fixed.
        line_index.offset_at_utf16(line, Utf16Col::new(character), source)
    };

    let profile = analysis.resolved_profile()?;
    let config = analysis.body_lexer_config?;
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return None;
    }
    let identities = analysis.retained_command_realm()?;
    let region = crate::executable_regions::innermost_analysis_executable_region_at(
        source,
        analysis,
        registry,
        cursor_offset as usize,
    )?;
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
        eprintln!(
            "SIGNATURE_HEADER cursor={} stage=region start={} end={} depth={}",
            cursor_offset, region.start, region.end, region.depth
        );
    }

    // Lex only the executable region's prefix up to the cursor. A braced word
    // that is ordinary data remains in the containing region and therefore in
    // the outer command. A registry-declared body starts a deeper region, so
    // newlines/comments inside it reset that body's command context instead of
    // leaving the outer command (notably `proc ... body`) active indefinitely.
    let prefix = &source[region.start..cursor_offset as usize];
    let local_config = config.at_depth(region.depth);
    let lexer = Lexer::with_config(prefix, local_config);
    let Ok(mut tokens) = lexer.tokenise_all() else {
        return None;
    };
    let groups = tcl_lexer::group_commands(&tokens, prefix, local_config);
    let group = groups.last()?;
    let last = tokens.iter().rev().find(|token| {
        !matches!(token.kind, TokenType::Eof | TokenType::Comment)
            && !(token.kind == TokenType::Eol && token.span.is_empty())
    })?;
    if last.kind == TokenType::Eol {
        return None;
    }
    let at_new_word = last.kind == TokenType::Sep;
    let arg_token_count = group.words.len().saturating_sub(1);
    let active_param = if at_new_word {
        u32::try_from(arg_token_count).ok()?
    } else {
        u32::try_from(arg_token_count.saturating_sub(1)).ok()?
    };
    if arg_token_count == 0 && !at_new_word {
        // Cursor still inside the command-name word itself (no argument typed
        // yet) — no signature.  When an argument *is* present, a cursor on it
        // is a real arg position even at index 0 (`greet World`).
        return None;
    }
    let base = u32::try_from(region.start).ok()?;
    for token in &mut tokens {
        token.span = tcl_lexer::Span::new(
            token.span.start().checked_add(base)?,
            token.span.end().checked_add(base)?,
        );
    }
    let image = tcl_lexer::SourceImage::document(source);
    let words: Vec<_> = group
        .words
        .iter()
        .map(|group| {
            let mut group = group.clone();
            group.span = tcl_lexer::Span::new(
                group.span.start().checked_add(base)?,
                group.span.end().checked_add(base)?,
            );
            tcl_lexer::NativeWord::from_group(image.clone(), local_config, &tokens, &group).ok()
        })
        .collect();
    let head = words.first()?.as_ref()?;
    let head_offset = head.tokens().first()?.span.start();
    let policy = tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy();
    let name_key = policy.and_then(|policy| {
        tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            head,
            tcl_syntax::word_rules::WordValueRules::from_config(&local_config),
            policy,
        )
    });
    let command = name_key
        .as_ref()
        .and_then(|key| key.display())
        .unwrap_or(head.try_text().ok()?)
        .to_owned();
    let mut source_words = None;
    let mut argument_count = arg_token_count;
    let mut active_parameter = active_param;
    {
        let segments = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source.get(region.start..region.end)?,
            base,
            local_config,
        );
        let segment = segments
            .iter()
            .find(|segment| segment.span.start() == head_offset)?;
        let mut original = tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            segment,
        );
        identities.stamp_original_tokens(&mut original);
        let effective = tcl_compiler::registry_invocation::effective_command_words(&original);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
            eprintln!(
                "SIGNATURE_HEADER cursor={} stage=command site={} written={} effective={} binding={}",
                cursor_offset,
                head_offset,
                segment.argv.len(),
                effective.is_some(),
                original.source_binding.is_some()
            );
        }
        if let Some(effective) = effective {
            let wanted = usize::try_from(active_param).ok()?.checked_add(1)?;
            active_parameter = crate::original_invocation::effective_argument_at(
                &original,
                &effective,
                wanted,
                cursor_offset,
            )
            .or_else(|| {
                (at_new_word && wanted == segment.argv.len())
                    .then(|| u32::try_from(effective.origins.len().saturating_sub(1)).ok())
                    .flatten()
            })?;
        }
        let selected = crate::original_invocation::source_registry_words(source, analysis, segment);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_INCOMPLETE_HEADER").is_some() {
            eprintln!(
                "SIGNATURE_HEADER cursor={} stage=selected site={} available={}",
                cursor_offset,
                head_offset,
                selected.is_some()
            );
        }
        if let Some(selected) = selected {
            let wanted = usize::try_from(active_param).ok()?.checked_add(1)?;
            active_parameter =
                selected
                    .active_argument_at(wanted, cursor_offset)
                    .or_else(|| {
                        (at_new_word && wanted == segment.argv.len())
                            .then(|| u32::try_from(selected.arguments.len()).ok())
                            .flatten()
                    })?;
            argument_count = selected.arguments.len();
            source_words = Some(selected);
        }
    }
    Some(SignatureCommandContext {
        command,
        source_words,
        head_offset,
        argument_count,
        active_parameter,
    })
}

/// Render signature help for a `command subcommand` form.
///
/// Uses the subcommand's `synopsis` as the signature label and
/// `detail` as the documentation.  Parameters are the
/// whitespace-separated tokens of the synopsis after the
/// leading command + subcommand pair.
fn subcommand_signature_help(
    command: &str,
    sub: &tcl_registry::SubCommand,
    active_param: u32,
) -> Option<SignatureHelp> {
    // The synopsis typically reads like `"string length string"`
    // — first token is the command, second is the subcommand
    // name, remaining tokens are parameters.
    let synopsis = sub.synopsis;
    let mut tokens = synopsis.split_whitespace();
    tokens.next()?; // command word
    tokens.next()?; // subcommand word
    let parameters: Vec<ParameterInformation> = tokens
        .map(|t| ParameterInformation {
            label: t.to_owned(),
        })
        .collect();

    let active_parameter = if parameters.is_empty() {
        0
    } else {
        let max_idx = u32::try_from(parameters.len() - 1).unwrap_or(0);
        active_param.min(max_idx)
    };

    let documentation = if sub.detail.is_empty() {
        None
    } else {
        Some(sub.detail.to_owned())
    };

    let label = if synopsis.is_empty() {
        format!("{command} {}", sub.name)
    } else {
        synopsis.to_owned()
    };

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            parameters,
            documentation,
        }],
        active_signature: 0,
        active_parameter,
    })
}

fn lookup_proc<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    namespace: &str,
    name: &str,
    call_off: u32,
    ctx: crate::definition::CallResolution<'_>,
) -> Option<&'a ProcDef> {
    if let Some(proc_def) =
        crate::definition::resolve_called_proc(analysis, source, namespace, name, call_off, ctx)
    {
        return Some(proc_def);
    }
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    // Alias resolution.  When the cursor's command isn't a visible proc, check
    // whether it matches an `interp alias {} ALIAS {} TARGET` record and follow
    // the chain, resolving the target the same namespace-aware way.
    let resolved_target = resolve_alias_chain(analysis, name, call_off)?;
    crate::definition::resolve_called_proc(analysis, source, "::", &resolved_target, call_off, ctx)
}

/// Original alias/rename targets use the shared ordered indirection receipt.
/// The terminal reported name is never passed back as written lookup input.
fn resolve_alias_chain(analysis: &AnalysisResult, name: &str, call_off: u32) -> Option<String> {
    let hop = tcl_compiler::analyser::indirection::walk(
        analysis,
        name,
        call_off,
        &tcl_syntax::naming::normalise_qualified_name,
    )?;
    hop.lookup_spelling().map(std::borrow::Cow::into_owned)
}

/// Render signature help for a built-in command spec.
///
/// Uses the first entry of `spec.hover.synopsis` as the
/// signature label.  Parameters are whitespace-separated
/// tokens after the leading command word in that synopsis.
/// Returns `None` when the spec has no hover record or the
/// synopsis is empty.
fn builtin_signature_help(
    spec: &tcl_registry::CommandSpec,
    active_param: u32,
) -> Option<SignatureHelp> {
    let hover = spec.hover.as_ref()?;
    let synopsis_line = *hover.synopsis.first()?;

    // The leading token of the synopsis is the command word
    // itself ("puts"); everything after it is a parameter
    // token (including bracketed optionals like
    // `?-nonewline?`).
    let mut tokens = synopsis_line.split_whitespace();
    tokens.next()?;
    let parameters: Vec<ParameterInformation> = tokens
        .map(|t| ParameterInformation {
            label: t.to_owned(),
        })
        .collect();

    let active_parameter = if parameters.is_empty() {
        0
    } else {
        let max_idx = u32::try_from(parameters.len() - 1).unwrap_or(0);
        active_param.min(max_idx)
    };

    // Signature documentation renders the fuller hover text (summary +
    // extended snippet) that hover itself omits, so e.g. `set`
    // surfaces its "With one argument…" description.
    let mut doc_parts: Vec<&str> = Vec::new();
    if !hover.summary.is_empty() {
        doc_parts.push(hover.summary);
    }
    if !hover.snippet.is_empty() {
        doc_parts.push(hover.snippet);
    }
    let documentation = (!doc_parts.is_empty()).then(|| doc_parts.join("\n\n"));

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: synopsis_line.to_owned(),
            parameters,
            documentation,
        }],
        active_signature: 0,
        active_parameter,
    })
}

fn proc_signature_help(proc_def: &ProcDef, active_param: u32) -> SignatureHelp {
    // Optional (defaulted) params show
    // as `?name?` in the parameter list and `?name default?` in the signature
    // label, which itself is `<short-name> <params>` (no `proc` prefix, short
    // — not qualified — name).
    let mut parameters: Vec<ParameterInformation> = Vec::with_capacity(proc_def.params.len());
    let mut label_parts: Vec<String> = Vec::with_capacity(proc_def.params.len());
    for p in &proc_def.params {
        if p.has_default {
            let default = p.default_value.as_deref().unwrap_or("");
            parameters.push(ParameterInformation {
                label: format!("?{}?", p.name),
            });
            label_parts.push(format!("?{} {}?", p.name, default));
        } else {
            parameters.push(ParameterInformation {
                label: p.name.clone(),
            });
            label_parts.push(p.name.clone());
        }
    }

    let label = if label_parts.is_empty() {
        proc_def.name.clone()
    } else {
        format!("{} {}", proc_def.name, label_parts.join(" "))
    };

    let active_parameter = if parameters.is_empty() {
        0
    } else {
        let max_idx = u32::try_from(parameters.len() - 1).unwrap_or(0);
        active_param.min(max_idx)
    };

    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            parameters,
            documentation: if proc_def.doc.is_empty() {
                None
            } else {
                Some(proc_def.doc.clone())
            },
        }],
        active_signature: 0,
        active_parameter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn signature_uses_the_original_call_point_before_replacement() {
        let source = "proc greet {first} {}\ngreet one\nrename greet archived\nproc greet {first second} {}\ngreet one two\narchived one\n";
        let analysis = analyse(source);
        for (line, column, parameters) in [(1, 9, 1), (4, 13, 2), (5, 12, 1)] {
            let help = signature_help(source, line, column, &analysis, None).unwrap();
            assert_eq!(help.signatures[0].parameters.len(), parameters);
        }
    }

    #[test]
    fn signature_subcommands_use_original_values_and_shared_prefix_rules() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = crate::profile_for_dialect(dialect);
            let registry = crate::registry_for_dialect_profile(profile);
            for source in [
                "string le value",
                r#""string" "le" value"#,
                r"\x73tring \u006ce value",
            ] {
                let mut analyser = Analyser::new();
                let analysis = analyser.analyse(source, dialect);
                let help = signature_help(
                    source,
                    0,
                    u32::try_from(source.len()).unwrap(),
                    &analysis,
                    Some(registry),
                )
                .unwrap();
                assert!(
                    help.signatures[0].label.contains("length"),
                    "{dialect} {source:?}"
                );
            }
            for source in [
                "string l value",
                "string $selector value",
                r"string le\u0000tail value",
                "string {le\0tail} value",
            ] {
                let mut analyser = Analyser::new();
                let analysis = analyser.analyse(source, dialect);
                assert!(
                    signature_help(
                        source,
                        0,
                        u32::try_from(source.len()).unwrap(),
                        &analysis,
                        Some(registry)
                    )
                    .is_none(),
                    "{dialect} {source:?}"
                );
            }
        }
    }

    #[test]
    fn original_signature_argument_mapping_keeps_captured_prefixes_without_source_anchors() {
        // Implementation contract: naming.consumer.original-signature-argument-mapping
        // docs/design/analysis/name-resolution-proofs/original-signature-argument-mapping.md
        // Rust consumer contract; runtime alias behavior is recorded separately.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let builtin = "interp alias {} strlen {} string length\nstrlen value";
            let analysis = Analyser::new().analyse(builtin, dialect);
            let help = signature_help(builtin, 1, 12, &analysis, analysis.resolved_registry())
                .expect(dialect);
            assert!(help.signatures[0].label.contains("length"));
            assert_eq!(help.active_parameter, 0);

            let procedure = "proc join {first second} {}\ninterp alias {} joined {} join captured\njoined value";
            let analysis = Analyser::new().analyse(procedure, dialect);
            let help = signature_help(procedure, 2, 12, &analysis, None).expect(dialect);
            assert_eq!(help.signatures[0].parameters.len(), 2);
            assert_eq!(help.active_parameter, 1);
        }
    }

    #[test]
    fn original_signature_expansion_cursor_selects_its_own_list_child() {
        // Implementation contract: naming.consumer.original-signature-argument-mapping
        // docs/design/analysis/name-resolution-proofs/original-signature-argument-mapping.md
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let builtin = "string equal {*}{first second}";
            let analysis = Analyser::new().analyse(builtin, dialect);
            for (word, expected) in [("first", 0), ("second", 1)] {
                let cursor = u32::try_from(builtin.find(word).unwrap() + 2).unwrap();
                let help =
                    signature_help(builtin, 0, cursor, &analysis, analysis.resolved_registry())
                        .expect(dialect);
                assert_eq!(help.active_parameter, expected, "{dialect}/{word}");
            }
            let procedure = "proc join {first second} {}\njoin {*}{alpha beta}";
            let analysis = Analyser::new().analyse(procedure, dialect);
            for (word, expected) in [("alpha", 0), ("beta", 1)] {
                let cursor = u32::try_from("join {*}{alpha beta}".find(word).unwrap() + 2).unwrap();
                let help = signature_help(procedure, 1, cursor, &analysis, None).expect(dialect);
                assert_eq!(help.active_parameter, expected, "{dialect}/{word}");
            }
        }
    }

    #[test]
    fn original_signature_currency_and_registry_survive_reporting_counterfactuals() {
        // Implementation contract: naming.consumer.original-signature-argument-mapping
        // docs/design/analysis/name-resolution-proofs/original-signature-argument-mapping.md
        let source = "string length value";
        let mut analysis = analyse(source);
        analysis.dialect = "f5-irules".to_owned();
        analysis.all_procs.clear();
        analysis.global_scope.variables.clear();
        let help = signature_help(source, 0, 19, &analysis, analysis.resolved_registry()).unwrap();
        assert!(help.signatures[0].label.contains("length"));
        assert!(
            signature_help(
                "string length other",
                0,
                19,
                &analysis,
                analysis.resolved_registry()
            )
            .is_none()
        );
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax = false;
        assert!(signature_help(source, 0, 19, &analysis, analysis.resolved_registry()).is_none());
    }

    #[test]
    fn no_help_at_command_name() {
        let src = "proc greet {name body} {}\ngre\n";
        let analysis = analyse(src);
        // Cursor mid-command name — should not surface signature help.
        assert!(signature_help(src, 1, 3, &analysis, None).is_none());
    }

    #[test]
    fn help_on_first_argument() {
        let src = "proc greet {name body} {}\ngreet \n";
        let analysis = analyse(src);
        // Cursor right after the trailing space following `greet`.
        let h = signature_help(src, 1, 6, &analysis, None).expect("signature help");
        assert_eq!(h.signatures.len(), 1);
        assert_eq!(h.signatures[0].parameters.len(), 2);
        assert_eq!(h.active_parameter, 0);
        assert!(h.signatures[0].label.contains("greet"));
    }

    #[test]
    fn active_param_advances_with_typed_args() {
        let src = "proc greet {name body} {}\ngreet alice \n";
        let analysis = analyse(src);
        // Cursor right after the second space (advances to second arg).
        let h = signature_help(src, 1, 12, &analysis, None).expect("signature help");
        assert_eq!(h.active_parameter, 1);
    }

    #[test]
    fn help_clamps_active_param_to_last_known() {
        let src = "proc one {a} {}\none alice extra \n";
        let analysis = analyse(src);
        // Cursor at position 16 — three arg tokens typed for a
        // 1-param proc; clamp to last known param.
        let h = signature_help(src, 1, 16, &analysis, None).expect("signature help");
        assert_eq!(h.active_parameter, 0, "{h:?}");
    }

    #[test]
    fn help_returns_none_for_unknown_command() {
        let src = "fakecmd arg \n";
        let analysis = analyse(src);
        assert!(signature_help(src, 0, 12, &analysis, None).is_none());
    }

    #[test]
    fn proc_doc_surfaces_as_documentation() {
        // The harvested doc-comment from the line above the
        // proc lands in `proc_def.doc`. Verify it surfaces.
        let src = "# greets the user\nproc greet {name} {}\ngreet \n";
        let analysis = analyse(src);
        let h = signature_help(src, 2, 6, &analysis, None).expect("signature help");
        // Doc may or may not be picked up depending on the
        // analyser's heuristics; if present, it should be
        // surfaced verbatim.
        if let Some(doc) = &h.signatures[0].documentation {
            assert!(doc.contains("greets") || !doc.is_empty(), "doc: {doc}");
        }
    }

    // built-in command signatures
    //
    // These tests pin the contract that passing a registry
    // lets the cursor's command resolve to a built-in spec
    // when no user proc matches, with the spec's first
    // `hover.synopsis` entry rendering as the signature.

    #[test]
    fn builtin_signature_surfaces_for_known_command() {
        // No user proc named `puts`, but the registry has the
        // spec.  Cursor in the argument list — signature help
        // should fire.
        let src = "puts \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let h = signature_help(src, 0, 5, &analysis, Some(&registry))
            .expect("expected built-in signature help");
        assert_eq!(h.signatures.len(), 1);
        // Synopsis starts with the command word.
        assert!(
            h.signatures[0].label.starts_with("puts"),
            "expected label to start with `puts`, got {label}",
            label = h.signatures[0].label,
        );
        // At least one parameter (puts takes ?-nonewline?
        // ?channelId? string).
        assert!(
            !h.signatures[0].parameters.is_empty(),
            "expected non-empty parameters for `puts`",
        );
    }

    #[test]
    fn builtin_signature_active_param_advances() {
        // `puts string`<cursor at last char> — active param
        // should be at the last parameter index for `puts`
        // (clamped if fewer parameters than typed args).
        let src = "puts arg1 arg2 \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let h = signature_help(src, 0, 15, &analysis, Some(&registry))
            .expect("expected built-in signature help");
        // Active parameter is clamped to the last known param;
        // exact value depends on the synopsis shape, but it
        // must be a valid index.
        let max_idx =
            u32::try_from(h.signatures[0].parameters.len() - 1).expect("param count fits u32");
        assert!(
            h.active_parameter <= max_idx,
            "active_parameter {} out of bounds (max {})",
            h.active_parameter,
            max_idx,
        );
    }

    #[test]
    fn builtin_signature_documentation_surfaces_summary() {
        // The `puts` spec carries a non-empty
        // `hover.summary`; the signature help should surface
        // it as `documentation`.
        let src = "puts \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let h = signature_help(src, 0, 5, &analysis, Some(&registry))
            .expect("expected built-in signature help");
        let doc = h.signatures[0]
            .documentation
            .as_ref()
            .expect("expected non-empty documentation for `puts`");
        assert!(!doc.is_empty(), "doc should be non-empty: {doc:?}");
    }

    #[test]
    fn user_proc_wins_over_builtin_with_same_name() {
        // If a user `proc puts {a b c} {}` exists, the user
        // proc's signature should take precedence over the
        // built-in.
        let src = "proc puts {custom_arg} {}\nputs \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let h = signature_help(src, 1, 5, &analysis, Some(&registry))
            .expect("expected user proc signature help");
        // User-proc label uses `proc ` prefix; built-in label
        // The user-proc label is `<name> <params>` (short name, no `proc`
        // prefix); the built-in synopsis would not carry `custom_arg`.
        assert!(
            h.signatures[0].label.starts_with("puts"),
            "expected user proc to win; got label {label}",
            label = h.signatures[0].label,
        );
        // Parameters must reflect the user proc, not `puts`'s
        // synopsis.
        assert!(
            h.signatures[0]
                .parameters
                .iter()
                .any(|p| p.label == "custom_arg"),
            "expected user param `custom_arg`; got {:?}",
            h.signatures[0].parameters,
        );
    }

    /// (line, character) just past the `occurrence`-th `needle`.
    fn pos_after(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
        let mut start = 0;
        for _ in 0..occurrence {
            let idx = src[start..].find(needle).expect("needle not found") + start;
            start = idx + needle.len();
        }
        let prefix = &src[..start];
        let line = u32::try_from(prefix.matches('\n').count()).unwrap();
        let col = u32::try_from(start - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
        (line, col)
    }

    #[test]
    fn qualified_call_resolves_to_namespaced_proc() {
        // A `A::greet` call resolves to the proc in `::A`, surfacing its
        // parameter list — the qualified name is honoured, not tail-matched to
        // an arbitrary same-named proc.
        let src = "namespace eval A {\n    proc greet {alpha} {}\n}\nA::greet \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let (l, c) = pos_after(src, "A::greet ", 1);
        let h = signature_help(src, l, c, &analysis, Some(&registry)).expect("signature help");
        assert!(
            h.signatures[0]
                .parameters
                .iter()
                .any(|p| p.label == "alpha"),
            "{:?}",
            h.signatures[0].parameters
        );
    }

    #[test]
    fn namespaced_proc_does_not_hijack_builtin_from_global() {
        // A `proc puts` buried in `::foo` must not shadow the builtin `puts`
        // when `puts` is called from the global namespace — C Tcl resolves the
        // builtin there, so the builtin synopsis (never the proc's `custom`
        // param) surfaces.
        let src = "namespace eval foo {\n    proc puts {custom} {}\n}\nputs \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let (l, c) = pos_after(src, "puts ", 2);
        let h = signature_help(src, l, c, &analysis, Some(&registry)).expect("signature help");
        assert!(
            !h.signatures[0]
                .parameters
                .iter()
                .any(|p| p.label == "custom"),
            "namespaced proc hijacked the builtin: {:?}",
            h.signatures[0].parameters
        );
    }

    #[test]
    fn bare_global_call_cannot_select_an_unrelated_namespace_proc() {
        let src = "namespace eval A {\n    proc greet {alpha} {}\n}\nnamespace eval B {\n    proc greet {beta} {}\n}\ngreet \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let (line, character) = pos_after(src, "greet ", 3);
        assert!(signature_help(src, line, character, &analysis, Some(&registry)).is_none());
    }

    #[test]
    fn bare_global_call_selects_the_genuine_namespace_path_proc() {
        let src = "namespace eval A {\n    proc greet {alpha} {}\n}\nnamespace eval B {\n    proc greet {beta} {}\n}\nnamespace path A\ngreet \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let (line, character) = pos_after(src, "greet ", 3);
        let help = signature_help(src, line, character, &analysis, Some(&registry))
            .expect("the actual namespace path selects A");
        assert!(
            help.signatures[0]
                .parameters
                .iter()
                .any(|parameter| parameter.label == "alpha")
        );
        assert!(
            !help.signatures[0]
                .parameters
                .iter()
                .any(|parameter| parameter.label == "beta")
        );
    }

    #[test]
    fn builtin_signature_returns_none_without_registry() {
        // Without a registry, unknown commands still return
        // `None` — preserves the minimal port's behaviour.
        let src = "puts \n";
        let analysis = analyse(src);
        assert!(signature_help(src, 0, 5, &analysis, None).is_none());
    }

    // multi-line / semicolon segments

    #[test]
    fn signature_help_continues_across_open_brace() {
        // The proc call is split across two physical lines via
        // an unclosed brace body — the active command segment
        // is `greet alice `, so signature help should be at
        // active param 1.
        let src = "proc greet {a b} {}\ngreet alice {\n  hello\n}\n";
        let analysis = analyse(src);
        // Cursor on line 2 just before `hello` — still inside
        // the braced body which is an argument to `greet`.
        let h = signature_help(src, 2, 0, &analysis, None);
        // The cursor on a fresh line at col 0 is still inside
        // the open brace body, so the command segment is `greet
        // alice {…`.  Active param should be at the third
        // position (index 2) since the open brace is treated as
        // the start of arg 1 and the cursor sits in its body
        // (which the segmenter rolls into the same word).
        assert!(h.is_some(), "expected signature help on continuation line");
    }

    #[test]
    fn proc_signature_stops_at_its_script_body() {
        // The caret at the end of either body line belongs to neither the
        // outer `proc` invocation: the body is a registry-declared nested Tcl
        // script.
        let src = concat!(
            "proc someproc {arg1 arg2} {\n",
            "    # this box always appears\n",
            "    # can be closed with ESC but comes back on the next SPACE press..\n",
            "    set evenWhenWritingCode\n",
            "}\n",
        );
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();

        for (needle, occurrence) in [
            ("# this box always appears", 1),
            (
                "# can be closed with ESC but comes back on the next SPACE press..",
                1,
            ),
        ] {
            let (line, character) = pos_after(src, needle, occurrence);
            assert!(
                signature_help(src, line, character, &analysis, Some(&registry)).is_none(),
                "a body comment must close the outer proc signature: {needle}",
            );
        }

        let (line, character) = pos_after(src, "set evenWhenWritingCode", 1);
        let help = signature_help(src, line, character, &analysis, Some(&registry))
            .expect("the nested set call has its own signature");
        assert!(
            help.signatures[0].label.starts_with("set "),
            "outer proc signature leaked into body: {help:?}",
        );
    }

    #[test]
    fn empty_registry_declared_scripts_are_nested_cursor_regions() {
        let registry = CommandRegistry::build_default();
        for (src, delimiter, occurrence) in [
            ("proc p {} {}", "{", 2),
            ("proc p {} \"\"", "\"", 1),
            ("switch value {default {}}", "{", 2),
        ] {
            let analysis = analyse(src);
            let (line, character) = pos_after(src, delimiter, occurrence);
            assert!(
                signature_help(src, line, character, &analysis, Some(&registry)).is_none(),
                "the containing command signature leaked into an empty script: {src}",
            );
        }
    }

    #[test]
    fn substitution_free_quoted_proc_body_is_a_nested_script_region() {
        // Implementation contract: naming.source.incomplete-body-header-metadata
        // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
        let src = "proc p {} \"puts hi\"";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let (line, character) = pos_after(src, "puts ", 1);
        let help = signature_help(src, line, character, &analysis, Some(&registry))
            .expect("quoted body command signature");
        assert!(
            help.signatures[0].label.starts_with("puts "),
            "outer proc signature leaked into quoted body: {help:?}",
        );

        let src = "proc p {} \"puts ";
        let analysis = analyse(src);
        let (line, character) = pos_after(src, "puts ", 1);
        let help = signature_help(src, line, character, &analysis, Some(&registry))
            .expect("unterminated quoted body command signature");
        assert!(
            help.signatures[0].label.starts_with("puts "),
            "outer proc signature leaked into unterminated quoted body: {help:?}",
        );
    }

    #[test]
    fn signature_help_is_available_in_proc_header_and_nested_proc_call() {
        let src = concat!(
            "proc helper {value} {}\n",
            "proc outer {arg} {\n",
            "    helper \n",
            "}\n",
        );
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();

        let (header_line, header_character) = pos_after(src, "proc outer ", 1);
        let header = signature_help(
            src,
            header_line,
            header_character,
            &analysis,
            Some(&registry),
        )
        .expect("proc declaration header signature");
        assert!(header.signatures[0].label.starts_with("proc "));

        let (call_line, call_character) = pos_after(src, "helper ", 2);
        let call = signature_help(src, call_line, call_character, &analysis, Some(&registry))
            .expect("nested proc call signature");
        assert_eq!(call.signatures[0].label, "helper value");
    }

    #[test]
    fn disabled_builtin_signatures_do_not_disable_other_commands_or_procs() {
        let registry = CommandRegistry::build_default();
        let disabled = vec!["::set".to_owned(), "::incr".to_owned()];
        // Each rendering control is tested at an independently retained call.
        // A preceding wrong-arity `set` supplies no normal source continuation
        // from which to infer later command identities.
        let help = |src, needle, occurrence| {
            let analysis = analyse(src);
            let (line, character) = pos_after(src, needle, occurrence);
            signature_help_in_program_with_options(
                src,
                line,
                character,
                &analysis,
                crate::definition::CallResolution {
                    registry: Some(&registry),
                    program: None,
                },
                SignatureHelpOptions {
                    disabled_builtin_commands: &disabled,
                },
            )
        };

        assert!(help("set \n", "set ", 1).is_none());
        assert!(
            help("format \n", "format ", 1)
                .is_some_and(|h| h.signatures[0].label.starts_with("format "))
        );
        assert!(
            help("proc custom {value} {}\ncustom \n", "custom ", 2)
                .is_some_and(|h| h.signatures[0].label == "custom value")
        );
    }

    #[test]
    fn signature_help_resets_on_semicolon() {
        // Two commands on one line separated by `;` — the
        // signature help at the second command's argument list
        // should reflect the second command, not the first.
        let src = "proc a {x} {}\nproc b {y} {}\na 1; b \n";
        let analysis = analyse(src);
        // Cursor at end of line 2 — past `b `.
        let h =
            signature_help(src, 2, 7, &analysis, None).expect("expected signature help for `b`");
        // The signature should be for `b`, not `a`.
        assert!(
            h.signatures[0].label.starts_with("b "),
            "expected label for `b`, got {label}",
            label = h.signatures[0].label,
        );
    }

    // `[greet …]` substitution-bracket recursion is not handled —
    // the lexer surfaces `[greet ]` as a single Cmd token, so
    // signature help for the inner command would need a recursive
    // lex of the bracket body.

    // alias resolution

    #[test]
    fn alias_resolves_to_target_proc_signature() {
        // `interp alias {} hello {} greet` makes `hello` an
        // alias for `greet`.  Signature help on `hello arg ` should
        // surface greet's signature.
        let src = concat!(
            "proc greet {name} {}\n",
            "interp alias {} hello {} greet\n",
            "hello \n",
        );
        let analysis = analyse(src);
        let h = signature_help(src, 2, 6, &analysis, None)
            .expect("expected alias-resolved signature help");
        assert_eq!(h.signatures.len(), 1);
        assert!(
            h.signatures[0].label.contains("greet"),
            "expected greet's signature via alias; got {label}",
            label = h.signatures[0].label,
        );
        assert!(
            h.signatures[0].parameters.iter().any(|p| p.label == "name"),
            "expected `name` parameter from greet; got {:?}",
            h.signatures[0].parameters,
        );
    }

    #[test]
    fn subcommand_signature_resolves_for_string_length() {
        // `string length $name` should surface the
        // `string length string` subcommand signature, not the
        // generic `string` synopsis.
        let src = "string length \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let h = signature_help(src, 0, 14, &analysis, Some(&registry))
            .expect("expected subcommand signature");
        // The synopsis label should contain `string length`.
        assert!(
            h.signatures[0].label.contains("string length"),
            "got label {label}",
            label = h.signatures[0].label,
        );
    }

    #[test]
    fn subcommand_signature_none_for_unknown_subcommand() {
        // `string nonsense $arg` — `nonsense` isn't a string subcommand, so
        // the provider offers no signature (it must not surface the generic
        // command-level `string option arg …`).
        let src = "string nonsense \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        assert!(
            signature_help(src, 0, 16, &analysis, Some(&registry)).is_none(),
            "unknown subcommand should yield no signature",
        );
    }

    #[test]
    fn alias_chain_returns_target_through_multiple_hops() {
        // `a` → `b` → `c` chain.  The analyser records two
        // alias entries; signature help on `a ` should follow
        // both hops to land on `c`'s signature.
        let src = concat!(
            "proc c {x} {}\n",
            "interp alias {} b {} c\n",
            "interp alias {} a {} b\n",
            "a \n",
        );
        let analysis = analyse(src);
        let h =
            signature_help(src, 3, 2, &analysis, None).expect("expected chained alias resolution");
        assert!(
            h.signatures[0].label.contains('c'),
            "expected target `c`; got {label}",
            label = h.signatures[0].label,
        );
    }
}

#[cfg(test)]
mod original_schema_tests {
    use super::*;
    use crate::original_invocation::OriginalRegistrySource;
    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

    fn help(source: &str, analysis: &AnalysisResult, cursor: usize) -> Option<SignatureHelp> {
        let index = tcl_lexer::LineIndex::new(source);
        let position = index.position_at_utf16(u32::try_from(cursor).ok()?, source);
        signature_help(
            source,
            position.line,
            position.character.get(),
            analysis,
            analysis.resolved_registry(),
        )
    }

    #[test]
    fn original_signature_keeps_conditional_source_schema_and_alias_prefixes() {
        // naming.consumer.original-signature-schema-rendering
        // docs/design/analysis/name-resolution-proofs/original-signature-schema-rendering.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let deferred = "package ifneeded P 1.0 {string length value}";
            let analysis = Analyser::new().analyse(deferred, dialect);
            let cursor = deferred.find("value").unwrap() + 3;
            let context = command_context_with_args(
                deferred,
                0,
                u32::try_from(cursor).unwrap(),
                &analysis,
                analysis.resolved_registry().unwrap(),
            )
            .unwrap();
            let words = context
                .source_words
                .as_ref()
                .expect("original source schema");
            assert!(matches!(
                words.source,
                OriginalRegistrySource::Conditional(_)
            ));
            let signature = help(deferred, &analysis, cursor).expect(dialect);
            assert!(signature.signatures[0].label.contains("length"));
            assert!(
                signature.signatures[0]
                    .documentation
                    .as_deref()
                    .unwrap()
                    .contains("availability is unresolved")
            );

            let alias = "unknown; interp alias {} strlen {} string length; strlen value";
            let analysis = Analyser::new().analyse(alias, dialect);
            let cursor = alias.len();
            let context = command_context_with_args(
                alias,
                0,
                u32::try_from(cursor).unwrap(),
                &analysis,
                analysis.resolved_registry().unwrap(),
            )
            .unwrap();
            let words = context
                .source_words
                .as_ref()
                .expect("original authored alias schema");
            assert!(matches!(
                words.source,
                OriginalRegistrySource::SourceTransitions(_)
            ));
            assert_eq!(words.arguments.len(), 2);
            assert!(
                words.operands[0].is_none(),
                "captured prefix owns no written cursor extent"
            );
            let signature = help(alias, &analysis, cursor).expect(dialect);
            assert!(signature.signatures[0].label.contains("length"));
            assert_eq!(signature.active_parameter, 0);
        }
    }

    #[test]
    fn original_signature_schema_rejects_foreign_context_source_and_known_shadow() {
        // naming.consumer.original-signature-schema-rendering
        // docs/design/analysis/name-resolution-proofs/original-signature-schema-rendering.md
        let source = "package ifneeded P 1.0 {string length value}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let cursor = source.find("value").unwrap() + 3;
        assert!(help(source, &analysis, cursor).is_some());
        assert!(help(&source.replace("value", "other"), &analysis, cursor).is_none());
        let mut foreign = analysis.clone();
        let profile = crate::profile_for_dialect("jim");
        foreign.resolved_input = Some(ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            analysis.body_lexer_config.unwrap(),
        ));
        assert!(help(source, &foreign, cursor).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(help(source, &changed, cursor).is_none());
        let shadow = "proc string args {}; string length value";
        let analysis = Analyser::new().analyse(shadow, "tcl8.6");
        let context = command_context_with_args(
            shadow,
            0,
            u32::try_from(shadow.len()).unwrap(),
            &analysis,
            analysis.resolved_registry().unwrap(),
        )
        .unwrap();
        assert!(
            context.source_words.is_none(),
            "actual shadow cannot become a Registry schema"
        );
        let signature = help(shadow, &analysis, shadow.len()).unwrap();
        assert_eq!(signature.signatures[0].label, "string args");
        assert_eq!(signature.signatures[0].parameters[0].label, "args");
    }

    #[test]
    fn original_signature_jim_selector_uses_its_own_source_schema() {
        // naming.consumer.original-signature-schema-rendering
        // docs/design/analysis/name-resolution-proofs/original-signature-schema-rendering.md
        let source = "proc p {} {info complete value}";
        let analysis = Analyser::new().analyse(source, "jim");
        let cursor = source.find("value").unwrap() + 3;
        let context = command_context_with_args(
            source,
            0,
            u32::try_from(cursor).unwrap(),
            &analysis,
            analysis.resolved_registry().unwrap(),
        )
        .unwrap();
        let words = context.source_words.as_ref().unwrap();
        let generation = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(
            words.with_source_schema(&generation, |schema| schema
                .subcommand
                .resolved()
                .map(|sub| sub.canonical_name)),
            Some(Some("complete"))
        );
        assert!(
            help(source, &analysis, cursor).unwrap().signatures[0]
                .label
                .contains("?missing?")
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.6");
        assert!(words.with_source_schema(&foreign, |_| ()).is_none());
    }

    #[test]
    fn original_signature_keeps_explicit_logical_advice_separate() {
        // naming.consumer.original-signature-schema-rendering
        // docs/design/analysis/name-resolution-proofs/original-signature-schema-rendering.md
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "signature-explicit-logical-source",
            &[],
            "Explicit lexical advice without a selected native name recipe",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )
        .intern();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            config,
        );
        let source = "puts value";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_lexical_declaration_advice());
        let context = command_context_with_args(
            source,
            0,
            u32::try_from(source.len()).unwrap(),
            &analysis,
            analysis.resolved_registry().unwrap(),
        )
        .unwrap();
        let words = context
            .source_words
            .as_ref()
            .expect("retained Logical source schema");
        let OriginalRegistrySource::SourceTransitions(advice) = &words.source else {
            panic!("Logical input must retain its separate source advice domain");
        };
        assert_eq!(
            advice.logical_source_input(),
            analysis.resolved_input.as_ref()
        );
        for cursor in [source.len(), source.find("value").unwrap() + 2] {
            let signature =
                help(source, &analysis, cursor).expect("complete original Logical vector");
            assert!(signature.signatures[0].label.contains("puts"));
            assert!(
                signature.signatures[0]
                    .documentation
                    .as_deref()
                    .unwrap()
                    .contains("availability is unresolved")
            );
        }
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.6");
        assert!(words.with_source_schema(&foreign, |_| ()).is_none());
        assert!(help(&source.replace("value", "other"), &analysis, source.len()).is_none());
    }
}
