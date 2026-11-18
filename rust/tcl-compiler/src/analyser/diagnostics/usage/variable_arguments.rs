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

//! Arguments that name or extend a variable: `lappend` on a non-list (W104), a value where a name belongs (W212) and a broken `${name}(...)` element (W216).

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use rustc_hash::FxHashSet;
use tcl_core_types::DiagCode;

/// Original executable text parts supply lexical padding without materialising
/// substituted variables/commands or reparsing a reporting string.
fn original_word_has_pad_space(word: &tcl_lexer::NativeWord) -> bool {
    let arena = word.executable_parts();
    let parts = arena.list(arena.root());
    parts
        .first()
        .and_then(|part| arena.text(part))
        .is_some_and(|text| text.starts_with(b" "))
        || parts
            .last()
            .and_then(|part| arena.text(part))
            .is_some_and(|text| text.ends_with(b" "))
}

impl Analyser {
    /// W104: selected append payloads whose original executable text has a
    /// leading or trailing space. Captured payloads have no written call word.
    pub(in crate::analyser) fn emit_w104_append_list(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind as Kind, source_append_arguments,
        };
        let Some(original) = original else { return };
        let Some(layout) = original.with_schema(source_append_arguments).flatten() else {
            return;
        };
        for argument in layout.values.clone() {
            // A captured padded value has no call-site error behind it
            // (tclsh runs it); style advice declines on a captured slot.
            if original.is_captured(argument) {
                continue;
            }
            let Some(word) = original.word(argument) else {
                continue;
            };
            if !original_word_has_pad_space(word) {
                continue;
            }
            let Some(subject) = original.subject(Kind::AppendList, Some(argument)) else {
                continue;
            };
            let fixes = self
                .w104_lappend_fix(original, &layout)
                .into_iter()
                .collect();
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W104, word.span(),
                    "append with space-separated values looks like list construction. Use [lappend] instead to safely handle values containing spaces, braces, or backslashes.".to_owned(),
                    Severity::Hint,
                ).with_fixes(fixes).with_subject(subject),
            );
            return;
        }
    }

    /// Review-only proposal from exactly two complete original written operands.
    /// Captured variable/value positions and expansions cannot acquire an edit.
    fn w104_lappend_fix(
        &self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        layout: &tcl_registry::AuthoredSourceAppendArguments,
    ) -> Option<super::super::types::CodeFix> {
        if layout.variable != 0
            || layout.values != (1..2)
            || original.words().arguments().len() != 2
            || original.written_index(0)? != 0
            || original.written_index(1)? != 1
        {
            return None;
        }
        if !original.matches_analysis(&self.result, &self.source) {
            return None;
        }
        let variable_word = original.word(0)?;
        if variable_word.group().expand {
            return None;
        }
        let variable = if let Some(value) =
            tcl_syntax::word_rules::original_static_word_unicode_value(variable_word)
        {
            tcl_syntax::word_rules::lexical_unicode_source_word(
                std::str::from_utf8(&value).ok()?,
                variable_word.config(),
            )?
        } else {
            let source = variable_word.try_text().ok()?;
            tcl_syntax::word_rules::lexical_source_word_preserves_components(variable_word, source)
                .then_some(source.to_owned())?
        };
        let value = original.word(1)?;
        let piece = tcl_syntax::word_rules::quoted_leading_space_source_word(value)?;
        let input = self.result.resolved_input.as_ref()?.clone();
        let head =
            tcl_syntax::word_rules::lexical_unicode_source_word("lappend", input.lexer_config())?;
        let proposed_source = format!("{head} {variable} {piece}");
        // This is a separately authored stock-destination premise, not a new
        // interpretation of the original call or a claim about current lookup.
        let destination =
            crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
                &proposed_source,
                input,
            );
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            &proposed_source,
            0,
            destination.editing_input().lexer_config(),
        );
        let [segment] = segments.as_slice() else {
            return None;
        };
        let selected = destination.words(segment)?;
        let context = destination.editing_input().context_registry();
        let layout = selected.with_source_schema(&context, |schema| {
            schema.authored_source_list_append_arguments()
        })??;
        if layout.variable != 0 || layout.values != (1..2) {
            return None;
        }
        Some(super::super::types::CodeFix {
            span: original.invocation_span(),
            new_text: proposed_source,
            description: "Rewrite with `lappend`".to_owned(),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        })
    }

    /// Argument indices (0-based, command-name excluded) that `cmd` reads as a
    /// plain variable *name* — the positions where a `$`-substitution is a
    /// name/value confusion (W212) and the braced `${arr}(idx)` idiom is
    /// legitimate rather than a typo (W216).
    ///
    /// Registry-driven via the `VarWrite` / `VarRead` argument roles, so every
    /// command that declares them (`set`, `incr`, `append`, `lappend`, `unset`,
    /// `info exists`, `vwait`, `catch`, `scan`, `regexp`/`regsub` captures,
    /// `dict with`/`update`, `array set`, `lassign`, …) is covered without a
    /// hand-maintained list, which drifts: separate lists for the W212 and
    /// W216 positions each end up missing what the other has.  The one
    /// structural exception is `upvar`: its frame-linking
    /// semantics are modelled by the registry's repeated layout, and only its
    /// *local*-name slots are name positions — a computed `$remote` in an
    /// other-var slot is a legitimate indirect link.
    pub(in crate::analyser) fn variable_name_positions(
        original: &super::super::super::diagnostic_registry::OriginalDiagnosticSource,
    ) -> Vec<(usize, tcl_registry::ArgRole)> {
        original.variable_name_arguments()
    }

    /// W212: a command argument that must be a variable
    /// *name* (`set $x 1`, `incr $x`, `info exists $x`, `upvar 1 a $b`)
    /// instead uses a `$`-substitution.  `args` / `arg_tokens` exclude
    /// the command name.  Fires when the resolved name-position argument
    /// is a `Var` token.
    ///
    /// The "Did you mean…?" suggestion is the de-sigiled name itself
    /// when that name is defined in the enclosing scope (or nothing
    /// close is); when it is *undefined* but a close in-scope variable
    /// sits within the length-scaled edit budget, the suggestion names
    /// that variable instead — `set counter 1; incr $countr` suggests
    /// 'counter', not the nonexistent 'countr'.
    pub(in crate::analyser) fn emit_w212_name_vs_value(
        &mut self,
        original: Option<&super::super::super::diagnostic_registry::OriginalDiagnosticSource>,
        cmd_name: &str,
        scope_path: &[usize],
    ) {
        let Some(original) = original else {
            return;
        };
        let mut scope_vars: Option<Vec<String>> = None;
        let lone_argument = original.arguments().len() == 1;
        for (index, role) in Self::variable_name_positions(original) {
            // A lone read-only name whose value is the command's result
            // (`[set $name]`) is Tcl's indirection idiom: `$$name` does not
            // nest, so the substituted name is the intent, not a confusion.
            if lone_argument && role == tcl_registry::ArgRole::VarRead {
                continue;
            }
            let Some(word) = original.word(index) else {
                continue;
            };
            let Some(tok) = original.token(index) else {
                continue;
            };
            if tok.kind != tcl_lexer::TokenType::Var {
                continue;
            }
            let Ok(text) = word.try_text() else {
                continue;
            };
            if tcl_syntax::naming::is_braced_indirect_array_ref(text) {
                continue;
            }
            // The shared token-text owner supplies the written variable name;
            // a compound dynamic word cannot be promoted to a variable read.
            if word.tokens().len() != 1 {
                continue;
            }
            let source_map = self.cached_source_map();
            let bare = source_map.token_text(tok).to_owned();
            let bare = bare.as_str();
            let display_cmd = original.display_command(cmd_name);
            let vars = scope_vars.get_or_insert_with(|| self.scope_variable_names(scope_path));
            let suggestion = if vars.iter().any(|name| name == bare) {
                bare
            } else {
                crate::text::suggest_similar(
                    bare,
                    vars.iter().map(String::as_str),
                    1,
                    crate::text::scaled_max_distance_strict(bare),
                )
                .first()
                .copied()
                .unwrap_or(bare)
            };
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::W212,
                    tok.span,
                    format!(
                        "'{display_cmd}' expects a variable name, got substitution (${bare}). \
                     Did you mean '{suggestion}'?"
                    ),
                    super::super::types::Severity::Warning,
                )
                .with_subject(
                    original
                        .variable_name_subject(index)
                        .expect("original written operand"),
                ),
            );
        }
    }

    fn push_w216_replacement(
        diagnostics: &mut Vec<crate::analyser::types::Diagnostic>,
        span: tcl_lexer::Span,
        mut message: String,
        corrected: Option<String>,
    ) {
        let fixes = corrected.map(|corrected| {
            use std::fmt::Write;
            let premise = if corrected.starts_with("[::set ") {
                "; this suggestion requires `::set` to retain its stock variable-reading meaning"
            } else { "" };
            let _ = write!(message, "; did you mean `{corrected}` for array element access?{premise}");
            super::super::types::CodeFix {
                span,
                new_text: corrected.clone(),
                description: format!("Replace with `{corrected}`{premise}"),
                safety: crate::irules_checks::FixSafety::RequiresReview,
            }
        }).into_iter().collect();
        diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W216,
                span,
                message,
                Severity::Warning,
            )
            .with_fixes(fixes),
        );
    }

    /// **W216** — broken brace-form variants of array element access.
    ///
    /// Two related shapes both look like array-element access but parse
    /// differently than the user intends:
    ///
    /// 1. `${arr}(foo)` — the lexer ends the variable substitution at the
    ///    `}`, so this is scalar `${arr}` followed by *literal* `(foo)`; no
    ///    array access happens.
    /// 2. `${arr($foo)}` — the brace form applies no further substitution to
    ///    its content, so `$foo` inside the braces is the literal four-char
    ///    string, not the value of `foo`.
    ///
    /// In a *variable-name* position (`set` / `incr` / `append` / `lappend` /
    /// `unset` / `info exists` / `vwait`) Pattern (1) is the legitimate
    /// indirect-array-element idiom (`token` holds the array name) and must
    /// not fire — see [`tcl_syntax::naming::is_braced_indirect_array_ref`].
    pub(in crate::analyser) fn emit_w216_brace_then_paren(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
    ) {
        if cmd.texts.is_empty() {
            return;
        }
        let mut varname_words = Vec::new();
        if let Some(original) = self.original_diagnostic_source_for_segment(cmd) {
            for (argument, _) in Self::variable_name_positions(&original) {
                if original.written_index(argument).is_some()
                    && let Some(word) = original.word(argument)
                {
                    varname_words.push(word.clone());
                }
            }
        }
        let mut seen = FxHashSet::default();
        for &token in &cmd.argv {
            // Only a `${` reference can be either pattern; looking the word up
            // costs the whole command, so a long literal command was quadratic
            // (#2422).
            if self
                .source
                .get(token.span.as_range())
                .is_some_and(|text| !text.contains("${"))
            {
                continue;
            }
            let Some(word) = self.retained_value_word(token) else {
                continue;
            };
            for proposal in tcl_syntax::word_rules::braced_variable_array_sources(&word) {
                let span = proposal.span();
                let name_word_leading_variable = varname_words.contains(&word)
                    && word
                        .executable_parts()
                        .list(word.executable_parts().root())
                        .first()
                        .is_some_and(|part| {
                            part.span.start() == span.start()
                                && matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
                        });
                if !seen.insert(span)
                    || (proposal.has_adjacent_index() && name_word_leading_variable)
                {
                    continue;
                }
                let root = proposal.array_root();
                let inner = proposal.index_source();
                let scalar = proposal.scalar_name();
                let corrected = build_w216_replacement(root, inner, word.config());
                let message = if proposal.has_adjacent_index() {
                    format!(
                        "`${{{scalar}}}({inner})` is parsed as scalar `${{{scalar}}}` followed by literal text `({inner})`"
                    )
                } else {
                    format!(
                        "`${{{root}({inner})}}` does not substitute `{inner}` (the brace form applies no further substitution to its content)"
                    )
                };
                Self::push_w216_replacement(&mut self.result.diagnostics, span, message, corrected);
            }
        }
    }
}

/// Propose array-index substitution while preserving the literal source root.
/// The fallback invokes an unwritten `::set`; its stock meaning is a separate
/// `RequiresReview` premise, never an original command selection proof.
fn build_w216_replacement(
    name: &str,
    inner: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    // naming.diagnostics.original-w216-literal-source-replacement
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-w216-literal-source-replacement.md
    if name.is_empty() || name.contains('(') {
        return None;
    }
    let candidate = format!("$a({inner})");
    let reference = tcl_lexer::scan_var_ref(candidate.as_bytes(), 0, config).ok()??;
    if reference.next != candidate.len() || reference.index.is_none() {
        return None;
    }
    let direct = format!("${name}({inner})");
    let direct_matches = tcl_lexer::word_parts::whole_var_ref(direct.as_bytes(), config)
        .ok()
        .flatten()
        .is_some_and(|reference| {
            reference.name == name.as_bytes() && reference.index == Some(inner.as_bytes())
        });
    if tcl_syntax::naming::is_bare_var_name(name) && direct_matches {
        Some(direct)
    } else {
        let root = tcl_syntax::backslash::literal_quoted_source_fragment(name, config.escapes)?;
        let index = tcl_syntax::backslash::substituting_quoted_source_fragment(inner, config)?;
        Some(format!("[::set \"{root}({index})\"]"))
    }
}
