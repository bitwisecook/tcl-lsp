#![allow(clippy::doc_markdown, clippy::similar_names, clippy::unused_self)]

//! TclOO class / method body parsing — Rust port of
//! `core/analysis/_analyser/_oo.py`.
//!
//! Walks the body of an ``oo::class create Name { ... }`` or
//! ``oo::define Name { ... }`` block and populates the
//! [`super::types::ClassDef`] structural fields
//! (``superclasses``, ``mixins``, ``methods``,
//! ``class_methods``) that **C41e0** added but left empty.
//!
//! Subcommand coverage (mirrors Python's
//! ``_parse_oo_definition_body``):
//!
//! - ``superclass <names>`` — assigns ``ClassDef::superclasses``.
//! - ``mixin ?-append? <names>`` — assigns
//!   ``ClassDef::mixins`` (the ``-append`` flag is consumed and
//!   ignored — class-hierarchy state machines belong to the
//!   workspace index, not the per-file analyser).
//! - ``method NAME PARAMS BODY`` — adds an entry to
//!   ``ClassDef::methods``.
//! - ``classmethod NAME PARAMS BODY`` — adds an entry to
//!   ``ClassDef::class_methods``.
//! - ``constructor PARAMS BODY`` / ``destructor BODY`` —
//!   recognised but currently stored as a method named
//!   ``"<constructor>"`` / ``"<destructor>"`` since the Rust
//!   ``ClassDef`` doesn't carry separate constructor /
//!   destructor fields yet (Python's
//!   ``constructors: list[MethodDef]`` and
//!   ``destructor: MethodDef | None`` land in a follow-up).
//! - ``forward NAME ?TARGET ARGS?`` — added with ``kind = "forward"``.
//!
//! Other subcommands recognised by Python (``filter``,
//! ``export``, ``unexport``, ``property``, ``private``,
//! ``initialise`` / ``initialize``, ``variable``) are visited
//! and consumed but their data stays unrecorded for now —
//! ``ClassDef`` doesn't carry the matching fields yet.  C41e3
//! brings the property + variable storage; the rest land
//! incrementally as needed.

use tcl_lexer::{Token, TokenType};

use super::state::Analyser;
use super::types::{ClassDef, MethodDef};
use super::utils::parse_param_list;

impl Analyser {
    /// Walk the body of a ``oo::class create`` / ``oo::define``
    /// block, populating `class_def` from each subcommand.
    ///
    /// Mirrors `_parse_oo_definition_body` in
    /// `core/analysis/_analyser/_oo.py:146-237`.  The body is
    /// re-segmented via [`crate::segmenter::segment_commands_with_offset`]
    /// (no recovery — recovery is top-level only, mirroring
    /// Python).  Dynamic bodies (non-`Str` tokens) skip the
    /// walk because they can't be statically re-segmented.
    pub(super) fn parse_oo_definition_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset(body_text, base_offset);
        for cmd in cmds {
            if cmd.is_partial || cmd.argv.is_empty() {
                continue;
            }
            apply_oo_subcommand(&cmd.texts, &cmd.argv, class_def);
        }
    }

    /// Walk an inline ``oo::define Class subcmd ...`` form,
    /// dispatching the same per-subcommand logic.
    ///
    /// Mirrors `_parse_oo_define_inline` in `_oo.py:239-289`.
    /// The Rust port reuses [`apply_oo_subcommand`] — the
    /// inline form differs from the body form only in how
    /// arguments are framed; the per-subcommand handling is
    /// identical.
    pub(super) fn parse_oo_define_inline(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        class_def: &mut ClassDef,
    ) {
        if args.is_empty() {
            return;
        }
        // Synthesise a single fake "command" matching what the
        // body walker would have produced.
        apply_oo_subcommand(args, arg_tokens, class_def);
    }
}

/// Per-subcommand dispatcher shared by the body-form and
/// inline-form walkers.
///
/// `texts` and `argv` are parallel: `texts[0]` / `argv[0]` is
/// the subcommand name (``superclass`` / ``method`` / etc.).
fn apply_oo_subcommand(texts: &[String], argv: &[Token], class_def: &mut ClassDef) {
    let Some(subcmd) = texts.first().map(String::as_str) else {
        return;
    };
    let sub_args: &[String] = if texts.len() > 1 { &texts[1..] } else { &[] };
    let sub_tokens: &[Token] = if argv.len() > 1 { &argv[1..] } else { &[] };

    match subcmd {
        "superclass" => {
            class_def.superclasses = sub_args.to_vec();
        }
        "mixin" => {
            // Skip ``-append`` and similar flags — mirrors
            // Python's ``[a for a in sub_args if not a.startswith("-")]``.
            class_def.mixins = sub_args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .cloned()
                .collect();
        }
        "method" => {
            if let Some(md) = extract_method_def(sub_args, sub_tokens, "method", "public", "") {
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "classmethod" => {
            if let Some(md) = extract_method_def(sub_args, sub_tokens, "classmethod", "public", "")
            {
                class_def.class_methods.insert(md.name.clone(), md);
            }
        }
        "constructor" => {
            if let Some(md) = extract_method_def(
                sub_args,
                sub_tokens,
                "constructor",
                "public",
                "<constructor>",
            ) {
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "destructor" => {
            if let Some(md) =
                extract_method_def(sub_args, sub_tokens, "destructor", "public", "<destructor>")
            {
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "forward" => {
            if let Some(name) = sub_args.first() {
                let span = sub_tokens
                    .first()
                    .map_or_else(|| tcl_lexer::Span::new(0, 0), |t| t.span);
                let md = MethodDef {
                    name: name.clone(),
                    name_span: span,
                    body_span: span,
                    kind: "forward".to_string(),
                    visibility: "public".to_string(),
                    doc: String::new(),
                };
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "private" => {
            // ``private`` wraps another definition subcommand.
            // The wrapped subcommand fires with ``visibility =
            // "private"``.
            if sub_args.is_empty() {
                return;
            }
            let inner_subcmd = sub_args[0].as_str();
            let inner_args: &[String] = &sub_args[1..];
            let inner_tokens: &[Token] = if sub_tokens.len() > 1 {
                &sub_tokens[1..]
            } else {
                &[]
            };
            match inner_subcmd {
                "method" => {
                    if let Some(md) =
                        extract_method_def(inner_args, inner_tokens, "method", "private", "")
                    {
                        class_def.methods.insert(md.name.clone(), md);
                    }
                }
                "classmethod" => {
                    if let Some(md) =
                        extract_method_def(inner_args, inner_tokens, "classmethod", "private", "")
                    {
                        class_def.class_methods.insert(md.name.clone(), md);
                    }
                }
                _ => {}
            }
        }
        // Subcommands recognised but not yet stored: variable,
        // filter, export, unexport, property, initialise /
        // initialize.  ClassDef doesn't carry the matching
        // fields yet — they land in C41e3 / follow-ups.
        _ => {}
    }
}

/// Extract a [`MethodDef`] from method-style args.
///
/// Mirrors `_extract_method_def` in `_oo.py:290-349`.  Three
/// shapes:
///
/// - **method / classmethod**: `args = [name, params, body]`.
/// - **constructor**: `args = [params, body]`; `synthetic_name`
///   provides the placeholder name (``"<constructor>"``).
/// - **destructor**: `args = [body]`; same synthetic-name trick.
///
/// Returns `None` when the argument count is too short to
/// match any of the shapes.
fn extract_method_def(
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
) -> Option<MethodDef> {
    let zero = tcl_lexer::Span::new(0, 0);
    match kind {
        "constructor" => {
            // ``constructor PARAMS BODY``.
            if args.len() < 2 {
                return None;
            }
            let _params = parse_param_list(&args[0]);
            let name_span = zero;
            let body_span = arg_tokens.get(1).map_or(zero, |t| t.span);
            Some(MethodDef {
                name: synthetic_name.to_string(),
                name_span,
                body_span,
                kind: kind.to_string(),
                visibility: visibility.to_string(),
                doc: String::new(),
            })
        }
        "destructor" => {
            // ``destructor BODY``.
            if args.is_empty() {
                return None;
            }
            let name_span = zero;
            let body_span = arg_tokens.first().map_or(zero, |t| t.span);
            Some(MethodDef {
                name: synthetic_name.to_string(),
                name_span,
                body_span,
                kind: kind.to_string(),
                visibility: visibility.to_string(),
                doc: String::new(),
            })
        }
        _ => {
            // ``method NAME PARAMS BODY`` / ``classmethod NAME PARAMS BODY``.
            if args.len() < 3 {
                return None;
            }
            let name = args[0].clone();
            let _params = parse_param_list(&args[1]);
            let name_span = arg_tokens.first().map_or(zero, |t| t.span);
            let body_span = arg_tokens.get(2).map_or(zero, |t| t.span);
            Some(MethodDef {
                name,
                name_span,
                body_span,
                kind: kind.to_string(),
                visibility: visibility.to_string(),
                doc: String::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn class() -> ClassDef {
        ClassDef {
            name: "C".to_string(),
            qualified_name: "::C".to_string(),
            name_span: tcl_lexer::Span::new(0, 0),
            body_span: tcl_lexer::Span::new(0, 0),
            superclasses: Vec::new(),
            mixins: Vec::new(),
            methods: HashMap::new(),
            class_methods: HashMap::new(),
        }
    }

    fn tok(span: (u32, u32)) -> Token {
        Token::new(TokenType::Esc, tcl_lexer::Span::new(span.0, span.1))
    }

    fn str_tok(span: (u32, u32)) -> Token {
        Token {
            kind: TokenType::Str,
            span: tcl_lexer::Span::new(span.0, span.1),
            content_offset: 1,
            in_quote: false,
        }
    }

    #[test]
    fn superclass_subcommand_assigns_supers() {
        let mut cd = class();
        let texts: Vec<String> = ["superclass", "::A", "::B"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 10)), tok((11, 14)), tok((15, 18))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.superclasses, vec!["::A", "::B"]);
    }

    #[test]
    fn mixin_subcommand_strips_dash_flags() {
        let mut cd = class();
        let texts: Vec<String> = ["mixin", "-append", "::M1", "::M2"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 5)), tok((6, 13)), tok((14, 18)), tok((19, 23))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert_eq!(cd.mixins, vec!["::M1", "::M2"]);
    }

    #[test]
    fn method_subcommand_records_method_def() {
        let mut cd = class();
        let texts: Vec<String> = ["method", "greet", "name", "puts $name"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 6)), tok((7, 12)), tok((13, 17)), str_tok((18, 32))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("greet"));
        let md = &cd.methods["greet"];
        assert_eq!(md.kind, "method");
        assert_eq!(md.visibility, "public");
    }

    #[test]
    fn classmethod_subcommand_records_class_method() {
        let mut cd = class();
        let texts: Vec<String> = ["classmethod", "build", "args", "return $args"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 11)),
            tok((12, 17)),
            tok((18, 22)),
            str_tok((23, 38)),
        ];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.class_methods.contains_key("build"));
        assert!(!cd.methods.contains_key("build"));
    }

    #[test]
    fn constructor_records_synthetic_name() {
        let mut cd = class();
        let texts: Vec<String> = ["constructor", "args", "puts ctor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 11)), tok((12, 16)), str_tok((17, 28))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("<constructor>"));
        assert_eq!(cd.methods["<constructor>"].kind, "constructor");
    }

    #[test]
    fn destructor_records_synthetic_name() {
        let mut cd = class();
        let texts: Vec<String> = ["destructor", "puts dtor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 10)), str_tok((11, 22))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("<destructor>"));
        assert_eq!(cd.methods["<destructor>"].kind, "destructor");
    }

    #[test]
    fn forward_records_method_with_forward_kind() {
        let mut cd = class();
        let texts: Vec<String> = ["forward", "delegate", "::other::cmd"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 7)), tok((8, 16)), tok((17, 29))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("delegate"));
        assert_eq!(cd.methods["delegate"].kind, "forward");
    }

    #[test]
    fn private_method_records_with_private_visibility() {
        let mut cd = class();
        let texts: Vec<String> = ["private", "method", "internal", "args", "puts hi"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 7)),
            tok((8, 14)),
            tok((15, 23)),
            tok((24, 28)),
            str_tok((29, 37)),
        ];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("internal"));
        assert_eq!(cd.methods["internal"].visibility, "private");
    }

    #[test]
    fn unrecognised_subcommand_is_silent_noop() {
        let mut cd = class();
        let texts: Vec<String> = ["filter", "f1"].iter().map(|s| (*s).to_string()).collect();
        let argv = [tok((0, 6)), tok((7, 9))];
        apply_oo_subcommand(&texts, &argv, &mut cd);
        // No fields populated; no panic.
        assert!(cd.methods.is_empty());
        assert!(cd.superclasses.is_empty());
        assert!(cd.mixins.is_empty());
    }

    #[test]
    fn extract_method_def_too_few_args_returns_none() {
        // ``method`` with only 1 arg (just the name) — needs 3.
        let args: Vec<String> = vec!["foo".to_string()];
        let argv: Vec<Token> = vec![tok((0, 3))];
        let md = extract_method_def(&args, &argv, "method", "public", "");
        assert!(md.is_none());
    }
}
