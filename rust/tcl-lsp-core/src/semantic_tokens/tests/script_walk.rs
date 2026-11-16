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

//! Recursion into script-shaped arguments: case lists, definition bodies, `apply` lambdas and `uplevel`.

use super::*;

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
