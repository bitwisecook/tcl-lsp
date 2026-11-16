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

use crate::{INVALID_CHARACTER_IN_ARRAY_INDEX, MISSING_CLOSE_BRACE_FOR_VAR};

/// #2396: `[a][b]` starts with `[` and ends with `]` but is two
/// substitutions joined, not one.
#[test]
fn a_whole_command_substitution_closes_at_the_word_end() {
    let config = crate::lexer::LexerConfig::default();
    for whole in ["[a b]", "[a [b] c]", "[a {]} b]", "[a \"]\"]"] {
        assert!(
            super::is_whole_command_substitution(whole.as_bytes(), config),
            "{whole}"
        );
    }
    for not in ["[a][b]", "[a]x", "x[a]", "[a", "a", ""] {
        assert!(
            !super::is_whole_command_substitution(not.as_bytes(), config),
            "{not}"
        );
    }
}

#[test]
fn original_bracket_boundaries_keep_selected_variable_name_grammar() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original lexical geometry only; no variable read or command entry.
    let selected = LexerConfig {
        braced_var: BracedVarStyle::FirstClose,
        leading_bom: crate::LeadingBom::Skip,
        strict_quoting: true,
        ..LexerConfig::for_dialect("tcl9.1")
    };
    let operand = "[\u{feff}quote ${a{b}]";
    assert_eq!(
        decompose(operand.as_bytes(), SubstFlags::default(), selected),
        WordBody::Parts(vec![WordPart::Command("\u{feff}quote ${a{b}".as_bytes())])
    );
    for written in [operand.to_owned(), format!("\"{operand}\"")] {
        let source = format!(
            "\u{feff}interp alias {{}} \u{feff}quote {{}} regex::quote\nregexp {written} VALUE"
        );
        for image in [
            crate::SourceImage::document(&source),
            crate::SourceImage::native(source.as_bytes()),
        ] {
            let plan = crate::native_script_words_in(
                image.clone(),
                crate::Span::new(0, u32::try_from(image.len()).unwrap()),
                selected,
            )
            .unwrap();
            assert!(plan.fatal_tail.is_none());
            assert_eq!(plan.commands.len(), 2);
            let word = &plan.commands[1].words[1];
            // This is written source, including quotes, not its evaluated
            // value. C Tcl 8.6 info frame also retains this quote pair.
            assert_eq!(word.try_text().unwrap(), written);
            assert_eq!(word.image(), &image);
            assert_eq!(word.config(), selected);
        }
    }
    let nesting = LexerConfig {
        braced_var: BracedVarStyle::Tcl9Nesting,
        ..selected
    };
    assert!(matches!(
        decompose(operand.as_bytes(), SubstFlags::default(), nesting),
        WordBody::Parts(parts) if parts.iter().any(|part| matches!(part, WordPart::ParseError(_)))
    ));
    assert_eq!(
        decompose(b"[quote ${a]b}]", SubstFlags::default(), selected),
        WordBody::Parts(vec![WordPart::Command(b"quote ${a]b}")])
    );
}

#[test]
fn outer_substitution_mask_does_not_disable_array_index_commands() {
    // Source proof: naming.substitution.template-token-and-index-source
    // docs/design/analysis/name-resolution-proofs/substitution-template-token-and-index-source.md
    // Native proof: naming.substitution.counted-template-completions
    // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
    // The C v6 array-index control runs its command with outer commands
    // disabled. JimExpandDictSugar separately selects JIM_NONE.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = LexerConfig::for_dialect(dialect);
        let flags = SubstFlags {
            cmds: false,
            backslashes: false,
            ..SubstFlags::default()
        };
        let WordBody::Parts(parts) = decompose(br"$a($k\x21[side])", flags, config) else {
            panic!("one array reference");
        };
        let [
            WordPart::Variable(VarRef {
                index: Some(index), ..
            }),
        ] = parts.as_slice()
        else {
            panic!("retained original index");
        };
        assert!(matches!(index[0], WordPart::Variable(_)));
        assert!(matches!(&index[1], WordPart::Text(text) if text.as_ref() == b"!"));
        assert_eq!(index[2], WordPart::Command(b"side"));
        assert_eq!(
            decompose(
                br"$a($k\x21[side])",
                SubstFlags {
                    vars: false,
                    ..flags
                },
                config,
            ),
            WordBody::Literal(br"$a($k\x21[side])"),
        );
    }
}

/// An unterminated construct reports the *innermost* cause, as C does:
/// `subst {[list "abc}` is `missing "` and `subst {$a([foo)}` is
/// `missing close-bracket` on tclsh 8.6.16 and 9.0.4 — not the outer
/// `missing close-bracket` / `missing )`.
#[test]
fn an_unterminated_construct_reports_its_innermost_cause() {
    let config = LexerConfig::default();
    let flags = SubstFlags::default();
    let err = |src: &[u8]| match decompose(src, flags, config) {
        WordBody::Parts(parts) => parts.iter().find_map(|p| match p {
            WordPart::ParseError(m) => Some(*m),
            _ => None,
        }),
        WordBody::Literal(_) => None,
    };
    assert_eq!(err(br#"[list "abc"#), Some(MISSING_QUOTE));
    assert_eq!(err(b"[list {abc"), Some(MISSING_CLOSE_BRACE));
    assert_eq!(err(b"$a([foo)"), Some(MISSING_CLOSE_BRACKET));
    // A bracket whose words all close still reports the missing bracket,
    // and a closed one is no error at all.
    assert_eq!(err(br#"[list "abc" def"#), Some(MISSING_CLOSE_BRACKET));
    assert_eq!(err(br#"[list "abc"]"#), None);
    assert_eq!(err(b"$a(plain"), Some(MISSING_PAREN));
    // An incomplete substitution *inside* a quoted word is the bracket's
    // error, not the quote's, and a `"` inside a comment opens nothing.
    // tclsh 9.0.4 answers `missing close-bracket` for both.
    assert_eq!(err(br#"[list "[foo"#), Some(MISSING_CLOSE_BRACKET));
    assert_eq!(
        err(br#"[list
# a " comment
"#),
        Some(MISSING_CLOSE_BRACKET)
    );
    // An escaped byte is word content, so the `#` after `\;` is data (the
    // `{` then opens a word) and the `"` after the escaped blank is inside
    // the `foo` word, not a word opener. tclsh 9.0.4 answers
    // `missing close-brace` and `missing close-bracket`.
    assert_eq!(err(br"[\;# {"), Some(MISSING_CLOSE_BRACE));
    assert_eq!(err(br#"[foo\ "abc"#), Some(MISSING_CLOSE_BRACKET));
}

/// The walk over an unterminated bracket is iterative: a template of many
/// unmatched `[` must return C's catchable error, not exhaust the native
/// stack. Recursing per bracket aborted the process instead.
#[test]
fn many_unmatched_brackets_do_not_exhaust_the_stack() {
    let src = vec![b'['; 100_000];
    assert_eq!(
        decompose(&src, SubstFlags::default(), LexerConfig::default()),
        WordBody::Parts(vec![WordPart::ParseError(MISSING_CLOSE_BRACKET)])
    );
}

/// A compiled word's unclosed `[` is data, not `missing close-bracket`:
/// the codegen decoded the source's `\[` to a bare `[` before the VM saw
/// it, so re-raising C's parse error breaks `expr {$ch eq "\["}`
/// (tclsh 8.6.16 / 9.0.4 both answer with the comparison, not an error).
/// A source word keeps the error.
#[test]
fn an_unclosed_bracket_is_data_only_for_a_compiled_word() {
    let config = LexerConfig::default();
    let compiled = SubstFlags::compiled_word();
    // Nothing substituted after all, so both collapse to a borrowed
    // `Literal` by the `TCL_TOKEN_SIMPLE_WORD` rule.
    assert_eq!(decompose(b"[", compiled, config), WordBody::Literal(b"["));
    assert_eq!(
        decompose(b"a[b", compiled, config),
        WordBody::Literal(b"a[b")
    );
    // A closed bracket still substitutes, and text after an unclosed one
    // is not emitted twice.
    assert_eq!(
        decompose(b"x[y]z[w", compiled, config),
        WordBody::Parts(vec![
            WordPart::Text(Cow::Borrowed(b"x")),
            WordPart::Command(b"y"),
            WordPart::Text(Cow::Borrowed(b"z[w")),
        ])
    );
    // A source word keeps C's parse error.
    assert_eq!(
        decompose(b"a[b", SubstFlags::default(), config),
        WordBody::Parts(vec![
            WordPart::Text(Cow::Borrowed(b"a")),
            WordPart::ParseError(MISSING_CLOSE_BRACKET),
        ])
    );
}
use std::fmt::Write as _;

use super::*;
use tcl_dialect::{ArrayIndexSyntax, BracedVarStyle};

/// The oracle sheet behind the expectations in this module. Every message
/// and every release-split reading below was taken from running these
/// lines under `tclsh9.0` (9.0.4) and `tclsh8.6` (8.6.16); paste the sheet
/// into either interpreter to re-derive them without this harness.
///
/// ```text
/// % set a hi ; set arr(k) v
/// % subst {${a{b}c}}      9.0: can't read "a{b}c"   8.6: can't read "a{b"
/// % subst {${a\}b}}       9.0: can't read "a\}b"    8.6: can't read "a\"
/// % subst {${a}           both: missing close-brace for variable name
/// % subst {$arr({k})}     9.0: invalid character in array index
/// %                       8.6: can't read "arr({k})": no such element…
/// % subst {x[b}           both: missing close-bracket
/// % subst {$arr(}         both: missing )
/// % eval {list a "b}      both: missing "
/// % subst {[list {a]b}]}  both: a\]b          (the `]` in braces is inert)
/// % subst {[list "a]b"]}  both: a\]b          (…and in a quoted word)
/// % subst "\[list a\n# ]\nb]"
/// %                       both: invalid command name "b"
/// %                                           (…and in a comment)
/// % subst {price$ x}      both: price$ x      (a `$` with no name is data)
/// % subst {\$a}           both: $a
/// ```
const ORACLE_SHEET: () = ();

fn config(braced_var: BracedVarStyle, array_index: ArrayIndexSyntax) -> LexerConfig {
    LexerConfig {
        braced_var,
        array_index,
        ..LexerConfig::default()
    }
}

fn nine() -> LexerConfig {
    config(BracedVarStyle::Tcl9Nesting, ArrayIndexSyntax::Tcl9)
}

fn eight() -> LexerConfig {
    config(BracedVarStyle::FirstClose, ArrayIndexSyntax::Tcl8)
}

fn parts(src: &[u8], cfg: LexerConfig) -> Vec<WordPart<'_>> {
    match decompose(src, SubstFlags::default(), cfg) {
        WordBody::Parts(p) => p,
        WordBody::Literal(b) => panic!("expected parts, got literal {b:?}"),
    }
}

fn text(bytes: &[u8]) -> WordPart<'_> {
    WordPart::Text(Cow::Borrowed(bytes))
}

fn scalar(name: &[u8]) -> WordPart<'_> {
    WordPart::Variable(VarRef { name, index: None })
}

#[test]
fn original_name_extent_requires_the_actual_source_allocation() {
    // naming.inlining.original-frame-source-context
    // docs/design/analysis/name-resolution-proofs/inlining-original-frame-source-context.md
    // Exact lexical extent only; no Native name, frame or edit grant.
    for original in ["${café}", "${$literal}", "${arr(key)}", "${}", "$arr($key)"] {
        let source = original.as_bytes().to_vec();
        let reference = scan_var_ref(&source, 0, eight()).unwrap().unwrap();
        let range = reference.name_range_in(&source).unwrap();
        assert_eq!(&source[range], reference.name);
        let equal_source = source.clone();
        assert!(reference.name_range_in(&equal_source).is_none());
    }
    let source = "prefix${café}suffix".as_bytes().to_vec();
    let reference = scan_var_ref(&source, 6, eight()).unwrap().unwrap();
    assert_eq!(reference.name_range_in(&source), Some(8..13));
}

#[test]
fn a_word_with_no_trigger_is_a_borrowed_literal() {
    let src = b"plainword";
    let WordBody::Literal(got) = decompose(src, SubstFlags::default(), nine()) else {
        panic!("expected the literal fast path");
    };
    // Zero-copy: the same allocation, not a copy of it. This is the
    // property `parse_cache` (memory-management.md MM-B.6) rests on.
    assert!(std::ptr::eq(got.as_ptr(), src.as_ptr()));
}

#[test]
fn text_variable_and_command_split_out() {
    assert_eq!(
        parts(b"x${name}y", nine()),
        vec![text(b"x"), scalar(b"name"), text(b"y")]
    );
    assert_eq!(
        parts(b"a[clock seconds]b", nine()),
        vec![text(b"a"), WordPart::Command(b"clock seconds"), text(b"b")]
    );
    assert_eq!(
        parts(b"$arr($i)", nine()),
        vec![WordPart::Variable(VarRef {
            name: b"arr",
            index: Some(vec![scalar(b"i")]),
        })]
    );
}

/// A `$` that is not followed by `{` or a name byte is literal text
/// (`Tcl_ParseVarName`): `subst {price$ x}` is `price$ x` on both oracles.
#[test]
fn a_dollar_with_no_name_is_data() {
    assert_eq!(
        decompose(b"price$ x", SubstFlags::default(), nine()),
        WordBody::Literal(b"price$ x")
    );
    // …and an escaped `$` is data too — the escape is folded into the run,
    // not left as a separate part (`subst {\$a}` is `$a`).
    assert_eq!(parts(b"\\$a", nine()), vec![text(b"$a")]);
    assert_eq!(parts(b"\\[a]", nine()), vec![text(b"[a]")]);
}

/// `Tcl_ParseVarName`'s `${…}` close rule differs
/// between the 8.x family (first literal `}`) and 9.x (brace nesting, with
/// `\X` inert), and the difference is user-visible in the *name* read.
#[test]
fn braced_var_close_rule_follows_the_release() {
    assert_eq!(parts(b"${a{b}c}", nine()), vec![scalar(b"a{b}c")]);
    assert_eq!(
        parts(b"${a{b}c}", eight()),
        vec![scalar(b"a{b"), text(b"c}")]
    );
    assert_eq!(parts(b"${a\\}b}", nine()), vec![scalar(b"a\\}b")]);
    // 8.x closes at the first `}`; the rest of the template is text, and
    // its escapes decode with it (`\}` → `}`).
    assert_eq!(
        parts(b"${a\\}b}", eight()),
        vec![scalar(b"a\\"), text(b"b}")]
    );
}

/// An unterminated `${…}` is C's error, not a name that runs to end of
/// input. The 9.x nesting rule also *widens* what counts as unterminated.
#[test]
fn unterminated_braced_var_is_a_parse_error() {
    let err = WordPart::ParseError(MISSING_CLOSE_BRACE_FOR_VAR);
    for cfg in [nine(), eight()] {
        assert_eq!(parts(b"${abc", cfg), vec![err.clone()]);
    }
    assert_eq!(parts(b"${a{b}", nine()), vec![err.clone()]);
    assert_eq!(parts(b"${a{b}", eight()), vec![scalar(b"a{b")]);
}

/// Tcl 9 rejects raw `{`, `"`, `(`, `}` written in an
/// array index; Tcl 8 passes them through. The mask applies to *source*
/// bytes only — an escape or a substitution result is legal on both.
#[test]
fn array_index_source_mask_follows_the_release() {
    assert_eq!(
        parts(b"$arr({k})", nine()),
        vec![WordPart::ParseError(INVALID_CHARACTER_IN_ARRAY_INDEX)]
    );
    assert_eq!(
        parts(b"$arr({k})", eight()),
        vec![WordPart::Variable(VarRef {
            name: b"arr",
            index: Some(vec![text(b"{k}")]),
        })]
    );
    for src in [&b"$a(\\{k\\})"[..], b"$a(${k})", b"$a([format \\{])"] {
        assert!(
            !parts(src, nine())
                .iter()
                .any(|p| matches!(p, WordPart::ParseError(_))),
            "Tcl 9 accepts escaped/substituted index source: {src:?}"
        );
    }
}

/// The three unterminated forms C names, with its exact messages. The
/// parts scanned *before* the failure are kept: `subst {[side][b}` runs
/// `side` and then reports `missing close-bracket` on both oracles.
#[test]
fn unterminated_constructs_carry_c_tcls_exact_message() {
    assert_eq!(
        parts(b"x[b", nine()),
        vec![text(b"x"), WordPart::ParseError(MISSING_CLOSE_BRACKET)]
    );
    assert_eq!(
        parts(b"[side][b", nine()),
        vec![
            WordPart::Command(b"side"),
            WordPart::ParseError(MISSING_CLOSE_BRACKET)
        ]
    );
    assert_eq!(
        parts(b"$arr(", nine()),
        vec![WordPart::ParseError(MISSING_PAREN)]
    );
    // A substituted `[…]` is a script, so C recurses into it rather than
    // hunting for the `]`: an error inside an unterminated bracket is what
    // surfaces. `subst [format {[set y $%sa%sb]} "{" "{"]` reports
    // `missing close-brace for variable name` on both oracles.
    assert_eq!(
        parts(b"[set y ${a{b]", nine()),
        vec![WordPart::ParseError(MISSING_CLOSE_BRACE_FOR_VAR)]
    );
    assert_eq!(MISSING_QUOTE, "missing \"");
    assert_eq!(MISSING_CLOSE_BRACE, "missing close-brace");
}

/// The `]` search is brace-, quote- and comment-aware, because the
/// substituted text is a *script*. Each of the three private copies this
/// module replaced got at least one of these wrong.
#[test]
fn the_bracket_search_respects_braces_quotes_and_comments() {
    assert_eq!(
        parts(b"[list {a]b}]", nine()),
        vec![WordPart::Command(b"list {a]b}")]
    );
    assert_eq!(
        parts(b"[list \"a]b\"]", nine()),
        vec![WordPart::Command(b"list \"a]b\"")]
    );
    assert_eq!(
        parts(b"[list a\n# ]\nb]", nine()),
        vec![WordPart::Command(b"list a\n# ]\nb")]
    );
    // Nesting and `\]` still work.
    assert_eq!(
        parts(b"[a [b] c]", nine()),
        vec![WordPart::Command(b"a [b] c")]
    );
    assert_eq!(parts(b"[a\\]b]", nine()), vec![WordPart::Command(b"a\\]b")]);
}

/// A literal run decodes under the emulated release's escape grammar:
/// TIP 388 capped `\x` at two hex digits from 8.6 and added
/// `\U`, so `\x4142` is `B` under 8.5 and `A42` from 8.6.
#[test]
fn literal_runs_decode_under_the_releases_escape_grammar() {
    let with = |escapes| LexerConfig { escapes, ..nine() };
    for (escapes, hex, wide) in [
        (EscapeSyntax::Tcl84, &b"B"[..], &b"U0001F600"[..]),
        (EscapeSyntax::Tcl86, b"A42", "\u{FFFD}".as_bytes()),
        (EscapeSyntax::Tcl90, b"A42", "\u{1F600}".as_bytes()),
    ] {
        assert_eq!(parts(b"\\x4142", with(escapes)), vec![text(hex)]);
        assert_eq!(parts(b"\\U0001F600", with(escapes)), vec![text(wide)]);
    }
}

/// Each substitution kind switches independently — `subst`'s `-no*`
/// options. With backslashes off a run borrows through undecoded.
#[test]
fn each_substitution_kind_switches_independently() {
    let no_vars = SubstFlags {
        vars: false,
        ..SubstFlags::default()
    };
    assert_eq!(
        decompose(b"$x\\t", no_vars, nine()),
        WordBody::Parts(vec![text(b"$x\t")])
    );
    let no_bs = SubstFlags {
        backslashes: false,
        ..SubstFlags::default()
    };
    assert_eq!(
        decompose(b"a\\tb$x", no_bs, nine()),
        WordBody::Parts(vec![text(b"a\\tb"), scalar(b"x")])
    );
}

/// The compiled-word flavour (`tcl-vm`'s `PUSH` operands): the compiler
/// has already inlined or normalised every real variable reference, so a
/// surviving bare `$` is data while `${…}` and `[…]` still substitute.
#[test]
fn compiled_word_flags_keep_a_bare_dollar_literal() {
    assert_eq!(
        decompose(b"x$y", SubstFlags::compiled_word(), nine()),
        WordBody::Literal(b"x$y")
    );
    assert_eq!(
        decompose(b"x${y}", SubstFlags::compiled_word(), nine()),
        WordBody::Parts(vec![text(b"x"), scalar(b"y")])
    );
    // …and the escapes of that literal run are still decoded:
    // `string length "x\$y"` is 3, not 4.
    assert_eq!(
        decompose(b"x\\$y", SubstFlags::compiled_word(), nine()),
        WordBody::Parts(vec![text(b"x$y")])
    );
}

/// The extents [`decompose_spanned`] reports are the raw source runs:
/// a decoded text run keeps its undecoded width, a reference covers its
/// closer, and a parse error runs to the end of the source (where C
/// stopped). Nested index components carry no extents of their own.
#[test]
fn spanned_parts_carry_their_raw_extents() {
    let src = b"a\\tb${x}[c]$arr($i)\\$z";
    let spanned = decompose_spanned(src, SubstFlags::default(), nine());
    let extents: Vec<(usize, usize)> = spanned.iter().map(|p| (p.start, p.end)).collect();
    assert_eq!(extents, vec![(0, 4), (4, 8), (8, 11), (11, 19), (19, 22)]);
    assert_eq!(&src[4..8], b"${x}");
    assert_eq!(&src[8..11], b"[c]");
    assert_eq!(&src[11..19], b"$arr($i)");
    assert_eq!(spanned[0].part, text(b"a\tb"));
    assert_eq!(spanned[4].part, text(b"$z"));
    // Every extent tiles the source without gaps.
    assert!(spanned.windows(2).all(|w| w[0].end == w[1].start));

    let stopped = decompose_spanned(b"x[side][b", SubstFlags::default(), nine());
    assert_eq!(
        stopped.last().map(|p| (p.part.clone(), p.start, p.end)),
        Some((WordPart::ParseError(MISSING_CLOSE_BRACKET), 7, 9))
    );
    // The literal fast path is one borrowed run over the whole source.
    assert_eq!(
        decompose_spanned(b"plain", SubstFlags::default(), nine()),
        vec![SpannedPart {
            part: text(b"plain"),
            start: 0,
            end: 5,
            error_term: None,
            error_kind: None,
        }]
    );
}

#[test]
fn quoted_word_close_reports_missing_quote() {
    assert_eq!(quoted_word_close("\"abc\" rest", 0), Ok(4));
    assert_eq!(quoted_word_close("\"abc", 0), Err(MISSING_QUOTE));
    // A `"` inside a complete `[…]` of the word does not close it.
    assert_eq!(quoted_word_close("\"a[foo \"b\"]c\"", 0), Ok(12));
}

#[test]
fn whole_variable_references_follow_native_empty_name_and_jim_grammar() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let config = LexerConfig::for_dialect(dialect);
        let empty_array = whole_var_ref(b"$(k)", config).unwrap().unwrap();
        assert_eq!(empty_array.name, b"");
        assert_eq!(empty_array.index, Some(b"k".as_slice()));
        assert_eq!(
            parts(b"$(k)", config),
            vec![WordPart::Variable(VarRef {
                name: b"",
                index: Some(vec![text(b"k")]),
            })]
        );
        assert_eq!(
            decompose("$é".as_bytes(), SubstFlags::default(), config),
            WordBody::Literal("$é".as_bytes())
        );
        assert_eq!(whole_var_ref(b"${}", config).unwrap().unwrap().name, b"");
        assert!(whole_var_ref("$café".as_bytes(), config).unwrap().is_none());
        if matches!(dialect, "tcl9.0" | "tcl9.1") {
            assert_eq!(
                whole_var_ref(b"$a(k(x))", config),
                Err(INVALID_CHARACTER_IN_ARRAY_INDEX)
            );
        } else {
            assert!(whole_var_ref(b"$a(k(x))", config).unwrap().is_none());
        }
        assert!(whole_var_ref(b"$a(k)suffix", config).unwrap().is_none());
    }
    let jim = LexerConfig::for_dialect("jim");
    assert!(jim.var_syntax.has_expr_sugar());
    assert_eq!(
        parts("$é".as_bytes(), jim),
        vec![WordPart::Variable(VarRef {
            name: "é".as_bytes(),
            index: None,
        })]
    );
    assert!(whole_var_ref(b"$(1+2)", jim).unwrap().is_none());
    assert_eq!(
        whole_var_ref("$café".as_bytes(), jim)
            .unwrap()
            .unwrap()
            .name,
        "café".as_bytes()
    );
    assert_eq!(
        whole_var_ref(b"$a(k(x))", jim).unwrap().unwrap().index,
        Some(b"k(x)".as_slice())
    );
    assert_eq!(whole_var_ref(b"${(k)}", jim).unwrap().unwrap().name, b"(k)");
    assert!(whole_var_ref(b"$a(k", jim).is_err());
}

#[test]
fn checked_decomposition_keeps_index_capacity_distinct_from_parse_errors() {
    let mut source = Vec::new();
    for _ in 0..80 {
        source.extend_from_slice(b"$a(");
    }
    source.extend_from_slice(b"[compileMe]");
    source.extend(std::iter::repeat_n(b')', 80));
    assert_eq!(
        decompose_spanned_checked(&source, SubstFlags::default(), nine()),
        Err(DecompositionUnavailable::IndexNesting),
    );
    // The advisory API retains its documented bounded projection.
    assert_ne!(
        decompose_spanned(&source, SubstFlags::default(), nine()),
        [] as [SpannedPart<'_>; 0]
    );
    let malformed = decompose_spanned_checked(b"$a(", SubstFlags::default(), nine()).unwrap();
    assert!(
        malformed
            .iter()
            .any(|part| matches!(part.part, WordPart::ParseError(_)))
    );
}

#[test]
fn scan_var_ref_leaves_the_index_raw() {
    let cfg = nine();
    let got = scan_var_ref(b"$arr($i)x", 0, cfg).unwrap().unwrap();
    assert_eq!(got.name, b"arr");
    assert_eq!(got.index, Some(&b"$i"[..]));
    assert_eq!(got.next, 8);
    // Colon runs belong to the name (`namespace` separators).
    let got = scan_var_ref(b"$a:::b rest", 0, cfg).unwrap().unwrap();
    assert_eq!(got.name, b"a:::b");
    assert_eq!(got.next, 6);
    // Not a reference at all.
    assert_eq!(scan_var_ref(b"$ x", 0, cfg), Ok(None));
    // A trailing `$` has no name behind it, so it is data, not an error.
    assert_eq!(scan_var_ref(b"a$", 1, cfg), Ok(None));
}

#[test]
fn raw_index_geometry_keeps_empty_and_foreign_source_distinct() {
    // Implementation contract: naming.grammar.checked-expression-substitution-context
    // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md

    let source = b"$a()".to_vec();
    let reference = scan_var_ref(&source, 0, nine()).unwrap().unwrap();
    assert_eq!(reference.index_range_in(&source), Some(3..3));
    let equal_source = source.clone();
    assert_eq!(reference.index_range_in(&equal_source), None);

    let source = "$café($clé)".as_bytes();
    let mut config = nine();
    config.var_syntax = tcl_dialect::VarSyntax::Jim;
    let reference = scan_var_ref(source, 0, config).unwrap().unwrap();
    let range = reference.index_range_in(source).unwrap();
    assert_eq!(&source[range], "$clé".as_bytes());
}

/// The index parse recurses once per
/// `$name(index)` level, reachable from ordinary `subst` with no special
/// syntax. The same construct overflowed a 256 KiB native stack between
/// depth 100-150. Past `MAX_INDEX_DEPTH` the index is kept as literal
/// text; the assertion is that the scan returns at all.
#[test]
fn deeply_nested_array_index_survives() {
    const DEPTH: usize = 5000;
    let mut src = String::from("$a0");
    for i in 0..DEPTH {
        src.push('(');
        write!(src, "$a{}", i + 1).expect("writing to a String cannot fail");
    }
    src.push('1');
    for _ in 0..DEPTH {
        src.push(')');
    }
    let _ = decompose(src.as_bytes(), SubstFlags::default(), nine());
}

/// …while realistic nesting is still scanned in full: the `)` search steps
/// over a nested `$name(…)`'s own parens, so each level closes on its own
/// (`set c(1) inner; set b(inner) mid; set a(mid) outer; set x $a($b($c(1)))`
/// yields `outer` under `tclsh9.0`).
#[test]
fn moderate_nesting_is_scanned_in_full() {
    assert_eq!(
        parts(b"$a($b($c(1)))", nine()),
        vec![WordPart::Variable(VarRef {
            name: b"a",
            index: Some(vec![WordPart::Variable(VarRef {
                name: b"b",
                index: Some(vec![WordPart::Variable(VarRef {
                    name: b"c",
                    index: Some(vec![text(b"1")]),
                })]),
            })]),
        })]
    );
}

#[test]
fn oracle_sheet_is_recorded() {
    let () = ORACLE_SHEET;
}
