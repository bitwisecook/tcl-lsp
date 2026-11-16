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

#[test]
fn original_image_segmentation_keeps_channel_and_unavailable_text_distinct() {
    let source = "set a A\\\r\nset b B";
    let config = LexerConfig::default();
    let native = tcl_lexer::SourceImage::native(source.as_bytes());
    let document = tcl_lexer::SourceImage::document(source);
    let native_commands =
        segment_commands_image_with_offset_and_config(&native, 17, config).unwrap();
    let document_commands =
        segment_commands_image_with_offset_and_config(&document, 17, config).unwrap();
    assert_eq!(native_commands.len(), 2);
    assert_eq!(document_commands.len(), 1);
    assert_eq!(native_commands[0].span.start(), 17);
    assert_eq!(native_commands[1].texts, ["set", "b", "B"]);
    assert!(
        segment_commands_image_with_offset_and_config(
            &tcl_lexer::SourceImage::native(b"set \xff V".as_slice()),
            0,
            config,
        )
        .is_none()
    );
    assert_eq!(
        segment_commands_image_with_offset_and_config(
            &tcl_lexer::SourceImage::native(b"".as_slice()),
            0,
            config,
        )
        .unwrap(),
        []
    );
}

use super::*;
use tcl_core_types::DiagCode;

#[test]
fn flatten_clause_list_uses_list_offsets_for_comment_and_separator_patterns() {
    for pattern in ["#", ";"] {
        let source = format!("expect {{{pattern} {{helper}} default {{other}}}}");
        let body_start = source.find('{').unwrap();
        let body_end = source.len() - 1;
        let body = &source[(body_start + 1)..body_end];
        let token = Token::with_content_offset(
            TokenType::Str,
            Span::new(
                u32::try_from(body_start).unwrap(),
                u32::try_from(body_end).unwrap(),
            ),
            1,
        );

        let elements =
            flatten_clause_list_elements_with_config(&source, body, token, LexerConfig::default());
        assert_eq!(
            elements
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<Vec<_>>(),
            vec![pattern, "helper", "default", "other"]
        );
        assert_eq!(
            elements[1].1.span,
            Span::new(
                u32::try_from(body_start + 3).unwrap(),
                u32::try_from(body_start + 10).unwrap(),
            )
        );
        assert_eq!(elements[1].1.kind, TokenType::Str);
    }
}

#[test]
fn flatten_clause_list_keeps_an_unbraced_action_as_one_span() {
    let source = "expect {ready puts;expect_semicolon_unknown}";
    let body_start = source.find('{').unwrap();
    let body_end = source.len() - 1;
    let body = &source[(body_start + 1)..body_end];
    let token = Token::with_content_offset(
        TokenType::Str,
        Span::new(
            u32::try_from(body_start).unwrap(),
            u32::try_from(body_end).unwrap(),
        ),
        1,
    );

    let elements =
        flatten_clause_list_elements_with_config(source, body, token, LexerConfig::default());
    assert_eq!(
        elements
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["ready", "puts;expect_semicolon_unknown"]
    );
    let action_start = source.find("puts;").unwrap();
    assert_eq!(
        elements[1].1.span,
        Span::new(
            u32::try_from(action_start).unwrap(),
            u32::try_from(body_end).unwrap(),
        ),
        "the action token must cover the complete list element, not only `puts`"
    );
}

/// `(start, end, texts, is_partial, token-spans)` — the fields that
/// define a command's identity for incremental-vs-full comparison.
type CmdProjection = (u32, u32, Vec<String>, bool, Vec<(u32, u32)>);

fn project(c: &SegmentedCommand) -> CmdProjection {
    (
        c.span.start(),
        c.span.end(),
        c.texts.clone(),
        c.is_partial,
        c.all_tokens
            .iter()
            .map(|t| (t.span.start(), t.span.end()))
            .collect(),
    )
}

fn projected(cmds: &[SegmentedCommand]) -> Vec<CmdProjection> {
    cmds.iter().map(project).collect()
}

struct Lcg(u64);
impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 33) as u32
    }
}

#[test]
fn original_incremental_suffix_keeps_whole_file_and_midfile_bom_purposes() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original Document command geometry only; no entered/native source grant.
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl9.1").unwrap();
    for leading_bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let config = tcl_lexer::LexerConfig {
            leading_bom,
            ..input.lexer_config()
        };
        let old = "\u{feff}set prefix OLD\n\u{feff}unknown old";
        let new = "\u{feff}set prefix OLD\n\u{feff}unknown new";
        let previous = segment_commands_with_offset_and_config(old, 0, config);
        assert_eq!(previous.len(), 2);
        assert!(previous[1].span.start() > 0);
        let incremental = segment_commands_incremental(old, &previous, new, config);
        let complete = segment_commands_with_offset_and_config(new, 0, config);
        assert_eq!(projected(&incremental), projected(&complete));
        assert_eq!(incremental[1].texts[0], "\u{feff}unknown");
        assert_eq!(
            new.get(incremental[1].argv[0].span.as_range()),
            Some("\u{feff}unknown")
        );

        // An edit before the first retained command actually reparses the
        // whole input, and must keep its explicitly selected file policy.
        let old = "set prefix OLD\nputs tail";
        let new = "\u{feff}set prefix OLD\nputs tail";
        let previous = segment_commands_with_offset_and_config(old, 0, config);
        assert_eq!(previous[0].span.start(), 0);
        let incremental = segment_commands_incremental(old, &previous, new, config);
        assert_eq!(
            projected(&incremental),
            projected(&segment_commands_with_offset_and_config(new, 0, config))
        );
        assert_eq!(
            incremental[0].texts[0],
            if leading_bom == tcl_lexer::LeadingBom::Skip {
                "set"
            } else {
                "\u{feff}set"
            }
        );
    }
    // Genuine non-BOM grammar override remains selected in the suffix.
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..input.lexer_config()
    };
    let old = "puts prefix\n\u{feff}puts ${name{inner}} OLD";
    let new = "puts prefix\n\u{feff}puts ${name{inner}} NEW";
    let previous = segment_commands_with_offset_and_config(old, 0, config);
    assert_eq!(
        projected(&segment_commands_incremental(old, &previous, new, config)),
        projected(&segment_commands_with_offset_and_config(new, 0, config))
    );
}

#[test]
fn segment_commands_incremental_pins() {
    // Edit inside one command.
    let old = "set x 1\nputs hi\nset y 2\n";
    let new = "set x 1\nputs bye\nset y 2\n";
    assert_eq!(
        projected(&segment_commands_incremental(
            old,
            &segment_commands(old),
            new,
            tcl_lexer::LexerConfig::default()
        )),
        projected(&segment_commands(new)),
    );
    // Insert a whole new command.
    let new2 = "set x 1\nputs hi\nincr n\nset y 2\n";
    assert_eq!(
        projected(&segment_commands_incremental(
            old,
            &segment_commands(old),
            new2,
            tcl_lexer::LexerConfig::default()
        )),
        projected(&segment_commands(new2)),
    );
    // Delete a command.
    let new3 = "set x 1\nset y 2\n";
    assert_eq!(
        projected(&segment_commands_incremental(
            old,
            &segment_commands(old),
            new3,
            tcl_lexer::LexerConfig::default()
        )),
        projected(&segment_commands(new3)),
    );
    // Append at the end.
    let new4 = "set x 1\nputs hi\nset y 2\nputs done\n";
    assert_eq!(
        projected(&segment_commands_incremental(
            old,
            &segment_commands(old),
            new4,
            tcl_lexer::LexerConfig::default()
        )),
        projected(&segment_commands(new4)),
    );
}

#[test]
#[ignore = "seeded edit-storm oracle (3000 edit/segment pairs); the deterministic \
                incremental gates stay in CI — segment_commands_incremental_pins and the \
                *_incremental_matches_fresh* family here, and tcl-lexer's \
                differential_boundaries for the reparse split points; run explicitly \
                with --ignored"]
fn segment_commands_incremental_matches_full_under_fuzz() {
    // Differential acceptance gate: a random edit applied to a base
    // document, then incremental re-segmentation must byte-for-byte
    // match a full re-segmentation of the edited text.
    let bases = [
        "set x 1\nputs hi\nset y 2\nproc p {} { return 1 }\n",
        "if {$a} {\n  puts a\n} else {\n  puts b\n}\nputs done\n",
        "namespace eval n {\n  variable v 1\n  proc q {} {}\n}\nn::q\n",
        "set s {a\nb\nc}\nputs $s\nset t \"x;y\"\n",
        "# comment\nset x [expr {1 + 2}]\nputs $x\n",
    ];
    let inserts = [
        "", "z", "puts Z\n", "}", "{", "\n", "[a]", "; ", "\"q\"", "\\",
    ];
    let mut rng = Lcg(0xC0FF_EE12_3456_789A);
    let mut checked = 0usize;
    for base in bases {
        let old_cmds = segment_commands(base);
        let blen = base.len();
        for _ in 0..600 {
            // Random byte range [s, e) on char boundaries.
            let mut s = (rng.next_u32() as usize) % (blen + 1);
            while !base.is_char_boundary(s) {
                s -= 1;
            }
            let mut e = s + (rng.next_u32() as usize) % (blen + 1 - s);
            while !base.is_char_boundary(e) {
                e += 1;
            }
            let ins = inserts[(rng.next_u32() as usize) % inserts.len()];
            let new = format!("{}{}{}", &base[..s], ins, &base[e..]);
            let inc = segment_commands_incremental(
                base,
                &old_cmds,
                &new,
                tcl_lexer::LexerConfig::default(),
            );
            let full = segment_commands(&new);
            assert_eq!(
                projected(&inc),
                projected(&full),
                "incremental != full for base {base:?} edit [{s},{e}) ins {ins:?} -> {new:?}",
            );
            checked += 1;
        }
    }
    assert!(checked > 2000, "fuzz corpus too small: {checked}");
}

#[test]
fn empty_source() {
    assert_eq!(
        segment_commands(""),
        [] as [crate::segmenter::SegmentedCommand; 0]
    );
}

#[test]
fn recovery_splits_swallowed_following_command() {
    // An unterminated `[` whose next line is a
    // known command re-lexes into a clean two-command stream and
    // yields an E201 diagnostic.
    let reg = tcl_registry::CommandRegistry::build_default();
    let known: crate::analyser::utils::RecoveryKnownCommands =
        reg.command_names().map(str::to_owned).collect();
    let cfg = LexerConfig::default();
    let (rec, diags) = segment_with_recovery("set x [foo bar\nputs done\n", cfg, &known);
    assert_eq!(rec.len(), 2);
    assert_eq!(rec[0].texts, vec!["set", "x", "[foo bar]"]);
    assert_eq!(rec[1].texts, vec!["puts", "done"]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagCode::E201);
}

#[test]
fn recovery_is_a_noop_on_clean_input() {
    let reg = tcl_registry::CommandRegistry::build_default();
    let known: crate::analyser::utils::RecoveryKnownCommands =
        reg.command_names().map(str::to_owned).collect();
    let cfg = LexerConfig::default();
    let src = "set ok [foo]\nputs hi\n";
    let (rec, diags) = segment_with_recovery(src, cfg, &known);
    let plain = segment_commands(src);
    // No recoverable imbalance → byte-identical words, no diagnostics.
    let words = |cs: &[SegmentedCommand]| cs.iter().map(|c| c.texts.clone()).collect::<Vec<_>>();
    assert_eq!(words(&rec), words(&plain));
    assert_eq!(diags, [] as [crate::analyser::types::Diagnostic; 0]);
}

#[test]
fn config_variant_threads_dialect_expand_syntax() {
    // The `_and_config` variants let
    // the analyser segment under a document's dialect.  `{*}$x` is the
    // expansion operator on 8.5+ (the default) but a literal brace
    // word on 8.4 / iRules, so the dialect flag must reach the lexer.
    // 8.5+ default → word 1 is an expanded `$x`.
    let on = segment_commands_with_offset_and_config("cmd {*}$x", 0, LexerConfig::default());
    assert_eq!(on.len(), 1);
    assert_eq!(on[0].expand_word, Some(vec![false, true]));
    // 8.4 → `{*}` is literal, so the word is an ordinary composite and
    // no expansion is recorded.
    let off =
        segment_commands_with_offset_and_config("cmd {*}$x", 0, LexerConfig::for_dialect("tcl8.4"));
    assert_eq!(off.len(), 1);
    assert!(off[0].expand_word.is_none());
    // The recovery variant threads the same config to both the initial
    // pass and any recovered tail.
    let kc: std::collections::HashSet<&str> = ["cmd"].into_iter().collect();
    let rec = segment_commands_with_recovery_and_config(
        "cmd {*}$x",
        &kc,
        LexerConfig::for_dialect("tcl8.4"),
    );
    assert!(rec[0].expand_word.is_none());
}

#[test]
fn config_variant_threads_irules_brace_separator() {
    // The other dialect flag carried by
    // `LexerConfig::for_dialect` — iRules injects a zero-width SEP at a
    // `}{` brace boundary, so `cmd {a}{b}` is three words under
    // `f5-irules` but a two-word command (`{a}{b}` is one composite
    // word) under the vanilla default.
    let irules = segment_commands_with_offset_and_config(
        "cmd {a}{b}",
        0,
        LexerConfig::for_dialect("f5-irules"),
    );
    assert_eq!(irules.len(), 1);
    assert_eq!(
        irules[0].argv.len(),
        3,
        "iRules `}}{{` splits into two words: {:?}",
        irules[0].texts,
    );
    let vanilla = segment_commands_with_offset_and_config("cmd {a}{b}", 0, LexerConfig::default());
    assert_eq!(
        vanilla[0].argv.len(),
        2,
        "vanilla keeps `{{a}}{{b}}` as one composite word: {:?}",
        vanilla[0].texts,
    );
}

#[test]
fn single_command() {
    let cmds = segment_commands("puts hello");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name(), "puts");
    assert_eq!(cmds[0].args(), &["hello"]);
}

#[test]
fn exact_one_command_uses_segmenter_boundaries() {
    let config = LexerConfig::default();
    assert!(has_exactly_one_command_with_config("puts hello", config));
    assert!(has_exactly_one_command_with_config("puts hello;", config));
    assert!(has_exactly_one_command_with_config(
        "puts hello\n# trailing comment",
        config,
    ));
    assert!(!has_exactly_one_command_with_config(
        "puts hello; puts goodbye",
        config,
    ));

    let f5 = LexerConfig::for_dialect("f5-irules");
    let if_else = "if {1} {puts yes}\nelse {puts no}";
    assert!(!has_exactly_one_command_with_config(if_else, config));
    assert!(has_exactly_one_command_with_config(if_else, f5));
}

#[test]
fn two_commands() {
    let cmds = segment_commands("set x 1\nputs $x");
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].name(), "set");
    assert_eq!(cmds[0].texts.len(), 3); // set, x, 1
    assert_eq!(cmds[1].name(), "puts");
}

#[test]
fn semicolon_separator() {
    let cmds = segment_commands("set x 1; set y 2");
    assert_eq!(cmds.len(), 2);
}

#[test]
fn variable_word() {
    let cmds = segment_commands("puts $name");
    assert_eq!(cmds.len(), 1);
    // Variable should be wrapped as ${name}.
    assert_eq!(cmds[0].texts[1], "${name}");
}

#[test]
fn array_with_literal_index_braces_canonically() {
    // Literal-index bare `$arr(idx)` normalises to `${arr(idx)}`
    // so the bytecode codegen can emit array loads.
    let cmds = segment_commands("puts $arr(idx)");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].texts[1], "${arr(idx)}");
}

#[test]
fn array_with_dollar_substituted_index_round_trips_bare() {
    // Bare `$arr($idx)` with a `$`-substituted index must NOT be
    // braced — wrapping in `${...}` would disable array-element
    // interpretation and turn the recursive substitution into a
    // literal scalar lookup (cmdAH-1.4 / 1.5 idiom).
    let cmds = segment_commands("puts $numargErrors($cmd)");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].texts[1], "$numargErrors($cmd)");
}

#[test]
fn array_with_cmd_substituted_index_round_trips_bare() {
    // Same as above but with a `[…]` command substitution in the
    // index — must also stay bare.
    let cmds = segment_commands("puts $arr([f x])");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].texts[1], "$arr([f x])");
}

#[test]
fn braced_var_form_preserved_when_explicit() {
    // Explicit `${arr(idx)}` keeps the braced form since
    // `content_offset == 2` marks the token as braced.
    let cmds = segment_commands("puts ${arr(idx)}");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].texts[1], "${arr(idx)}");
}

#[test]
fn command_substitution() {
    let cmds = segment_commands("puts [expr 1+2]");
    assert_eq!(cmds.len(), 1);
    // Command substitution wrapped in brackets.
    assert!(cmds[0].texts[1].starts_with('['));
    assert!(cmds[0].texts[1].ends_with(']'));
}

#[test]
fn braced_string() {
    let cmds = segment_commands("if {$x > 0} {puts yes}");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name(), "if");
    assert_eq!(cmds[0].texts.len(), 3);
}

#[test]
fn single_token_tracking() {
    let cmds = segment_commands("set x {hello world}");
    assert_eq!(cmds.len(), 1);
    // "set" is single, "x" is single, "{hello world}" is single.
    assert!(cmds[0].single_token_word.iter().all(|&s| s));
}

#[test]
fn multi_token_word() {
    let cmds = segment_commands("puts $a$b");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].texts.len(), 2); // "puts", "${a}${b}"
    assert!(!cmds[0].single_token_word[1]); // multi-token word
}

#[test]
fn multi_token_word_argv_spans_full_word() {
    // For a multi-token word the representative argv token's
    // span must cover the whole reconstructed word, not just
    // the first sub-token.
    let src = "source $script_dir/init.tcl";
    let cmds = segment_commands(src);
    assert_eq!(cmds.len(), 1);
    let arg_span = cmds[0].argv[1].span;
    assert_eq!(
        &src[arg_span.as_range()],
        "$script_dir/init.tcl",
        "argv[1].span must cover the whole multi-token word",
    );
    assert!(!cmds[0].single_token_word[1]);
}

#[test]
fn comment_ignored() {
    let cmds = segment_commands("# this is a comment\nputs hello");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name(), "puts");
}

#[test]
fn arg_tokens_and_arg_single() {
    let cmds = segment_commands("set x 1");
    assert_eq!(cmds[0].arg_tokens().len(), 2); // x, 1
    assert_eq!(cmds[0].arg_single_token().len(), 2);
}

#[test]
fn blank_lines_between_commands() {
    let cmds = segment_commands("set x 1\n\nset y 2");
    assert_eq!(cmds.len(), 2);
}

// -- whole-command range covers the last
// word's closing delimiter

#[test]
fn command_span_includes_trailing_brace() {
    // `if {$x} {body}` — the final word is a braced body whose
    // STR token stops on the `}`. The whole-command span must
    // reach past the closer so consumers don't drop it.
    let src = "if {$x} {body}";
    let cmds = segment_commands(src);
    assert_eq!(cmds.len(), 1);
    assert_eq!(&src[cmds[0].span.as_range()], "if {$x} {body}");
}

#[test]
fn command_span_includes_trailing_bracket() {
    // Final word is a command substitution `[...]`; the CMD
    // token stops on `]`, so the span must include it.
    let src = "set x [expr 1+2]";
    let cmds = segment_commands(src);
    assert_eq!(cmds.len(), 1);
    assert_eq!(&src[cmds[0].span.as_range()], "set x [expr 1+2]");
}

#[test]
fn execution_span_includes_a_quoted_last_word_for_replay() {
    for src in ["error \"boom\"", "error \"boom-$x\""] {
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            &src[cmds[0].execution_span(src).as_range()],
            src,
            "execution text must retain the final quote"
        );
    }
}

#[test]
fn command_span_includes_multiline_closing_brace() {
    // Multi-line braced body — the closer is on its own line.
    let src = "proc f {} {\n  set x 1\n}";
    let cmds = segment_commands(src);
    assert_eq!(cmds.len(), 1);
    assert!(
        src[cmds[0].span.as_range()].ends_with('}'),
        "span text: {:?}",
        &src[cmds[0].span.as_range()],
    );
}

#[test]
fn command_span_unaffected_by_degenerate_empty_brace() {
    // Degenerate `{}` already covers its closer in the token
    // span (the lexer extends it by one), so widening must not
    // over-reach past the `}`.
    let src = "proc f {}";
    let cmds = segment_commands(src);
    assert_eq!(cmds.len(), 1);
    assert_eq!(&src[cmds[0].span.as_range()], "proc f {}");
}

#[test]
fn widen_word_end_does_not_overshoot_empty_brace_before_enclosing_closer() {
    // The empty-`{}` case made reachable directly: an empty
    // `{}` as the final word of a command, with an enclosing `}`
    // immediately after it in the *same* buffer. `command_span` must
    // stop at the empty brace's own closer, never the enclosing one.
    //
    // The old byte-check `source[span.end()] == '}'` over-reached here
    // (the byte one past the empty `{}` span *is* the enclosing `}`);
    // the faithful text-empty predicate does not. The public
    // segmenter lexes `a {}}` as `a`, `{}`, `}` (the stray `}` is its
    // own word), so the overshoot is constructed at the token level.
    let src = "a {}}";
    let sm = SourceMap::new(src);
    let toks = tcl_lexer::Lexer::new(src).tokenise_all().unwrap();
    let words: Vec<Token> = toks
        .iter()
        .copied()
        .filter(|t| !matches!(t.kind, TokenType::Sep | TokenType::Eol | TokenType::Eof))
        .collect();
    // words == [`a`, `{}`, `}`]; drop the trailing stray `}` so the
    // empty `{}` is the command's final word.
    assert_eq!(words[1].kind, TokenType::Str);
    assert_eq!(words[1].span.end(), 4);
    let span = command_span(&words[..2], &sm);
    assert_eq!(&src[span.as_range()], "a {}");
    assert_eq!(span.end(), 4, "must not swallow the enclosing `}}`");
    // The lone widen helper agrees.
    assert_eq!(widen_word_end(words[1], &sm), 4);
}

#[test]
fn command_span_unaffected_for_plain_last_word() {
    // A bare-word final argument has no closer to widen.
    let src = "set x 1";
    let cmds = segment_commands(src);
    assert_eq!(cmds.len(), 1);
    assert_eq!(&src[cmds[0].span.as_range()], "set x 1");
}
