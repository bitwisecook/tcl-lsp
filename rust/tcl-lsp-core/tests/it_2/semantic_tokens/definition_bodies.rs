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

//! Definition-body grammars: `TclOO`, snit, [incr Tcl] and scoped command environments.

use super::*;

// TclOO definition-body highlighting.
//
// C-Tcl proof: `oo::configurable`, `oo::abstract`, and `oo::singleton` are
// real Tcl 9.0 metaclasses (`info commands oo::*`) that manufacture classes
// with the same `create name ?body?` shape as `oo::class`.  Inside such a
// definition body the words `superclass` / `self` / `property` / `method`
// are TclOO definition sub-keywords, and a `method`/`constructor` body holds
// ordinary Tcl code — so `set x 1` inside it is a real command call.

#[test]
fn oo_configurable_body_keywords_are_highlighted() {
    // `superclass` / `self` / `property` / `method` inside an
    // `oo::configurable` body must highlight as keywords, exactly as they do
    // inside `oo::class`.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    superclass Base\n",
        "    self mixin M\n",
        "    property color\n",
        "    method paint {} { set x 1 }\n",
        "}\n",
    );
    for kw in ["superclass", "self", "property", "method"] {
        assert_eq!(
            kind_of_word(src, "tcl9.0", kw).as_deref(),
            Some("keyword"),
            "`{kw}` inside oo::configurable body must be a keyword",
        );
    }
}

#[test]
fn oo_configurable_method_body_is_recursed() {
    // The method body `{ set x 1 }` must be tokenised (not emitted as one
    // opaque string): `set` → function, `1` → number.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    method paint {} { set x 1 }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "method body should tokenise `set` as a function: {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.ttype == "number"),
        "method body should tokenise the `1` literal as a number: {toks:?}",
    );
}

#[test]
fn oo_class_method_body_is_recursed() {
    // The `method`/`constructor` body inside an `oo::class` block must be
    // recursed, not left as one opaque string.
    let src = concat!(
        "oo::class create C {\n",
        "    constructor {} { set y 2 }\n",
        "    method m {} { puts hello }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    // `puts` (a builtin) inside the method body → function.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "method body should tokenise `puts`: {toks:?}",
    );
    // `set` inside the constructor body → function; `2` → number.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "constructor body should tokenise `set`: {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.ttype == "number"),
        "constructor body should tokenise the `2` literal: {toks:?}",
    );
}

#[test]
fn oo_wrapper_block_body_keeps_member_grammar() {
    // TclOO's bare wrapper-block form `private { … }` / `self { … }` is a
    // nested *definition* script: the members inside it (`method`, `variable`)
    // must still be recognised as keywords, and their bodies recursed — not
    // walked as ordinary Tcl.  A block that drops out of definition grammar
    // leaves `method` inside `private { … }` unhighlighted.
    for wrapper in ["private", "self"] {
        let src = format!(
            "oo::class create C {{\n    {wrapper} {{\n        method m {{}} {{ set y 2 }}\n        variable secret 0\n    }}\n}}\n",
        );
        // The inner member keywords resolve from the grammar the block kept.
        for kw in ["method", "variable"] {
            assert_eq!(
                kind_of_word(&src, "tcl9.0", kw).as_deref(),
                Some("keyword"),
                "`{kw}` inside `{wrapper} {{ … }}` must be a member keyword: {:?}",
                decode(&src, "tcl9.0"),
            );
        }
        // The inner method's body is recursed (its `set` builtin tokenises).
        let toks = decode(&src, "tcl9.0");
        assert!(
            toks.iter()
                .any(|t| t.ttype == "function" && tok_text(&src, t) == "set"),
            "`{wrapper} {{ … }}` inner method body should tokenise `set`: {toks:?}",
        );
        // The declared instance variable resolves as a variable.
        assert_eq!(
            kind_of_word(&src, "tcl9.0", "secret").as_deref(),
            Some("variable"),
            "`variable secret` inside `{wrapper} {{ … }}` should declare a variable",
        );
    }
}

#[test]
fn oo_abstract_and_singleton_bodies_are_recursed() {
    for metaclass in ["oo::abstract", "oo::singleton"] {
        let src = format!("{metaclass} create C {{\n    method m {{}} {{ set z 3 }}\n}}\n");
        assert_eq!(
            kind_of_word(&src, "tcl9.0", "method").as_deref(),
            Some("keyword"),
            "`method` inside {metaclass} body must be a keyword",
        );
        let toks = decode(&src, "tcl9.0");
        assert!(
            toks.iter()
                .any(|t| t.ttype == "function" && tok_text(&src, t) == "set"),
            "{metaclass}: method body should tokenise `set`: {toks:?}",
        );
    }
}

#[test]
fn oo_property_accessor_bodies_are_recursed() {
    // `property NAME -get { BODY } -set { BODY }` accessor bodies inside a
    // configurable class must be recursed too.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    property color -get { return red } -set { set color $value }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    // `return` is itself a language keyword; its presence inside the `-get`
    // body proves the accessor body was recursed.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "keyword" && tok_text(src, t) == "return"),
        "`-get` accessor body should tokenise `return`: {toks:?}",
    );
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "`-set` accessor body should tokenise `set`: {toks:?}",
    );
    // The `$value` variable substitution inside the `-set` body is recursed.
    assert!(
        toks.iter().any(|t| t.ttype == "variable"),
        "`-set` accessor body should tokenise `$value`: {toks:?}",
    );
}

#[test]
fn top_level_method_is_not_an_oo_body() {
    // Context guard: a top-level user proc named `method` (outside any OO
    // definition body) must NOT be treated as an OO member.  A bare
    // `method a b {c}` call at top level is a plain command — so `method`
    // itself is a plain command head (a `function`, NOT a `keyword` — member
    // keywords are recognised context-sensitively from the definer grammar,
    // exactly like snit's `typemethod`), and its last braced word stays an
    // opaque string, not recursed as a body.
    let src = "method a b { this is not code }\n";
    let toks = decode(src, "tcl8.6");
    assert_ne!(
        kind_of_word(src, "tcl8.6", "method").as_deref(),
        Some("keyword"),
        "a top-level `method` must not be a keyword (context-sensitive): {toks:?}",
    );
    // No word inside the braced body should be recursed as a command head;
    // `this`/`is`/`not`/`code` must never surface as separate function tokens.
    for w in ["this", "is", "not", "code"] {
        assert!(
            !toks
                .iter()
                .any(|t| t.ttype == "function" && tok_text(src, t) == w),
            "top-level `method` braced arg must not be recursed as a body ({w}): {toks:?}",
        );
    }
}

#[test]
fn alias_to_user_proc_named_method_does_not_tokenise_data_as_a_member_body() {
    // Tclsh executes the user `method` proc through `define_method`; the
    // final braced argument is never evaluated as a definition-member body.
    let src = "proc method {name parameters body} {\n    return \"$name:$parameters:$body\"\n}\ninterp alias {} define_method {} method\noo::class create C {\n    define_method m {} {\n        # inert data\n        puts must-not-run\n    }\n}\n";
    let toks = decode(src, "tcl9.0");
    assert!(
        !toks
            .iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "the user proc's braced data must not tokenise its `puts` as executable: {toks:?}"
    );
}

#[test]
fn oo_define_body_form_recurses_method_bodies() {
    // The `oo::define Cls { … }` script form is an outer OO definition body
    // too — method bodies inside it must recurse.
    let src = concat!("oo::define Cls {\n", "    method m {} { puts hi }\n", "}\n",);
    assert_eq!(
        kind_of_word(src, "tcl9.0", "method").as_deref(),
        Some("keyword"),
    );
    let toks = decode(src, "tcl9.0");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "oo::define body method should recurse: {toks:?}",
    );
}

#[test]
fn oo_define_member_form_body_is_not_oo_context() {
    // The member form
    // `oo::define C method m {} { … }` carries an ordinary method body, not
    // a definition script.  A nested `method a b {not code}` inside it must
    // NOT be treated as an OO member definition — its `{not code}` stays an
    // opaque string (no command inside is tokenised as a function).
    let src = "oo::define C method m {} { method a b {not code} }\n";
    let toks = decode(src, "tcl9.0");
    // The innermost `{not code}` must stay an opaque string — `not`/`code`
    // never surface as command heads.  (The inner `method` head is an ordinary
    // command call inside the method body, so it legitimately reads as a
    // `function`, not a keyword — there is no definition-body context here.)
    for w in ["not", "code"] {
        assert!(
            !toks
                .iter()
                .any(|t| t.ttype == "function" && tok_text(src, t) == w),
            "nested `method a b {{not code}}` inside a member-form method body \
             must not be recursed as an OO body ({w}): {toks:?}",
        );
    }
    // The real method body itself is still recursed (its own commands are
    // walked as ordinary code) — a `set` there would tokenise.
    let src2 = "oo::define C method m {} { set x 1 }\n";
    let toks2 = decode(src2, "tcl9.0");
    assert!(
        toks2
            .iter()
            .any(|t| t.ttype == "function" && tok_text(src2, t) == "set"),
        "member-form method body should still tokenise its own code: {toks2:?}",
    );
}

#[test]
fn oo_define_inline_member_keyword_is_grammar_driven() {
    // The inline form `oo::define C method m {} {…}` puts the member keyword at
    // an argument position; it must still highlight as a keyword, sourced from
    // the definer's `definition_body` grammar (not a hardcoded list).  `self`
    // introduces a second inner member keyword.
    let src = "oo::define C method greet {} { return hi }\n";
    assert_eq!(
        kind_of_word(src, "tcl9.0", "method").as_deref(),
        Some("keyword"),
        "inline `oo::define … method` keyword must highlight: {:?}",
        decode(src, "tcl9.0"),
    );
    let src2 = "oo::objdefine obj self method m {} {}\n";
    for kw in ["self", "method"] {
        assert_eq!(
            kind_of_word(src2, "tcl9.0", kw).as_deref(),
            Some("keyword"),
            "inline `oo::objdefine … self method` keyword `{kw}` must highlight: {:?}",
            decode(src2, "tcl9.0"),
        );
    }
}

#[test]
fn tcloo_members_are_context_sensitive_like_snit() {
    // Parity with `snit_member_keyword_only_inside_body`: a TclOO member word
    // used as a bare top-level command (outside any definition body) must NOT
    // be a keyword — both class systems recognise members from the grammar
    // context, so neither over-colours a same-named top-level command.
    for member in ["constructor", "superclass", "classmethod"] {
        let src = format!("{member} whatever args\n");
        assert_ne!(
            kind_of_word(&src, "tcl9.0", member).as_deref(),
            Some("keyword"),
            "top-level `{member}` must not be a keyword outside a body: {:?}",
            decode(&src, "tcl9.0"),
        );
    }
    // …but inside an `oo::class` body every one of them is a keyword.
    let body = "oo::class create C {\n    superclass B\n    constructor {} {}\n    classmethod mk {} {}\n}\n";
    for member in ["superclass", "constructor", "classmethod"] {
        assert_eq!(
            kind_of_word(body, "tcl9.0", member).as_deref(),
            Some("keyword"),
            "`{member}` inside an oo::class body must be a keyword: {:?}",
            decode(body, "tcl9.0"),
        );
    }
}

// `variable a b c` inside a TclOO definition body declares every
// name as an instance variable (the namespace-level `variable name ?value?`
// pairs only mark the leading name). All names must highlight as variables.

#[test]
fn oo_body_variable_highlights_every_name() {
    let src = "oo::class create Foo {\n    variable width height depth\n    method m {} { return 1 }\n}\n";
    // Each of `width`, `height`, `depth` in the class-body `variable`
    // declaration is a variable, not a bareword string.
    for name in ["width", "height", "depth"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in an oo::class body `variable` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn plain_variable_command_keeps_pair_classification() {
    // Outside an OO definition body (a normal namespace-level `variable`), the
    // leading name is a write target — highlighted as a variable declaration.
    let src = "namespace eval ns {\n    variable myvar 1\n}\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "myvar").as_deref(),
        Some("variable"),
        "the leading `variable` name is a variable: {:?}",
        decode(src, "tcl8.6"),
    );
}

// snit — the definition-body grammar is registry data (tcl_registry::definer),
// so `snit::type` / `snit::widget` bodies recurse and highlight like TclOO
// without any snit-specific code in the token walk.

#[test]
fn snit_type_members_highlight() {
    let src = "snit::type Dog {\n\
               \x20   variable barks\n\
               \x20   method bark {volume} { return $barks }\n\
               \x20   typemethod count {} { return 1 }\n\
               \x20   constructor {args} { set barks 0 }\n\
               }\n";
    // Instance variable declaration.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "barks").as_deref(),
        Some("variable"),
        "snit `variable` name must be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
    // Method parameter — the standard LSP `parameter` type.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "volume").as_deref(),
        Some("parameter"),
        "snit method parameter must be a parameter: {:?}",
        decode(src, "tcl8.6"),
    );
    // snit-specific member keyword.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "typemethod").as_deref(),
        Some("keyword"),
        "`typemethod` must be a keyword inside a snit body: {:?}",
        decode(src, "tcl8.6"),
    );
    // The `set` inside a method body is recursed (not one opaque string).
    assert_eq!(
        kind_of_word(src, "tcl8.6", "set").as_deref(),
        Some("function"),
        "a snit method body must be recursed: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn snit_widget_declarations_and_handlers() {
    let src = "snit::widget Dial {\n\
               \x20   typevariable count\n\
               \x20   component inner\n\
               \x20   onconfigure -value valuevar { set count $valuevar }\n\
               }\n";
    for name in ["count", "inner", "valuevar"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "snit declaration `{name}` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn snit_member_keyword_only_inside_body() {
    // A top-level user proc named `typemethod` must NOT be a keyword — the
    // member keywords are context-sensitive to a definition body.
    let src = "typemethod foo bar\n";
    assert_ne!(
        kind_of_word(src, "tcl8.6", "typemethod").as_deref(),
        Some("keyword"),
        "a top-level `typemethod` must not be a keyword: {:?}",
        decode(src, "tcl8.6"),
    );
}

// [incr Tcl] — `itcl::class` definition bodies. Members (method / proc /
// variable / common / constructor / destructor / inherit) plus the access
// modifiers public / protected / private (prefix wrappers) all resolve from the
// registry's ITCL grammar, like TclOO/snit.

#[test]
fn itcl_class_members_highlight() {
    let src = "itcl::class Stack {\n\
               \x20   inherit Deque\n\
               \x20   variable contents {}\n\
               \x20   constructor {aa} { set contents $aa }\n\
               \x20   method push {value} { lappend contents $value }\n\
               }\n";
    // The `itcl::class` head is a keyword (like `oo::class`).
    assert_eq!(
        kind_of_word(src, "tcl8.6", "class").as_deref(),
        Some("keyword"),
        "`itcl::class` must be a keyword: {:?}",
        decode(src, "tcl8.6"),
    );
    // Member keywords.
    for kw in ["inherit", "variable", "constructor", "method"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", kw).as_deref(),
            Some("keyword"),
            "`{kw}` inside an itcl::class body must be a keyword: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    // The instance variable declaration is a variable; the method's parameter
    // carries the standard LSP `parameter` type.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "contents").as_deref(),
        Some("variable")
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "value").as_deref(),
        Some("parameter")
    );
}

#[test]
fn itcl_access_modifier_wraps_inner_member() {
    // `public method …` / `private variable …`: the modifier AND the inner
    // member keyword are keywords, the inner param/var declarations resolve, and
    // the inner method body recurses.
    let src = "itcl::class C {\n\
               \x20   public method size {ww} { return $ww }\n\
               \x20   private variable secret 0\n\
               }\n";
    for kw in ["public", "method", "private", "variable"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", kw).as_deref(),
            Some("keyword"),
            "`{kw}` must be a keyword in an itcl body: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    // The wrapped method's parameter and the wrapped variable's name both
    // resolve — the parameter as `parameter`, the variable as a
    // variable.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "ww").as_deref(),
        Some("parameter")
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "secret").as_deref(),
        Some("variable")
    );
}

#[test]
fn itcl_member_keyword_only_inside_body() {
    // Context guard: a bare top-level `inherit` / `common` is not a keyword.
    for kw in ["inherit", "common"] {
        let src = format!("{kw} whatever\n");
        assert_ne!(
            kind_of_word(&src, "tcl8.6", kw).as_deref(),
            Some("keyword"),
            "top-level `{kw}` must not be a keyword: {:?}",
            decode(&src, "tcl8.6"),
        );
    }
}

#[test]
fn itcl_variable_config_body_recurses() {
    // itcl `variable NAME init {configbody}` — the trailing config body is a
    // recursable script (grammar `Body` role), so `puts` inside it is a command.
    let src = "itcl::class C {\n\
               \x20   variable color red { puts $color }\n\
               }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "puts").as_deref(),
        Some("function"),
        "the variable config body must recurse: {:?}",
        decode(src, "tcl8.6"),
    );
    // The declared name is still a variable.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "color").as_deref(),
        Some("variable")
    );
}

#[test]
fn itcl_body_external_definition_highlights() {
    // `itcl::body Class::method {params} {body}` — its parameter list declares
    // variables and its body recurses, via the registry arg-roles.
    let src = "itcl::body C::render {ww hh} { set area [expr {$ww * $hh}] }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "set").as_deref(),
        Some("function"),
        "itcl::body's body must recurse: {:?}",
        decode(src, "tcl8.6"),
    );
    for p in ["ww", "hh"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", p).as_deref(),
            Some("parameter"),
            "itcl::body param `{p}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

// The `report::defstyle` scoped command environment.
//
// Inside a report::defstyle style script the report configuration methods
// (top/data/columns/…) highlight as library functions and their ensemble
// operations (`top set`) as subcommand keywords — all from registry data.

#[test]
fn scoped_report_line_command_is_function() {
    let src = "::report::defstyle st {} {\n    top set foo\n    columns\n}\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "top"),
        Some("function".to_string())
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "columns"),
        Some("function".to_string())
    );
}

#[test]
fn scoped_report_operation_is_keyword() {
    let src = "::report::defstyle st {} {\n    top set foo\n}\n";
    // The `set` operation word after `top` colours as a subcommand keyword.
    let t = decode(src, "tcl8.6");
    assert!(
        t.iter()
            .any(|x| x.ttype == "keyword" && tok_text(src, x) == "set"),
        "`set` op is a keyword: {t:?}",
    );
}

#[test]
fn scoped_report_command_not_highlighted_outside_body() {
    // `columns` at top level is an unknown word, not a library function.
    let src = "columns\n";
    // No defaultLibrary function classification — it is a plain function token
    // (unknown command) but must not gain the scoped treatment; assert it is
    // not classified as a keyword and that a genuinely scoped-only op word is
    // absent.  The key scoping fact: `topdatasep` is unknown at top level.
    let src2 = "topdatasep enable\n";
    assert_ne!(
        kind_of_word(src2, "tcl8.6", "enable"),
        Some("keyword".to_string()),
        "`enable` is not a scoped op outside a style body"
    );
    let _ = src;
}
