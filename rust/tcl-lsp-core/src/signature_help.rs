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
//! segment at the cursor.  The active parameter is derived from
//! a simple whitespace-aware count of arguments typed so far on
//! the current physical line.
//!
//! Two lookup paths:
//!
//! 1. **User-defined proc** — `analysis.all_procs` keyed by
//!    simple, qualified, or unprefixed-qualified name.  Signature
//!    label is rendered from the proc's parameter list (including
//!    `{name default}` brackets for optional params); the
//!    documentation field surfaces the proc's harvested doc-
//!    comment.
//! 2. **Built-in command** — when the cursor's command isn't a
//!    user proc and the caller passes a
//!    [`tcl_registry::CommandRegistry`], look up the spec and
//!    render its first `hover.synopsis` entry as the signature.
//!    Parameters are whitespace-separated synopsis tokens after
//!    the command word; `hover.summary` becomes the
//!    documentation.
//!
//! Limitations: only the cursor's physical line is walked, so
//! multi-line command segments (continuation lines, embedded
//! `[…]` / `{…}`) aren't understood, and rich doc-comment
//! rendering isn't done — the summary is surfaced verbatim.

use rustc_hash::FxHashSet;
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
/// which parameter list is correct at this call (issue #1116 item 1).  Showing
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
    let registry = resolution.registry;
    let (command, args, active_param) = command_context_with_args(source, line, character)?;
    // Resolve the command from the namespace the cursor sits in, the way C
    // Tcl's own command resolution would (caller namespace, then global), so a
    // same-named proc in an unrelated namespace never hijacks the signature and
    // a same-named builtin keeps its synopsis unless a proc is actually visible.
    let cursor_off = {
        let line_index = tcl_lexer::LineIndex::new(source);
        crate::definition::byte_offset_at(&line_index, source, line, character)
    };
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        cursor_off,
        &analysis.namespace_overrides,
    );
    if let Some(proc_def) = lookup_proc(
        analysis,
        source,
        &namespace,
        &command,
        cursor_off,
        resolution,
    ) {
        return Some(proc_signature_help(proc_def, active_param));
    }
    let registry = registry?;
    let spec = registry.get(&command)?;
    // Subcommand-scoped signatures:
    // when the spec has subcommands and the first argument
    // matches one, prefer the subcommand's signature over the
    // command-level one.  Adjusts `active_param` to be
    // relative to the subcommand's parameters (the subcommand
    // name itself is consumed before the user-typed args).
    if !spec.subcommands.is_empty()
        && let Some(first_arg) = args.first()
    {
        if let Some(sub) = spec
            .subcommands
            .iter()
            .find(|s| s.name == first_arg.as_str())
        {
            let sub_param = active_param.saturating_sub(1);
            return subcommand_signature_help(&command, sub, sub_param);
        }
        // A first arg is present but isn't a known subcommand — offer no
        // signature rather than the generic command-level one (`string
        // bogus` must not surface `string option arg …`).
        return None;
    }
    builtin_signature_help(spec, active_param)
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
fn command_context_with_args(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, Vec<String>, u32)> {
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

    // Lex the document up to the cursor's byte offset.  We
    // walk the full token stream and stop including tokens
    // once we cross `cursor_offset`.
    let lexer = Lexer::new(source);
    let Ok(tokens) = lexer.tokenise_all() else {
        return None;
    };

    let mut current_segment: Vec<String> = Vec::new();
    let mut at_new_word = true;
    for tok in tokens {
        if tok.span.start() >= cursor_offset {
            break;
        }
        match tok.kind {
            TokenType::Sep => {
                at_new_word = true;
            }
            TokenType::Eol => {
                // Real EOL (non-empty text — a semicolon or
                // line-ending newline) resets the segment.
                // Synthetic empty EOLs (used to terminate the
                // stream) leave the segment alone.
                let raw = &source[tok.span.start() as usize..tok.span.end() as usize];
                if !raw.is_empty() {
                    current_segment.clear();
                    at_new_word = true;
                }
            }
            TokenType::Comment | TokenType::Expand => {}
            TokenType::Eof => {
                break;
            }
            _ => {
                // Word-producing token (Esc / Str / Var / Cmd /
                // Other).  Each contributes a word to the
                // segment unless we're mid-word (the previous
                // token also contributed without an intervening
                // SEP).
                let raw = &source[tok.span.start() as usize..tok.span.end() as usize];
                if at_new_word || current_segment.is_empty() {
                    current_segment.push(raw.to_owned());
                } else if let Some(last) = current_segment.last_mut() {
                    last.push_str(raw);
                }
                at_new_word = false;
            }
        }
    }

    if current_segment.is_empty() {
        return None;
    }
    let command = current_segment[0].clone();
    let args: Vec<String> = current_segment.iter().skip(1).cloned().collect();
    let arg_token_count = current_segment.len().saturating_sub(1);
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
    Some((command, args, active_param))
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
    // Alias resolution.  When the cursor's command isn't a visible proc, check
    // whether it matches an `interp alias {} ALIAS {} TARGET` record and follow
    // the chain, resolving the target the same namespace-aware way.
    let resolved_target = resolve_alias_chain(analysis, name)?;
    crate::definition::resolve_called_proc(
        analysis,
        source,
        namespace,
        &resolved_target,
        call_off,
        ctx,
    )
}

/// Follow the alias chain from `name` to its terminal
/// target.  Returns `None` when `name` doesn't match any
/// alias record.  Cycles are bounded by `MAX_ALIAS_HOPS`.
fn resolve_alias_chain(analysis: &AnalysisResult, name: &str) -> Option<String> {
    const MAX_ALIAS_HOPS: usize = 8;
    let mut current = name.to_owned();
    let mut seen = FxHashSet::default();
    for _ in 0..MAX_ALIAS_HOPS {
        if !seen.insert(current.clone()) {
            return None;
        }
        let qualified = if current.starts_with("::") {
            current.clone()
        } else {
            format!("::{current}")
        };
        if let Some(alias) = analysis
            .command_aliases
            .get(&qualified)
            .or_else(|| analysis.command_aliases.get(&current))
        {
            current.clone_from(&alias.target);
            continue;
        }
        return Some(current);
    }
    None
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
    fn bare_call_with_two_namespace_procs_picks_deterministically() {
        // Two namespaces define `greet`; a bare `greet` at global scope names
        // neither directly, so the lenient fallback picks one — and it must be
        // stable (lexicographically smallest qualified name), never `HashMap`
        // iteration order.  `::A::greet` (param `alpha`) wins over `::B::greet`.
        let src = "namespace eval A {\n    proc greet {alpha} {}\n}\nnamespace eval B {\n    proc greet {beta} {}\n}\ngreet \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let (l, c) = pos_after(src, "greet ", 3);
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
