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

//! The role a command gives its words: heads and their `defaultLibrary` modifier, option decorators, event names, declared variables and parameters.

use super::*;

#[test]
fn test_when_event_name_highlighted_as_event() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "when HTTP_REQUEST { puts hello }\n");
    let events: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "event")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].length,
        i64::try_from("HTTP_REQUEST".len()).unwrap()
    );
}

#[test]
fn test_when_lowercase_arg_not_event() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "when some_thing { puts ok }\n");
    let events: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "event")
        .collect();
    assert!(events.is_empty());
}

#[test]
fn test_regexp_option_highlighted_as_decorator() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regexp -nocase {pat} $str\n");
    let decs: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "decorator")
        .collect();
    assert!(
        decs.iter()
            .any(|t| t.length == i64::try_from("-nocase".len()).unwrap())
    );
}

/// End-to-end: `return -code error "bad"` must reach the client
/// with `-code` as a `decorator` and `error` as an `enumMember`, not as
/// plain strings.  The `tcl-lsp-core` unit test pins the classifier; this
/// pins the whole server pipeline (registry `OptionSpec` → token pass →
/// legend-encoded `textDocument/semanticTokens/full` response), which is what
/// the editor actually sees.
#[test]
fn test_return_code_option_decorator_issue_967() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "return -code error \"bad\"\n");
    let toks = typed(&mut lsp, &lg, &uri);
    assert!(
        toks.iter()
            .any(|t| t.ttype == "decorator" && t.length == i64::try_from("-code".len()).unwrap()),
        "expected `-code` to arrive as a decorator; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|t| t.ttype == "enumMember" && t.length == i64::try_from("error".len()).unwrap()),
        "expected `error` to arrive as an enumMember; got {toks:?}"
    );

    // TN: the same word passed to a command that declares no options at all
    // stays a plain argument.
    let uri2 = open_doc(&mut lsp, "concat -code error\n");
    let toks2 = typed(&mut lsp, &lg, &uri2);
    assert!(
        !toks2.iter().any(|t| t.ttype == "decorator"),
        "`-code` is not a declared option of `concat`; got {toks2:?}"
    );
}

#[test]
fn test_non_option_dash_word_not_decorator() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts -foo\n");
    let decs: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "decorator")
        .collect();
    assert!(decs.is_empty());
}

#[test]
fn test_oo_class_split() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "oo::class create Dog {}\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert!(
        tokens
            .iter()
            .any(|t| t.ttype == "namespace" && t.length == i64::try_from("oo::".len()).unwrap())
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.ttype == "keyword" && t.length == i64::try_from("class".len()).unwrap())
    );
}

#[test]
fn test_global_qualified_command() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "::set x 1\n");
    let ns: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "namespace")
        .collect();
    assert_eq!(ns.len(), 1);
    assert_eq!(ns[0].length, i64::try_from("::".len()).unwrap());
}

#[test]
fn test_builtin_command_has_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "puts hello\n");
    let cmd = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .find(|t| t.ttype == "function")
        .expect("function token");
    assert!(cmd.modifiers & modifier_bit(&mods, "defaultLibrary") != 0);
}

#[test]
fn test_user_command_no_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "mycommand arg\n");
    let cmd = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .find(|t| t.ttype == "function")
        .expect("function token");
    assert_eq!(cmd.modifiers & modifier_bit(&mods, "defaultLibrary"), 0);
}

#[test]
fn test_subcommand_has_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "string length foo\n");
    let sub: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "keyword" && t.char == 7)
        .collect();
    assert_eq!(sub.len(), 1);
    assert!(sub[0].modifiers & modifier_bit(&mods, "defaultLibrary") != 0);
}

#[test]
fn test_expr_function_has_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "expr {abs(-1)}\n");
    let fns: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "function" && t.length == 3)
        .collect();
    assert_eq!(fns.len(), 1);
    assert!(fns[0].modifiers & modifier_bit(&mods, "defaultLibrary") != 0);
}

#[test]
fn test_proc_definition_no_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "proc greet {name} {}\n");
    let defn = modifier_bit(&mods, "definition");
    let fn_tok = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .find(|t| t.ttype == "function" && (t.modifiers & defn != 0))
        .expect("definition function token");
    assert_eq!(fn_tok.modifiers & modifier_bit(&mods, "defaultLibrary"), 0);
}

#[test]
fn test_imported_tcltest_test_structure_recognised() {
    // After `namespace import tcltest::*`, a bare `test` resolves to
    // the tcltest spec — its `-body`/`-result` are options and the body script
    // is recursed.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "namespace import tcltest::*\n\
               test t-1 {desc} -body {\n    set x 1\n} -result 1\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let kind_of = |word: &str| {
        tokens
            .iter()
            .find(|t| covered(src, t) == word)
            .map(|t| t.ttype.clone())
    };
    assert_eq!(
        kind_of("-body").as_deref(),
        Some("decorator"),
        "imported test's -body must be an option: {tokens:?}",
    );
    assert_eq!(
        kind_of("-result").as_deref(),
        Some("decorator"),
        "imported test's -result must be an option: {tokens:?}",
    );
    assert_eq!(
        kind_of("set").as_deref(),
        Some("function"),
        "the -body script must be recursed: {tokens:?}",
    );
}

#[test]
fn test_namespaced_imported_tcltest_test_structure_recognised() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = concat!(
        "package require tcltest\n",
        "package require tclopt\n",
        "namespace eval ::nativeTest {\n",
        "    namespace path ::tclopt\n",
        "    namespace import ::tcltest::*\n",
        "    test native-empty {empty Euclidean norm} ",
        "-body {mp_enorm {}} -result 0.0\n",
        "    test native-input-preserved ",
        "{C kernels modify private copies, not shared Tcl inputs} -body {\n",
        "        set r {2 0 1 3}\n",
        "        set alias $r\n",
        "        set result [mp_covar 2 $r 2 {0 1} 1e-14]\n",
        "        list $r $alias [expr {[dict get $result r] ne $r}]\n",
        "    } -result {{2 0 1 3} {2 0 1 3} 1}\n",
        "}\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    for option in ["-body", "-result"] {
        let options = tokens
            .iter()
            .filter(|token| covered(src, token) == option)
            .collect::<Vec<_>>();
        assert_eq!(options.len(), 2, "both imported tests carry {option}");
        assert!(
            options.iter().all(|token| token.ttype == "decorator"),
            "namespace-imported test option {option} must be classified: {options:?}",
        );
    }
    for command in ["mp_enorm", "mp_covar", "set", "list", "expr", "dict"] {
        assert!(
            tokens
                .iter()
                .any(|token| covered(src, token) == command && token.ttype == "function"),
            "test-body command {command} must be classified: {tokens:?}",
        );
    }
}

#[test]
fn test_source_command_substitution_argument_is_tokenised() {
    // A command substitution in `source`'s argument is highlighted
    // as a command sequence.
    //
    // The whole sequence, in order, not spot checks on two words: the claim
    // is that the argument reads as *code*, and that claim is only
    // as strong as the weakest word in it.  `join` in particular has to be the
    // subcommand keyword rather than the `join` list command that shares its
    // spelling — a lookup that goes through `file`'s ensemble, so getting it
    // right is evidence the substitution was resolved and not merely lexed.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let src = "source [file join $currentDir testUtilities.tcl]\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let actual: Vec<(&str, &str)> = tokens
        .iter()
        .map(|t| (covered(src, t), t.ttype.as_str()))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("source", "keyword"),
            ("file", "function"),
            ("join", "keyword"),
            ("$currentDir", "variable"),
            ("testUtilities.tcl", "string"),
        ],
        "the [file join …] substitution must tokenise as a command sequence",
    );
    // `file` and `join` resolve to the built-in ensemble, not to user code —
    // the modifier an editor colours differently from a local proc call.
    let library = modifier_bit(&mods, "defaultLibrary");
    for word in ["file", "join"] {
        let tok = tokens
            .iter()
            .find(|t| covered(src, t) == word)
            .unwrap_or_else(|| panic!("no token covers {word:?}"));
        assert!(
            tok.modifiers & library != 0,
            "`{word}` must carry defaultLibrary: {tok:?}",
        );
    }
}

#[test]
fn test_document_link_never_spans_more_than_one_semantic_token() {
    // Correct semantic tokens (the test above) are not enough to make the
    // argument *look* highlighted: an editor paints a document-link range
    // in one flat link colour plus an underline, so a link spanning `file
    // join $currentDir esd_pulse_circuit.tcl` erases every token boundary
    // inside it and the whole substitution reads as one word.
    //
    // The token assertions above do not cover this: a link may cover at
    // most one token, because covering two means hiding the boundary
    // between them.  Stated over links in general, so it also holds for
    // `package require` and for whatever gains a link next.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "package require tcltest\n\
               set currentDir [file dirname [info script]]\n\
               source [file join $currentDir esd_pulse_circuit.tcl]\n\
               source helper.tcl\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let links = lsp.document_links(&uri);
    let links = links.as_array().expect("documentLink array");
    assert!(
        !links.is_empty(),
        "expected links to assert about: {links:?}"
    );
    for link in links {
        let range = &link["range"];
        let (line, start, end) = (
            range["start"]["line"].as_i64().expect("start line"),
            range["start"]["character"].as_i64().expect("start char"),
            range["end"]["character"].as_i64().expect("end char"),
        );
        assert_eq!(
            line,
            range["end"]["line"].as_i64().expect("end line"),
            "a source/package link is single-line: {link}",
        );
        let overlapping: Vec<&str> = tokens
            .iter()
            .filter(|t| t.line == line && t.char < end && t.char + t.length > start)
            .map(|t| covered(src, t))
            .collect();
        assert!(
            overlapping.len() <= 1,
            "link over {:?} covers {} semantic tokens ({overlapping:?}) — an editor \
             paints it as one flat run, hiding every boundary inside it: {link}",
            src.split('\n')
                .nth(usize::try_from(line).unwrap())
                .unwrap_or("")
                .get(usize::try_from(start).unwrap()..usize::try_from(end).unwrap())
                .unwrap_or(""),
            overlapping.len(),
        );
    }
}

#[test]
fn test_global_declares_every_name() {
    // Like `variable`, `global a b c` declares every name as a variable.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "proc p {} {\n    global alpha beta gamma\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    for name in ["alpha", "beta", "gamma"] {
        let t = tokens
            .iter()
            .find(|t| covered(src, t) == name)
            .unwrap_or_else(|| panic!("no token covers {name:?}: {tokens:?}"));
        assert_eq!(
            t.ttype, "variable",
            "`{name}` in `global` must be a variable"
        );
    }
}

#[test]
fn test_foreach_loop_variables_are_variables() {
    // `foreach`/`dict for` bind their iteration variables — a single bareword
    // and each element of a braced list read as variable declarations.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "foreach item $lst { puts $item }\nforeach {key val} $pairs { puts $key }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let is_var = |word: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == word && t.ttype == "variable")
    };
    for name in ["item", "key", "val"] {
        assert!(
            is_var(name),
            "loop variable `{name}` must be a variable: {tokens:?}"
        );
    }
}

#[test]
fn test_proc_and_lambda_parameters_are_parameters() {
    // Procedure / apply-lambda parameter names carry the standard LSP
    // `parameter` type — distinguishable from an ordinary local — while
    // `dict map`'s loop variables stay plain variable declarations.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "proc greet {name age} { return $name }\n\
               apply {{alpha beta} { return $alpha }} 1 2\n\
               dict map {mk mv} $d { set mk $mv }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let has = |word: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == word && t.ttype == ttype)
    };
    for name in ["name", "age", "alpha", "beta"] {
        assert!(
            has(name, "parameter"),
            "`{name}` must be a parameter declaration: {tokens:?}"
        );
    }
    for name in ["mk", "mv"] {
        assert!(
            has(name, "variable"),
            "loop var `{name}` must stay a variable declaration: {tokens:?}"
        );
    }
}

#[test]
fn test_upvar_and_dict_update_locals_are_variables() {
    // `upvar` / `namespace upvar` locals and `dict update` var names read as
    // variable declarations; the other-frame names and dict keys stay strings.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "upvar 1 othervar localvar\n\
               dict update mydict thekey thevar { set thevar 1 }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let kind_of = |word: &str| {
        tokens
            .iter()
            .find(|t| covered(src, t) == word)
            .map(|t| t.ttype.clone())
    };
    assert_eq!(
        kind_of("localvar").as_deref(),
        Some("variable"),
        "{tokens:?}"
    );
    assert_eq!(kind_of("thevar").as_deref(), Some("variable"), "{tokens:?}");
    assert_eq!(kind_of("othervar").as_deref(), Some("string"), "{tokens:?}");
    assert_eq!(kind_of("thekey").as_deref(), Some("string"), "{tokens:?}");
}

/// Semantic-token correctness, checked section by section against 1.11.4.
///
/// One test per section, so a regression names the section it broke.
#[test]
fn test_issue_898_semantic_token_corrections() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        // §1 — a literal word's closing delimiter must be covered.
        "set greeting \"hello world\"\n",
        "set braced {a b}\n",
        "set bound 3\n",
        "puts ${bound}\n",
        "set esc \"with \\\"q\\\" in\"\n",
        // §3 — an array ref with a *substituted* index is still a variable.
        "set env($greeting) 1\n",
        // §4 / §2 — parameters, method + class names.
        "proc greet {name} { return $name }\n",
        "oo::class create Shape {\n",
        "    method area {} { return 0 }\n",
        "}\n",
        // §5 — a regex group's closer is a group, not a quantifier.
        "regexp {^(foo)+$} $s\n",
        // §6 — expr parens and ternary are operators.
        "set r [expr {($a + $b) * ($c > 1 ? 1 : 2)}]\n",
        // §7 — an empty body emits no stray `}` token.
        "proc empty {args} {}\n",
        // §8 — a quoted string containing `::` is a string, not a namespace.
        "append cmd \"::scan x\"\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    // §1: both delimiters covered, for quotes, braces and `${…}` alike.
    assert!(has("\"hello world\"", "string"), "§1 quotes: {tokens:?}");
    assert!(has("{a b}", "string"), "§1 braces: {tokens:?}");
    assert!(has("${bound}", "variable"), "§1 braced var: {tokens:?}");
    // An escaped string keeps its delimiters too — that path drops *both*.
    assert!(has("\"", "string"), "§1 escaped-string quotes: {tokens:?}");
    assert!(has("\\\"", "escape"), "§1 escape: {tokens:?}");

    // §3: the literal fragments of `env($greeting)` are the variable, not strings.
    assert!(has("env(", "variable"), "§3 array name: {tokens:?}");
    assert!(has(")", "variable"), "§3 array close: {tokens:?}");

    // §4 / §2.
    assert!(has("name", "parameter"), "§4 parameter: {tokens:?}");
    assert!(has("Shape", "class"), "§2 class name: {tokens:?}");
    assert!(has("area", "method"), "§2 method name: {tokens:?}");

    // §5: `)` is a group delimiter, never a quantifier.
    assert!(has(")", "regexpGroup"), "§5 regex close: {tokens:?}");
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == ")" && t.ttype == "regexpQuantifier"),
        "§5: a regex `)` must not be a quantifier: {tokens:?}",
    );

    // §6: grouping and ternary punctuation are operators.
    for op in ["(", ")", "?", ":"] {
        assert!(
            has(op, "operator"),
            "§6 `{op}` must be an operator: {tokens:?}"
        );
    }

    // §7: the empty body `{}` must not emit a `}` as a command head.
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == "}" && t.ttype == "function"),
        "§7: an empty body must not emit `}}` as a function: {tokens:?}",
    );

    // §8: a `::`-bearing *quoted* word is a string literal, not a namespace.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "namespace" && covered(src, t).starts_with('"')),
        "§8: a quoted string must not be typed as a namespace: {tokens:?}",
    );

    // Nothing above may overlap — the LSP spec forbids it.
    let mut by_line: std::collections::BTreeMap<i64, Vec<&TypedToken>> =
        std::collections::BTreeMap::new();
    for t in &tokens {
        by_line.entry(t.line).or_default().push(t);
    }
    for (line, mut ts) in by_line {
        ts.sort_by_key(|t| (t.char, t.length));
        for w in ts.windows(2) {
            assert!(
                w[0].char + w[0].length <= w[1].char,
                "overlapping tokens on line {line}: {:?} then {:?}",
                w[0],
                w[1],
            );
        }
    }
}

/// §11 — `defaultLibrary` on a built-in reached through a namespace
/// prefix, or through a `namespace import`ed bare alias.
#[test]
fn test_issue_898_default_library_modifier() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    // The package is required first: without it tclsh 8.6.18 and 9.0.4 stop at
    // the import with `unknown namespace in import pattern "msgcat::*"`, and
    // nothing after it runs.
    let src = "package require msgcat\nnamespace import msgcat::*\nmc \"hi\"\n\
               puts [tcl::mathop::+ 1 2]\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let dl = modifier_bit(&modifiers(&lsp), "defaultLibrary");

    // An imported bare alias resolves to its qualified spec, so it is a built-in.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "mc" && t.ttype == "function" && t.modifiers & dl != 0),
        "an imported `msgcat::mc` alias must carry defaultLibrary: {tokens:?}",
    );
    // The namespace prefix of a built-in is part of that built-in's name.
    assert!(
        tokens.iter().any(|t| covered(src, t) == "tcl::mathop::"
            && t.ttype == "namespace"
            && t.modifiers & dl != 0),
        "a built-in's namespace prefix must carry defaultLibrary: {tokens:?}",
    );
}

/// Tk's `library/tk.tcl:289` builds its `upvar` with
/// `list` — `uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]` — and
/// declares the same cell as a plain `variable ::tk::Priv` on the next line.
/// A 3-way comparison on tclsh 9.0.4 / 8.6.16 (direct `upvar`, braced
/// `uplevel {upvar …}`, `uplevel [list upvar …]`) shows the three are
/// functionally identical, so the `[list …]` form — the one Tk uses — must
/// be recognised as a variable declaration too, not painted as a namespace
/// word.
#[test]
fn test_list_built_upvar_declares_its_local_like_the_literal_spelling() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let mods = modifiers(&lsp);
    let src = concat!(
        "proc f {disp} {\n",
        "    uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]\n",
        "    variable ::tk::Priv\n",
        "}\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let decl_bit = modifier_bit(&mods, "declaration");
    let at = |line: i64| {
        tokens
            .iter()
            .find(|t| t.line == line && covered(src, t) == "::tk::Priv")
            .unwrap_or_else(|| panic!("no `::tk::Priv` token on line {line}: {tokens:?}"))
    };
    let built = at(1);
    let literal = at(2);
    assert_eq!(
        (built.ttype.as_str(), built.modifiers & decl_bit != 0),
        ("variable", true),
        "the built `upvar`'s local is a variable declaration: {built:?}"
    );
    assert_eq!(
        (built.ttype.as_str(), built.modifiers),
        (literal.ttype.as_str(), literal.modifiers),
        "both spellings of the same declaration must agree: {tokens:?}"
    );

    // FP guard: `list` invokes nothing, so the same words in a *value* slot
    // declare nothing.  tclsh: `set x [list upvar 1 a b]; info exists b` -> 0.
    let value_src = "proc g {} {\n    set x [list upvar 1 a b]\n}\n";
    let value_uri = open_doc(&mut lsp, value_src);
    let value_tokens = typed(&mut lsp, &leg, &value_uri);
    assert!(
        !value_tokens.iter().any(|t| covered(value_src, t) == "b"
            && t.ttype == "variable"
            && t.modifiers & decl_bit != 0),
        "a value slot must not declare anything: {value_tokens:?}"
    );
}

/// A command head's grammar follows its **effective command
/// identity**, not its spelling — end-to-end through the packaged server.
///
/// tclsh-proof, byte-identical on 9.0.4 and 8.6.16: `interp alias {} myformat
/// {} format; myformat %08x 42` → `0000002a`; `rename upvar myupvar` leaves
/// `myupvar 1 outer local` a working `upvar`; `proc scan {args} {return USER};
/// scan %d 7` → `USER`.
#[test]
fn test_command_identity_drives_argument_grammar() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let mods = modifiers(&lsp);
    let decl_bit = modifier_bit(&mods, "declaration");
    let lib_bit = modifier_bit(&mods, "defaultLibrary");

    let src = concat!(
        "puts [::format {%08x} 42]\n",          // 0 — qualified spelling
        "interp alias {} myformat {} format\n", // 1
        "puts [myformat {%08x} 42]\n",          // 2 — aliased
        "rename upvar myupvar\n",               // 3
        "proc reader {} {\n",                   // 4
        "    myupvar 1 outer local\n",          // 5 — renamed
        "}\n",                                  // 6
        "proc scan {args} { return USER }\n",   // 7
        "puts [scan {%d} 7]\n",                 // 8 — shadowed
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let types_on = |line: i64| -> Vec<&str> {
        tokens
            .iter()
            .filter(|t| t.line == line)
            .map(|t| t.ttype.as_str())
            .collect()
    };

    // TP — the qualified spelling and the alias both surface printf
    // conversion sub-tokens, exactly as a bare `format` call does.
    for line in [0, 2] {
        let types = types_on(line);
        assert!(
            types.contains(&"formatSpec") && types.contains(&"formatPercent"),
            "line {line} must carry format conversion tokens: {types:?}"
        );
    }

    // TP — the renamed `upvar` keeps its keyword classification and still
    // declares its local variable.
    let head = tokens
        .iter()
        .find(|t| t.line == 5 && covered(src, t) == "myupvar")
        .unwrap_or_else(|| panic!("no `myupvar` head token: {tokens:?}"));
    assert_eq!(
        head.ttype, "keyword",
        "a renamed keyword stays a keyword: {head:?}"
    );
    let local = tokens
        .iter()
        .find(|t| t.line == 5 && covered(src, t) == "local")
        .unwrap_or_else(|| panic!("no `local` token: {tokens:?}"));
    assert_eq!(
        (local.ttype.as_str(), local.modifiers & decl_bit != 0),
        ("variable", true),
        "the renamed upvar must still declare its local: {local:?}"
    );

    // FP — a top-level `proc scan` takes the built-in's name over: no scan
    // grammar, no `defaultLibrary` modifier, and the `%d` word stays a string.
    let types = types_on(8);
    assert!(
        !types.contains(&"formatSpec") && !types.contains(&"formatPercent"),
        "a shadowed `scan` must get no conversion tokens: {types:?}"
    );
    let shadowed = tokens
        .iter()
        .find(|t| t.line == 8 && covered(src, t) == "scan")
        .unwrap_or_else(|| panic!("no shadowed `scan` head token: {tokens:?}"));
    assert_eq!(
        shadowed.modifiers & lib_bit,
        0,
        "a shadowed built-in must lose `defaultLibrary`: {shadowed:?}"
    );
    assert!(
        types.contains(&"string"),
        "the shadowed call's `{{%d}}` stays a string: {types:?}"
    );

    // TN — an unprovable binding states nothing, so the name keeps its own
    // (unknown-command) identity and its argument stays a plain string.
    let dyn_src = "interp alias {} myfmt {} $target\nputs [myfmt {%08x} 42]\n";
    let dyn_uri = open_doc(&mut lsp, dyn_src);
    let dyn_tokens = typed(&mut lsp, &leg, &dyn_uri);
    assert!(
        !dyn_tokens
            .iter()
            .any(|t| t.line == 1 && t.ttype == "formatSpec"),
        "a dynamic alias target must not confer format grammar: {dyn_tokens:?}"
    );
}
