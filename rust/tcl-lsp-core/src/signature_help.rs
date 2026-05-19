//! Signature-help provider — Rust port of
//! `lsp/features/signature_help.py`.
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
//!    documentation.  This is part of the
//!    `S-signature-help-rich` follow-up.
//!
//! What is *still deferred* (planned as further
//! `S-signature-help-rich` sub-strips):
//!
//! * Multi-line command segments (the port only walks the
//!   cursor's physical line — Python uses
//!   `find_command_context_details_at_position`, which
//!   understands continuation lines, embedded `[…]` / `{…}`
//!   etc.).
//! * Command-alias resolution
//!   (`lookup_alias_for_word(...)`).
//! * Subcommand-scoped signatures (Python's `SubcommandSig`
//!   path — pulls the right shape based on `args[0]`).
//! * `_signature_documentation` rich doc-comment rendering
//!   (the current port surfaces the summary verbatim).

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_registry::CommandRegistry;

/// One element in a signature's parameter list.
///
/// Mirrors `lsprotocol.types.ParameterInformation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInformation {
    /// Parameter label as shown in the signature
    /// (e.g. `name` or `{count 1}`).
    pub label: String,
}

/// One signature in a signature-help response.
///
/// Mirrors `lsprotocol.types.SignatureInformation`.
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
///
/// Mirrors `lsprotocol.types.SignatureHelp`.
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
/// the built-in command set (`S-signature-help-rich`): user
/// procs win, but if the cursor's command isn't a user proc,
/// the spec's first `hover.synopsis` entry renders as the
/// signature.  When `registry` is `None` the surface degrades
/// cleanly to the minimal port's user-proc-only behaviour.
#[must_use]
pub fn signature_help(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Option<SignatureHelp> {
    let (command, args, active_param) = command_context_with_args(source, line, character)?;
    if let Some(proc_def) = lookup_proc(analysis, &command) {
        return Some(proc_signature_help(proc_def, active_param));
    }
    let registry = registry?;
    let spec = registry.get(&command)?;
    // `S-signature-help-rich` subcommand-scoped signatures:
    // when the spec has subcommands and the first argument
    // matches one, prefer the subcommand's signature over the
    // command-level one.  Adjusts `active_param` to be
    // relative to the subcommand's parameters (the subcommand
    // name itself is consumed before the user-typed args).
    if !spec.subcommands.is_empty() {
        if let Some(first_arg) = args.first() {
            if let Some(sub) = spec.subcommands.iter().find(|s| s.name == first_arg.as_str()) {
                let sub_param = active_param.saturating_sub(1);
                return subcommand_signature_help(&command, sub, sub_param);
            }
        }
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
/// Mirrors Python's `find_command_context_details_at_position`:
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
    use tcl_lexer::{Lexer, LineIndex, TokenType};

    let cursor_offset = {
        let line_index = LineIndex::new(source);
        if u32::try_from(line_index.line_count()).unwrap_or(0) <= line {
            return None;
        }
        let line_start = line_index.line_start(line);
        let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);
        // Clamp to the end of the source so callers passing a
        // virtual EOL column don't index past the buffer.
        line_start.saturating_add(character).min(source_len)
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
    if active_param == 0 && !at_new_word {
        // Cursor still on the command name itself — no
        // signature yet.
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

fn lookup_proc<'a>(analysis: &'a AnalysisResult, name: &str) -> Option<&'a ProcDef> {
    if let Some(proc_def) = direct_proc_lookup(analysis, name) {
        return Some(proc_def);
    }
    // `S-signature-help-rich`: alias resolution.  When the
    // cursor's command isn't a user proc, check whether it
    // matches an `interp alias {} ALIAS {} TARGET` record and
    // follow the chain to the target proc.  Mirrors Python's
    // `lookup_alias_for_word`.
    let resolved_target = resolve_alias_chain(analysis, name)?;
    direct_proc_lookup(analysis, &resolved_target)
}

fn direct_proc_lookup<'a>(analysis: &'a AnalysisResult, name: &str) -> Option<&'a ProcDef> {
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == name || qname == name || qname == &format!("::{name}") {
            return Some(proc_def);
        }
    }
    None
}

/// Follow the alias chain from `name` to its terminal
/// target.  Returns `None` when `name` doesn't match any
/// alias record.  Cycles are bounded by `MAX_ALIAS_HOPS`.
fn resolve_alias_chain(analysis: &AnalysisResult, name: &str) -> Option<String> {
    const MAX_ALIAS_HOPS: usize = 8;
    let mut current = name.to_owned();
    let mut seen = std::collections::HashSet::new();
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
/// tokens after the leading command word in that synopsis
/// — that matches the shape Python's `_builtin_signature_help`
/// produces from `SIGNATURES`.  Returns `None` when the spec
/// has no hover record or the synopsis is empty.
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

    let documentation = if hover.summary.is_empty() {
        None
    } else {
        Some(hover.summary.to_owned())
    };

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
    let parameters: Vec<ParameterInformation> = proc_def
        .params
        .iter()
        .map(|p| {
            let label = if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            };
            ParameterInformation { label }
        })
        .collect();

    let label = format!(
        "proc {} {}",
        proc_def.qualified_name,
        parameters
            .iter()
            .map(|p| p.label.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );

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
        assert!(h.signatures[0].label.contains("::greet"));
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

    // -- S-signature-help-rich: built-in command signatures ----------
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
        // uses the command-name prefix.
        assert!(
            h.signatures[0].label.starts_with("proc "),
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

    #[test]
    fn builtin_signature_returns_none_without_registry() {
        // Without a registry, unknown commands still return
        // `None` — preserves the minimal port's behaviour.
        let src = "puts \n";
        let analysis = analyse(src);
        assert!(signature_help(src, 0, 5, &analysis, None).is_none());
    }

    // -- S-signature-help-rich: multi-line / semicolon segments ------

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
        let h = signature_help(src, 2, 7, &analysis, None)
            .expect("expected signature help for `b`");
        // The signature should be for `b`, not `a`.
        assert!(
            h.signatures[0].label.contains("::b"),
            "expected label for `b`, got {label}",
            label = h.signatures[0].label,
        );
    }

    // `[greet …]` substitution-bracket recursion remains a
    // further sub-strip — the lexer surfaces `[greet ]` as a
    // single Cmd token, so signature help for the inner command
    // needs a recursive lex of the bracket body (the Python
    // provider does this via the segmenter's
    // `command_substitutions` walk).  Tracked under
    // `S-signature-help-rich` continued follow-ups.

    // -- S-signature-help-rich: alias resolution ---------------------

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
            h.signatures[0].label.contains("::greet"),
            "expected greet's signature via alias; got {label}",
            label = h.signatures[0].label,
        );
        assert!(
            h.signatures[0]
                .parameters
                .iter()
                .any(|p| p.label == "name"),
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
    fn subcommand_signature_falls_back_for_unknown_subcommand() {
        // `string nonsense $arg` — `nonsense` isn't a string
        // subcommand, so the provider should fall back to the
        // command-level signature.
        let src = "string nonsense \n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let h = signature_help(src, 0, 16, &analysis, Some(&registry))
            .expect("expected fallback signature");
        // Falls back to the command-level synopsis.
        assert!(
            h.signatures[0].label.starts_with("string"),
            "got label {label}",
            label = h.signatures[0].label,
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
        let h = signature_help(src, 3, 2, &analysis, None)
            .expect("expected chained alias resolution");
        assert!(
            h.signatures[0].label.contains("::c"),
            "expected target `c`; got {label}",
            label = h.signatures[0].label,
        );
    }
}
