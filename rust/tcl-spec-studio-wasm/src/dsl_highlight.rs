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

//! Readonly DSL display from original whole-image word geometry and the
//! actual SpecTcl descriptor/field metadata. No runtime binding is inferred.

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use tcl_lexer::{SourceSegmentationPurpose, Span, TokenType};

/// Recursion guard, matching [`tcl_lexer::highlight`]'s own bound — a
/// maliciously (or accidentally) deeply nested `{{{{…}}}}` document must not
/// blow the stack.
const MAX_DEPTH: usize = 64;

/// One classified byte span. Kept separate from the JSON `Value` so the
/// native tests below can assert on typed fields instead of re-parsing JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HighlightToken {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) class: &'static str,
    /// The source text of the span, included alongside `start`/`end` so a
    /// browser caller never has to re-slice the original string by *byte*
    /// offset (a UTF-16 `String.slice` is the wrong tool for that). Not part
    /// of the documented `{start, end, class}` contract's minimum shape, but
    /// additive JSON fields are free and this is the one a caller always
    /// wants next.
    pub(crate) text: String,
    statement: Option<(
        String,
        String,
        Option<tcl_spec_studio::schema::DslDisplayGrammar>,
    )>,
}

/// Whether a braced/bracketed body continues the DSL's statement grammar
/// (so its first word is a further `command-word`) or is a flat word list /
/// prose the DSL does not itself structure.
#[derive(Debug, Clone, Copy)]
enum Mode {
    Statement,
    Flat,
    Grammar(tcl_spec_studio::schema::DslDisplayGrammar),
}
impl Mode {
    fn is_statement(self) -> bool {
        !matches!(self, Self::Flat)
    }
    fn grammar(self) -> Option<tcl_spec_studio::schema::DslDisplayGrammar> {
        match self {
            Self::Grammar(grammar) => Some(grammar),
            _ => None,
        }
    }
}

/// `true` for a bare word that reads as a number: an optional leading sign, a
/// digit, then any run of digits and `.` — covers `arity`'s `3..`, `3..5`,
/// plain counts, and version-shaped values like `8.5`. Deliberately loose:
/// this is a colour hint, not a validator.
fn looks_like_number(word: &str) -> bool {
    let unsigned = word.strip_prefix(['-', '+']).unwrap_or(word);
    let core = unsigned.strip_suffix('+').unwrap_or(unsigned);
    !core.is_empty()
        && core.starts_with(|c: char| c.is_ascii_digit())
        && core.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// `true` for a bare word that reads as a DSL property flag: `-introduced`,
/// `-detail`, `-step`, … — a leading `-` that is not itself a negative
/// number's sign.
fn looks_like_flag(word: &str) -> bool {
    word.len() > 1 && word.starts_with('-') && !looks_like_number(word)
}

/// Classify one bare (`Esc`-kind) word.
fn classify_bare_word(text: &str, mode: Mode, is_head: bool) -> &'static str {
    if is_head && mode.is_statement() {
        "command-word"
    } else if looks_like_flag(text) {
        "property-flag"
    } else if looks_like_number(text) {
        "number"
    } else {
        "value"
    }
}

/// Enter one original image; recursive walks retain that same issuer.
fn collect(
    src: &str,
    purpose: SourceSegmentationPurpose,
    depth: usize,
    base: usize,
    mode: Mode,
    out: &mut Vec<HighlightToken>,
) {
    let context = tcl_spec_studio::schema::DslDisplayContext::for_source(src);
    let Ok(end) = u32::try_from(src.len()) else {
        return;
    };
    collect_region(&context, Span::new(0, end), purpose, depth, base, mode, out);
}

fn collect_region(
    context: &tcl_spec_studio::schema::DslDisplayContext,
    region: Span,
    purpose: SourceSegmentationPurpose,
    depth: usize,
    base: usize,
    mode: Mode,
    out: &mut Vec<HighlightToken>,
) {
    if region.is_empty() || depth >= MAX_DEPTH {
        return;
    }
    let image = context.image();
    let Ok(source) = image.try_text() else {
        return;
    };
    let config = tcl_spec_studio::spectcl::pack_source_lexer_config(purpose);
    let Ok(tokens) = tcl_lexer::source_region_tokens_in(image, region, config) else {
        return;
    };
    let plan =
        tcl_lexer::native_script_words_in_with_purpose(image.clone(), region, config, purpose).ok();
    let mut children = BTreeMap::new();
    let mut statements = BTreeMap::new();
    if let Some(plan) = &plan {
        for command in &plan.commands {
            let display = mode
                .is_statement()
                .then(|| context.statement(&command.words, mode.grammar()))
                .flatten();
            for word in &command.words {
                let value = display.as_ref().and_then(|display| {
                    tcl_spec_studio::schema::dsl_word_value(word)
                        .map(|value| (display.head.clone(), value, mode.grammar()))
                });
                for token in word.tokens() {
                    statements.insert(token.span.start(), value.clone());
                }
                let structured = display
                    .as_ref()
                    .filter(|display| {
                        mode.is_statement() && display.bodies.iter().any(|body| body == word)
                    })
                    .and_then(|display| display.grammar);
                original_word_children(word, structured, &mut children);
            }
        }
    }
    let mut at_stmt_start = true;
    for token in &tokens {
        let start = token.span.start() as usize;
        let end = token.span.end() as usize;
        let Some(text) = source.get(start..end) else {
            continue;
        };
        let statement = statements.get(&token.span.start()).cloned().flatten();
        match token.kind {
            TokenType::Comment => push(out, base + start, base + end, "comment", text, None),
            TokenType::Var | TokenType::ExprSugar => {
                push(out, base + start, base + end, "variable", text, None);
                at_stmt_start = false;
            }
            TokenType::Str | TokenType::Cmd => {
                if let Some(&(content, child_mode)) = children.get(&token.span.start()) {
                    collect_region(
                        context,
                        content,
                        SourceSegmentationPurpose::NestedScript,
                        depth + 1,
                        base,
                        child_mode,
                        out,
                    );
                }
                at_stmt_start = false;
            }
            TokenType::Expand => {
                push(out, base + start, base + end, "punctuation", text, None);
                at_stmt_start = false;
            }
            TokenType::Esc => {
                let class = classify_bare_word(text, mode, at_stmt_start);
                push(out, base + start, base + end, class, text, statement);
                at_stmt_start = false;
            }
            TokenType::Eol => {
                if text == ";" {
                    push(out, base + start, base + end, "punctuation", text, None);
                }
                at_stmt_start = true;
            }
            TokenType::Sep | TokenType::Eof => {}
        }
    }
}

fn original_word_children(
    word: &tcl_lexer::NativeWord,
    structured: Option<tcl_spec_studio::schema::DslDisplayGrammar>,
    children: &mut BTreeMap<u32, (Span, Mode)>,
) {
    for token in word.tokens() {
        let child = match token.kind {
            TokenType::Str => word
                .content_span()
                .ok()
                .map(|content| (content, structured.map_or(Mode::Flat, Mode::Grammar))),
            TokenType::Cmd => {
                word.executable_parts()
                    .all_parts()
                    .find_map(|part| match &part.part {
                        tcl_lexer::ExecutablePart::Command { body }
                            if part.span.start() == token.span.start() =>
                        {
                            Some((*body, Mode::Flat))
                        }
                        _ => None,
                    })
            }
            _ => None,
        };
        if let Some(child) = child {
            children.insert(token.span.start(), child);
        }
    }
}

fn push(
    out: &mut Vec<HighlightToken>,
    start: usize,
    end: usize,
    class: &'static str,
    text: &str,
    statement: Option<(
        String,
        String,
        Option<tcl_spec_studio::schema::DslDisplayGrammar>,
    )>,
) {
    if end > start {
        out.push(HighlightToken {
            start,
            end,
            class,
            text: text.to_owned(),
            statement,
        });
    }
}

/// The full token list for `source`, in source order.
pub(crate) fn highlight_tokens(source: &str) -> Vec<HighlightToken> {
    let mut out = Vec::new();
    collect(
        source,
        SourceSegmentationPurpose::WholeInput,
        0,
        0,
        Mode::Statement,
        &mut out,
    );
    out.sort_by_key(|t| (t.start, t.end));
    out
}

/// `dsl_highlight`'s JSON body: `[{start, end, class, text}, …]`.
pub(crate) fn highlight_json(source: &str) -> Value {
    Value::Array(
        highlight_tokens(source)
            .iter()
            .map(|t| json!({ "start": t.start, "end": t.end, "class": t.class, "text": t.text }))
            .collect(),
    )
}

// Hover

/// `dsl_hover`'s JSON body: `{found, title, body}`.
pub(crate) fn hover_json(source: &str, offset: usize) -> Value {
    let Some(token) = highlight_tokens(source)
        .into_iter()
        .find(|t| t.start <= offset && offset < t.end)
    else {
        return json!({ "found": false, "title": "", "body": "" });
    };
    // Only these two classes name a word from the DSL's own vocabulary — a
    // `value`/`number`/`variable`/`string` is call-site data the pack author
    // wrote, not a keyword the studio has anything to say about.
    if !matches!(token.class, "command-word" | "property-flag") {
        return json!({ "found": false, "title": "", "body": "" });
    }
    match token.statement.as_ref().and_then(|(head, word)| {
        tcl_spec_studio::schema::DslDisplayContext::for_source(source).help(head, word)
    }) {
        Some((title, body)) => json!({ "found": true, "title": title, "body": body }),
        None => {
            let mut title = String::new();
            let _ = write!(title, "`{}`", token.text);
            json!({ "found": false, "title": title, "body": "" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(source: &str) -> Vec<(&'static str, String)> {
        highlight_tokens(source)
            .into_iter()
            .map(|t| (t.class, t.text))
            .collect()
    }

    #[test]
    fn comment_is_classified_and_positioned() {
        let src = "# a comment\ncommand foo {\n}\n";
        let toks = highlight_tokens(src);
        let comment = toks
            .iter()
            .find(|t| t.class == "comment")
            .expect("comment token");
        assert_eq!(&src[comment.start..comment.end], "# a comment");
    }

    #[test]
    fn top_level_and_nested_statement_heads_are_command_words() {
        let src =
            "speclib mylib 1.0 {\n\ncommand foo {\n    arity 1..\n    traits {PURE}\n}\n\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("command-word", "speclib".to_owned())));
        assert!(cs.contains(&("command-word", "command".to_owned())));
        assert!(cs.contains(&("command-word", "arity".to_owned())));
        // `traits` is itself a command-word (a CommandSpec field, statement
        // head)…
        assert!(cs.contains(&("command-word", "traits".to_owned())));
        // …but its own block is a flat FlagSet, so `PURE` is a value, not a
        // nested command-word.
        assert!(cs.contains(&("value", "PURE".to_owned())));
        assert!(!cs.contains(&("command-word", "PURE".to_owned())));
    }

    #[test]
    fn arity_range_is_a_number_and_flags_are_property_flags() {
        let src = "command foo {\n    arity 3.. -step 2 -also 4\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("number", "3..".to_owned())));
        assert!(cs.contains(&("property-flag", "-step".to_owned())));
        assert!(cs.contains(&("number", "2".to_owned())));
        assert!(cs.contains(&("property-flag", "-also".to_owned())));
        assert!(cs.contains(&("number", "4".to_owned())));
    }

    #[test]
    fn negative_number_after_a_flag_is_not_a_flag() {
        // `-also` is a flag; its value `-1` starts with `-` too but must not
        // be painted as a second flag.
        let src = "command foo {\n    arity 1.. -also -1\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("property-flag", "-also".to_owned())));
        assert!(cs.contains(&("number", "-1".to_owned())));
        assert!(!cs.iter().any(|(c, t)| *c == "property-flag" && t == "-1"));
    }

    #[test]
    fn dollar_variable_inside_a_hook_body_is_still_classified() {
        let src = "hook myhook {words ctx} {\n    set x $words\n}\n";
        let cs = classes(src);
        // The raw token span includes the sigil (`$name`, not `name`) —
        // `content_offset` is what a `SourceMap` would use to strip it, and
        // this module deliberately keeps the sigil in `text` so a rendered
        // span reproduces the source byte-for-byte when concatenated.
        assert!(cs.contains(&("variable", "$words".to_owned())));
        // `set` sits in a flat (hook-body) block, so it is a value — not
        // promoted to command-word — matching the module's documented
        // "hook bodies are prose, not DSL statements" limitation.
        assert!(!cs.contains(&("command-word", "set".to_owned())));
    }

    #[test]
    fn option_row_flags_are_property_flags_and_detail_is_flat() {
        let src = "command foo {\n    option -ascii -detail {Compare using $x order.}\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("command-word", "option".to_owned())));
        // `-ascii` is the option row's own name — spelled as a flag in the
        // DSL, so it paints as one (there is no positional/shape difference
        // a lexer-level classifier can use to tell "the row's name" from
        // "a switch this row declares").
        assert!(cs.contains(&("property-flag", "-ascii".to_owned())));
        assert!(cs.contains(&("property-flag", "-detail".to_owned())));
        // Inside the `-detail` value, `Compare`/`using`/`order.` are flat
        // prose words, not a nested command-word — and the `$x` inside it
        // still highlights as a variable.
        assert!(cs.contains(&("value", "Compare".to_owned())));
        assert!(!cs.contains(&("command-word", "Compare".to_owned())));
        assert!(cs.contains(&("variable", "$x".to_owned())));
    }

    #[test]
    fn hover_block_summary_is_a_command_word_but_its_body_is_flat() {
        let src =
            "command foo {\n    hover {\n        summary {Sort a list; keep going.}\n    }\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("command-word", "hover".to_owned())));
        assert!(cs.contains(&("command-word", "summary".to_owned())));
        // The embedded `;` must not turn "keep" into a second command-word —
        // `summary`'s own body is prose (`field_lookup("summary")` misses,
        // since `summary` is a `HoverSnippet` field, not a `CommandSpec`
        // one, so `body_mode` falls back to `Flat`).
        assert!(!cs.contains(&("command-word", "keep".to_owned())));
        assert!(cs.contains(&("value", "keep".to_owned())));
    }

    #[test]
    fn tokens_are_non_overlapping_and_sorted() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/design/spec-dsl-examples/lsort.tclspec"
        ))
        .expect("fixture reads");
        let toks = highlight_tokens(&src);
        assert!(!toks.is_empty());
        for pair in toks.windows(2) {
            assert!(
                pair[0].end <= pair[1].start,
                "overlap: {:?} vs {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn highlight_json_has_the_documented_shape() {
        let value = highlight_json("command foo {\n    arity 1..\n}\n");
        let array = value.as_array().expect("array");
        assert!(!array.is_empty());
        for entry in array {
            assert!(entry.get("start").is_some_and(Value::is_u64));
            assert!(entry.get("end").is_some_and(Value::is_u64));
            assert!(entry.get("class").is_some_and(Value::is_string));
        }
    }

    #[test]
    fn hover_finds_a_command_field_by_dsl_keyword() {
        let src = "command foo {\n    arity 1..\n}\n";
        let offset = src.find("arity").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        assert_eq!(hover["title"], json!("Arity"));
        assert!(hover["body"].as_str().unwrap().contains("wrong # args"));
    }

    #[test]
    fn hover_finds_options_field_from_the_singular_option_keyword() {
        let src = "command foo {\n    option -ascii -detail {x}\n}\n";
        let offset = src.find("option ").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        // `option` (DSL, singular) resolves via the plural `options` field.
        assert!(
            hover["body"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("flag")
        );
    }

    #[test]
    fn hover_finds_a_flag_via_the_version_field_alias() {
        let src = "command foo {\n    option -x -introduced 8.5\n}\n";
        let offset = src.find("-introduced").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        assert!(
            hover["body"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("version")
        );
    }

    #[test]
    fn hover_finds_a_grammar_keyword_from_the_static_table() {
        let src = "speclib mylib 1.0 {\n}\n";
        let offset = 0;
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        assert!(hover["title"].as_str().unwrap().starts_with("speclib"));
    }

    #[test]
    fn hover_finds_a_trait_catalogue_variant() {
        let src = "command foo {\n    traits {PURE}\n}\n";
        let offset = src.find("PURE").unwrap();
        // `PURE` classifies as `value`, not `command-word`/`property-flag`,
        // so hover intentionally has nothing to say about it — it is call
        // site (well, pack-author-site) data, matching the doc comment on
        // `hover_json`.
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(false));
    }

    #[test]
    fn hover_on_whitespace_is_not_found() {
        let src = "command foo {\n}\n";
        let hover = hover_json(src, "command".len());
        assert_eq!(hover["found"], json!(false));
    }

    #[test]
    fn hover_on_an_unknown_word_reports_not_found_with_the_word_quoted() {
        let src = "command foo {\n    zzz-not-a-real-field 1\n}\n";
        let offset = src.find("zzz").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(false));
        assert_eq!(hover["title"], json!("`zzz-not-a-real-field`"));
    }

    #[test]
    fn original_dsl_highlight_keeps_loader_file_prologue_and_body_content_spans() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        let source = "\u{feff}speclib demo 1.0 {\ncommand visible {arity 0}\n\u{feff}command hidden {arity 0}\n}\n";
        let tokens = highlight_tokens(source);
        assert!(tokens.iter().any(|token| token.class == "command-word"
            && token.text == "speclib"
            && token.start == '\u{feff}'.len_utf8()));
        assert!(
            tokens
                .iter()
                .any(|token| token.class == "command-word" && token.text == "command")
        );
        assert!(tokens.iter().any(|token| token.text == "\u{feff}command"));
        assert!(
            tokens
                .iter()
                .any(|token| token.class == "command-word" && token.text == "\u{feff}command")
        );
        for token in &tokens {
            assert_eq!(source[token.start..token.end], token.text);
        }
        let mut body = Vec::new();
        collect(
            "\u{feff}command generated {}",
            SourceSegmentationPurpose::NestedScript,
            0,
            0,
            Mode::Statement,
            &mut body,
        );
        assert!(
            body.iter()
                .any(|token| token.start == 0 && token.text == "\u{feff}command")
        );
        assert!(
            !body
                .iter()
                .any(|token| token.class == "command-word" && token.text == "command")
        );
    }
}

#[cfg(test)]
mod original_dsl_display_owner_tests {
    use super::*;

    #[test]
    fn original_name_container_is_not_the_statement_body() {
        let source = "command {arity 100} {arity 0}";
        let tokens = highlight_tokens(source);
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.class == "command-word" && token.text == "arity")
                .count(),
            1
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.class == "value" && token.text == "arity")
        );
        for token in &tokens {
            assert_eq!(&source[token.start..token.end], token.text);
        }
    }

    #[test]
    fn undeclared_plural_and_flag_suffixes_cannot_invent_dsl_metadata() {
        let source = "command foo {options {arity 0}; option -x -introduced_version 8.6}";
        let tokens = highlight_tokens(source);
        assert!(
            !tokens
                .iter()
                .any(|token| token.class == "command-word" && token.text == "arity")
        );
        let offset = source.find("-introduced_version").unwrap();
        assert_eq!(hover_json(source, offset)["found"], false);
        let source = "command foo {hover {summary {🙂 ; [not_a_dsl]}}; arity 0}";
        let tokens = highlight_tokens(source);
        assert!(
            !tokens
                .iter()
                .any(|token| token.class == "command-word" && token.text == "not_a_dsl")
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.class == "command-word" && token.text == "arity")
        );
        for token in &tokens {
            assert_eq!(&source[token.start..token.end], token.text);
        }
    }
    #[test]
    fn source_vocabulary_and_actual_nested_grammar_bound_help() {
        let source = "speclib demo 3.0 {command foo {arity 0}}";
        assert_eq!(
            hover_json(source, source.find("arity").unwrap())["found"],
            false
        );
        let source = "speclib demo 2.99 {command foo {arity 0}}";
        assert_eq!(
            hover_json(source, source.find("arity").unwrap())["found"],
            true
        );
        let source = "command foo {hover {arity 0}}";
        assert_eq!(
            hover_json(source, source.find("arity").unwrap())["found"],
            false
        );
    }
}
