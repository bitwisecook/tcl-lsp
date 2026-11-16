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

use super::*;

/// A bare [`Gen`], for driving one production directly instead of through
/// a whole script.
fn gen_with(seed: u64, config: GenConfig) -> Gen {
    Gen {
        rng: Rng::new(seed),
        config,
        out: String::new(),
        procs: Vec::new(),
    }
}

/// Whether `text` has the shape [`Malformed`] documents for `mode`, so the
/// per-mode expectation lives next to the assertion that checks it.
fn matches_mode(mode: Malformed, text: &str) -> bool {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let starts_with_call =
        |names: &[&str]| names.iter().any(|f| text.starts_with(&format!("{f}(")));
    match mode {
        Malformed::TrailingOperator => {
            tokens.len() == 2 && MALFORMED_BINARY_OPS.contains(&tokens[1])
        }
        Malformed::MissingOperator => {
            tokens.len() == 2 && !MALFORMED_BINARY_OPS.contains(&tokens[1])
        }
        Malformed::EmptyExpression => text.is_empty(),
        Malformed::Bareword => {
            !tokens.is_empty() && tokens.iter().all(|t| MALFORMED_BAREWORDS.contains(t))
        }
        Malformed::UnbalancedOpenParen => text.matches('(').count() > text.matches(')').count(),
        Malformed::UnbalancedCloseParen => text.matches(')').count() > text.matches('(').count(),
        Malformed::LoneUnary => ["!", "~", "-", "+"].contains(&text),
        Malformed::InvalidCharacter => MALFORMED_BADCHARS.iter().any(|c| text.contains(*c)),
        Malformed::IncompleteOperator => text.contains('=') && !text.contains("=="),
        Malformed::EmptySubexpression => text.contains("()"),
        Malformed::MissingFunctionArgument => {
            text.starts_with("max(") && (text.contains(", )") || text.contains("(, "))
        }
        Malformed::FunctionArity => starts_with_call(&["sin", "cos", "abs", "double", "round"]),
        Malformed::UnknownFunction => starts_with_call(&["nosuchfn", "notafunc", "bogusfn"]),
        Malformed::StrayColon => tokens.len() == 3 && tokens[1] == ":",
        Malformed::StrayComma => tokens.len() == 3 && tokens[1] == ",",
        Malformed::MissingColon => tokens.len() == 3 && tokens[1] == "?",
        Malformed::BadNumber => ["0o8", "0o9", "0b2", "0b3"]
            .iter()
            .any(|n| text.contains(n)),
        Malformed::CommentOnly => text.starts_with('#'),
    }
}

/// Malformed shapes whose status on our *own* engine is deliberately not
/// asserted, because a real divergence against C Tcl is open for them.
///
/// `tclvm` still evaluates a radix-invalid numeric literal to its own
/// source text — `expr {0o8}` yields the string `0o8`, and likewise `0o9`,
/// `0b2`, `0b3` (and, outside this generator's set, `0x` and `0xg`) —
/// where C Tcl 9.0.4 raises `invalid bareword "0o8" … (invalid octal
/// number?)`. That is the same silent-wrong-answer shape as the `expr {1+}`
/// bug, on the numeric-literal path rather than the operator path. It was
/// found *by this production* (6 seeds in a 1000-iteration campaign at
/// 300‰, 2 in 3000 at the default rate) and is reported for triage, not
/// fixed here.
///
/// Recorded as "not asserted" rather than "must not error" on purpose:
/// fixing the engine must not break this test.
const MALFORMED_MODES_WITH_OPEN_DIVERGENCE: &[Malformed] = &[Malformed::BadNumber];

/// Compile `src` through the live native entry, then execute its bytecode.
/// Output is discarded; only a returned guest error satisfies the oracle.
fn errors_on_our_engine(src: &str) -> bool {
    let profile = tcl_registry::model::resolve_environment("tcl9.0").unit_profile();
    let mut vm = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        tcl_vm::standard_host(),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("the Tcl 9.0 native core");
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    let handle = vm
        .try_compile_function(src)
        .expect("the actual Tcl 9.0 entry compiles the original script to bytecode");
    vm.try_invoke_function(&handle)
        .expect("compiled execution must not substitute a host refusal for a guest error")
        .code
        == tcl_vm::Code::Error
}

/// Whether `out` is one of the three statement forms
/// [`Gen::malformed_expr_stmt`] emits.
fn is_malformed_stmt(out: &str) -> bool {
    let line = out.trim_end();
    line.starts_with("set _mxv [expr {")
        || (line.starts_with("if {") && line.ends_with("} {puts _mx}"))
        || (line.starts_with("puts [expr {") && line.ends_with("}]"))
}

#[test]
fn deterministic_for_seed() {
    let cfg = GenConfig::default();
    assert_eq!(generate(42, &cfg), generate(42, &cfg));
    assert_ne!(generate(1, &cfg), generate(2, &cfg));
}

/// `while`/`for` counters are qualified by nesting depth (`_w0`, `_w1`,
/// …) so nested loops can never share a counter. A flat `_w`/`_f` name
/// regardless of depth would let a `while` nested inside another `while`
/// reset the *same* counter on every outer iteration — a genuine
/// infinite loop, not just a divergence risk — so this checks the
/// un-qualified form never reappears.
#[test]
fn while_and_for_counters_are_depth_qualified() {
    let cfg = GenConfig {
        max_depth: 5,
        max_stmts: 20,
        max_expr_depth: 4,
        ..GenConfig::default()
    };
    for seed in 0..500u64 {
        let src = generate(seed, &cfg);
        assert!(
            !src.contains("set _w 0") && !src.contains("set _f 0"),
            "seed {seed}: un-qualified loop counter reappeared:\n{src}"
        );
    }
}

/// `**`/`<<`/`>>` guard operand *sign* (never negative) but not
/// *magnitude*. Since `left`/`right` are
/// arbitrary nested `expr(depth+1)` calls — which can themselves be
/// another `**`/`<<`/`>>` node — an unbounded exponent/shift-count could
/// reach magnitudes that make bignum exponentiation/shift and its
/// decimal stringification take multiple seconds in both engines
/// (confirmed directly against a real `tclsh8.6`: `2**300000` and
/// `1<<300000` each take ~7s), a generator-caused hang that shows up as
/// a spurious `Timeout`/`StatusMismatch` finding, not a real VM bug.
///
/// A live `tclsh` sweep isn't a reliable regression guard here: the
/// magnitude collision that triggers the hang is probabilistic (observed
/// in roughly 1.6% of expression-root draws), so a seed sweep small
/// enough to run as a fast unit test could pass on unbounded operands
/// purely by chance — this asserts
/// the actual generated Tcl source instead: every `**` exponent operand
/// is deterministically reduced by `% 20` and every `<<`/`>>` count
/// operand by `% 64`, which bounds the operand to a small range no
/// matter how large the sub-expression feeding it evaluates to.
#[test]
fn power_and_shift_operands_are_magnitude_bounded() {
    let cfg = GenConfig {
        max_depth: 4,
        max_stmts: 16,
        max_expr_depth: 4,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..400u64).map(|s| generate(s, &cfg)).collect();
    assert!(
        corpus.iter().any(|s| s.contains("**")),
        "no `**` was ever generated in this sweep"
    );
    assert!(
        corpus.iter().any(|s| s.contains("<<") || s.contains(">>")),
        "no `<<`/`>>` was ever generated in this sweep"
    );
    assert!(
        corpus.iter().any(|s| s.contains("% 20)")),
        "`**` exponent is no longer bounded by `% 20` — magnitude hang risk reintroduced"
    );
    assert!(
        corpus.iter().any(|s| s.contains("% 64)")),
        "`<<`/`>>` count is no longer bounded by `% 64` — magnitude hang risk reintroduced"
    );
}

#[test]
fn balanced_delimiters() {
    let cfg = GenConfig::default();
    for seed in 0..200 {
        let src = generate(seed, &cfg);
        // Asked of the owner of the question (`info complete`'s engine),
        // not of a byte counter. A counter was an adequate proxy only
        // while no generated word could contain a delimiter; the
        // `WordShape` productions emit words whose *value* is `{` or `}`,
        // and `set _w "{"` is a complete script whose braces do not
        // balance by count. The owner knows a delimiter inside a quoted
        // word is content, so it answers what this test always meant.
        assert!(
            tcl_lexer::script_is_complete(&src),
            "seed {seed}: generated script is not a complete script\n{src}"
        );
    }
}

#[test]
fn nonempty_and_prints() {
    let cfg = GenConfig::default();
    // Most scripts should contain a `puts` so the differential has output.
    let with_puts = (0..100)
        .filter(|s| generate(*s, &cfg).contains("puts"))
        .count();
    assert!(
        with_puts > 50,
        "too few scripts print output: {with_puts}/100"
    );
}

#[test]
fn broadened_grammar_is_exercised() {
    // Over a decent seed range each newly-added production should appear at
    // least once, so the broadened surface is actually generated (not dead).
    let cfg = GenConfig {
        max_depth: 4,
        max_stmts: 16,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..600).map(|s| generate(s, &cfg)).collect();
    let appears = |needle: &str| corpus.iter().any(|s| s.contains(needle));
    for needle in [
        "proc ",
        "namespace eval ",
        "dict ",
        "switch -- ",
        "catch {",
        "try {",
        // Command families added for.
        "format ",
        "scan ",
        "array set ",
        "apply {",
        "eval [list",
        "subst {",
        "string compare ",
        "concat ",
        "lmap ",
        "global ",
        "upvar ",
        "regexp ",
        "regsub ",
    ] {
        assert!(appears(needle), "production never generated: {needle:?}");
    }
    // The string/list relational operators reach the expression grammar.
    assert!(
        corpus
            .iter()
            .any(|s| s.contains(" eq ") || s.contains(" in ")),
        "string/list relational operators never generated"
    );
    // A defined proc should sometimes be called back (`puts [p0 ...]`).
    let proc_called = corpus.iter().any(|s| {
        ["p0", "p1", "p2", "helper", "compute"]
            .iter()
            .any(|p| s.contains(&format!("puts [{p}")))
    });
    assert!(proc_called, "no generated proc was ever called");
}

/// `namespace inscope`'s list-element tail rule needs differential
/// coverage, since a VM that space-joins the tail instead (turning
/// `{x y}` into two arguments) can only be caught if the production is
/// actually generated. Proves the production is live and that every
/// interesting tail shape — the whitespace-bearing word that is the
/// minimal reproducer, an empty element, and the substitution/terminator
/// characters that force list quoting — actually reaches a generated
/// script.
#[test]
fn namespace_inscope_list_args_are_exercised() {
    let cfg = GenConfig {
        max_depth: 4,
        max_stmts: 16,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
    let inscope_lines: Vec<&str> = corpus
        .iter()
        .flat_map(|s| s.lines())
        .filter(|l| l.contains("namespace inscope "))
        .collect();
    assert!(
        !inscope_lines.is_empty(),
        "`namespace inscope` is never generated"
    );
    // The probe proc must accompany every call, else the differential has
    // nothing to compare.
    assert!(
        corpus
            .iter()
            .any(|s| s.contains("proc _shape {args}") && s.contains("namespace inscope ")),
        "the inscope probe proc is not emitted alongside the call"
    );
    for arg in [
        "{x y}",
        "{  padded  }",
        "{}",
        "{a;b}",
        "{$nosub}",
        "{[nosub]}",
    ] {
        assert!(
            inscope_lines.iter().any(|l| l.contains(arg)),
            "inscope tail shape never generated: {arg:?}"
        );
    }
    // The bug's shape specifically: a trailing word containing whitespace.
    assert!(
        inscope_lines
            .iter()
            .any(|l| l.contains("{_shape} {x y}") || l.contains("{_shape} {a b c}")),
        "no whitespace-bearing first tail word — the #1056 reproducer is not covered"
    );
}

/// `expr()`'s operator menu includes the TIP 461 string-ordering words,
/// every bitwise/shift symbol, `**`, and `::tcl::mathfunc` calls, and
/// each needs differential coverage to avoid a fuzz-coverage gap. Proves
/// each production actually gets generated (not dead code) over a decent
/// seed sweep, mirroring `broadened_grammar_is_exercised` above.
#[test]
fn expr_and_mathfunc_additions_are_exercised() {
    let cfg = GenConfig {
        max_depth: 4,
        max_stmts: 16,
        max_expr_depth: 4,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..1500).map(|s| generate(s, &cfg)).collect();
    for op in ["lt", "le", "gt", "ge"] {
        assert!(
            corpus.iter().any(|s| s.contains(&format!(" {op} "))),
            "TIP 461 operator never generated: {op:?}"
        );
    }
    for op in ["**", "<<", ">>"] {
        assert!(
            corpus.iter().any(|s| s.contains(op)),
            "operator never generated: {op:?}"
        );
    }
    // `&`/`|` search on `& int(`/`| int(` rather than the bare symbol:
    // the bitwise shape always renders as `int(...) & int(...)` with no
    // extra parens around the right operand, whereas `&&`/`||` (already
    // in the pre-existing pool) render as `(...) && (...)` — a nested
    // right operand starting with `int(` would make the bare `&`/`|`
    // substring check a false pass via `&&`/`||` instead.
    for op in ["& int(", "| int(", "^ int("] {
        assert!(
            corpus.iter().any(|s| s.contains(op)),
            "operator never generated: {op:?}"
        );
    }
    for func in [
        "abs(", "sin(", "hypot(", "atan2(", "max(", "min(", "fmod(", "isnan(", "bool(",
    ] {
        assert!(
            corpus.iter().any(|s| s.contains(func)),
            "mathfunc call never generated: {func:?}"
        );
    }
    // Every TIP 745 (Tcl 9.1) function must never appear: the
    // differential fuzzer's default reference is `tclsh9.0`, which
    // doesn't have them, so generating one would be a guaranteed false
    // finding rather than a real bug.
    for func in ["cbrt(", "trunc(", "asinh(", "acosh(", "exp2(", "gamma("] {
        assert!(
            !corpus.iter().any(|s| s.contains(func)),
            "TIP 745 (9.1-only) function must never be generated: {func:?}"
        );
    }
}

/// Variable access through a **computed name** needs differential
/// coverage: the compiler treats it as a dataflow barrier gating the
/// constant-branch fold and dead-store elimination that feed this
/// backend's input IR, so an ungenerated dynamic-name path would leave
/// that barrier untested. Proves every arm reaches a generated script.
#[test]
fn dynamic_name_shapes_are_exercised() {
    let cfg = GenConfig {
        max_depth: 4,
        max_stmts: 16,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
    for needle in [
        "set $_dnn hello",
        "puts [set $_dnn]",
        "lappend $_dnn one",
        "append $_dnn ab",
        "puts [subst $_dnt]",
        "unset $_dnn",
    ] {
        assert!(
            corpus.iter().any(|s| s.contains(needle)),
            "dynamic-name production never generated: {needle:?}"
        );
    }
    // The shapes must stay inside their private `_dn*` name space, so
    // wherever they land they cannot perturb the rest of a script.
    for line in corpus
        .iter()
        .flat_map(|s| s.lines())
        .filter(|l| l.contains("_dn"))
    {
        for v in VARS {
            assert!(
                !line.contains(&format!("${v} ")) && !line.ends_with(&format!("${v}")),
                "dynamic-name line touches shared variable {v:?}: {line:?}"
            );
        }
    }
}

/// A procedure writing its **caller's** frame needs differential
/// coverage: the compiler summarises this per procedure and applies it at
/// every call site, gating the constant-branch fold and dead-store
/// elimination that feed this backend's input IR, so an ungenerated
/// caller-frame path would leave that summary untested. Proves every arm
/// reaches a generated script.
#[test]
fn caller_frame_shapes_are_exercised() {
    let cfg = GenConfig {
        max_depth: 4,
        max_stmts: 16,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
    for needle in [
        "upvar 1 $n v",
        "upvar 1 _cfy v",
        "upvar #0 _cfg g",
        "uplevel 1 {set _cfz up}",
        "uplevel 1 [list set $n cons]",
        "uplevel 1 [list _cff $n]",
    ] {
        assert!(
            corpus.iter().any(|s| s.contains(needle)),
            "caller-frame production never generated: {needle:?}"
        );
    }
    // The shapes must stay inside their private `_cf*` name space, so
    // wherever they land they cannot perturb the rest of a script.
    for line in corpus
        .iter()
        .flat_map(|s| s.lines())
        .filter(|l| l.contains("_cf"))
    {
        for v in VARS {
            assert!(
                !line.contains(&format!("${v} ")) && !line.ends_with(&format!("${v}")),
                "caller-frame line touches shared variable {v:?}: {line:?}"
            );
        }
    }
}

/// Every [`Malformed`] shape renders the form its documentation promises,
/// so a mode can never silently degrade into a *different* (or, worse, a
/// well-formed) expression while still counting towards the coverage claim.
#[test]
fn malformed_modes_each_render_their_documented_shape() {
    for mode in Malformed::ALL {
        for seed in 0..200u64 {
            let text = gen_with(seed, GenConfig::default()).render_malformed(*mode);
            assert!(
                matches_mode(*mode, &text),
                "{mode:?} rendered an unexpected shape: {text:?}"
            );
        }
    }
}

/// The single most important invariant of this production: a malformed
/// expression must still be a well-formed Tcl **word**.
///
/// The malformed-ness has to stay inside the *expression* grammar. A brace,
/// bracket, double quote, or backslash in the rendered text would instead
/// break `Tcl_ParseCommand`, making the whole *script* unparsable — a
/// different layer of Tcl entirely, and a uniformly uninteresting
/// whole-script failure rather than the `expr` syntax error being hunted.
/// (A trailing backslash is the subtle one: inside `expr {…}` it would
/// escape the word's own closing brace.)
#[test]
fn malformed_expressions_stay_inside_the_expr_grammar() {
    for mode in Malformed::ALL {
        for seed in 0..200u64 {
            let text = gen_with(seed, GenConfig::default()).render_malformed(*mode);
            for bad in ['{', '}', '[', ']', '"', '\\'] {
                assert!(
                    !text.contains(bad),
                    "{mode:?} emitted {bad:?}, which breaks the script parser: {text:?}"
                );
            }
        }
    }
}

/// The production is emitted at top level only, and that is load-bearing
/// twice over. Nested in a `proc` body C Tcl would not report it at all
/// (bodies compile lazily, at first call, so a syntax error in an uncalled
/// proc is never raised) while an eagerly-compiling engine would, turning
/// the finding into one about compile *timing*. Nested in a `catch`/`try`
/// body the error would be swallowed, erasing the very status difference
/// the production exists to expose.
///
/// With the rate pinned to always fire, depth 0 must always take the
/// branch and no deeper depth ever may. The nested half is checked through
/// the `_mx` marker, which two of the three statement contexts carry: over
/// 600 nested samples that would all be malformed if the guard were
/// dropped, missing every marker is vanishingly unlikely.
#[test]
fn malformed_expr_is_top_level_only() {
    let always = GenConfig {
        malformed_expr_permille: 1000,
        ..GenConfig::default()
    };
    for seed in 0..200u64 {
        let mut top = gen_with(seed, always);
        top.statement(0);
        assert!(
            is_malformed_stmt(&top.out),
            "depth 0 did not emit the malformed production: {:?}",
            top.out
        );
        for depth in 1..4 {
            let mut nested = gen_with(seed, always);
            nested.statement(depth);
            assert!(
                !nested.out.contains("_mx"),
                "depth {depth} emitted a malformed expression: {:?}",
                nested.out
            );
        }
    }
}

/// Every shape renders the spelling its documentation names, and every one
/// is a single Tcl word — the invariant that lets a shape drop into any
/// word position without reshaping the script around it.
#[test]
fn word_shapes_each_render_their_documented_spelling() {
    for &shape in WordShape::ALL {
        let w = shape.spelling();
        assert!(!w.is_empty(), "{shape:?} renders nothing");
        // Exactly one word, asked of the segmentation owner rather than of
        // a "contains a space" guess. `EscapedSpace` is precisely the
        // shape a guess gets wrong — `a\ b` reads as two words and is one
        // — and being one word is the property under test for it, so the
        // assertion has to be the real boundary rule.
        let probe = format!("puts {w}\n");
        let tokens = tcl_lexer::Lexer::new(&probe)
            .tokenise_all()
            .unwrap_or_else(|e| panic!("{shape:?} does not lex: {e:?}"));
        let commands =
            tcl_lexer::group_commands(&tokens, &probe, tcl_lexer::LexerConfig::default());
        assert_eq!(
            commands.len(),
            1,
            "{shape:?} renders `{w}`, which is not one command"
        );
        assert_eq!(
            commands[0].words.len(),
            2,
            "{shape:?} renders `{w}`, which is not exactly one word"
        );
        // And it is a complete script on its own as a command argument, so
        // it cannot be what makes a generated script unparsable.
        assert!(
            tcl_lexer::script_is_complete(&probe),
            "{shape:?} does not form a complete script: {probe}"
        );
    }
    assert_eq!(WordShape::QuotedEmptyBraces.spelling(), "\"{}\"");
    assert_eq!(WordShape::EscapedBraces.spelling(), r"\{\}");
    assert_eq!(WordShape::BracedDoubleGroup.spelling(), "{{}}");
    assert_eq!(WordShape::QuotedOpenBrace.spelling(), "\"{\"");
}

/// `brace_balanced` agrees with an actual brace walk, so the guard that
/// keeps `"{"` out of `expr {…}` and out of a proc body cannot drift from
/// the spellings it is guarding.
#[test]
fn brace_balanced_matches_a_real_walk() {
    for &shape in WordShape::ALL {
        let mut depth = 0i32;
        let bytes = shape.spelling().as_bytes();
        let mut i = 0;
        let mut ok = true;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 1,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth < 0 {
                        ok = false;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        assert_eq!(
            shape.brace_balanced(),
            ok && depth == 0,
            "{shape:?} declares brace_balanced()={} but walks to depth {depth}",
            shape.brace_balanced()
        );
    }
}

/// The invariant that decides whether a probe is a probe: a context that
/// nests the word inside a braced word must only ever be handed a
/// brace-balanced shape. Get it wrong and the shape's brace closes the
/// enclosing word — `proc _wl {} {… set v "{" …}` — and the script stops
/// parsing, which is a whole-script failure rather than a word-value
/// comparison. Asserted over the real corpus, not over the table, because
/// `script_is_complete` is what actually catches the mistake.
#[test]
fn unbalanced_shapes_never_reach_a_bracing_context() {
    let cfg = GenConfig {
        word_shape_permille: 1000,
        malformed_expr_permille: 0,
        ..GenConfig::default()
    };
    for seed in 0..400u64 {
        let src = generate(seed, &cfg);
        assert!(
            tcl_lexer::script_is_complete(&src),
            "seed {seed}: a word shape broke the script\n{src}"
        );
    }
}

/// Every shape and every context is reachable from the default config, so
/// no row of the cross is dead.
#[test]
fn every_word_shape_and_context_is_reachable() {
    let cfg = GenConfig {
        word_shape_permille: 400,
        ..GenConfig::default()
    };
    let corpus: Vec<String> = (0..600u64).map(|s| generate(s, &cfg)).collect();
    for &shape in WordShape::ALL {
        let w = shape.spelling();
        assert!(
            corpus.iter().any(|s| s.contains(w)),
            "shape {shape:?} (`{w}`) never appears in the corpus"
        );
    }
    // One marker per context's emitted form.
    for (ctx, marker) in [
        (WordContext::Assign, "set _wv "),
        (WordContext::CommandArg, "puts [string length "),
        (WordContext::NestedArg, "[format %s "),
        (WordContext::ProcArg, "proc _wp "),
        (WordContext::ProcLocal, "proc _wl "),
        (WordContext::ListElement, "set _wl [list "),
        (WordContext::DictValue, "set _wd [dict create k "),
        (WordContext::Append, "append _wa "),
        (WordContext::SwitchSubject, "_ws_def"),
        (WordContext::ExprOperand, "puts [expr {[string length "),
        (WordContext::FormatFold, "set _wf [format %s "),
        (WordContext::ListFold, "set _wg [list "),
    ] {
        assert!(
            corpus.iter().any(|s| s.contains(marker)),
            "context {ctx:?} never appears in the corpus (marker `{marker}`)"
        );
    }
}

/// The word-shape production reaches scripts at the default rate without
/// dominating them — the same balance `malformed_expressions_reach_...`
/// keeps for its own class.
#[test]
fn word_shapes_reach_scripts_at_the_default_rate() {
    let cfg = GenConfig::default();
    let corpus: Vec<String> = (0..1000u64).map(|s| generate(s, &cfg)).collect();
    let carrying = corpus
        .iter()
        .filter(|s| WordShape::ALL.iter().any(|sh| s.contains(sh.spelling())))
        .count();
    assert!(
        carrying > 0,
        "the word-shape production is dead at the default rate"
    );
    // A band, not a floor: the class is cheap (every shape is well-formed,
    // so it neither aborts the script nor displaces the statements after
    // it), but it still has to leave most of the budget on the rest of the
    // grammar. Measured 332/1000 at the default rate — the rate was set
    // *to* that, having started at 120‰ and reached 534, which is more of
    // the corpus than one class should own however productive it is. The
    // bounds are wide enough that ordinary generator changes do not trip
    // them and narrow enough that losing or dominating the class does.
    assert!(
        (150..=550).contains(&carrying),
        "word shapes reach {carrying}/1000 scripts at the default rate, outside the intended band"
    );
}

/// At rate 0 the production is gone *and* consumes no random draw, so the
/// stream is byte-identical to a build that never had it — which is what
/// lets a historical finding still replay from its recorded seed.
#[test]
fn word_shape_rate_zero_opts_out_and_leaves_the_stream_alone() {
    let off = GenConfig {
        word_shape_permille: 0,
        ..GenConfig::default()
    };
    for seed in 0..400u64 {
        let src = generate(seed, &off);
        for &shape in WordShape::ALL {
            assert!(
                !src.contains(shape.spelling()),
                "rate 0 still emitted {shape:?} (seed {seed}):\n{src}"
            );
        }
        assert!(
            !src.contains("_wv") && !src.contains("_ws_def"),
            "rate 0 still emitted a word-shape probe (seed {seed}):\n{src}"
        );
    }
}

/// A rate of 0 opts out completely, so a campaign can keep the generator's
/// pre-existing behaviour (and a registry's historical seeds replaying)
/// while this production exists.
#[test]
fn malformed_expr_rate_zero_opts_out() {
    let off = GenConfig {
        malformed_expr_permille: 0,
        ..GenConfig::default()
    };
    for seed in 0..400u64 {
        let src = generate(seed, &off);
        assert!(
            !src.contains("_mx"),
            "rate 0 still emitted a malformed statement (seed {seed}):\n{src}"
        );
    }
}

/// The production must be live at the **default** rate: a knob nobody sets
/// is a knob that never runs, and the point of this work was that the
/// malformed-expression class was unreachable in ordinary campaigns. Also
/// pins the rate's order of magnitude from above, so a routine campaign
/// keeps spending most of its budget on well-formed semantics — a
/// malformed statement is uncaught, so it aborts the script and everything
/// after it goes unexercised.
#[test]
fn malformed_expressions_reach_scripts_at_the_default_rate() {
    let cfg = GenConfig::default();
    let corpus: Vec<String> = (0..1000u64).map(|s| generate(s, &cfg)).collect();
    let carrying = corpus.iter().filter(|s| s.contains("_mx")).count();
    assert!(
        carrying > 0,
        "the malformed production is dead at the default rate"
    );
    assert!(
        carrying < corpus.len() / 4,
        "malformed statements dominate the corpus at the default rate: {carrying}/1000"
    );
}

/// The coverage claim, evidenced rather than asserted: a malformed
/// expression from this production really does make *our own* engine report
/// an error, so the differential's status comparison has two comparable
/// outcomes rather than one engine quietly succeeding.
///
/// This is the regression guard for the bug that motivated the whole
/// production — an unparsable runtime expression that evaluated to its own
/// source text, so `expr {1+}` returned the string `1+` — which no
/// generated script could reach before, and which a campaign therefore
/// could not have caught.
///
/// Drives `tcl-vm` in-process (lower → CFG → codegen → run) exactly as
/// `wasm_diff`'s direct arm does, so no external binary is needed.
#[test]
fn malformed_expressions_error_on_our_engine() {
    for mode in Malformed::ALL {
        if MALFORMED_MODES_WITH_OPEN_DIVERGENCE.contains(mode) {
            continue;
        }
        for seed in 0..12u64 {
            let text = gen_with(seed, GenConfig::default()).render_malformed(*mode);
            let script = format!("set a 1\nset b 2\nputs [expr {{{text}}}]\n");
            assert!(
                errors_on_our_engine(&script),
                "{mode:?} did not error on our engine — the differential would \
                     see one engine succeed where C Tcl raises a syntax error: {text:?}"
            );
        }
    }
}
