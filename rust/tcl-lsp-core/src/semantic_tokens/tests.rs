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

use super::object_dispatch::object_handle_name;
use super::*;
use tcl_dialect::model::{Family, SurfaceLayer};

#[test]
fn typed_token_kinds_project_custom_legend_names_in_wire_order() {
    let legend = legend_token_types();
    assert_eq!(ALL_TOKEN_KINDS.len(), legend.len());
    for (index, kind) in ALL_TOKEN_KINDS.iter().enumerate() {
        assert_eq!(
            *kind as usize, index,
            "token kind and legend order diverged"
        );
    }
    let custom = legend_custom_token_types().unwrap();
    assert!(custom.contains(&"bigipInterface"));
    assert!(!custom.contains(&"interface"));
}

/// The plain-Tcl release these tests tokenise under. The lenient `tcl`
/// span abstains wherever its releases could differ, so a test of plain
/// Tcl names one. A named helper keeps the 140-odd call sites readable —
/// spelling the resolution out at each one buries the assertion it
/// belongs to.
fn tcl() -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
}

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// The first-paint tier reads written words and the registry only. On
/// source whose colours need no analysis it must equal the analysed
/// result, so serving it first changes nothing the user sees. The cases
/// are release-invariant; run under Tcl 8.6.
#[test]
fn coarse_tokens_equal_analysed_tokens_on_plain_source() {
    let registry = release_reg("tcl8.6");
    let sources = [
        "namespace eval ::bench {\n    variable counter 0\n}\n",
        "# a comment\nproc ::bench::step {a b} {\n    set v [expr {$a + $b}]\n    \
             set msg \"step = $v\"\n    if {$v > 10} {\n        # nested comment\n        \
             set v [expr {$v + 1}]\n    }\n    return $v\n}\n",
        "set s [string length abc] ;# inline comment\nputs -nonewline $s\n",
    ];
    for source in sources {
        assert_eq!(
            coarse(source, tcl(), &registry).data,
            full(source, tcl(), &registry).data,
            "{source}"
        );
    }
}

/// What the first-paint tier colours without any analysis: a class body's
/// members and their parameters, a parameter default, and a `switch` arm's
/// script. Release-invariant; run under Tcl 8.6.
#[test]
fn coarse_tokens_read_definition_bodies_and_case_arms_from_written_words() {
    let registry = release_reg("tcl8.6");
    let source = "oo::class create Stack {\n    method push {x {n 1}} {\n        \
                      switch -exact -- $x {\n            a { puts one }\n        }\n    }\n}\n";
    let lines: Vec<&str> = source.lines().collect();
    let (mut line, mut column) = (0usize, 0usize);
    let mut kinds = FxHashMap::default();
    for token in coarse(source, tcl(), &registry).data.chunks(5) {
        if token[0] > 0 {
            line += token[0] as usize;
            column = token[1] as usize;
        } else {
            column += token[1] as usize;
        }
        let text = &lines[line][column..column + token[2] as usize];
        kinds.entry(text).or_insert(token[3]);
    }
    for (text, kind) in [
        ("method", TokenKind::Keyword),
        ("x", TokenKind::Parameter),
        ("n", TokenKind::Parameter),
        ("1", TokenKind::Number),
        ("switch", TokenKind::Keyword),
        ("puts", TokenKind::Function),
    ] {
        assert_eq!(kinds.get(text).copied(), Some(kind as u32), "{text}");
    }
}

/// The command set of one release, so a unit built from it carries that
/// release rather than the dialect-blind standalone configuration.
fn release_reg(dialect: &str) -> CommandRegistry {
    reg().project_for_profile(
        tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
    )
}

fn expect_reg() -> CommandRegistry {
    let mut registry = reg();
    registry.load_surface(SurfaceLayer::Package("expect"));
    registry
}

fn kinds(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<u32> {
    full(src, dialect, registry)
        .data
        .chunks(5)
        .map(|c| c[3])
        .collect()
}

/// Decode the packed stream into absolute `(line, col, len)` triples.
fn decode(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<(u32, u32, u32)> {
    let st = full(src, dialect, registry);
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len) = (c[0], c[1], c[2]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len));
    }
    out
}

/// Decode the packed stream into absolute
/// `(line, col, len, kind, modifiers)` tuples.
fn decode_full(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<(u32, u32, u32, u32, u32)> {
    let st = full(src, dialect, registry);
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len, kind, mods) = (c[0], c[1], c[2], c[3], c[4]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len, kind, mods));
    }
    out
}

/// Assert no two tokens on the same line overlap (next starts at or
/// after the previous token's end) — the client "Overlapping semantic
/// tokens detected" invariant.
fn assert_non_overlapping(src: &str, registry: &CommandRegistry) {
    let toks = decode(src, tcl(), registry);
    for w in toks.windows(2) {
        let (l0, c0, len0) = w[0];
        let (l1, c1, _) = w[1];
        if l0 == l1 {
            assert!(
                c1 >= c0 + len0,
                "overlap on line {l0}: token at col {c1} starts before \
                     previous token end {} (src={src:?}, toks={toks:?})",
                c0 + len0,
            );
        }
    }
}

#[test]
fn expression_commands_keep_nested_substitution_closers() {
    let source = "if {[llength [list $value]]} {}\n";
    for label in ["tcl8.6", "tcl9.1"] {
        let context = tcl_registry::model::ingress::static_context_for(label);
        let profile = context
            .commands()
            .profile()
            .expect("explicit release fixture");
        let tokens = decode_full(source, profile, context.commands());
        let list = u32::try_from(source.find("list").unwrap()).unwrap();
        assert!(
            tokens
                .iter()
                .any(|&(line, column, length, kind, modifiers)| {
                    line == 0
                        && column == list
                        && length == 4
                        && kind == TokenKind::Function as u32
                        && modifiers & MOD_DEFAULT_LIBRARY != 0
                }),
            "{label}: {tokens:?}"
        );
        let value = u32::try_from(source.find("$value").unwrap()).unwrap();
        assert!(
            tokens.iter().any(|&(line, column, length, kind, _)| {
                line == 0
                    && column <= value + 1
                    && column + length >= value + 6
                    && kind == TokenKind::Variable as u32
            }),
            "{label}: {tokens:?}"
        );
    }
}

#[test]
fn nested_command_head_substitution_in_an_expression_keeps_word_extents_2265() {
    let source = "if {[[list string] length abcdef] > 1} {}\n";
    let tokens = decode_full(source, tcl(), &reg());
    for word in ["length", "abcdef"] {
        let start = u32::try_from(source.find(word).unwrap()).unwrap();
        let length = u32::try_from(word.len()).unwrap();
        assert!(
            tokens
                .iter()
                .any(|&(line, column, len, _, _)| line == 0 && column == start && len == length),
            "{word}: {tokens:?}"
        );
    }
    assert_non_overlapping(source, &reg());
}

#[test]
fn case_list_patterns_keep_literal_braces_from_selected_elements() {
    // tclsh 8.4.20, 8.6.18 and 9.0.4 print `b` for this script and `a` when
    // the subject is `{default`: the quoted element is a literal pattern.
    // The brace is escaped because an unescaped one inside quotes still
    // counts towards the enclosing braces (`missing close-brace`).
    let source = "switch x {\"\\{default\" {puts a} {default} {puts b}}\n";
    let tokens = decode_full(source, tcl(), &release_reg("tcl8.6"));
    let literal = u32::try_from(source.find("{default").unwrap()).unwrap();
    assert!(
        !tokens.iter().any(|&(line, column, length, kind, _)| {
            line == 0
                && column <= literal + 1
                && column + length > literal + 1
                && kind == TokenKind::Keyword as u32
        }),
        "{tokens:?}"
    );
    let keyword = u32::try_from(source.rfind("{default").unwrap()).unwrap();
    assert!(
        tokens.iter().any(|&(line, column, length, kind, _)| {
            line == 0
                && column <= keyword + 1
                && column + length > keyword + 1
                && kind == TokenKind::Keyword as u32
        }),
        "{tokens:?}"
    );
}

#[test]
fn quoted_var_at_string_start_no_overlap() {
    // The lexer's empty-content clamp spans the opening `"` fragment over
    // `"$`, overlapping the `$x` variable token; the opening fragment must
    // shrink to just the `"`.
    let r = reg();
    assert_non_overlapping("puts \"$x y\"\n", &r);
    assert_non_overlapping("set x 1\nputs \"$x — résumé — 日本語\"\n", &r);
    // Adjacent substitutions: the empty ESC between `$a` and `$b`
    // carries no delimiter, so it must vanish entirely (no zero-area
    // overlap at the `$b`).
    assert_non_overlapping("puts \"$a$b\"\n", &r);
    // Command substitution introducer `[` at string start.
    assert_non_overlapping("puts \"[expr {1+2}] z\"\n", &r);
    // Dense line with several adjacent substitutions/strings.
    assert_non_overlapping("set a 1;set b 2;puts \"$a [expr {$a+$b}] $b\";# tail\n", &r);
}

#[test]
fn quoted_string_opening_fragment_is_single_quote() {
    // `puts "$x y"` — the opening string fragment is exactly the `"`
    // (col 5, len 1), not `"$` (len 2).
    let toks = decode("puts \"$x y\"\n", tcl(), &reg());
    // The opening `"` lands at byte/col 5 on line 0 with length 1.
    assert!(
        toks.contains(&(0, 5, 1)),
        "expected a length-1 string token at col 5, got {toks:?}",
    );
}

#[test]
fn expect_canonical_and_abbreviated_regex_flags_tokenise_in_both_shapes() {
    for (label, source, flag) in [
        (
            "braced canonical",
            "expect {\n    -regexp {a+} {\n        puts matched\n    }\n}\n",
            "-regexp",
        ),
        (
            "braced abbreviation",
            "expect {\n    -re {a+} {\n        puts matched\n    }\n}\n",
            "-re",
        ),
        (
            "inline canonical",
            "expect -regexp {a+} {\n    puts matched\n    puts again\n}\n",
            "-regexp",
        ),
        (
            "inline abbreviation",
            "expect -re {a+} {\n    puts matched\n    puts again\n}\n",
            "-re",
        ),
    ] {
        let tokens = decode_full(
            source,
            tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
            &expect_reg(),
        );
        let flag_len = u32::try_from(flag.len()).unwrap();
        assert!(
            tokens.iter().any(|&(_, _, len, kind, _)| {
                len == flag_len && kind == TokenKind::Decorator as u32
            }),
            "{label}: {flag} must be a clause decorator: {tokens:?}"
        );
        assert!(
            tokens
                .iter()
                .any(|&(_, _, _, kind, _)| kind == TokenKind::RegexpQuantifier as u32),
            "{label}: {flag} must enable regexp tokenisation: {tokens:?}"
        );
    }
}

#[test]
fn expect_literal_comment_and_separator_patterns_recurse_into_actions() {
    for pattern in ["#", ";"] {
        let source = format!("expect {{{pattern} {{puts matched}} default {{puts other}}}}\n");
        let tokens = decode_full(
            &source,
            tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
            &expect_reg(),
        );
        let puts_col = u32::try_from(source.find("puts matched").unwrap()).unwrap();
        assert!(
            tokens.iter().any(|&(line, col, len, kind, _)| {
                line == 0 && col == puts_col && len == 4 && kind == TokenKind::Function as u32
            }),
            "{pattern:?} is a literal list pattern, and its action must be tokenised: {tokens:?}"
        );
    }
}

#[test]
fn malformed_case_lists_do_not_tokenise_action_words_as_commands() {
    let source = "switch subject {a {puts hidden} orphan}\n";
    let tokens = decode_full(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    let puts_col = u32::try_from(source.find("puts hidden").unwrap()).unwrap();
    assert!(
        !tokens.iter().any(|&(line, col, len, kind, _)| {
            line == 0 && col == puts_col && len == 4 && kind == TokenKind::Function as u32
        }),
        "semantic tokens must abstain for an odd case list: {tokens:?}"
    );
}

#[test]
fn renamed_away_expect_spelling_gets_no_case_list_overrides() {
    let source = "rename expect other\nexpect -re {a+} { puts matched }\n";
    let kinds = kinds(
        source,
        tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
        &expect_reg(),
    );
    assert!(
        !kinds.contains(&(TokenKind::RegexpQuantifier as u32)),
        "a proven rebound head must not inherit Expect overrides: {kinds:?}"
    );
}

#[test]
fn known_option_classified_as_decorator() {
    // `regexp -nocase {pat} $s` — `-nocase` is a real option → decorator.
    let ks = kinds("regexp -nocase {pat} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
    // `puts -foo` — `-foo` is not an option of `puts` → not a decorator.
    let ks = kinds("puts -foo\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
}

#[test]
fn abbreviated_option_classified_as_decorator() {
    // Tcl option parsing accepts unique prefixes: `lsort -inc` ⇒
    // `-increasing`, `lsearch -ex` ⇒ `-exact`.
    for src in ["lsort -inc {3 1 2}\n", "lsearch -ex {a b} b\n"] {
        let ks = kinds(src, tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected decorator for {src:?}; got {ks:?}"
        );
    }
    // An ambiguous prefix (`lsort -i` → -index/-indices/-integer/…) is not
    // a recognised option and stays a string.
    let ks = kinds("lsort -i {3 1 2}\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "ambiguous prefix must not be a decorator; got {ks:?}"
    );
}

#[test]
fn lsearch_regexp_pattern_uses_the_registry_selected_language() {
    let ks = kinds("lsearch -regexp {a b} {a+}\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    // The default glob form must not acquire regex subtokens merely
    // because lsearch also supports -regexp.
    let glob = kinds("lsearch {a b} {a+}\n", tcl(), &reg());
    assert!(!glob.contains(&(TokenKind::Regexp as u32)), "{glob:?}");
}

#[test]
fn lsearch_pattern_tokens_abstain_for_invalid_profiled_option_prefixes() {
    // A dash-prefixed word inside lsearch's outer option scan is not a
    // positional list operand when it cannot resolve in that release.
    // In each invalid call the old fallback would put the regex override
    // on `{a+}`; the registry must instead abstain entirely.
    let registry = reg();
    let kinds_at_claimed_pattern = |source: &str, dialect: &str| {
        let claimed_pattern = u32::try_from(source.find("a+").expect("claimed pattern")).unwrap();
        let tokens = full(
            source,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            &registry,
        );
        let mut line = 0u32;
        let mut column = 0u32;
        tokens
            .data
            .chunks(5)
            .filter_map(|chunk| {
                if chunk[0] > 0 {
                    line += chunk[0];
                    column = chunk[1];
                } else {
                    column += chunk[1];
                }
                (line == 0 && column <= claimed_pattern && claimed_pattern < column + chunk[2])
                    .then_some(chunk[3])
            })
            .collect::<Vec<_>>()
    };

    for (dialect, option) in [("tcl8.6", "-str"), ("tcl9.0", "-st"), ("tcl9.0", "--")] {
        let source = format!("lsearch -regexp {option} {{a+}} {{a b}} x\n");
        let kinds = kinds_at_claimed_pattern(&source, dialect);
        assert!(
            !kinds.contains(&(TokenKind::Regexp as u32)),
            "{dialect} {option} is invalid inside the option scan: {kinds:?}"
        );
    }

    // Tcl 9's unique -str abbreviation does consume its stride value,
    // so the real final pattern remains regex-classified.
    let source = "lsearch -regexp -str 2 {a b} {a+}\n";
    let kinds = kinds_at_claimed_pattern(source, "tcl9.0");
    assert!(
        kinds.contains(&(TokenKind::Regexp as u32)),
        "tcl9.0 must parse -str as -stride: {kinds:?}"
    );
}

#[test]
fn subcommand_option_classified_as_decorator() {
    // `file delete -force filename`.  `-force`
    // is declared on the `delete` *subcommand* (not on `file` itself), so
    // it is only recognised once subcommand options are consulted.
    let ks = kinds("file delete -force filename\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -force decorator; got {ks:?}"
    );
    // A subcommand option on a different subcommand: `file link -symbolic`.
    let ks = kinds("file link -symbolic a b\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -symbolic decorator; got {ks:?}"
    );
    // A `-`-word that is not a declared option stays a plain string, even
    // on a command that has subcommand options elsewhere.
    let ks = kinds("file delete -bogus filename\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-bogus is not a real option; got {ks:?}"
    );
    // A `-$var` substitution word must never be treated as an option.
    let ks = kinds("file delete -$flag filename\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-$flag is a substitution, not an option; got {ks:?}"
    );
}

#[test]
fn unknown_head_options_classified_generically() {
    // The ngspice / ticklecharts pattern: `$chart Xaxis -name
    // {v(anode), V} -type value -min 0.4` — the head `$chart` is an object
    // handle, unknown to the registry, so its `-switch value` pairs are
    // highlighted by the generic heuristic.
    let ks = kinds(
        "$chart Xaxis -name {v} -type value -min 0.4\n",
        tcl(),
        &reg(),
    );
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -name/-type/-min decorators on an unknown head; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected option values on an unknown head; got {ks:?}"
    );
    // A negative number is not an option: `$obj move -5 10`.
    let ks = kinds("$obj move -5 10\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-5 is a negative number, not an option; got {ks:?}"
    );
    // A `-$var` substitution word is not an option.
    let ks = kinds("$obj configure -$flag v\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-$flag is a substitution, not an option; got {ks:?}"
    );
    // A known command keeps the strict declared-option behaviour: `puts
    // -foo` stays a string even though the generic pass exists.
    let ks = kinds("puts -foo\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "a known command's undeclared -foo must stay a string; got {ks:?}"
    );
    // A plain bareword unknown head is a (possibly user-defined) command
    // name, not a computed dispatch: `mycmd -foo bar` stays a string so a
    // user proc's argument is not mistaken for an option.
    let ks = kinds("mycmd -foo bar\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "a bareword user command's -foo must stay a string; got {ks:?}"
    );
    // Negative special-float literals are numbers, not options: `-inf`,
    // `-Inf`, `-nan` all start with a letter but Tcl parses them as values.
    let ks = kinds("$obj set -inf\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-inf is a negative float literal, not an option; got {ks:?}"
    );
    let ks = kinds("$obj set -NaN\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-NaN is a float literal, not an option; got {ks:?}"
    );
}

#[test]
fn generic_option_scan_stops_at_double_dash() {
    // Tcl's `--` ends option processing: `-real` before it is an option,
    // the `--` is the marker, and `-literal` after it is a positional
    // operand (a plain string), not an option.
    // Columns: `-real` at 9, `1` at 15, `--` at 17, `-literal` at 20.
    let mut deco: Vec<u32> = decode_full("$obj cfg -real 1 -- -literal\n", tcl(), &reg())
        .iter()
        .filter(|(_, _, _, k, _)| *k == TokenKind::Decorator as u32)
        .map(|&(_, c, _, _, _)| c)
        .collect();
    deco.sort_unstable();
    // Exactly `-real` (9) and `--` (17) — never `-literal` (20).
    assert_eq!(
        deco,
        vec![9, 17],
        "expected -real and -- as the only decorators, not -literal after --; got {deco:?}"
    );
}

#[test]
fn value_taking_option_value_classified_as_option_value() {
    // `lsort -index 2 $l` — `-index` takes a value, so the literal `2` is
    // an option value (distinct from the `-index` decorator).
    let ks = kinds("lsort -index 2 $l\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -index decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected the `2` value to be an OptionValue; got {ks:?}"
    );
    // A boolean option takes no value — the following word is not recoloured.
    // `lsort -unique $l`: `$l` stays a variable, not an OptionValue.
    let ks = kinds("lsort -unique $l\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::OptionValue as u32)),
        "boolean -unique must not mark a following value; got {ks:?}"
    );
    // A `$var` value keeps its variable highlight, not OptionValue.
    let ks = kinds("lsort -index $i $l\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::OptionValue as u32)),
        "a $var option value keeps its variable highlight; got {ks:?}"
    );
}

#[test]
fn argparse_global_switch_values_classified_as_option_value() {
    // The `argparse` package command's value-taking global switches
    // (`-template`, `-level`, …) colour their following literal as an
    // OptionValue, while boolean switches (`-inline`) do not. argparse is
    // registered (package-gated), so the classifier resolves its spec.
    let ks = kinds(
        "argparse -template foo -level 2 -inline {d}\n",
        tcl(),
        &reg(),
    );
    let n_val = ks
        .iter()
        .filter(|&&k| k == TokenKind::OptionValue as u32)
        .count();
    assert_eq!(
        n_val, 2,
        "expected `foo` and `2` as OptionValues (not boolean -inline's `{{d}}`); got {ks:?}"
    );
    // A boolean global switch does not recolour the following word.
    let ks = kinds("argparse -inline {d}\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::OptionValue as u32)),
        "boolean -inline must not mark a following value; got {ks:?}"
    );
}

#[test]
fn subcommand_enum_value_classified_as_enum_member() {
    // `string is alnum $s` — `alnum` is a closed-set value declared on
    // the `is` subcommand → enumMember, not a plain string.
    let ks = kinds("string is alnum $s\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::EnumMember as u32)),
        "expected an enumMember token; got {ks:?}"
    );
    // A value not in the set stays a string.
    let ks = kinds("string is bogusclass $s\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::EnumMember as u32)),
        "bogusclass is not a class; got {ks:?}"
    );
}

#[test]
fn return_code_option_classified_as_decorator_issue_967() {
    // `return -code error "bad"` must not highlight `-code` as a plain
    // string. `-code` is a declared OptionSpec
    // on `return` (a decorator) and `error` is one of its closed-set
    // values (an enumMember); `"bad"` stays a plain string.
    let ks = kinds("return -code error \"bad\"\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -code to be a decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::EnumMember as u32)),
        "expected error to be an enumMember; got {ks:?}"
    );

    // `-level` is likewise a declared option (Tcl 8.5+) and its value
    // (`0`) is an OptionValue, not a plain number/string.
    let ks = kinds("return -level 0 \"bad\"\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -level to be a decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected 0 to be an OptionValue; got {ks:?}"
    );

    // `-options $opts` — `-options` is a decorator and its dict value
    // stays highlighted as the variable it is (not recoloured away).
    let ks = kinds("return -options $opts\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Decorator as u32)),
        "expected -options to be a decorator; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::Variable as u32)),
        "expected $opts to keep its variable highlight; got {ks:?}"
    );

    // TN: a plain word `-code` passed to a command with no declared
    // OptionSpec (`concat` takes no options at all) must not be painted
    // as an option — it is just a string argument.
    let ks = kinds("concat -code error\n", tcl(), &reg());
    assert!(
        !ks.contains(&(TokenKind::Decorator as u32)),
        "-code is not a declared option of concat; got {ks:?}"
    );
}

#[test]
fn info_object_class_sub_subcommand_classified_as_keyword() {
    // In `info object class $obj`, the `class` word is a
    // second-level subcommand (OBJECT INTROSPECTION), not a string. Both the
    // first-level `object` and the second-level `class` must read as
    // keywords (the `info` head itself is a Function).
    let kind_at = |src: &str, col: u32| -> u32 {
        decode_full(src, tcl(), &reg())
            .into_iter()
            .find(|&(_, c, _, _, _)| c == col)
            .map_or_else(
                || panic!("no token at column {col} in {src:?}"),
                |(_, _, _, k, _)| k,
            )
    };

    // `info object class $obj` — column 12 is `class`.
    let src = "info object class $obj\n";
    assert_eq!(
        kind_at(src, 12),
        TokenKind::Keyword as u32,
        "`class` sub-subcommand should be a keyword"
    );

    // `info class superclasses $cls` — column 11 is `superclasses`.
    let src = "info class superclasses $cls\n";
    assert_eq!(
        kind_at(src, 11),
        TokenKind::Keyword as u32,
        "`superclasses` sub-subcommand should be a keyword"
    );

    // A non-subcommand third word stays a string: `info object frobnicate`
    // — `frobnicate` is not a recognised OBJECT INTROSPECTION operation.
    let src = "info object frobnicate $obj\n";
    assert_eq!(
        kind_at(src, 12),
        TokenKind::String as u32,
        "an unknown third word must stay a string, not a keyword"
    );

    // The issue's actual form: `info object class` nested inside a command
    // substitution within an `if` expression. Highlighting must recurse into
    // the bracketed inner command, not just top-level statements. The column
    // of `class` is located dynamically to stay robust.
    let src = "if {([info object class $element {::Foo::Analysis}]) && ([info exists C])} {\n}\n";
    let col = u32::try_from(src.find(" class ").expect("has ` class `") + 1).unwrap();
    assert_eq!(
        kind_at(src, col),
        TokenKind::Keyword as u32,
        "`class` must highlight as a keyword even nested in an if-expr command substitution"
    );

    // Unique-prefix abbreviation: `info object cl` is Tcl's
    // abbreviation of `class`; column 12 is `cl`.
    let src = "info object cl $obj\n";
    assert_eq!(
        kind_at(src, 12),
        TokenKind::Keyword as u32,
        "a unique-prefix sub-subcommand should highlight as a keyword"
    );
    // An ambiguous prefix stays a string: `info class c` matches both
    // `call` and `constructor`, so it must not be painted a keyword.
    let src = "info class c $cls\n";
    assert_eq!(
        kind_at(src, 11),
        TokenKind::String as u32,
        "an ambiguous prefix must not highlight as a keyword"
    );
}

#[test]
fn subcommand_prefix_resolution_is_dialect_aware() {
    let kind_at = |src: &str, dialect: &'static tcl_dialect::DialectProfile, col: u32| -> u32 {
        decode_full(src, dialect, &reg())
            .into_iter()
            .find(|&(_, c, _, _, _)| c == col)
            .map_or_else(
                || panic!("no token at col {col} in {src:?}"),
                |(_, _, _, k, _)| k,
            )
    };
    // `string rev` is `reverse` (added 8.5): a keyword in 8.6, but an
    // unknown word in 8.4 where `reverse` does not exist (verified: tclsh8.4
    // rejects `string rev`).  Column 7 is `rev`.
    let src = "string rev abc\n";
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            7
        ),
        TokenKind::Keyword as u32
    );
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
            7
        ),
        TokenKind::String as u32,
        "`string rev` is not a subcommand in 8.4"
    );

    // `info class def`: uniquely `definition` in 8.6 (keyword), but
    // ambiguous with `definitionnamespace` in 9.0 (verified against tclsh)
    // → stays a string.  Column 11 is `def`.
    let src = "info class def ::C\n";
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            11
        ),
        TokenKind::Keyword as u32,
        "`info class def` is `definition` in 8.6"
    );
    assert_eq!(
        kind_at(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            11
        ),
        TokenKind::String as u32,
        "`info class def` is ambiguous in 9.0"
    );
}

#[test]
fn abbreviated_first_level_subcommand_highlights_as_keyword() {
    // `string le $s` — `le` is Tcl's unique-prefix abbreviation of
    // `length`; it must highlight as a subcommand keyword (column 7).
    let src = "string le $s\n";
    let toks = decode_full(src, tcl(), &reg());
    let kind = toks
        .into_iter()
        .find(|&(_, c, _, _, _)| c == 7)
        .map(|(_, _, _, k, _)| k)
        .expect("token at col 7");
    assert_eq!(
        kind,
        TokenKind::Keyword as u32,
        "abbreviated subcommand `le` should be a keyword"
    );
    // An ambiguous prefix (`string t`) stays a string.
    let src = "string t $s\n";
    let kind = decode_full(src, tcl(), &reg())
        .into_iter()
        .find(|&(_, c, _, _, _)| c == 7)
        .map(|(_, _, _, k, _)| k)
        .expect("token at col 7");
    assert_eq!(
        kind,
        TokenKind::String as u32,
        "ambiguous prefix `t` must not be a keyword"
    );
}

#[test]
fn oo_define_inline_keyword_classified_as_keyword() {
    // `oo::define Cls method foo {} {}` — the inline `method` keyword sits
    // at an argument position and must read as a keyword, not a string.
    let ks = kinds("oo::define Cls method foo {} {}\n", tcl(), &reg());
    let n_kw = ks
        .iter()
        .filter(|&&k| k == TokenKind::Keyword as u32)
        .count();
    // Two keyword tokens: the `oo::define` head and the inline `method`.
    assert!(n_kw >= 2, "expected >=2 keyword tokens; got {ks:?}");
    // `oo::define Cls self method foo {} {}` — the inner keyword after
    // `self` is highlighted too.
    let ks = kinds("oo::define Cls self method foo {} {}\n", tcl(), &reg());
    let n_kw = ks
        .iter()
        .filter(|&&k| k == TokenKind::Keyword as u32)
        .count();
    assert!(
        n_kw >= 3,
        "expected >=3 keyword tokens (head+self+method); got {ks:?}"
    );
}

#[test]
fn var_write_target_carries_declaration_modifier() {
    // `set x 1` — `x` is a write target → variable + declaration.
    let toks = decode_full("set x 1\n", tcl(), &reg());
    let x = toks.iter().find(|(_, col, len, kind, _)| {
        *col == 4 && *len == 1 && *kind == TokenKind::Variable as u32
    });
    assert!(x.is_some(), "expected variable token for `x`; got {toks:?}");
    assert_eq!(
        x.unwrap().4,
        MOD_DECLARATION,
        "expected declaration modifier"
    );
}

#[test]
fn bare_set_read_is_not_a_declaration() {
    // `set x` (no value) reads the variable — not a declaration.
    let ks = kinds("set x\n", tcl(), &reg());
    // No token should carry the declaration modifier here; `x` stays a
    // plain string.  (Modifier is checked in the full decode.)
    let toks = decode_full("set x\n", tcl(), &reg());
    assert!(
        !toks.iter().any(|(_, _, _, _, m)| *m == MOD_DECLARATION),
        "bare `set x` must not declare; got {toks:?} kinds {ks:?}"
    );
}

#[test]
fn unset_marks_every_name_as_variable() {
    // `unset x y z` — every name is a variable, not just the first.
    let toks = decode_full("unset x y z\n", tcl(), &reg());
    let vars = toks
        .iter()
        .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
        .count();
    assert_eq!(
        vars, 3,
        "all three unset names must highlight as variables; got {toks:?}"
    );
    // Leading `-nocomplain` / `--` options are not variables.
    let toks = decode_full("unset -nocomplain -- a b\n", tcl(), &reg());
    let vars = toks
        .iter()
        .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
        .count();
    assert_eq!(
        vars, 2,
        "only `a` and `b` are variables, not the leading options; got {toks:?}"
    );
}

#[test]
fn global_marks_every_name_as_variable() {
    // `global a b c` — every name is a variable, not just the first.
    let toks = decode_full("proc p {} { global a b c }\n", tcl(), &reg());
    let vars = toks
        .iter()
        .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
        .count();
    assert_eq!(
        vars, 3,
        "all three global names must highlight as variables; got {toks:?}"
    );
}

#[test]
fn array_element_write_not_retagged() {
    // `set arr($i) 1` — the target has a `$` substitution; leave it to the
    // default classifier so the inner `$i` variable still tokenises.
    let toks = decode_full("set arr($i) 1\n", tcl(), &reg());
    // The `$i` inside must still surface as a variable token.
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
        "expected inner $i variable token; got {toks:?}"
    );
}

#[test]
fn literal_array_element_write_is_variable_declaration() {
    // A literal array-element write target highlights as one whole-word
    // `Variable` declaration, matching the `$arr(key)` read.
    for src in [
        "set arr(key) 1\n",
        "incr count(hits)\n",
        "append log(err) x\n",
    ] {
        let toks = decode_full(src, tcl(), &reg());
        let decl = toks
            .iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
        assert!(
            decl,
            "expected an array-element variable declaration; got {toks:?} for {src:?}"
        );
    }
    // The target `arr(key)` is a single token spanning the whole element.
    let toks = decode_full("set arr(key) 1\n", tcl(), &reg());
    let whole = toks.iter().any(|(_, col, len, k, m)| {
        *col == 4 && *len == 8 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
    });
    assert!(
        whole,
        "expected a single length-8 variable token over `arr(key)`; got {toks:?}"
    );
    assert_non_overlapping("set arr(key) 1\n", &reg());
}

#[test]
fn namespaced_array_element_write_is_variable_declaration() {
    // A namespaced array (`::ns::arr(key)`) is still a plain element.
    let toks = decode_full("set ::ns::arr(key) 1\n", tcl(), &reg());
    let decl = toks
        .iter()
        .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
    assert!(
        decl,
        "expected a variable declaration for the namespaced array element; got {toks:?}"
    );
}

#[test]
fn unset_array_element_is_variable() {
    // `unset arr(key)` — `unset` is a VarWrite command, so the literal
    // element retags as one whole-word `Variable` declaration spanning
    // `arr(key)` (col 6, len 8).
    let toks = decode_full("unset arr(key)\n", tcl(), &reg());
    let whole = toks.iter().any(|(_, col, len, k, m)| {
        *col == 6 && *len == 8 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
    });
    assert!(
        whole,
        "expected `arr(key)` as one variable declaration; got {toks:?}"
    );
    // `unset arr($i)` — the computed subscript stays multi-token: the inner
    // `$i` (col 10) survives as its own variable, and the whole word is not
    // painted (no declaration at the word start, col 6).
    // `unset arr($i)` — the computed subscript keeps the word multi-token.
    // The inner `$i` must survive as its own variable, so the word must NOT
    // be painted whole (that would swallow the substitution)…
    let toks = decode_full("unset arr($i)\n", tcl(), &reg());
    let inner_i = toks
        .iter()
        .any(|(_, col, _, k, _)| *col == 10 && *k == TokenKind::Variable as u32);
    assert!(inner_i, "expected the inner `$i` variable; got {toks:?}");
    let painted_whole = toks
        .iter()
        .any(|(_, col, len, k, _)| *col == 6 && *len == 8 && *k == TokenKind::Variable as u32);
    assert!(
        !painted_whole,
        "computed subscript must not retag the whole word; got {toks:?}"
    );
    // …but its *literal* fragments — the array name and the closing paren —
    // are still part of the variable reference, not free-floating strings.
    // Falling through to the default classification would paint them as
    // `string`: `set env($lo)`, `unset UnknownPending($name)` and friends
    // all over Tcl's own library.
    let name_frag = toks.iter().any(|(_, col, len, k, m)| {
        *col == 6 && *len == 4 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
    });
    assert!(
        name_frag,
        "expected the `arr(` fragment as a variable; got {toks:?}"
    );
    let close_frag = toks
        .iter()
        .any(|(_, col, len, k, _)| *col == 12 && *len == 1 && *k == TokenKind::Variable as u32);
    assert!(
        close_frag,
        "expected the `)` fragment as a variable; got {toks:?}"
    );
}

#[test]
fn varread_role_highlights_variable_name() {
    // A read-role variable-name argument (`info exists`, `array names`)
    // highlights its name as a plain `Variable` — no `declaration`
    // modifier, since a read references an existing variable.
    for src in [
        "info exists arr(key)\n",
        "info exists scalar\n",
        "array names arr\n",
        "array get arr\n",
    ] {
        let toks = decode_full(src, tcl(), &reg());
        let var_ref = toks
            .iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == 0);
        assert!(
            var_ref,
            "expected a plain Variable reference for {src:?}; got {toks:?}"
        );
        // A read must not carry the declaration modifier.
        let decl = toks
            .iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
        assert!(!decl, "a read must not declare; got {toks:?} for {src:?}");
    }
}

#[test]
fn upvar_other_var_highlights_the_linked_cell() {
    // `otherVar` names the cell an `upvar` links (#2259), so it is a
    // `Variable` rather than a plain string.
    for (src, col) in [
        ("namespace upvar ::ns version local\n", 21),
        ("proc p {} { upvar #0 version local }\n", 21),
    ] {
        let toks = decode_full(src, tcl(), &reg());
        assert!(
            toks.iter().any(|(line, c, len, k, _)| {
                *line == 0 && *c == col && *len == 7 && *k == TokenKind::Variable as u32
            }),
            "expected `version` as a Variable in {src:?}; got {toks:?}"
        );
    }
}

#[test]
fn varread_value_argument_is_not_a_variable() {
    // `dict get $d k` — `$d` is a value (a dict), not a var-name spot, so
    // the read-role retag must not fire on it; only the `$d` substitution
    // itself is a variable, and `k` (the key) is a plain string.
    let toks = decode_full("dict get $d k\n", tcl(), &reg());
    // Exactly one variable: the `$d` substitution.
    let vars = toks
        .iter()
        .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
        .count();
    assert_eq!(vars, 1, "only `$d` is a variable; got {toks:?}");
}

#[test]
fn stub_var_arg_highlights_array_element() {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    // A `# tcl-lsp: stub` with a `:var` argument marks that position a
    // variable-name spot, so a literal array element passed there
    // highlights like `set arr(key) …` — even on the registry-only path,
    // since stub roles are derived from the document source.
    let src = "# tcl-lsp: stubs-begin\n\
                   # tcl-lsp: stub mywrite {varName:var value}\n\
                   # tcl-lsp: stubs-end\n\
                   mywrite arr(key) 1\n";
    let toks = decode_full(src, tcl(), &reg());
    let decl = toks.iter().any(|(line, _, _, k, m)| {
        *line == 3 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
    });
    assert!(
        decl,
        "expected the stubbed :var array element to highlight; got {toks:?}"
    );
}

#[test]
fn stub_var_read_arg_highlights_as_reference() {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    // A `# tcl-lsp: stub … :var_read` argument marks a read-position
    // variable name, so a literal array element there highlights as a plain
    // `Variable` reference (no `declaration` modifier).
    let src = "# tcl-lsp: stubs-begin\n\
                   # tcl-lsp: stub myexists {varName:var_read}\n\
                   # tcl-lsp: stubs-end\n\
                   myexists arr(key)\n";
    let toks = decode_full(src, tcl(), &reg());
    let var_ref = toks
        .iter()
        .any(|(line, _, _, k, m)| *line == 3 && *k == TokenKind::Variable as u32 && *m == 0);
    assert!(
        var_ref,
        "expected the stubbed :var_read array element as a reference; got {toks:?}"
    );
    let decl_on_call = toks.iter().any(|(line, _, _, k, m)| {
        *line == 3 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
    });
    assert!(!decl_on_call, "a read must not declare; got {toks:?}");
}

#[test]
fn user_proc_var_name_arg_highlights_array_element() {
    // A user proc whose parameter the analyser infers to alias a caller
    // variable (`upvar $varName` + write) makes a literal array element at
    // that call-site position a variable-name spot — so `myset arr(key) 1`
    // highlights `arr(key)` like `set arr(key) 1`.
    // The plumbing is analysis-driven, so it needs the enriched path.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "proc myset {varName value} {\n\
                     upvar 1 $varName v\n\
                     set v $value\n\
                   }\n\
                   myset arr(key) 1\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let decl_on_call = |toks: &[(u32, u32, u32, u32, u32)]| {
        toks.iter().any(|&(line, _, _, k, m)| {
            line == 4 && k == TokenKind::Variable as u32 && m == MOD_DECLARATION
        })
    };
    // The first-paint tier interprets nothing: `myset` is an unknown
    // command there and `arr(key)` at the call stays a plain string.
    let plain = decode_semantic(&coarse(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    ));
    assert!(
        !decl_on_call(&plain),
        "no proc-role highlight without analysis; got {plain:?}"
    );
    // With analysis, the inferred `varName` role retags `arr(key)`.
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        decl_on_call(&toks),
        "expected arr(key) at the myset call to highlight; got {toks:?}"
    );
}

#[test]
fn user_proc_var_name_computed_subscript_survives() {
    // `myset arr($i) 1` — even with the proc role, a computed subscript is
    // multi-token, so it is not painted whole and the inner `$i` survives.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "proc myset {varName value} {\n\
                     upvar 1 $varName v\n\
                     set v $value\n\
                   }\n\
                   myset arr($i) 1\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    let inner_i = toks
        .iter()
        .any(|&(line, _, _, k, _)| line == 4 && k == TokenKind::Variable as u32);
    assert!(
        inner_i,
        "expected the inner $i on the call line to survive; got {toks:?}"
    );
}

#[test]
fn user_proc_var_read_arg_highlights_as_reference() {
    // A proc that only *reads* its upvar'd parameter (`upvar $varName v`
    // then reads `v`) infers a `VarRead` role, so a literal name at the
    // call site highlights as a plain `Variable` reference — no
    // `declaration` modifier.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "proc myexists {varName} {\n\
                     upvar 1 $varName v\n\
                     return [info exists v]\n\
                   }\n\
                   myexists arr(key)\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    let ref_on_call = toks
        .iter()
        .any(|&(line, _, _, k, m)| line == 4 && k == TokenKind::Variable as u32 && m == 0);
    assert!(
        ref_on_call,
        "expected arr(key) at the myexists call as a reference; got {toks:?}"
    );
}

#[test]
fn user_proc_dynamic_name_read_args_highlight() {
    // A proc that reads a dynamic variable name built from its parameters
    // inside a command substitution — `return [set ${v}($k)]` — infers a
    // read role for both `v` and `k`, so the literal call-site args
    // (`b aa foo`) highlight as plain `Variable` references.  Exercises the
    // full path: cmd-substitution recursion + compound-name inference in
    // the analyser, through to the token retag.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "array set aa {foo 1}\n\
                   proc b {v k} {\n\
                       return [set ${v}($k)]\n\
                   }\n\
                   b aa foo\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // On the call line (`b aa foo`, line 4) both `aa` (col 2) and `foo`
    // (col 5) highlight as plain `Variable` references (no declaration).
    let refs: Vec<_> = toks
        .iter()
        .filter(|&&(line, _, _, k, m)| line == 4 && k == TokenKind::Variable as u32 && m == 0)
        .collect();
    assert!(
        refs.iter().any(|&&(_, col, _, _, _)| col == 2)
            && refs.iter().any(|&&(_, col, _, _, _)| col == 5),
        "expected `aa` and `foo` as variable references on the call line; got {toks:?}"
    );
}

#[test]
fn user_proc_command_arg_highlights_as_function() {
    // A dispatcher proc invokes its parameter as a command (`$cmd …`), so
    // the analyser infers `cmd` is a `Command`.  The literal command name at
    // the call site (`dispatch greet …`) then highlights as a `Function`.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "proc dispatch {cmd arg} {\n\
                     $cmd $arg\n\
                   }\n\
                   dispatch greet hello\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // The first-paint tier interprets nothing, so `greet` is an unknown
    // command's plain string argument there.
    let plain = decode_semantic(&coarse(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    ));
    let greet_fn = |toks: &[(u32, u32, u32, u32, u32)]| {
        toks.iter()
            .any(|&(line, col, _, k, _)| line == 3 && col == 9 && k == TokenKind::Function as u32)
    };
    assert!(
        !greet_fn(&plain),
        "no command role without analysis; got {plain:?}"
    );
    // With analysis, `greet` (col 9 on the call line) highlights as a command.
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        greet_fn(&toks),
        "expected `greet` at the dispatch call to highlight as a Function; got {toks:?}"
    );
}

#[test]
fn command_prefix_callback_head_highlights_as_function() {
    // A registry `CommandPrefix` callback head retags as a Function — driven
    // by the declarative role (no analysis needed).  Covers a core
    // `-command` and a Tk `scale -command` (the script→prefix conversion):
    // under the old `script()` a bareword single-word callback head was not
    // recursed, so it did not highlight; as a prefix it now does.
    let registry = reg();
    let slice = |src: &str, line: u32, col: u32, len: u32| -> String {
        src.lines()
            .nth(line as usize)
            .and_then(|l| l.get(col as usize..(col + len) as usize))
            .unwrap_or_default()
            .to_owned()
    };
    let highlights_fn = |src: &str, name: &str| {
        decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        )
        .iter()
        .any(|&(line, col, len, k, _)| {
            k == TokenKind::Function as u32 && slice(src, line, col, len) == name
        })
    };
    assert!(
        highlights_fn(
            "proc myCompare {a b} { expr {$a - $b} }\nlsort -command myCompare {3 1 2}\n",
            "myCompare",
        ),
        "the lsort -command callback head must highlight as a Function"
    );
    assert!(
        highlights_fn(
            "proc onScale {v} { }\nscale .s -command onScale\n",
            "onScale",
        ),
        "the Tk scale -command callback head must highlight as a Function (script→prefix conversion)"
    );
}

#[test]
fn method_body_recurses_in_class_definition_script() {
    // `oo::class create C { method m {} { set z 3 } }` — the method body
    // must be tokenised (C Tcl evaluates it as a script), so `set` reads
    // as a function and `z` as a variable declaration, not one opaque
    // string.
    let src = "oo::class create C {\n  method m {} {\n    set z 3\n  }\n}\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
        "expected `z` as a variable declaration inside the method body; got {toks:?}"
    );
    // constructor body too.
    let src = "oo::class create C {\n  constructor {} {\n    set q 9\n  }\n}\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
        "expected `q` declared inside the constructor body; got {toks:?}"
    );
}

#[test]
fn option_command_value_recurses_as_body_script() {
    // `button .b -command {puts $x}` — the `-command` value is a script
    // body (`ArgRole::Body`), so it recurses: `$x` inside the braces
    // resolves as a Variable rather than one opaque string.
    let toks = decode_full(
        "button .b -command {puts $x}\n",
        crate::profile_for_dialect("tk"),
        &reg(),
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
        "expected $x resolved inside the -command body; got {toks:?}"
    );
}

#[test]
fn console_eval_body_recurses_into_script() {
    // `console eval {puts $x}` — the `console eval` script
    // argument is a body (ArgRole::Body via the `console` SubCommand
    // table), so it recurses: `$x` inside the braces resolves as a
    // Variable rather than the whole `{...}` staying one opaque string.
    let toks = decode_full(
        "console eval {puts $x}\n",
        crate::profile_for_dialect("tk"),
        &reg(),
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
        "expected $x resolved inside the `console eval` body; got {toks:?}"
    );
}

#[test]
fn consoleinterp_eval_and_record_bodies_recurse_into_script() {
    // `consoleinterp eval`/`record` — both take a
    // script argument that should recurse the same way `console eval`
    // does.
    for sub in ["eval", "record"] {
        let src = format!("consoleinterp {sub} {{puts $x}}\n");
        let toks = decode_full(&src, crate::profile_for_dialect("tk"), &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected $x resolved inside `consoleinterp {sub}` body; got {toks:?}"
        );
    }
}

#[test]
fn option_enum_value_is_enum_member() {
    // `button .b -relief raised` — the closed-set option value is coloured
    // as an EnumMember, not a generic OptionValue.
    let toks = decode_full(
        "button .b -relief raised\n",
        crate::profile_for_dialect("tk"),
        &reg(),
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::EnumMember as u32),
        "expected `raised` as EnumMember; got {toks:?}"
    );
}

#[test]
fn option_textvariable_value_is_variable_declaration() {
    // `entry .e -textvariable myvar` — the value names a variable the widget
    // reads/writes (`ArgRole::VarWrite`), so it is a Variable
    // declaration, not a plain `OptionValue` string.
    let toks = decode_full(
        "entry .e -textvariable myvar\n",
        crate::profile_for_dialect("tk"),
        &reg(),
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
        "expected myvar as a variable declaration; got {toks:?}"
    );
}

#[test]
fn regex_pattern_with_substitution_splits_regex_and_tcl() {
    // `regexp "abc$var.*" $s` — literal `abc` / `.*` sub-tokenise as
    // regex, but `$var` stays a Tcl variable (Tcl resolves it before
    // regexp runs), with no overlap.
    let toks = decode_full("regexp \"abc$var.*\" $s\n", tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
        "expected $var as a variable; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpQuantifier as u32),
        "expected the `*` quantifier from the literal part; got {toks:?}"
    );
    // No overlaps.
    let simple = decode("regexp \"abc$var.*\" $s\n", tcl(), &reg());
    for w in simple.windows(2) {
        let (l0, c0, len0) = w[0];
        let (l1, c1, _) = w[1];
        if l0 == l1 {
            assert!(c1 >= c0 + len0, "overlap; toks={simple:?}");
        }
    }
    // `regexp "$only" $s` — a bare-substitution pattern is just a
    // variable, not a regex anchor.
    let toks = decode_full("regexp \"$only\" $s\n", tcl(), &reg());
    assert!(
        !toks
            .iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpAnchor as u32),
        "the `$` must not be a regex anchor; got {toks:?}"
    );
}

#[test]
fn regex_source_variable_highlights_def_site_literal() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    // The subject is bound: reading an unset `$s` raises before `regexp`
    // runs, so the pattern would never be used.
    let src = "set my_re \".*abc\"\nset s subject\nregexp $my_re $s\n";
    // Without a CompilationUnit the `set` value is a plain string.
    let plain = decode_full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(
        !plain
            .iter()
            .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
        "no regex tokens without a CU; got {plain:?}"
    );
    // With a CU, the `.*abc` literal at the `set` reads as a regex.
    let cu = CompilationUnit::build_for(src, &registry, false);
    let st = full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    );
    let toks = decode_semantic(&st);
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
        "expected the `*` from the def-site literal as a regex quantifier; got {toks:?}"
    );
    // No overlaps introduced.
    for w in toks.windows(2) {
        let (l0, c0, len0, _, _) = w[0];
        let (l1, c1, _, _, _) = w[1];
        if l0 == l1 {
            assert!(c1 >= c0 + len0, "overlap; toks={toks:?}");
        }
    }
}

/// A regex literal bound to a variable is retagged at its definition in
/// every kind of body the unit lowers on its own: a method, a
/// `namespace eval` script and an `apply` lambda. Runs under Tcl 9.0; the
/// rule does not differ between releases.
#[test]
fn regex_source_literal_is_retagged_in_every_body_kind() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    for (body, src) in [
        // Rooted calls: TclOO selects the receiver's private namespace at
        // run time, where a relative command may be shadowed.
        (
            "method",
            "oo::class create C {\n  method m {s} {\n    ::set re \".*x\"\n    ::regexp $re $s\n  }\n}\n",
        ),
        // The subject is bound, so the `regexp` call is reached.
        (
            "namespace eval",
            "namespace eval ::ns {\n  set s subject\n  set re \".*x\"\n  regexp $re $s\n}\n",
        ),
        (
            "apply lambda",
            "apply {{s} {\n  set re \".*x\"\n  regexp $re $s\n}} foo\n",
        ),
    ] {
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, profile, &registry, Some(&cu)));
        assert!(
            toks.iter()
                .any(|&(_, _, _, k, _)| k == TokenKind::RegexpQuantifier as u32),
            "{body}: expected a regex quantifier from the definition literal; got {toks:?}"
        );
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "{body}: overlap; toks={toks:?}");
            }
        }
    }
}

#[test]
fn object_method_options_resolve_via_registry() {
    // The ngspice / ticklecharts pattern, end-to-end through
    // the CompilationUnit: `set chart [ticklecharts::chart new]` binds the
    // handle's class, so `$chart Xaxis -name … -type value …` resolves the
    // method and its declared options through the registry's object-class
    // model — the precise path, not the shape-based fallback.
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "package require ticklecharts\nset chart [ticklecharts::chart new]\n$chart Xaxis -name {v(anode), V} -type value -min 0.4 -splitLine {show True}\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let toks = decode_semantic(&full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    ));
    let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
    // `Xaxis` resolved as a callable method.
    assert!(
        ks.contains(&(TokenKind::Function as u32)),
        "expected Xaxis as a Function token; got {ks:?}"
    );
    // `-name` / `-type` / `-min` / `-splitLine` are decorators.
    assert!(
        ks.iter()
            .filter(|&&k| k == TokenKind::Decorator as u32)
            .count()
            >= 4,
        "expected the four axis options as decorators; got {ks:?}"
    );
    // `-type value` is a closed-set member; `-min 0.4` a generic value.
    assert!(
        ks.contains(&(TokenKind::EnumMember as u32)),
        "expected -type's `value` as an EnumMember; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected -min's `0.4` as an OptionValue; got {ks:?}"
    );
}

#[test]
fn direct_constructor_dispatch_resolves_method() {
    // A direct `[Class new] method …` dispatch (no intermediate variable)
    // resolves the method and its options through the registry too.
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "[ticklecharts::chart new] Yaxis -name Y -max 10\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let toks = decode_semantic(&full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    ));
    let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
    assert!(
        ks.contains(&(TokenKind::Decorator as u32))
            && ks.contains(&(TokenKind::OptionValue as u32)),
        "expected -name/-max decorators + values on direct dispatch; got {ks:?}"
    );
}

#[test]
fn collection_dispatch_resolves_user_configurable_method() {
    // End-to-end: a `Pins` dict is filled with `[Pin new]`
    // handles in one method and an element is dispatched with
    // `[dict get $Pins $pin] configure -node …` in another.  The receiver
    // resolves to the user `oo::configurable` class, so `configure` is a
    // method and `-node` (a declared property) an option — not the plain
    // strings they read as without collection-element + user-class
    // resolution.
    // Root registry calls because TclOO selects the receiver's private
    // namespace at runtime, where a relative command may be shadowed.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {pin} { ::dict append Pins $pin [::Pin new] }\n\
                     method cfg {pin node} { [::dict get $Pins $pin] configure -node $node }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // The dispatch line carries `dict` (a `defaultLibrary` Function); the
    // resolved *method* is a plain Function (no `defaultLibrary`), which is
    // the signal that distinguishes resolution from the built-in.
    let user_method_on_dispatch_line = |toks: &[(u32, u32, u32, u32, u32)]| {
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0)
    };
    // Without analysis: `configure` stays an unresolved string — only
    // `dict` (defaultLibrary) is a Function on the line.
    let plain = decode_semantic(&full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    ));
    assert!(
        !user_method_on_dispatch_line(&plain),
        "without analysis, no user method resolves; got {plain:?}"
    );
    // With analysis: the dynamic dispatch resolves `configure` as a method.
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        user_method_on_dispatch_line(&toks),
        "`configure` on the retrieved Pin should resolve to a method; got {toks:?}"
    );
}

#[test]
fn dict_for_loop_var_dispatch_resolves() {
    // `dict for {k pin} $Pins {$pin configure …}` — iterating an object
    // collection binds `pin` to an element, so the loop-body dispatch
    // resolves the user method just like the `[dict get …]` retrieval
    // (the SpiceGenTcl `allNodes` / `floating` shape).
    // Root registry calls because TclOO selects the receiver's private
    // namespace at runtime, where a relative command may be shadowed.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { ::dict append Pins $p [::Pin new] }\n\
                     method dump {} { ::dict for {k pin} $Pins { ::puts [$pin configure -node] } }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // `configure` on the loop var resolves to a user method (plain Function,
    // no `defaultLibrary`) on the `dump` method's line.
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
        "expected `configure` on the dict-for value var to resolve; got {toks:?}"
    );
}

/// A method call through each kind of receiver is coloured as a method on
/// the stated line. A `false` row pins a receiver that states no class.
#[test]
fn receiver_dispatch_colours_the_method_word() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    for (receiver, src, line, resolves) in [
        (
            "`my` inside a class body",
            "oo::class create C {\n    method helper {} {}\n    method run {} { my helper }\n}\n",
            2,
            true,
        ),
        // tclsh 9.0.4: an `oo::configurable` instance answers `configure`,
        // an `oo::class` instance `unknown method "configure"`. The
        // metaclass decides, not a declared property.
        (
            "`configure` on an `oo::configurable` instance",
            "oo::configurable create Point {}\nset pt [Point new]\n$pt configure -x 1\n",
            2,
            true,
        ),
        (
            "`configure` on a plain `oo::class` instance",
            "oo::class create Plain { method m {} {} }\nset p [Plain new]\n$p configure -x 1\n",
            2,
            false,
        ),
        (
            "snit `$self`",
            "package require snit\nsnit::type C {\n    method helper {} {}\n    method run {} { $self helper }\n}\n",
            3,
            true,
        ),
        (
            "snit component installed with a type",
            "package require snit\nsnit::widget verticalAxis { method draw {} {} }\nsnit::widget chart {\n    component axis\n    constructor {args} {\n      install axis using verticalAxis $win.a\n      $axis draw\n    }\n}\n",
            6,
            true,
        ),
        // snit(n): `installhull using TYPE` creates the implicit hull;
        // `installhull $win` names an existing widget and no type.
        (
            "snit hull installed with a type",
            "package require snit\nsnit::widget frameish { method draw {} {} }\nsnit::widgetadaptor chart {\n    constructor {args} {\n      installhull using frameish\n      $hull draw\n    }\n}\n",
            5,
            true,
        ),
        (
            "snit hull installed from an existing widget",
            "package require snit\nsnit::widget frameish { method draw {} {} }\nsnit::widgetadaptor chart {\n    constructor {args} {\n      installhull $win\n      $hull draw\n    }\n}\n",
            5,
            false,
        ),
        (
            "snit bare-word constructor with a computed name",
            "package require snit\nsnit::type Engine { method run {} {} }\nsnit::type Wrapper {\n    variable eng\n    constructor {} { set eng [Engine ${selfns}::e] }\n    method go {} { $eng run }\n}\n",
            5,
            true,
        ),
        (
            "result of a factory procedure",
            "oo::class create C { method mrun {} {} }\nproc make {} { return [C new] }\nset o [make]\n$o mrun\n",
            3,
            true,
        ),
        (
            "object passed as a procedure parameter",
            "oo::configurable create Pin { property node }\nproc connect {dev t} { $dev configure -node $t }\nset p [Pin new]\nconnect $p n1\n",
            1,
            true,
        ),
        (
            "handle bound by `set`",
            "oo::configurable create Pin { property node }\nset p [Pin new]\n$p configure -node 5\n",
            2,
            true,
        ),
    ] {
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert_eq!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == line && k == TokenKind::Method as u32 && m == 0),
            resolves,
            "{receiver}: {toks:?}"
        );
    }
}

#[test]
fn widget_method_tokens_follow_the_resolved_tk_lifecycle() {
    // `current` belongs to ttk::treeview only from Tk 9.1. The semantic
    // provider must consume the registry's filtered object-method table,
    // not the raw class descriptor that contains future methods.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;

    let src = "package require Tk\nttk::treeview .tree\n.tree current\n";
    for (dialect, expected) in [("tcl9.0", false), ("tcl9.1", true)] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let cu = CompilationUnit::build_for(src, registry, false);
        let analysis = Analyser::new().analyse(src, dialect);
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            src,
            profile,
            registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert_eq!(
            tokens
                .iter()
                .any(|&(line, _, _, kind, _)| { line == 2 && kind == TokenKind::Method as u32 }),
            expected,
            "ttk::treeview current semantic token under {dialect}: {tokens:?}"
        );
    }
}

/// A bare (namespace-less) `apply {{} {...}}` runs its body in a *fresh*
/// call frame in the global namespace by default — never the enclosing
/// method's object namespace — so `my helper` inside it is not actually a
/// call to the enclosing class's method at runtime (`my` isn't defined in
/// `::`); a class-definition-body scan would raise "invalid command name
/// my" here. The `enclosing_class` context must not leak into an
/// `apply`-lambda body the way it correctly persists into an ordinary
/// nested `if`/`foreach` body, or this gets painted as a resolved,
/// legitimate method call anyway — the same fresh-frame rule call-graph
/// namespace resolution, param traits and declarations apply.
#[test]
fn my_call_inside_apply_lambda_body_does_not_resolve() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "oo::class create C {\n\
                   \x20   method helper {} {}\n\
                   \x20   method run {} { apply {{} {my helper}} }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        !toks
            .iter()
            .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
        "a bare apply lambda's `my helper` must not resolve as the \
             enclosing class's method — it isn't one at runtime; got {toks:?}"
    );
}

#[test]
fn snit_typemethod_call_does_not_type_handle() {
    // `set x [Engine spawn]` is a *typemethod* call, not a construction —
    // its result is not an Engine, so `$x run` must NOT resolve (soundness:
    // the bare-constructor scan excludes declared typemethods).
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "snit::type Engine { method run {} {} \n\
                   \x20   typemethod spawn {} {} }\n\
                   set x [Engine spawn]\n\
                   $x run\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // `$x run` on line 3 must stay unresolved (`run` not a Function token).
    assert!(
        !toks
            .iter()
            .any(|&(l, _, _, k, _)| l == 3 && k == TokenKind::Function as u32),
        "expected `$x run` on a typemethod result to abstain; got {toks:?}"
    );
}

#[test]
fn my_configure_property_options_resolve() {
    // `my configure -prop` inside an oo::configurable body colours the
    // property option too.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "oo::configurable create C {\n\
                   \x20   property node\n\
                   \x20   constructor {} { my configure -node 0 }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // `configure` resolves (Function) and `-node` is a decorator on line 2.
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
        "expected `my configure` to resolve; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, _)| l == 2 && k == TokenKind::Decorator as u32),
        "expected `-node` property option; got {toks:?}"
    );
}

/// Expected outcome for a `tcloo_dispatch_cases` fixture row.
#[derive(Clone, Copy, PartialEq)]
enum Expect {
    Resolve,
    Abstain,
}

use Expect::{Abstain, Resolve};

/// The `TclOO` object-method dispatch fixture rows:
/// `(name, source, method word, 0-based dispatch line, expectation)`.
const TCLOO_DISPATCH_CASES: &[(&str, &str, &str, u32, Expect)] = &[
    // ---- statically determinable → resolve ----
    (
        "var_new",
        "oo::class create C { method mrun {} {} }\nset o [C new]\n$o mrun\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "direct_new",
        "oo::class create C { method mrun {} {} }\n[C new] mrun\n",
        "mrun",
        1,
        Resolve,
    ),
    (
        // TclOO's own same-object dispatch idiom — `self`
        // called with no argument returns the current object's own
        // command name, and dispatching through it (`[self] m`) reaches
        // the enclosing class exactly like `my m`.
        "bare_self_receiver",
        "oo::class create C {\n    method mrun {} {}\n    method call {} { [self] mrun }\n}\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        // The explicit spelling `self object` — documented as
        // equivalent to a bare `self`.
        "self_object_receiver",
        "oo::class create C {\n    method mrun {} {}\n    method call {} { [self object] mrun }\n}\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "configurable_property",
        "oo::configurable create C { property node }\nset o [C new]\n$o configure -node 1\n",
        "configure",
        2,
        Resolve,
    ),
    (
        "inherited_method",
        "oo::class create B { method base {} {} }\noo::class create D { superclass B }\nset o [D new]\n$o base\n",
        "base",
        3,
        Resolve,
    ),
    (
        "mixin_method",
        "oo::class create M { method mixm {} {} }\noo::class create C { mixin M }\nset o [C new]\n$o mixm\n",
        "mixm",
        3,
        Resolve,
    ),
    (
        "dict_collection",
        "oo::class create C { method mrun {} {} }\ndict set d k [C new]\n[dict get $d k] mrun\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "foreach_loopvar",
        "oo::class create C { method mrun {} {} }\nlappend objs [C new]\nforeach o $objs { $o mrun }\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "interproc_param",
        "oo::class create C { method mrun {} {} }\nproc f {o} { $o mrun }\nset p [C new]\nf $p\n",
        "mrun",
        1,
        Resolve,
    ),
    (
        "proc_return",
        "oo::class create C { method mrun {} {} }\nproc make {} { return [C new] }\nset o [make]\n$o mrun\n",
        "mrun",
        3,
        Resolve,
    ),
    (
        "my_self_call",
        "oo::class create C {\n  method helper {} {}\n  method run {} { my helper }\n}\n",
        "helper",
        2,
        Resolve,
    ),
    (
        "snit_self_call",
        "snit::type C {\n  method helper {} {}\n  method run {} { $self helper }\n}\n",
        "helper",
        2,
        Resolve,
    ),
    (
        "itcl_this_call",
        "itcl::class C {\n  method helper {} {}\n  method run {} { $this helper }\n}\n",
        "helper",
        2,
        Resolve,
    ),
    (
        "snit_install_component",
        "snit::widget Ax { method draw {} {} }\nsnit::widget C {\n  constructor {} { install ax using Ax $win.a\n    $ax draw }\n}\n",
        "draw",
        3,
        Resolve,
    ),
    (
        "snit_bare_constructor",
        "snit::type Eng { method run {} {} }\nsnit::type C {\n  variable e\n  constructor {} { set e [Eng ${selfns}::x] }\n  method go {} { $e run }\n}\n",
        "run",
        4,
        Resolve,
    ),
    (
        "oo_define_added",
        "oo::class create C {}\noo::define C { method added {} {} }\nset o [C new]\n$o added\n",
        "added",
        3,
        Resolve,
    ),
    (
        "registry_class",
        "set c [ticklecharts::chart new]\n$c Xaxis -name x\n",
        "Xaxis",
        1,
        Resolve,
    ),
    (
        // Tk widget instance dispatch, bareword receiver:
        // `ttk::treeview .t` names a widget path exactly like a registry
        // naming factory (`struct::graph g`) — `.t instate …` resolves
        // through the same self-referential `object_class`.
        "widget_bareword",
        "ttk::treeview .t\n.t instate {selected} {}\n",
        "instate",
        1,
        Resolve,
    ),
    (
        // Tk widget instance dispatch, `$var`-captured constructor
        // return value (`set lb [listbox .l]`).
        "widget_var_captured",
        "set lb [listbox .l]\n$lb curselection\n",
        "curselection",
        1,
        Resolve,
    ),
    // ---- genuinely dynamic → must abstain (soundness) ----
    (
        "introspection_class",
        "oo::class create C { method mrun {} {} }\nset o [C new]\nset cls [info object class $o]\n[$cls new] mrun\n",
        "mrun",
        3,
        Abstain,
    ),
    (
        "oo_copy",
        "oo::class create C { method mrun {} {} }\nset a [C new]\nset b [oo::copy $a]\n$b mrun\n",
        "mrun",
        3,
        Abstain,
    ),
    (
        "unknown_param",
        "proc f {o} { $o mrun }\n",
        "mrun",
        0,
        Abstain,
    ),
    (
        // `CLASS create NAME` — resolved via
        // `instance_classes` gated on `created_instance_commands`,
        // merged into the object-class map `insert_object_method_overrides`'s
        // bareword branch already reads (see `NamedInstanceMap`).
        "named_object",
        "oo::class create C { method mrun {} {} }\nC create obj\nobj mrun\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        // The snit *named-constructor* shape: `$o` bound by `foo create
        // x` types as `foo` (the signature scan records snit types as
        // classes), so the dispatch resolves like any handle.
        "snit_named_object",
        "snit::type foo { method smeth {} {} }\nset o [foo create x]\n$o smeth\n",
        "smeth",
        2,
        Resolve,
    ),
];

/// Golden fixture for `TclOO` object-method dispatch resolution — validated
/// against the Tcler's-wiki pattern catalogue and a real corpus (tcllib,
/// tklib, `SpiceGenTcl`).  Two guarantees:
///
/// * **`Resolve`** — a statically-determinable dispatch colours its method a
///   callable, for every form the resolver supports.
/// * **`Abstain`** — a genuinely-dynamic dispatch (or a form we do not model)
///   leaves its method a plain string, never a *mis-highlighted* callable
///   (soundness guard: no false positives).
///
/// Adding a pattern here is how object-dispatch support is expanded and
/// measured; flip an `Abstain` to `Resolve` when a form becomes supported.
#[test]
fn tcloo_dispatch_pattern_fixture() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let cases = TCLOO_DISPATCH_CASES;
    let registry = release_reg("tcl9.0");
    let mut failures = Vec::new();
    for &(name, src, method, line, expect) in cases {
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // Column of the method word on the dispatch line (word-boundary,
        // first occurrence — unambiguous in these snippets).  ASCII, so byte
        // == UTF-16 column.
        let src_line = src.lines().nth(line as usize).unwrap_or("");
        let mcol = src_line.match_indices(method).find_map(|(i, _)| {
            let before = src_line.as_bytes().get(i.wrapping_sub(1)).copied();
            let after = src_line.as_bytes().get(i + method.len()).copied();
            let boundary =
                |b: Option<u8>| b.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_');
            (boundary(before) && boundary(after))
                .then(|| u32::try_from(i).expect("column fits u32"))
        });
        // The method resolves iff *its own* token is a callable `Function`.
        let resolved = mcol.is_some_and(|c| {
            toks.iter()
                .any(|&(l, tc, _, k, _)| l == line && tc == c && k == TokenKind::Method as u32)
        });
        let ok = match expect {
            Resolve => resolved,
            Abstain => !resolved,
        };
        if !ok {
            let want = if expect == Resolve {
                "resolve"
            } else {
                "abstain"
            };
            failures.push(format!(
                "  {name}: `{method}` expected to {want} but did not"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "TclOO dispatch fixture regressions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn cross_file_constructor_dispatch_resolves() {
    // `[::Pin new] configure -node 5` where `Pin` is defined in another
    // file: the project's class cards supply the class, as the project
    // token query does. Without a provider the selector is not a method.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards;
    let library = Analyser::new().analyse(
        "oo::configurable create ::Pin { property node }\n",
        "tcl9.0",
    );
    let providers = [OriginalProjectClassCards::capture(&library).unwrap()];
    let user = "[::Pin new] configure -node 5\n";
    let caller = Analyser::new().analyse(user, "tcl9.0");
    let method = |project_classes| {
        let tokens = decode_semantic(&full_with_cu_and_facts(
            user,
            caller.resolved_profile().unwrap(),
            caller.resolved_registry().unwrap(),
            None,
            WorkspaceTokenFacts {
                classes: Some(library.class_hierarchy()),
                project_classes,
                analysis: Some(&caller),
                ..WorkspaceTokenFacts::default()
            },
        ));
        tokens
            .iter()
            .any(|&(l, c, _, k, m)| l == 0 && c == 12 && k == TokenKind::Method as u32 && m == 0)
    };
    assert!(
        method(Some(&providers[..])),
        "the provider's class resolves"
    );
    assert!(!method(None), "no provider, no method");
}

#[test]
fn dict_map_in_return_dispatch_resolves() {
    // `return [dict map {k v} $coll {$v method …}]` — the loop is nested in
    // a command substitution, so the IR never surfaces it as a loop; the
    // syntactic scan still binds `v` to the collection element (SpiceGenTcl
    // `getPinsNodes` / `getParams` shape).
    // Root registry calls because TclOO selects the receiver's private
    // namespace at runtime, where a relative command may be shadowed.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { ::dict append Pins $p [::Pin new] }\n\
                     method nodes {} { ::return [::dict map {k pin} $Pins {$pin configure -node}] }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
        "expected `configure` in the return-nested dict map to resolve; got {toks:?}"
    );
}

#[test]
fn malformed_braced_object_handle_does_not_resolve_a_method() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "oo::configurable create Pin { property node }\n\
                   set p [Pin new]\n\
                   ${p configure -node 5\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        !toks
            .iter()
            .any(|&(l, _, _, k, _)| { l == 2 && k == TokenKind::Method as u32 }),
        "malformed `${{p` must not dispatch to Pin: {toks:?}"
    );
    assert_eq!(object_handle_name("${x}", None), Some("x"));
    assert_eq!(object_handle_name("$x", None), Some("x"));
    assert_eq!(object_handle_name("$arr(idx)", None), Some("arr(idx)"));
    assert_eq!(object_handle_name("$::a:::b", None), Some("::a:::b"));
    assert_eq!(object_handle_name("$foo:::", None), Some("foo:::"));
    assert_eq!(object_handle_name("${x", None), Some("{x"));
}

/// Decode a `SemanticTokens` value directly into
/// `(line, col, len, kind, mods)` tuples.
pub(super) fn decode_semantic(st: &SemanticTokens) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len, kind, mods) = (c[0], c[1], c[2], c[3], c[4]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        out.push((line, col, len, kind, mods));
    }
    out
}

#[test]
fn comment_inside_multiline_string_is_not_a_comment() {
    // A `#`-first line inside a multi-line `"…"` string is string text,
    // not a command comment — it must not emit a Comment token (which
    // would overlap the `$x` variable substitution).
    let src = "append s \"line1\n# not a comment $x\nline3\"\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        !toks
            .iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
        "a `#` inside a string must not be a comment; got {toks:?}"
    );
    // A real comment still is one.
    let toks = decode_full("# real\nset x 1\n", tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
        "expected a real comment token; got {toks:?}"
    );
}

#[test]
fn computed_command_head_does_not_overlap() {
    // `chartV$node SetOptions …` — a command head containing a `$node`
    // substitution is a *computed* (non-static) command name, so it is not
    // painted as a single command token.  Its fragments tokenise
    // individually (`$node` as a variable) and must not overlap each other
    // (LSP clients reject overlapping semantic tokens).
    let toks = decode_full("chartV$node SetOptions -x {}\n", tcl(), &reg());
    for w in toks.windows(2) {
        let (l0, c0, len0, ..) = w[0];
        let (l1, c1, ..) = w[1];
        if l0 == l1 {
            assert!(
                c1 >= c0 + len0,
                "overlap: token at col {c1} starts before prev end {}; toks={toks:?}",
                c0 + len0
            );
        }
    }
    // The `$node` fragment inside the head reads as a variable — the head
    // is not swallowed into one command-head token (which would hide the
    // substitution and mislabel a dynamic command as a resolved one).
    assert!(
        toks.iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 6 && k == TokenKind::Variable as u32),
        "expected a `$node` variable fragment at col 6; got {toks:?}"
    );
    // No token is a `Function` command head spanning the computed word.
    assert!(
        !toks
            .iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
        "computed head must not emit a function command token; got {toks:?}"
    );
}

#[test]
fn command_substitution_head_recurses_not_command_token() {
    // `[dict get $Pins $pin] configure -node $node` — the head
    // is a `[…]` command substitution, a runtime-computed command name, not
    // a resolvable command.  It must recurse into its inner script (`dict`
    // as a builtin, `get` as its subcommand, `$Pins` / `$pin` as variables)
    // rather than paint the whole `[…]` word as one function command token.
    let src = "[dict get $Pins $pin] configure -node $node\n";
    let toks = decode_full(src, tcl(), &reg());
    // `dict` inside the substitution head is a builtin function token.
    assert!(
        toks.iter().any(|&(l, c, _, k, m)| l == 0
            && c == 1
            && k == TokenKind::Function as u32
            && m == MOD_DEFAULT_LIBRARY),
        "expected `dict` builtin token at col 1; got {toks:?}"
    );
    // `$Pins` inside the head is a variable, not swallowed by a command token.
    assert!(
        toks.iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 10 && k == TokenKind::Variable as u32),
        "expected `$Pins` variable at col 10; got {toks:?}"
    );
    // Nothing paints the computed head as a function command token at col 0.
    assert!(
        !toks
            .iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
        "the `[…]` head must not be a function command token; got {toks:?}"
    );
}

#[test]
fn variable_command_head_is_a_variable() {
    // `$obj configure -node $node` — an object-handle dispatch whose class
    // is unknown.  The `$obj` head is a variable substitution, not a
    // command name, so it reads as a variable rather than a function token
    // (the `$chart` object-dispatch shape).
    let src = "$obj configure -node $node\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Variable as u32),
        "expected `$obj` head to read as a variable; got {toks:?}"
    );
    assert!(
        !toks
            .iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
        "the `$obj` head must not be a function command token; got {toks:?}"
    );
}

#[test]
fn apply_lambda_body_recurses() {
    // `apply {{} { set z 3 }}` — the lambda body (list element 1) is a
    // script; `set`/`z` must tokenise rather than sit inside one string.
    let src = "apply {{} { set z 3 }}\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Function as u32),
        "expected `set` function token inside the lambda body; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
        "expected `z` declared inside the lambda body; got {toks:?}"
    );
    // `apply $lambda` (a variable, not a literal) must not be recursed.
    let ks = kinds("apply $lambda a b\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Variable as u32)),
        "expected the $lambda variable token; got {ks:?}"
    );
}

#[test]
fn apply_bare_arglist_param_is_a_parameter() {
    // `apply {dir { … }}` — the argument list is a bare,
    // unbraced single name.  Its parameter (`dir`) must highlight as a
    // `Parameter` declaration (not a `string`), and the body commands
    // must still tokenise as a script.
    let registry = reg();
    let src = "apply {dir {\n    puts $dir\n}} /tmp\n";
    assert!(
        has_token_kind(src, tcl(), &registry, "dir", TokenKind::Parameter),
        "bare arg-list param `dir` must be a Parameter declaration; got {:?}",
        decode_full(src, tcl(), &registry)
    );
    assert!(
        has_token_kind(src, tcl(), &registry, "puts", TokenKind::Function),
        "apply body command `puts` must tokenise as a Function; got {:?}",
        decode_full(src, tcl(), &registry)
    );
    // A braced arg list still emits its names as parameters.
    let braced = "apply {{a b} { expr {$a + $b} }} 1 2\n";
    assert!(
        has_token_kind(braced, tcl(), &registry, "a", TokenKind::Parameter)
            && has_token_kind(braced, tcl(), &registry, "b", TokenKind::Parameter),
        "braced arg-list params must stay parameters; got {:?}",
        decode_full(braced, tcl(), &registry)
    );
    // A computed (`$dynamic`) arg list is not painted as a parameter.
    assert!(
        !has_token_kind(
            "apply [list $al $body]\n",
            tcl(),
            &registry,
            "al",
            TokenKind::Parameter
        ),
        "a computed arg list must not be a parameter declaration"
    );
}

/// `apply`'s lambda body is reachable indirectly through
/// `[list apply {…} $x]`, the idiomatic way
/// to build a deferred command around a dynamic value — most commonly a
/// pkgIndex.tcl `package ifneeded name ver [list apply {dir {…}} $dir]`
/// entry. TP cases: the reported repro, a namespace-qualified `::apply`,
/// and the same idiom under a *different* enclosing Body-role command
/// (`after idle`) to prove the recognition is registry-driven (any
/// Body-role position), not special-cased to `package ifneeded`.
#[test]
fn list_quoted_apply_lambda_body_recurses() {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let registry = reg();

    // The exact reported repro: a pkgIndex.tcl-style entry.
    let pkgindex = "package ifneeded myPackage 1.0.0 [list apply {dir {\n    source [file join $dir x.tcl]\n}} $dir]\n";
    assert!(
        has_token_kind(pkgindex, tcl(), &registry, "source", TokenKind::Keyword),
        "pkgIndex-style list-quoted apply body: `source` must tokenise; got {:?}",
        decode_full(pkgindex, tcl(), &registry)
    );
    assert!(
        has_token_kind(pkgindex, tcl(), &registry, "dir", TokenKind::Parameter),
        "pkgIndex-style list-quoted apply: `dir` param must be a Parameter; got {:?}",
        decode_full(pkgindex, tcl(), &registry)
    );
    // The reconstructed command-name word itself reads as a call-site
    // reference, same as a literal head — not a plain string.
    assert!(
        has_token_kind(pkgindex, tcl(), &registry, "apply", TokenKind::Function),
        "the list-quoted `apply` word must read as a Function reference; got {:?}",
        decode_full(pkgindex, tcl(), &registry)
    );

    // A `::`-qualified spelling resolves the same way (registry `get`
    // strips a leading `::`, exactly like a direct `::apply {…}` call).
    let qualified = "package ifneeded p 1.0 [list ::apply {dir {puts $dir}} $dir]\n";
    assert!(
        has_token_kind(qualified, tcl(), &registry, "puts", TokenKind::Function),
        "qualified `::apply` list-quoted body: `puts` must tokenise; got {:?}",
        decode_full(qualified, tcl(), &registry)
    );

    // A *different* Body-role enclosing command (`after idle`) proves
    // the fix isn't specific to `package ifneeded`.
    let after_idle = "after idle [list apply {{x} {puts $x}} 5]\n";
    assert!(
        has_token_kind(after_idle, tcl(), &registry, "puts", TokenKind::Function),
        "after-idle list-quoted apply body: `puts` must tokenise; got {:?}",
        decode_full(after_idle, tcl(), &registry)
    );
    assert!(
        has_token_kind(after_idle, tcl(), &registry, "x", TokenKind::Parameter),
        "after-idle list-quoted apply: `x` param must be a Parameter; got {:?}",
        decode_full(after_idle, tcl(), &registry)
    );
}

/// FP guards for the `[list …]`-quoted lambda recognition: a plain data
/// list, a list naming an unregistered head, and a dynamic `list` head
/// must never be split as if they were an apply lambda.
#[test]
fn list_quoted_apply_lambda_false_positive_guards() {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let registry = reg();

    // Ordinary data list: `list`'s own args stay whatever the default
    // classifier gives them — none of them becomes a `Parameter`
    // declaration (which only a recognised lambda-literal split emits).
    let data_list = "set data [list puts hello world]\n";
    assert!(
        !has_token_kind(data_list, tcl(), &registry, "hello", TokenKind::Parameter),
        "a plain data list must not be split as a lambda literal"
    );

    // `list`'s first argument names a command that does not carry
    // `ArgRole::LambdaLiteral` (`puts` is an ordinary command) — its
    // second argument must not be treated as a lambda body either.
    assert!(
        !has_token_kind(
            "set cb [list puts hello]\n",
            tcl(),
            &registry,
            "hello",
            TokenKind::Parameter
        ),
        "list-quoting a non-lambda-literal command must not split its trailing arg"
    );

    // An unregistered head: no crash, no spurious split.
    let unknown = "set cb [list notARealCommand {dir {source x.tcl}} $dir]\n";
    assert!(
        !has_token_kind(unknown, tcl(), &registry, "dir", TokenKind::Parameter),
        "an unresolvable list-quoted head must not be split as a lambda literal"
    );

    // A dynamic `list` head (`$cmd`) can't be resolved statically.
    let dynamic_head = "set cb [list $cmd {dir {source x.tcl}} $dir]\n";
    assert!(
        !has_token_kind(dynamic_head, tcl(), &registry, "dir", TokenKind::Parameter),
        "a dynamic list head must not be split as a lambda literal"
    );

    // `[llength $x]` — a `Cmd` substitution whose head is `llength`, not
    // `list` (no `BUILDS_COMMAND_PREFIX` trait) — must not be misread.
    assert!(
        !has_token_kind(
            "set n [llength $items]\n",
            tcl(),
            &registry,
            "items",
            TokenKind::Parameter
        ),
        "llength must never be treated as a command-quoting construct"
    );

    // A well-formed `[list apply …]` shape sitting in an *inert* argument slot (here `set`'s value,
    // which carries no `Body` / `LambdaLiteral` / `CommandPrefix` role)
    // must not be treated as a deferred invocation — `list` only ever
    // returns a value here; nothing ever invokes `apply`.
    let inert_data = "set data [list apply {x {puts $x}} value]\n";
    assert!(
        !has_token_kind(inert_data, tcl(), &registry, "x", TokenKind::Parameter),
        "an inert `[list apply …]` value must not paint its param as a \
             Parameter; got {:?}",
        decode_full(inert_data, tcl(), &registry)
    );
    assert!(
        !has_token_kind(inert_data, tcl(), &registry, "puts", TokenKind::Function),
        "an inert `[list apply …]` value must not recurse into its body \
             as executable code; got {:?}",
        decode_full(inert_data, tcl(), &registry)
    );
    assert!(
        !has_token_kind(inert_data, tcl(), &registry, "apply", TokenKind::Function),
        "an inert `[list apply …]` value's `apply` word must not read as \
             a call-site reference; got {:?}",
        decode_full(inert_data, tcl(), &registry)
    );
}

/// TN: `package ifneeded`'s script argument, when a literal braced script
/// (no `[list …]` wrapper), is recognised generically as `ArgRole::Body`.
#[test]
fn package_ifneeded_literal_script_recurses_as_body() {
    let registry = reg();
    let src = "package ifneeded myPackage 1.0.0 {\n    source [file join $dir x.tcl]\n}\n";
    assert!(
        has_token_kind(src, tcl(), &registry, "source", TokenKind::Keyword),
        "package ifneeded literal script: `source` must tokenise; got {:?}",
        decode_full(src, tcl(), &registry)
    );
}

/// True if any token in `src` covers exactly `text` with kind `kind`.
fn has_token_kind(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    text: &str,
    kind: TokenKind,
) -> bool {
    decode_full(src, dialect, registry)
        .iter()
        .any(|&(l, c, len, k, _)| {
            k == kind as u32
                && src
                    .lines()
                    .nth(l as usize)
                    .and_then(|line| {
                        let s = line.char_indices().nth(c as usize).map(|(i, _)| i)?;
                        let e = line
                            .char_indices()
                            .nth((c + len) as usize)
                            .map_or(line.len(), |(i, _)| i);
                        line.get(s..e)
                    })
                    .is_some_and(|got| got == text)
        })
}

#[test]
fn uplevel_body_recurses_as_script() {
    // The braced body of `uplevel ?level? {body}` runs in
    // another stack frame but is still a Tcl script — it must be
    // recursed and highlighted, not rendered as one opaque string.
    let registry = reg();

    // `uplevel 1 {body}` — literal relative level, body at arg 1.
    let src = "uplevel 1 {foreach x $l { puts $x }}\n";
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "foreach",
            TokenKind::Keyword
        ),
        "uplevel 1 body: `foreach` must tokenise as a keyword; got {:?}",
        decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry
        )
    );
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "puts",
            TokenKind::Function
        ),
        "uplevel 1 body: `puts` must tokenise as a function"
    );

    // `uplevel {body}` — no level, body at arg 0.
    let src = "uplevel {foreach x $l { puts $x }}\n";
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "foreach",
            TokenKind::Keyword
        ),
        "uplevel (no level) body: `foreach` must tokenise as a keyword"
    );

    // `uplevel #0 {body}` — absolute global level.
    let src = "uplevel #0 {foreach x $l { puts $x }}\n";
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "foreach",
            TokenKind::Keyword
        ),
        "uplevel #0 body: `foreach` must tokenise as a keyword"
    );

    // `uplevel $lvl {body}` — dynamic level word, body still recurses.
    let src = "uplevel $lvl {foreach x $l { puts $x }}\n";
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "foreach",
            TokenKind::Keyword
        ),
        "uplevel $lvl body: `foreach` must tokenise as a keyword"
    );
}

#[test]
fn uplevel_dynamic_body_not_recursed() {
    // `uplevel 1 $body` — the body is a bare variable, not a braced
    // literal, so it stays a `$body` variable token (the const-lattice
    // lowering resolves it on the compiler side, not the token layer).
    let registry = reg();
    let src = "uplevel 1 $body\n";
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "$body",
            TokenKind::Variable
        ),
        "uplevel 1 $body: the body variable must stay a variable token; got {:?}",
        decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry
        )
    );
}

#[test]
fn uplevel_issue_837_repro_recurses() {
    // A `foreach` /
    // `namespace children` / `namespace forget` body inside
    // `uplevel 1 {…}` must highlight, not sit inside one string.
    let registry = reg();
    let src = "proc forgetXyce {} {\n    uplevel 1 {foreach nameSpc [namespace children ::SpiceGenTcl::Xyce] {\n        namespace forget ${nameSpc}::*\n    }}\n}\n";
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "foreach",
            TokenKind::Keyword
        ),
        "issue #837: `foreach` inside the uplevel body must be a keyword; got {:?}",
        decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry
        )
    );
    assert!(
        has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "namespace",
            TokenKind::Keyword
        ) || has_token_kind(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            "namespace",
            TokenKind::Function
        ),
        "issue #837: `namespace` inside the uplevel body must be highlighted"
    );
    // The `${nameSpc}` reference deep inside the body highlights as a
    // variable — proof the whole body was re-lexed, not stringified.
    assert!(
        decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry
        )
        .iter()
        .any(|&(_, _, _, k, _)| k == TokenKind::Variable as u32),
        "issue #837: a variable token is expected from the recursed body"
    );
}

#[test]
fn operator_command_head_classified_as_operator() {
    // `+ 3 4` — the operator head is `operator`, not `function`.
    let ks = kinds("+ 3 4\n", tcl(), &reg());
    assert_eq!(ks.first(), Some(&(TokenKind::Operator as u32)), "{ks:?}");
}

/// A hand-typed symbol list would miss every word-form comparison operator
/// — `eq 1 1` (a bare `::tcl::mathop::eq` invocation via `namespace
/// import`) classifying as `function` rather than `operator`. Also covers
/// the TIP 461 `lt`/`le`/`gt`/`ge` mathop commands (9.0+).
#[test]
fn word_form_operator_command_heads_classified_as_operator() {
    for op in ["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"] {
        let src = format!("{op} 1 1\n");
        let ks = kinds(
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &reg(),
        );
        assert_eq!(
            ks.first(),
            Some(&(TokenKind::Operator as u32)),
            "{op}: {ks:?}"
        );
    }
}

#[test]
fn bareword_argument_classified_as_string() {
    // `puts hello` → function head + a `string` token for the bareword
    // arg, not a dropped arg.
    let ks = kinds("puts hello\n", tcl(), &reg());
    assert_eq!(ks.len(), 2, "expected head + arg token; got {ks:?}");
    assert!(
        ks.contains(&(TokenKind::String as u32)),
        "bareword arg not classified as string; got {ks:?}"
    );
}

#[test]
fn legend_includes_regexp_and_event() {
    let types = legend_token_types();
    assert_eq!(types[TokenKind::Regexp as usize], "regexp");
    assert_eq!(types[TokenKind::Event as usize], "event");
}

#[test]
fn regexp_pattern_classified_as_regexp() {
    // `regexp {abc} $s` — the `{abc}` pattern argument is `regexp`,
    // not `string`.
    let ks = kinds("regexp {abc} $s\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Regexp as u32)),
        "expected a regexp token; got {ks:?}"
    );
    // `regsub -all {x+} $s y out` — option-skip finds the pattern.
    let ks = kinds("regsub -all {x+} $s y out\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Regexp as u32)),
        "expected a regexp token after -all; got {ks:?}"
    );
}

/// Every resolver-owned pattern layout needs the same
/// source proof as lsearch. A dynamic leading word can be a value-taking
/// switch at runtime, so it cannot make the later literal a regexp; the
/// analogous regsub query must not steal the replacement as a template.
#[test]
fn dynamic_leading_options_do_not_claim_pattern_or_regsub_replacement_tokens() {
    let registry = reg();
    for src in [
        "regexp $mode {a+} $value\n",
        "regsub $mode {a+} $value {\\1}\n",
    ] {
        let tokens = decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            !tokens.iter().any(|&(_, _, _, kind, _)| {
                matches!(
                    kind,
                    value
                        if value == TokenKind::Regexp as u32
                            || value == TokenKind::RegexpQuantifier as u32
                )
            }),
            "{src:?}: unresolved leading option must not claim a regex word: {tokens:?}",
        );
    }

    let tokens = decode_full(
        "regsub $mode {a} $value {\\1}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(
        !tokens.iter().any(|&(_, _, _, kind, _)| {
            kind == TokenKind::Number as u32 || kind == TokenKind::Operator as u32
        }),
        "an unresolved regsub replacement is ordinary Tcl text: {tokens:?}",
    );

    for src in [
        "regsub -c {a} $value {\\1}\n",
        "regsub -command {a} $value callback\n",
    ] {
        let tokens = decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            !tokens.iter().any(|&(_, _, _, kind, _)| {
                kind == TokenKind::Number as u32 || kind == TokenKind::Operator as u32
            }),
            "{src:?}: only a valid template layout may receive regsub sub-tokens: {tokens:?}",
        );
    }

    let tokens = decode_full(
        "regsub -start $start {a} $value {\\1}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(
        tokens
            .iter()
            .any(|&(_, _, _, kind, _)| kind == TokenKind::Number as u32),
        "a known fixed-width -start option preserves the regsub replacement: {tokens:?}",
    );
}

#[test]
fn event_name_classified_as_event() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let ks = kinds(
        "when HTTP_REQUEST {\n  set x 1\n}\n",
        tcl_dialect::DialectProfile::irules(),
        &registry,
    );
    assert!(
        ks.contains(&(TokenKind::Event as u32)),
        "expected an event token; got {ks:?}"
    );
}

#[test]
fn malformed_when_body_does_not_classify_an_event_declaration() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    for source in [
        "when HTTP_REQUEST bare_body\n",
        "when HTTP_REQUEST \"quoted body\"\n",
        "when HTTP_REQUEST { unterminated",
    ] {
        let ks = kinds(source, tcl_dialect::DialectProfile::irules(), &registry);
        assert!(
            !ks.contains(&(TokenKind::Event as u32)),
            "malformed declaration colored HTTP_REQUEST as an event: {source:?}; {ks:?}"
        );
    }
}

#[test]
fn bigip_object_ref_token_in_irules_body() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    // `pool web_pool` inside a multi-line `when` body → `object`.
    for dialect in ["f5-irules", "irules", "tcl-irule"] {
        let ks = kinds(
            "when HTTP_REQUEST {\n  pool web_pool\n}\n",
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            &registry,
        );
        assert!(
            ks.contains(&(TokenKind::Object as u32)),
            "{dialect}: {ks:?}"
        );
    }
}

#[test]
fn bigip_object_ref_not_emitted_in_plain_tcl() {
    // The object overlay is iRules-only.
    let ks = kinds("when HTTP_REQUEST {\n  pool web_pool\n}\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
}

#[test]
fn regex_pattern_subtokenised_into_components() {
    // `(a+)+` → group `(`, literal `a`, quantifier `+`, group `)`,
    // quantifier `+`.
    let ks = kinds("regexp {(a+)+} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
    // The whole-pattern `regexp` kind is replaced by sub-tokens, but
    // the literal `a` run is still `regexp`.
    assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
}

#[test]
fn regex_char_class_and_anchor_subtokens() {
    let ks = kinds("regexp {^[0-9]+$} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::RegexpAnchor as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
}

#[test]
fn regex_alternation_and_escape_subtokens() {
    let ks = kinds("regexp {a\\d|b} $s\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::RegexpAlternation as u32)),
        "{ks:?}"
    );
    // `\d` is an ARE class shortcut → char class.
    assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
}

#[test]
fn escape_subtoken_width_follows_the_selected_release_grammar() {
    let source = "puts \"\\x4142\"\n";
    let tcl84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let escape_width = |profile| {
        full(source, profile, &reg())
            .data
            .chunks(5)
            .find(|token| token[3] == TokenKind::Escape as u32)
            .map(|token| token[2])
            .expect("the string contains an escape sub-token")
    };

    // Tcl 8.4 consumes every hex digit; Tcl 9.0 consumes exactly two.
    // The source slice and token geometry therefore depend on the
    // original document's selected escape configuration.
    assert_eq!(escape_width(tcl84), 6);
    assert_eq!(escape_width(tcl90), 4);
}

#[test]
fn escape_before_multibyte_char_does_not_panic() {
    // Slicing a fixed 2 bytes at the backslash of a `\<non-ASCII>` escape
    // inside a string lands inside the multi-byte char and panics the whole
    // semantic-tokens request. The escape must span the backslash plus the
    // full UTF-8 char.
    for src in [
        "puts \"\\é\"\n",
        "puts \"a\\你b\"\n",
        "puts \"\\€\"\n",
        "puts \"x\\é\\你\"\n",
    ] {
        let ks = kinds(src, tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Escape as u32)),
            "expected an Escape sub-token for {src:?}, got {ks:?}",
        );
    }
}

#[test]
fn scan_are_class_spans_posix_collating_equivalence_subbrackets() {
    // `[[:alpha:]]` is one bracket expression (the inner `[:alpha:]` is a
    // POSIX class whose `]` does not close the outer bracket) — the scanner
    // must span the whole thing, not stop at the first `]`.
    assert_eq!(scan_are_class(b"[[:alpha:]]", 0), Some(11));
    assert_eq!(scan_are_class(b"[[:digit:]xyz]", 0), Some(14));
    assert_eq!(scan_are_class(b"[[.ch.]]", 0), Some(8));
    assert_eq!(scan_are_class(b"[[=a=]]", 0), Some(7));
    // A plain class is unaffected; a leading literal `]` still works.
    assert_eq!(scan_are_class(b"[a-z]", 0), Some(5));
    assert_eq!(scan_are_class(b"[]a]", 0), Some(4));
    // An unterminated sub-bracket is not a token.
    assert_eq!(scan_are_class(b"[[:alpha", 0), None);
}

#[test]
fn regex_posix_class_has_no_dangling_bracket_token() {
    // Without the sub-bracket rule `[[:alpha:]]+` scans as `[[:alpha:]`
    // (char class) + a stray literal `]` + `+`. The whole `[[:alpha:]]` is
    // one char class and `+` its quantifier — and, per the token-overlap
    // invariant, no token may start inside another.
    let src = "regexp {[[:alpha:]]+} $s\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpCharClass as u32),
        "expected a char-class token; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpQuantifier as u32),
        "expected the trailing `+` as a quantifier; got {toks:?}"
    );
    for w in toks.windows(2) {
        let (l0, c0, len0, _, _) = w[0];
        let (l1, c1, _, _, _) = w[1];
        if l0 == l1 {
            assert!(
                c1 >= c0 + len0,
                "token overlap in POSIX class; toks={toks:?}"
            );
        }
    }
}

#[test]
fn regex_without_metachars_stays_single_regexp() {
    // `abc` has no metacharacters → one `regexp` token, no sub-tokens.
    let ks = kinds("regexp {abc} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    assert!(!ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
    assert!(
        !ks.contains(&(TokenKind::RegexpQuantifier as u32)),
        "{ks:?}"
    );
}

#[test]
fn classify_regex_component_maps_each_kind() {
    assert_eq!(classify_regex_component("("), TokenKind::RegexpGroup);
    assert_eq!(classify_regex_component("(?:"), TokenKind::RegexpGroup);
    assert_eq!(
        classify_regex_component("[a-z]"),
        TokenKind::RegexpCharClass
    );
    assert_eq!(classify_regex_component("\\d"), TokenKind::RegexpCharClass);
    assert_eq!(classify_regex_component("."), TokenKind::RegexpCharClass);
    assert_eq!(classify_regex_component("+"), TokenKind::RegexpQuantifier);
    assert_eq!(
        classify_regex_component("{2,3}"),
        TokenKind::RegexpQuantifier
    );
    assert_eq!(classify_regex_component("^"), TokenKind::RegexpAnchor);
    assert_eq!(classify_regex_component("\\b"), TokenKind::RegexpAnchor);
    assert_eq!(classify_regex_component("\\n"), TokenKind::RegexpEscape);
    assert_eq!(classify_regex_component("\\3"), TokenKind::RegexpBackref);
    assert_eq!(classify_regex_component("|"), TokenKind::RegexpAlternation);
}

#[test]
fn sprintf_format_spec_subtokens() {
    // `format {%d}` → `%` percent, `d` spec.
    let ks = kinds("format {%d} $n\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn format_binary_specifier_is_release_gated() {
    let old = kinds(
        "format {%b} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        &reg(),
    );
    assert!(!old.contains(&(TokenKind::FormatSpec as u32)), "{old:?}");
    let modern = kinds(
        "format {%b} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(
        modern.contains(&(TokenKind::FormatSpec as u32)),
        "{modern:?}"
    );
    let tcl9 = kinds(
        "format {%b} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &reg(),
    );
    assert!(tcl9.contains(&(TokenKind::FormatSpec as u32)), "{tcl9:?}");
}

#[test]
fn sprintf_flags_and_width_subtokens() {
    // `%-5.2f` → percent, `-` flag, `5` width, `.` flag, `2` width,
    // `f` spec.
    let ks = kinds("format {%-5.2f} $x\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::FormatFlag as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatWidth as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn scan_format_arg_subtokenised() {
    // `scan`'s format string is arg 2.
    let ks = kinds("scan $s {%d} a\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn format_without_specifiers_stays_string() {
    let ks = kinds("format {plain} $x\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn format_literal_percent_stays_string() {
    let ks = kinds(
        "format {100%%} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(!ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn clock_format_subtokens() {
    // `clock format $t -format {%Y-%m-%d}` → %/letter pairs.
    let ks = kinds("clock format $t -format {%Y-%m-%d}\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
}

#[test]
fn clock_locale_modifier_uses_shared_grammar() {
    // Tcl accepts `%Ey`; the shared clock grammar owns both the modifier
    // and the conversion instead of leaving an editor-only table here.
    let ks = kinds("clock scan $s -format {%Ey}\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::ClockModifier as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
}

#[test]
fn unsupported_clock_g_specifier_stays_string() {
    let ks = kinds(
        "clock scan $s -format {%g}\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn clock_format_without_specifiers_stays_string() {
    let ks = kinds("clock format $t -format {plain}\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn binary_format_spec_and_count_subtokens() {
    // `binary format a3 $d` (arg 2) → spec `a`, count `3`.
    let ks = kinds("binary format a3 $d\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::BinaryCount as u32)), "{ks:?}");
}

#[test]
fn binary_scan_signed_modifier_and_star() {
    // `binary scan $d su r` (arg 3) → spec `s`, modifier `u`.
    let ks = kinds(
        "binary scan $d su r\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    // `c*` → spec `c`, `*` flag.
    let ks = kinds(
        "binary format c* $l\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
}

#[test]
fn binary_signed_modifier_suppressed_in_tcl84() {
    // The `u` suffix is 8.5+ (TIP 275), so under tcl8.4 the `u` is not a
    // binaryFlag.
    let ks = kinds(
        "binary scan $d su r\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        &reg(),
    );
    assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
    assert!(!ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
}

#[test]
fn regsub_replacement_backref_subtokens() {
    // `regsub {a} $s {\1-\&} out` → `\1` number, `\&` operator.
    let ks = kinds("regsub {a} $s {\\1-\\&} out\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Number as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
}

#[test]
fn regsub_replacement_without_backrefs_stays_string() {
    let ks = kinds("regsub {a} $s {plain} out\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
}

#[test]
fn is_event_name_matches_event_shape() {
    assert!(is_event_name("HTTP_REQUEST"));
    assert!(is_event_name("CLIENT_ACCEPTED"));
    assert!(!is_event_name("lowercase"));
    assert!(!is_event_name("X")); // single char — needs 2+
}

#[test]
fn semantic_tokens_are_dialect_aware_via_expand_syntax() {
    // The provider re-segments
    // under the document dialect.  In `foo {*}$x`, on 8.5+ the `{*}`
    // is the expansion operator (consumed — not a highlighted word),
    // but on 8.4 it is a literal braced string `{*}`, which adds an
    // extra `string` token.  So the packed token stream is longer on
    // 8.4.
    let src = "foo {*}$x\n";
    let on_90 = full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &reg(),
    )
    .data;
    let on_84 = full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        &reg(),
    )
    .data;
    assert!(
        on_84.len() > on_90.len(),
        "8.4 keeps `{{*}}` as a highlighted string token (longer stream): \
             8.4={} 9.0={}",
        on_84.len(),
        on_90.len(),
    );
}

#[test]
fn full_returns_non_empty_data_for_simple_proc() {
    let s = full("proc foo {} {}\n", tcl(), &reg());
    // Should have at least: `proc` (keyword), `foo`
    // (function), `{}` (string), `{}` (string).
    assert!(!s.data.is_empty(), "{:?}", s.data);
    // 5 ints per token.
    assert_eq!(s.data.len() % 5, 0);
}

#[test]
fn diff_returns_none_for_identical_streams() {
    let a = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
    assert_eq!(diff(&a, &a), None);
}

#[test]
fn diff_isolates_a_single_changed_token() {
    // Three tokens; only the middle one's type changes.
    let old = vec![
        0, 0, 4, 0, 0, /**/ 0, 5, 3, 1, 0, /**/ 1, 0, 2, 2, 0,
    ];
    let new = vec![
        0, 0, 4, 0, 0, /**/ 0, 5, 3, 4, 0, /**/ 1, 0, 2, 2, 0,
    ];
    let edit = diff(&old, &new).expect("an edit");
    // Skip the first token (5 ints), replace exactly one token.
    assert_eq!(edit.start, 5);
    assert_eq!(edit.delete_count, 5);
    assert_eq!(edit.data, vec![0, 5, 3, 4, 0]);
}

#[test]
fn diff_handles_appended_token() {
    let old = vec![0, 0, 4, 0, 0];
    let new = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
    let edit = diff(&old, &new).expect("an edit");
    // Nothing deleted; one token appended after the prefix.
    assert_eq!(edit.start, 5);
    assert_eq!(edit.delete_count, 0);
    assert_eq!(edit.data, vec![0, 5, 3, 1, 0]);
}

#[test]
fn diff_handles_removed_token() {
    let old = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
    let new = vec![0, 0, 4, 0, 0];
    let edit = diff(&old, &new).expect("an edit");
    // One trailing token removed, nothing spliced in.
    assert_eq!(edit.start, 5);
    assert_eq!(edit.delete_count, 5);
    assert_eq!(edit.data, [] as [u32; 0]);
}

#[test]
fn legend_has_expected_entries() {
    let types = legend_token_types();
    assert_eq!(types[TokenKind::Keyword as usize], "keyword");
    assert_eq!(types[TokenKind::Function as usize], "function");
    assert_eq!(types[TokenKind::Variable as usize], "variable");
    assert_eq!(types[TokenKind::String as usize], "string");
    assert_eq!(types[TokenKind::Number as usize], "number");
    assert_eq!(types[TokenKind::Comment as usize], "comment");
    assert_eq!(types[TokenKind::Namespace as usize], "namespace");
}

#[test]
fn legend_modifiers_order() {
    // Order is load-bearing: `defaultLibrary` must be bit index 3.
    let mods = legend_token_modifiers();
    assert_eq!(
        mods,
        vec!["declaration", "definition", "readonly", "defaultLibrary"]
    );
    assert_eq!(MOD_DEFAULT_LIBRARY, 1 << 3);
}

#[test]
fn builtin_command_head_gets_default_library_modifier() {
    // `puts` is a registry built-in classified as `function`, so its
    // head token carries the `defaultLibrary` modifier (bit 3 = 8).
    let s = full("puts hi\n", tcl(), &reg());
    assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
    assert_eq!(s.data[4], MOD_DEFAULT_LIBRARY, "{:?}", s.data);
}

#[test]
fn user_proc_head_has_no_default_library_modifier() {
    // A user-defined command isn't in the registry → `function`
    // with no modifier.
    let s = full("my_custom_cmd 1 2\n", tcl(), &reg());
    assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
    assert_eq!(s.data[4], 0, "{:?}", s.data);
}

#[test]
fn keyword_head_has_no_default_library_modifier() {
    // `if` is a language keyword, not a `function` — no defaultLibrary.
    let s = full("if {1} { puts hi }\n", tcl(), &reg());
    assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
    assert_eq!(s.data[4], 0, "{:?}", s.data);
}

#[test]
fn keywords_classified_as_keyword() {
    let s = full("if {1} { puts hi }\n", tcl(), &reg());
    // First token's type index should be 0 (Keyword) for `if`.
    // The encoded data: [deltaLine, deltaCol, length, type, modifiers].
    assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
}

/// Decode the packed stream into `(line, col, len, kind)` tuples plus
/// the covered source word (ASCII sources only — byte == utf16).
fn decode_words(src: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32, u32, String)> {
    let st = full(src, tcl(), registry);
    let lines: Vec<&str> = src.split('\n').collect();
    let mut line = 0u32;
    let mut col = 0u32;
    let mut out = Vec::new();
    for c in st.data.chunks(5) {
        let (dl, dc, len, kind) = (c[0], c[1], c[2], c[3]);
        if dl > 0 {
            line += dl;
            col = dc;
        } else {
            col += dc;
        }
        let word = lines
            .get(line as usize)
            .and_then(|l| l.get(col as usize..(col + len) as usize))
            .unwrap_or("")
            .to_string();
        out.push((line, col, len, kind, word));
    }
    out
}

fn keyword_words(src: &str, registry: &CommandRegistry) -> std::collections::HashSet<String> {
    decode_words(src, registry)
        .into_iter()
        .filter(|(_, _, _, kind, _)| *kind == TokenKind::Keyword as u32)
        .map(|(_, _, _, _, word)| word)
        .collect()
}

#[test]
fn if_else_elseif_are_keywords() {
    // else/elseif structural keywords highlight like `if`.
    let src = "if 1 {\n puts a\n} elseif 2 {\n puts b\n} else {\n puts c\n}";
    let kw = keyword_words(src, &reg());
    for expected in ["if", "elseif", "else"] {
        assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
    }
}

#[test]
fn try_on_finally_are_keywords() {
    // try's on/trap/finally structural keywords highlight as keywords.
    let src = "try {\n set x 1\n} on error {e} {\n puts $e\n} finally {\n puts d\n}";
    let kw = keyword_words(src, &reg());
    for expected in ["try", "on", "finally"] {
        assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
    }
}

#[test]
fn builtin_name_as_bareword_arg_is_string() {
    // A builtin name used as a plain dict value stays a string, not a
    // keyword — the KEYWORD role is position-aware (if/try only).
    let src = "dict set frame proc \"asasdas asd\"";
    let proc = decode_words(src, &reg())
        .into_iter()
        .find(|(_, _, _, _, word)| word == "proc")
        .expect("a `proc` token");
    assert_eq!(proc.3, TokenKind::String as u32, "{proc:?}");
}

#[test]
fn original_keyword_contents_preserve_literal_closing_bytes() {
    let config = tcl_lexer::LexerConfig::for_profile(Some(tcl()));
    for (source, expected) in [
        ("{else{}}", "else{}"),
        ("\"else}\"", "else}"),
        ("{else}", "else"),
    ] {
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[0];
        let token = word.tokens()[0];
        assert_eq!(
            keyword_operand_content(source, token, Some(word), config)
                .unwrap()
                .1,
            expected
        );
        assert!(keyword_operand_content("foreign", token, Some(word), config).is_none());
    }
}

#[test]
fn quoted_structural_keyword_offsets_past_quote() {
    // A quoted `"else"` keyword marks `else`, not `"els`.
    let src = "if 0 {} \"else\" {puts ok}";
    let kw = decode_words(src, &reg())
        .into_iter()
        .find(|(_, col, _, kind, _)| *kind == TokenKind::Keyword as u32 && *col >= 8)
        .expect("a keyword token past the first word");
    assert_eq!(kw.4, "else", "{kw:?}");
}

#[test]
fn comments_classified_as_comment() {
    let s = full("# this is a comment\nset x 1\n", tcl(), &reg());
    // The first token should be the comment.
    assert_eq!(s.data[3], TokenKind::Comment as u32, "{:?}", s.data);
}

#[test]
fn variables_classified_as_variable() {
    let s = full("set $x 1\n", tcl(), &reg());
    // The `$x` token kind should be Variable.
    let kinds: Vec<u32> = s.data.chunks(5).map(|c| c[3]).collect();
    assert!(
        kinds.contains(&(TokenKind::Variable as u32)),
        "expected Variable in kinds; got {kinds:?}",
    );
}

/// End-to-end: an `0o17` argument paints as a `Number` under an 8.6
/// document and not under an 8.4 one, where `0o` is not a prefix at all and
/// the word is an ordinary bareword.
#[test]
fn number_tokens_follow_the_document_dialect() {
    let registry = reg();
    let modern = kinds(
        "puts 0o17\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    let old = kinds(
        "puts 0o17\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        &registry,
    );
    assert!(
        modern.contains(&(TokenKind::Number as u32)),
        "0o17 is a number from 8.5: {modern:?}"
    );
    assert!(
        !old.contains(&(TokenKind::Number as u32)),
        "0o17 is not a number under 8.4: {old:?}"
    );
}

#[test]
fn empty_source_returns_empty_data() {
    assert_eq!(full("", tcl(), &reg()).data, [] as [u32; 0]);
}

#[test]
fn semantic_token_lengths_use_utf16_code_units() {
    let data = full("# 😀x\n", tcl(), &reg()).data;
    assert_eq!(
        &data[..5],
        &[0, 0, 5, TokenKind::Comment as u32, 0],
        "comment token length must count the emoji as two UTF-16 code units",
    );
}

#[test]
fn many_comment_lines_do_not_drift_out_of_bounds() {
    // Hand-incrementing a byte cursor to the end of each comment line
    // while the `chars()` iterator advances one char drifts the cursor past
    // the buffer and slices out of bounds (panic) on files with several
    // comment lines.
    use std::fmt::Write as _;
    let mut src = String::new();
    for i in 0..40 {
        let _ = writeln!(src, "# comment line number {i} with some padding text");
    }
    src.push_str("set x 1\n");
    src.push_str("# trailing comment after code, no final newline");
    let st = full(&src, tcl(), &reg()); // must not panic
    let comments = st
        .data
        .chunks(5)
        .filter(|c| c[3] == TokenKind::Comment as u32)
        .count();
    assert_eq!(comments, 41, "expected one token per comment line");
}

#[test]
fn classify_command_head_picks_namespace_for_qualified() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let rows = decode_words("::string equal A a\nunknown arg", &reg());
    assert!(
        rows.iter()
            .any(|(_, _, _, kind, text)| *kind == TokenKind::Namespace as u32 && text == "::")
    );
    assert!(
        rows.iter()
            .any(|(_, _, _, kind, text)| *kind == TokenKind::Function as u32 && text == "unknown")
    );
}

#[test]
fn classify_command_head_follows_the_effective_identity() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source =
        "interp alias {} each {} foreach\neach v {} {}\nproc foreach args {}\nforeach v {} {}";
    let rows = decode_words(source, &reg());
    assert!(
        rows.iter()
            .any(|(_, _, _, kind, text)| *kind == TokenKind::Keyword as u32 && text == "each")
    );
    assert!(!rows.iter().any(|(line, _, _, kind, text)| *line == 3
        && *kind == TokenKind::Keyword as u32
        && text == "foreach"));
}

// range variant

#[test]
fn range_filters_tokens_outside_window() {
    // Three commands on three lines.  Range covers only
    // line 1 — the line-0 and line-2 tokens should drop.
    let src = "set a 1\nset b 2\nset c 3\n";
    let full_data = full(src, tcl(), &reg());
    let line1_only = range(
        src,
        tcl(),
        crate::definition::LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 10,
        },
        &reg(),
    );
    // Each tcl line emits at least one classified token.
    // The range result must be strictly smaller than the
    // full result.
    assert!(line1_only.data.len() < full_data.data.len());
    assert!(line1_only.data.len().is_multiple_of(5));
    assert!(!line1_only.data.is_empty(), "{:?}", line1_only.data);
}

#[test]
fn range_keeps_entire_document_when_range_covers_it() {
    let src = "proc foo {} { puts hi }\n";
    let full_data = full(src, tcl(), &reg());
    let wide = range(
        src,
        tcl(),
        crate::definition::LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 99,
            end_character: 0,
        },
        &reg(),
    );
    assert_eq!(wide.data, full_data.data);
}

#[test]
fn range_excludes_token_at_exact_end_position() {
    // LSP ranges are
    // half-open [start, end), so a token starting exactly
    // at `end` is OUTSIDE the range.
    let src = "set a 1\nset b 2\n";
    // Range whose end exactly coincides with line 1, col 0
    // (the `set` of the second command).  That token should
    // not appear in the range result.
    let r = range(
        src,
        tcl(),
        crate::definition::LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 1,
            end_character: 0,
        },
        &reg(),
    );
    // The full document has at least one line-1 token at col
    // 0 (the `set` of `set b 2`).  The half-open range must
    // exclude it; the range data must therefore be strictly
    // shorter than the full data.
    let full_data = full(src, tcl(), &reg());
    assert!(
        r.data.len() < full_data.data.len(),
        "range data {} should drop the line-1 token; full data {}",
        r.data.len(),
        full_data.data.len(),
    );
}

/// Every semantic-token type the server advertises in [`legend_token_types`]
/// must be handled by the VS Code extension — either a standard LSP type VS
/// Code themes natively, or an explicit `semanticTokenScopes` mapping. This
/// is the alignment guard: add a token type to the legend (e.g. from richer
/// lexing) without wiring the editor, and this test fails instead of the
/// token silently rendering as an unstyled default in every theme.
/// The BIG-IP config token types: emitted only by `bigip_conf_full` for
/// `tcl-bigip` documents (a `bigip.conf` is not Tcl), so the Tcl-family
/// blocks are not required to map them — `tcl-bigip` is.  `object` is
/// deliberately **not** in this set: it is shared, typing BIG-IP object
/// references inside iRules too, so the Tcl blocks must keep mapping it.
const BIGIP_ONLY: &[&str] = &[
    "partition",
    "pool",
    "monitor",
    "profile",
    "vlan",
    "bigipInterface",
    "ipAddress",
    "port",
    "routeDomain",
    "fqdn",
    "username",
    "encrypted",
];

/// Record a failure for every token in `required` that `mapped` (a
/// language's `semanticTokenScopes` block) does not handle.
fn require_mapped(
    lang: &str,
    required: &[&str],
    mapped: &std::collections::BTreeSet<&str>,
    failures: &mut Vec<String>,
) {
    for tok in required {
        if !mapped.contains(tok) {
            failures.push(format!(
                "language `{lang}` does not handle legend token `{tok}` \
                     (add it to contributes.semanticTokenScopes in \
                     editors/vscode/package.json)"
            ));
        }
    }
}

#[test]
fn vscode_semantic_token_scopes_cover_the_server_legend() {
    // Standard LSP `SemanticTokenTypes` VS Code styles out of the box, so
    // they need no custom `semanticTokenScopes` entry.
    const STANDARD_LSP_TYPES: &[&str] = &[
        "namespace",
        "type",
        "class",
        "enum",
        "interface",
        "struct",
        "typeParameter",
        "parameter",
        "variable",
        "property",
        "enumMember",
        "event",
        "function",
        "method",
        "macro",
        "keyword",
        "modifier",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "decorator",
    ];

    // The Tcl-family languages that exercise the *full* legend — iRules /
    // iApps / BIG-IP code references BIG-IP `object`s and fires `event`s, so
    // these blocks must map every non-standard token type. The narrower
    // dialects (`tcl8.4`, EDA, `expect`) emit a subset and the bespoke
    // `tcl-apl` uses its own token set, so they are not required to cover
    // the whole legend here. `tcl-irule` is a superset of plain Tcl, so
    // covering it covers every token a plain `.tcl` file can emit too.
    const FULL_VOCAB: &[&str] = &["tcl", "tcl-irule", "tcl-iapp", "tcl-bigip"];

    // The `apl*` types are emitted *only* by `apl_full`, for `tcl-apl`
    // documents (APL is not Tcl). They are therefore not required of the
    // Tcl-family blocks above — but `tcl-apl` must map every one of them,
    // which is checked separately below.
    let apl_types: Vec<&str> = legend_token_types()
        .into_iter()
        .filter(|t| t.starts_with("apl"))
        .collect();
    assert!(!apl_types.is_empty(), "legend lost its apl* token types");
    for tok in BIGIP_ONLY {
        assert!(
            legend_token_types().contains(tok),
            "legend lost the BIG-IP token type `{tok}`"
        );
    }

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let pkg = manifest.join("../../editors/vscode/package.json");
    let text =
        std::fs::read_to_string(&pkg).unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("package.json is valid JSON");

    let blocks = json["contributes"]["semanticTokenScopes"]
        .as_array()
        .expect("contributes.semanticTokenScopes is an array");

    let legend = legend_token_types();
    let mut failures = Vec::new();
    let mut checked_blocks = 0;

    let mut checked_apl = false;
    let mut checked_bigip = false;

    for block in blocks {
        let lang = block["language"].as_str().unwrap_or_default();
        let is_full_vocab = FULL_VOCAB.contains(&lang);
        if !is_full_vocab && lang != "tcl-apl" {
            continue;
        }
        let mapped: std::collections::BTreeSet<&str> = block["scopes"]
            .as_object()
            .map_or_default(|m| m.keys().map(String::as_str).collect());

        // `tcl-apl` owns the `apl*` types and nothing else needs them.
        if lang == "tcl-apl" {
            checked_apl = true;
            require_mapped(lang, &apl_types, &mapped, &mut failures);
            continue;
        }

        checked_blocks += 1;
        // `tcl-bigip` additionally owns the config-file types.
        if lang == "tcl-bigip" {
            checked_bigip = true;
            require_mapped(lang, BIGIP_ONLY, &mapped, &mut failures);
        }
        let shared: Vec<&str> = legend
            .iter()
            .copied()
            .filter(|t| {
                !STANDARD_LSP_TYPES.contains(t) && !apl_types.contains(t) && !BIGIP_ONLY.contains(t)
            })
            .collect();
        require_mapped(lang, &shared, &mapped, &mut failures);
    }

    assert!(
        checked_blocks > 0,
        "found no tcl* semanticTokenScopes blocks to check"
    );
    assert!(
        checked_apl,
        "found no `tcl-apl` semanticTokenScopes block to check the apl* types against"
    );
    assert!(
        checked_bigip,
        "found no `tcl-bigip` semanticTokenScopes block to check the BIG-IP types against"
    );
    assert!(
        failures.is_empty(),
        "semantic-token legend not fully handled:\n  {}",
        failures.join("\n  "),
    );
}

/// The *narrow* dialects — the versioned Tcls, the EDA tools, Expect — must
/// map the **shared** custom vocabulary too.
///
/// A `.exp` / EDA / `tcl8.4` document reaches exactly the same
/// regex / format / clock / binary / escape sub-tokenisers a plain `.tcl`
/// file does, so a type missing from *their* scope block renders unstyled
/// there while looking perfectly fine in `.tcl` — the failure mode this
/// whole test family exists to prevent, just one dialect over.
#[test]
fn vscode_semantic_token_scopes_cover_the_narrow_dialects() {
    const FULL_VOCAB: &[&str] = &["tcl", "tcl-irule", "tcl-iapp", "tcl-bigip"];
    const STANDARD_LSP_TYPES: &[&str] = &[
        "namespace",
        "type",
        "class",
        "enum",
        "interface",
        "struct",
        "typeParameter",
        "parameter",
        "variable",
        "property",
        "enumMember",
        "event",
        "function",
        "method",
        "macro",
        "keyword",
        "modifier",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "decorator",
    ];

    // `object` names a BIG-IP object, which only iRules / iApps / BIG-IP
    // config reach — a plain Tcl or EDA document never emits it.
    let shared_custom: Vec<&str> = legend_token_types()
        .into_iter()
        .filter(|t| {
            !STANDARD_LSP_TYPES.contains(t)
                && !t.starts_with("apl")
                && !BIGIP_ONLY.contains(t)
                && *t != "object"
        })
        .collect();
    assert!(
        !shared_custom.is_empty(),
        "legend lost its shared vocabulary"
    );

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let pkg = manifest.join("../../editors/vscode/package.json");
    let text =
        std::fs::read_to_string(&pkg).unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("package.json is valid JSON");
    let blocks = json["contributes"]["semanticTokenScopes"]
        .as_array()
        .expect("contributes.semanticTokenScopes is an array");

    let mut failures = Vec::new();
    let mut checked = 0;
    for block in blocks {
        let lang = block["language"].as_str().unwrap_or_default();
        if !lang.starts_with("tcl") || FULL_VOCAB.contains(&lang) || lang == "tcl-apl" {
            continue;
        }
        checked += 1;
        let mapped: std::collections::BTreeSet<&str> = block["scopes"]
            .as_object()
            .map_or_default(|m| m.keys().map(String::as_str).collect());
        require_mapped(lang, &shared_custom, &mapped, &mut failures);
    }
    assert!(
        checked > 0,
        "found no narrow tcl* semanticTokenScopes blocks (tcl8.4 / EDA / expect) to check"
    );
    assert!(
        failures.is_empty(),
        "narrow-dialect scope gaps:\n  {}",
        failures.join("\n  ")
    );

    assert!(
        failures.is_empty(),
        "semantic-token legend not fully handled:\n  {}",
        failures.join("\n  ")
    );
}

// Semantic tokens read command grammar from the registry,
// so the explicitly global spellings C Tcl resolves to the same commands
// (`namespace which -command ::format` → `::format`) are classified
// identically to their bare forms, and a same-named user proc is not.

/// The decoded token *kinds* of `src`, positions discarded.
fn kinds_only(src: &str, registry: &CommandRegistry) -> Vec<u32> {
    decode_full(src, tcl(), registry)
        .into_iter()
        .map(|(_, _, _, kind, _)| kind)
        .collect()
}

/// Assert that qualifying a command head with a leading `::` changes
/// nothing but the head itself: the qualified stream must *end with* the
/// bare stream's kinds (the qualified head contributes one extra
/// namespace-separator token at the front), so every argument is
/// classified identically.
fn assert_qualified_matches_bare(
    bare: &str,
    head: &str,
    qualified: &str,
    registry: &CommandRegistry,
) {
    let bare_kinds = kinds_only(bare, registry);
    let qualified_kinds = kinds_only(&bare.replacen(head, qualified, 1), registry);
    assert!(
        qualified_kinds.len() >= bare_kinds.len(),
        "{qualified} produced fewer tokens than {head}: \
             {qualified_kinds:?} vs {bare_kinds:?}"
    );
    let tail = &qualified_kinds[qualified_kinds.len() - bare_kinds.len()..];
    assert_eq!(
        tail,
        &bare_kinds[..],
        "{qualified} classified its arguments differently from {head}"
    );
}

/// The loop-variable-list scan is registry-driven,
/// so a `::`-qualified head classifies exactly like the bare one.
///
/// tclsh-proof (9.0.4): `::foreach {a b} {1 2} { puts $a }` and
/// `::dict for {k v} {a 1} { puts $v }` run identically to their bare forms —
/// `namespace which -command ::foreach` is `::foreach`.
#[test]
fn qualified_loop_headers_classify_like_the_bare_form() {
    let r = reg();
    for (bare, head, qualified) in [
        ("foreach {a b} {1 2} { puts $a }\n", "foreach", "::foreach"),
        ("lmap x {1 2} { expr {$x} }\n", "lmap", "::lmap"),
        ("dict for {k v} {a 1} { puts $v }\n", "dict", "::dict"),
        ("dict map {k v} {a 1} { set v }\n", "dict", "::dict"),
    ] {
        assert_qualified_matches_bare(bare, head, qualified, &r);
    }
}

/// The loop-variable binding pass reads the registry's own
/// `ArgRole::LoopVarList` indices, so every loop header it understands is
/// one the registry declares — not a `foreach` / `lmap` / `dict for`
/// spelling list in the walker.
#[test]
fn loop_var_list_roles_come_from_the_registry() {
    let r = reg();
    let roles = |cmd: &str, args: &[&str]| {
        r.arg_indices_for_role(cmd, args, tcl_registry::ArgRole::LoopVarList)
    };
    // `foreach VARS LIST ?VARS LIST …? BODY`: the repeated (start 0,
    // stride 2) layout marks every variable-list word.
    assert_eq!(roles("foreach", &["{a b}", "$l", "{}"]), vec![0]);
    assert_eq!(
        roles("foreach", &["{a}", "$l", "{b}", "$m", "{}"]),
        vec![0, 2]
    );
    assert_eq!(roles("lmap", &["x", "$l", "{}"]), vec![0]);
    // `dict for {k v} $d body` marks its pair at index 1 (after the
    // subcommand word), and the collection at index 2 is declared a Dict —
    // which is what tells the binder this is a key/value pair rather than
    // a free variable list.
    assert_eq!(roles("dict", &["for", "{k v}", "$d", "{}"]), vec![1]);
    assert_eq!(roles("dict", &["map", "{k v}", "$d", "{}"]), vec![1]);
    assert_eq!(
        r.arg_type_hint("dict", &["for", "{k v}", "$d", "{}"], 2)
            .and_then(|h| h.expected),
        Some(tcl_registry::types::TclType::Dict)
    );
    // A `::`-qualified head resolves to the same spec, so it reports the
    // same roles.
    assert_eq!(roles("::foreach", &["{a b}", "$l", "{}"]), vec![0]);
    assert_eq!(roles("::dict", &["for", "{k v}", "$d", "{}"]), vec![1]);
    // A command with no loop-variable list reports none.
    assert_eq!(roles("puts", &["hi"]), [] as [usize; 0]);
}

#[test]
fn loop_var_binding_uses_tcl_list_grammar_and_abstains_when_dynamic() {
    use tcl_compiler::segmenter::segment_commands;

    let names = |src: &str| {
        let command = segment_commands(src)
            .into_iter()
            .next()
            .expect("one command");
        static_loop_var_names(
            src,
            &command,
            1,
            tcl_syntax::word_rules::WordValueRules::default(),
        )
    };

    assert_eq!(
        names("foreach {a b} $coll {}\n"),
        Some(vec![String::from("a"), String::from("b")])
    );
    assert_eq!(
        names("foreach \"a b\" $coll {}\n"),
        Some(vec![String::from("a"), String::from("b")])
    );
    assert_eq!(
        names("foreach a\\ b $coll {}\n"),
        Some(vec![String::from("a b")])
    );
    assert_eq!(names("foreach {} $coll {}\n"), Some(Vec::new()));
    assert_eq!(names("foreach $vars $coll {}\n"), None);
    assert_eq!(names("foreach [makeVars] $coll {}\n"), None);
    assert_eq!(names("foreach {a b $coll {}\n"), None);
}

#[test]
fn qualified_format_family_classifies_like_the_bare_form() {
    let r = reg();
    for (bare, head, qualified) in [
        ("format {%08x} 42\n", "format", "::format"),
        ("scan $s {%d %d} a b\n", "scan", "::scan"),
        ("binary format c3 {1 2 3}\n", "binary", "::binary"),
        ("binary scan $v c3 out\n", "binary", "::binary"),
        ("clock format $t -format {%Y-%m-%d}\n", "clock", "::clock"),
        ("clock scan $s -format {%Y}\n", "clock", "::clock"),
        ("regsub -all e $s {[&]} out\n", "regsub", "::regsub"),
    ] {
        assert_qualified_matches_bare(bare, head, qualified, &r);
    }
}

#[test]
fn qualified_grammar_commands_classify_like_the_bare_form() {
    let r = reg();
    for (bare, head, qualified) in [
        ("foreach {a b} {1 2} { puts $a }\n", "foreach", "::foreach"),
        ("lmap x $l { expr {$x} }\n", "lmap", "::lmap"),
        ("upvar 1 src local\n", "upvar", "::upvar"),
        ("upvar src local\n", "upvar", "::upvar"),
        ("global aa bb cc\n", "global", "::global"),
        ("variable x 1 y 2\n", "variable", "::variable"),
        (
            "oo::define C method m {args} { return $args }\n",
            "oo::define",
            "::oo::define",
        ),
        (
            "oo::objdefine $o method m {} { return }\n",
            "oo::objdefine",
            "::oo::objdefine",
        ),
        (
            "namespace upvar ::ns o1 l1 o2 l2\n",
            "namespace",
            "::namespace",
        ),
        ("dict update d k1 v1 k2 v2 { puts $v1 }\n", "dict", "::dict"),
    ] {
        assert_qualified_matches_bare(bare, head, qualified, &r);
    }
}

/// FP guard — a user proc in another namespace that happens to share a
/// built-in's tail name does not inherit its grammar, and data words that
/// merely look like format strings are not painted as ones.
#[test]
fn same_named_user_command_does_not_inherit_grammar() {
    let r = reg();
    // `ns::format` is a different command; its argument stays a plain
    // string, so no format-specifier sub-tokens appear.
    let user = decode_full("ns::format {%08x} 42\n", tcl(), &r);
    let builtin = decode_full("format {%08x} 42\n", tcl(), &r);
    assert!(
        builtin.len() > user.len(),
        "the built-in must produce specifier sub-tokens the user proc does not:\n\
             builtin={builtin:?}\nuser={user:?}"
    );
    // A data word that spells a format string is untouched.
    assert_eq!(
        kinds_only("puts {%08x}\n", &r),
        kinds_only("puts {plain}\n", &r)
    );
}

// A head's *effective command identity* (a static
// `interp alias`, a `rename`, a shadowing top-level `proc`) drives the
// grammar, so calling a built-in through a proven alias classifies exactly
// like calling it directly, and calling a name whose binding was taken
// over does not.

/// TP — `interp alias {} myfmt {} format` makes `myfmt %08x 42` classify
/// exactly like `format %08x 42`.
///
/// tclsh-proof (9.0.4 and 8.6.16, byte-identical): `interp alias {} myfmt
/// {} format; myfmt %08x 42` → `0000002a`.
#[test]
fn a_static_interp_alias_inherits_the_targets_grammar() {
    let r = reg();
    let bind = "interp alias {} myfmt {} format\n";
    // The argument kinds of a direct `format` call — its head token is the
    // one thing an aliased or qualified spelling legitimately differs in.
    let direct = kinds_only("format {%08x} 42\n", &r);
    let direct_args = &direct[1..];
    for head in ["myfmt", "::myfmt"] {
        let call = kinds_only(&format!("{bind}{head} {{%08x}} 42\n"), &r);
        assert_eq!(
            &call[call.len() - direct_args.len()..],
            direct_args,
            "`{head}` must classify its arguments like a direct `format` call"
        );
    }
    // Without the alias the same call is an ordinary unknown command whose
    // argument stays a plain string.
    let unaliased = kinds_only("myfmt {%08x} 42\n", &r);
    assert!(
        unaliased.len() < direct.len(),
        "an unbound name must not get format sub-tokens: {unaliased:?}"
    );
}

/// TP — `rename foreach myforeach` moves the loop grammar (and the
/// keyword classification) onto the new name.
#[test]
fn a_static_rename_moves_the_grammar_to_the_new_name() {
    let r = reg();
    let bind = "rename upvar myupvar\n";
    let direct = kinds_only("upvar 1 src local\n", &r);
    let renamed = kinds_only(&format!("{bind}myupvar 1 src local\n"), &r);
    let baseline = kinds_only(bind, &r);
    assert_eq!(
        &renamed[baseline.len()..],
        &direct[..],
        "a renamed call must classify like the original"
    );
}

/// FP — a name whose binding was taken over gets **no** registry grammar:
/// after `rename format origfmt` the bare `format` is gone, and a top-level
/// `proc format` shadows the built-in outright.
///
/// tclsh-proof (9.0.4 and 8.6.16): `rename format origfmt; proc format
/// {args} {return USER}; format x` → `USER`; `origfmt %d 7` → `7`.
#[test]
fn a_rebound_builtin_loses_its_grammar() {
    let r = reg();
    let direct = kinds_only("format {%08x} 42\n", &r);
    for bind in [
        "rename format origfmt\n",
        "rename format {}\n",
        "proc format {args} { return USER }\n",
        "interp alias {} format {} myformatter\n",
    ] {
        let baseline = kinds_only(bind, &r);
        let after = kinds_only(&format!("{bind}format {{%08x}} 42\n"), &r);
        assert!(
            after.len() - baseline.len() < direct.len(),
            "`{}` must strip format's specifier sub-tokens, got {after:?}",
            bind.trim()
        );
    }
}

/// FN guard — a binding only applies from its own statement onwards, so a
/// call *before* the `rename` still classifies as the built-in.
#[test]
fn a_binding_does_not_retroactively_retag_earlier_calls() {
    let r = reg();
    let direct = kinds_only("format {%08x} 42\n", &r);
    let before = kinds_only("format {%08x} 42\nrename format origfmt\n", &r);
    let rename_only = kinds_only("rename format origfmt\n", &r);
    assert_eq!(
        &before[..direct.len()],
        &direct[..],
        "the call before the rename must keep the built-in's grammar"
    );
    assert_eq!(&before[direct.len()..], &rename_only[..]);
}

/// TN — a binding the analyser cannot prove states nothing, so the head
/// keeps its literal identity rather than gaining a wrong one.
#[test]
fn unprovable_bindings_abstain() {
    let r = reg();
    let plain = kinds_only("myfmt {%08x} 42\n", &r);
    for bind in [
        // Dynamic alias target / name, dynamic rename source.
        "interp alias {} myfmt {} $target\n",
        "interp alias {} $n {} format\n",
        "rename $old myfmt\n",
        // Pre-bound arguments shift every index.
        "interp alias {} myfmt {} format %08x\n",
        // A child interpreter's command table is not this one's.
        "interp alias slave myfmt {} format\n",
        // Not an unconditional top-level statement.
        "if {$x} { interp alias {} myfmt {} format }\n",
    ] {
        let baseline = kinds_only(bind, &r);
        let after = kinds_only(&format!("{bind}myfmt {{%08x}} 42\n"), &r);
        assert_eq!(
            &after[baseline.len()..],
            &plain[..],
            "`{}` must not give `myfmt` a grammar",
            bind.trim()
        );
    }
}

/// FP — a `proc` in another namespace, or one that shadows nothing, states
/// no fact: only a global-namespace redefinition of a built-in takes a name
/// over (tclsh 9.0.4: inside `namespace eval ::n`, `proc format` defines
/// `::n::format` and the global `format` is untouched).
#[test]
fn a_namespaced_proc_does_not_shadow_the_global_builtin() {
    let r = reg();
    let direct = kinds_only("format {%08x} 42\n", &r);
    let bind = "namespace eval ::n { proc format {a} { return 1 } }\n";
    let baseline = kinds_only(bind, &r);
    let after = kinds_only(&format!("{bind}format {{%08x}} 42\n"), &r);
    assert_eq!(&after[baseline.len()..], &direct[..]);
}

/// TP — `apply`'s `ArgRole::LambdaLiteral` reaches the renamed and aliased
/// spellings too, closing the failure mode written up in the
/// apply-lambda-body KCS note: without them a `[list …]`-quoted or
/// directly-called lambda under `rename apply myapply` collapses into one
/// opaque `string` token.
///
/// tclsh-proof (9.0.4 / 8.6.16): `rename apply myapply; myapply {x {puts
/// $x}} 5` prints `5`, exactly as the literal call does.
#[test]
fn apply_lambda_literals_survive_a_rename_or_alias() {
    let r = reg();
    let direct = kinds_only("apply {x {puts $x}} 5\n", &r);
    for bind in [
        "rename apply myapply\n",
        "interp alias {} myapply {} apply\n",
    ] {
        let baseline = kinds_only(bind, &r);
        let bound = kinds_only(&format!("{bind}myapply {{x {{puts $x}}}} 5\n"), &r);
        assert_eq!(
            &bound[baseline.len()..],
            &direct[..],
            "`{}` must keep apply's lambda-literal split",
            bind.trim()
        );
    }
}

/// TN — a `{*}`-expanded (dynamic) head has no resolvable identity, so no
/// grammar is applied to its arguments.
#[test]
fn dynamic_head_gets_no_format_grammar() {
    let r = reg();
    let dynamic = decode_full("{*}$cmd {%08x} 42\n", tcl(), &r);
    let builtin = decode_full("format {%08x} 42\n", tcl(), &r);
    assert!(
        dynamic.len() < builtin.len(),
        "a dynamic head must not get format sub-tokens: {dynamic:?}"
    );
}

/// `set`, `lassign`, `incr`, `lappend`, `append`, `expr` (every plain
/// builtin — `function` + `defaultLibrary`) render as unstyled plain text
/// for users whose theme has no rule for the custom `support.function.tcl`
/// scope. A `semanticTokenScopes` override mapping
/// `function.defaultLibrary` to that scope **replaces** (not supplements)
/// VS Code's built-in cross-theme default for the standard
/// `function`/`defaultLibrary` combo — so themes lacking that exact scope
/// lose highlighting entirely instead of falling back to the built-in
/// default the way every other standard type does. `operator`, `decorator`
/// and `namespace` carry the same risk for the same reason (and
/// `operator`'s scope, `keyword.operator.format.tcl`, is outright wrong for
/// the general case — it covers every `expr` operator and the `regsub` `\&`
/// backref, not just `format`). Standard LSP types get no
/// override unless the override is either essentially universal across
/// themes (`number`, `regexp` — near-ubiquitous `TextMate` scopes that
/// match the grammar's own naming) or the type has no sane built-in
/// default at all (custom types like `object`, `event`, `escape`, the
/// `regexp*`/`format*`/`clock*`/`binary*` families).
#[test]
fn vscode_semantic_token_scopes_do_not_shadow_standard_defaults() {
    const MUST_NOT_OVERRIDE: &[&str] = &[
        "function.defaultLibrary",
        "operator",
        "decorator",
        "namespace",
    ];

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let pkg = manifest.join("../../editors/vscode/package.json");
    let text =
        std::fs::read_to_string(&pkg).unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("package.json is valid JSON");

    let blocks = json["contributes"]["semanticTokenScopes"]
        .as_array()
        .expect("contributes.semanticTokenScopes is an array");

    let mut failures = Vec::new();
    for block in blocks {
        let lang = block["language"].as_str().unwrap_or_default();
        let Some(scopes) = block["scopes"].as_object() else {
            continue;
        };
        for &key in MUST_NOT_OVERRIDE {
            if scopes.contains_key(key) {
                failures.push(format!(
                    "language `{lang}` overrides `{key}`, shadowing VS Code's \
                         built-in cross-theme default (issue #862) — remove it from \
                         contributes.semanticTokenScopes in editors/vscode/package.json"
                ));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n  "));
}

/// `insert_format_overrides` marks a format/clock/binary-role
/// argument by its registry-declared *position*, independent of
/// whether that word is literal — so a computed word (`$fmt`) reaches
/// the sub-tokeniser exactly as a literal one does, and each
/// sub-tokeniser already "falls back to the default classification"
/// (its own doc comment, `ArgOverride`) when it finds no specifier in
/// the token's own bytes. `$fmt`'s own text is `"$fmt"`, never a `%`
/// specifier, so it renders as a plain `variable` token — a computed
/// pattern is explained at its use through hover and inlay hints
/// (`hover.rs`/`inlay_hints.rs`), never painted here at a token
/// range it does not have.
#[test]
fn a_computed_format_word_falls_back_to_its_plain_classification() {
    let names = legend_token_types();
    let kind_of = |src: &str, needle: &str| {
        decode_words(src, &reg())
            .into_iter()
            .find(|(_, _, _, _, word)| word == needle)
            .map(|(_, _, _, kind, _)| names[kind as usize])
    };
    assert_eq!(
        kind_of("set fmt \"%-20s %d\"\nformat $fmt a 1\n", "$fmt"),
        Some("variable"),
        "a computed format argument is an ordinary variable token, not formatSpec"
    );
    assert_eq!(
        kind_of(
            "set fmt \"%Y-%m-%d\"\nclock format 0 -format $fmt\n",
            "$fmt"
        ),
        Some("variable")
    );
    assert_eq!(
        kind_of("set fmt \"a3 i\"\nbinary format $fmt foo 1\n", "$fmt"),
        Some("variable")
    );
}
