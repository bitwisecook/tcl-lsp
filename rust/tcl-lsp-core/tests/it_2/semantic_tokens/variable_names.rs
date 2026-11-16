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

//! Words that name variables: declarations, loop variables, parameters, `upvar` locals and brace-quoted or list-built names.

use super::*;

// The other multi-name variable-declaring commands, whose trailing names the
// registry's single leading `VarWrite` role leaves as strings.  The analyser
// tracks them through its lowering hooks; the highlighting must agree.

#[test]
fn global_highlights_every_name() {
    // `global a b c` declares every name — not just the first.
    let src = "proc p {} {\n    global alpha beta gamma\n}\n";
    for name in ["alpha", "beta", "gamma"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in `global` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn namespace_variable_pairs_highlight_names_not_values() {
    // Namespace-level `variable name value name value` — the names (even arg
    // positions) are variables; the interleaved values keep their own kind.
    let src = "namespace eval ns {\n    variable alpha 1 beta 2\n}\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "alpha").as_deref(),
        Some("variable"),
        "{:?}",
        decode(src, "tcl8.6"),
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "beta").as_deref(),
        Some("variable"),
        "second name must also be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
    // The value `1` stays a number, not a variable.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "1").as_deref(),
        Some("number"),
        "an interleaved value must not become a variable: {:?}",
        decode(src, "tcl8.6"),
    );
}

// Loop-variable highlighting — `foreach` / `lmap` / `dict for` bind their
// iteration variables, which must read as variable declarations, whether a
// single bareword or the elements of a braced list.

#[test]
fn foreach_single_loop_var_is_variable() {
    let src = "foreach item $lst { puts $item }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "item").as_deref(),
        Some("variable"),
        "the foreach loop variable must be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn foreach_braced_loop_vars_are_variables() {
    let src = "foreach {key val} $lst { puts $key }\n";
    for name in ["key", "val"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in the braced foreach varlist must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn foreach_multi_pair_loop_vars_are_variables() {
    // `foreach a $l1 b $l2 …` — the variable spec sits at every other word.
    let src = "foreach aa $l1 bb $l2 { puts $aa }\n";
    for name in ["aa", "bb"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in a multi-pair foreach must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn lmap_and_dict_for_loop_vars_are_variables() {
    assert_eq!(
        kind_of_word("lmap elem $xs { expr {$elem} }\n", "tcl8.6", "elem").as_deref(),
        Some("variable"),
        "lmap loop variable must be a variable",
    );
    let df = "dict for {dkey dval} $d { puts $dkey }\n";
    for name in ["dkey", "dval"] {
        assert_eq!(
            kind_of_word(df, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in `dict for` must be a variable: {:?}",
            decode(df, "tcl8.6"),
        );
    }
}

#[test]
fn non_loop_command_first_arg_stays_string() {
    // A user command that merely resembles a loop must not have its first
    // bareword argument reclassified as a loop variable.
    let src = "myproc item $lst { puts hi }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "item").as_deref(),
        Some("string"),
        "a non-loop command's bareword arg must stay a string: {:?}",
        decode(src, "tcl8.6"),
    );
}

// Parameter-list highlighting — proc / method / constructor / apply-lambda
// parameters, and the registry `LoopVarList` role for `dict map` (peer of
// `dict for`).  A *parameter* carries the standard LSP `parameter` type,
// so a theme can tell an argument from an ordinary local; a *loop*
// variable stays a plain variable declaration.

#[test]
fn proc_parameters_are_parameters() {
    let src = "proc greet {name age} { return $name }\n";
    for name in ["name", "age"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "proc parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn proc_parameter_default_pair_splits_name_and_value() {
    // `{b 5}` — `b` is the parameter name (variable), `5` the default (number).
    let src = "proc p {a {b 5} args} { return $a }\n";
    for name in ["a", "b", "args"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "`{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    assert_eq!(
        kind_of_word(src, "tcl8.6", "5").as_deref(),
        Some("number"),
        "the default value must stay a number: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn apply_lambda_parameters_are_parameters() {
    let src = "apply {{alpha beta} { return $alpha }} 1 2\n";
    for name in ["alpha", "beta"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "apply lambda parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn tcloo_method_and_constructor_parameters_are_parameters() {
    let src = "oo::class create C {\n    method greet {who} { return $who }\n    constructor {aa bb} { set x $aa }\n}\n";
    for name in ["who", "aa", "bb"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "TclOO parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn snit_macro_arglist_parameters_are_parameters() {
    // `snit::macro name arglist body` — the arglist word (registry arg-role
    // `ParamList`) holds the macro's formal parameters, so its names read as
    // variables and the body highlights, exactly like a proc.
    let src = "snit::macro mymac {alpha beta} { return $alpha }\n";
    for name in ["alpha", "beta"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "snit::macro parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn dict_map_loop_vars_are_variables() {
    // Peer of `dict for`: `dict map {k v}` binds its loop variables too.
    let src = "dict map {mk mv} $d { set mk $mv }\n";
    for name in ["mk", "mv"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in `dict map` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

// By-reference local variable highlighting — `upvar` / `namespace upvar`
// locals and `dict update` var names read as declarations; the "other" names,
// the level word, and dict keys stay strings.

#[test]
fn upvar_local_names_are_variables() {
    let src = "upvar 1 othervar localvar\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "localvar").as_deref(),
        Some("variable"),
        "the upvar local must be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "othervar").as_deref(),
        Some("string"),
        "the other-frame name must stay a string: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn upvar_without_level_and_multi_pair() {
    // No level word: the local is the second word; multi-pair marks each local.
    let src = "upvar aone bone atwo btwo\n";
    for local in ["bone", "btwo"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", local).as_deref(),
            Some("variable"),
            "`{local}` must be an upvar local: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    for other in ["aone", "atwo"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", other).as_deref(),
            Some("string"),
            "`{other}` must stay a string: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn namespace_upvar_locals_are_variables() {
    let src = "namespace upvar ::ns othervar localvar\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "localvar").as_deref(),
        Some("variable"),
        "{:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn dict_update_var_names_are_variables() {
    let src = "dict update mydict keyone varone keytwo vartwo { set varone 1 }\n";
    for v in ["varone", "vartwo"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", v).as_deref(),
            Some("variable"),
            "`{v}` must be a dict-update variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    // The keys are not variables.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "keyone").as_deref(),
        Some("string"),
        "a dict-update key must stay a string: {:?}",
        decode(src, "tcl8.6"),
    );
}

// A brace-quoted variable-name word is a variable, not a string.
//
// tclsh 9.0.4 / 8.6.14, byte-identical: `set {$n} v; info exists {$n}` → 1
// while `info exists n` → 0, so `{$n}` names a real variable — and quoting is
// the *only* way that variable can ever be written.  Painting the word as a
// plain `string` hides the declaration and invites the reader to see the `$n`
// inside as a substitution.

#[test]
fn brace_quoted_variable_name_word_paints_as_a_variable() {
    let src = "proc f {} {\n    set {$n} 1\n    set n 2\n    puts [set {$n}]\n}\n";
    let toks = decode(src, "tcl8.6");
    let declaration_bit = legend_token_modifiers()
        .iter()
        .position(|m| *m == "declaration")
        .map(|i| 1u32 << i)
        .expect("legend carries `declaration`");

    // The write target on line 1 — the whole `{$n}` word, tagged exactly like
    // the plain `n` write on line 2.
    let write = toks
        .iter()
        .find(|t| t.line == 1 && t.character == 8)
        .expect("a token at the `{$n}` write word");
    assert_eq!(write.ttype, "variable", "{write:?}");
    assert_eq!(write.length, 4, "the whole `{{$n}}` word: {write:?}");
    assert!(
        write.mods & declaration_bit != 0,
        "a write target is a declaration: {write:?}"
    );
    let plain_write = toks
        .iter()
        .find(|t| t.line == 2 && t.character == 8)
        .expect("a token at the `n` write word");
    assert_eq!(
        (
            plain_write.ttype.as_str(),
            plain_write.mods & declaration_bit != 0
        ),
        (write.ttype.as_str(), true),
        "the braced and plain write targets must be tagged alike"
    );

    // The one-arg read on line 3 — a variable *reference*, not a declaration.
    let read = toks
        .iter()
        .find(|t| t.line == 3 && t.character == 14)
        .expect("a token at the `{$n}` read word");
    assert_eq!(read.ttype, "variable", "{read:?}");
    assert_eq!(read.length, 4, "{read:?}");

    // TN control: a brace-quoted word that is *not* in a variable-name role
    // stays a string — the retag keys on the registry role, not on braces.
    assert_eq!(
        kind_of_word("puts {$n}\n", "tcl8.6", "{$n}").as_deref(),
        Some("string"),
        "`puts`'s argument is data, not a variable name"
    );
}

// A script argument built with `list` highlights like the
// command it provably is.
//
// Tk's `library/tk.tcl:289` is `uplevel #0 [list upvar #0 ::tk::Priv.$disp
// ::tk::Priv]`, and the very next line declares the same cell as `variable
// ::tk::Priv`.  A 3-way comparison on tclsh 9.0.4 and 8.6.16 — direct `upvar
// #0 g l`, braced `uplevel #0 {upvar #0 g l}`, and `uplevel #0 [list upvar #0
// g l]` — shows the three are functionally identical, so the `[list …]`
// form Tk uses must not be painted as a plain namespace word.

/// The `declaration` modifier bit, resolved from the legend.
fn declaration_bit() -> u32 {
    let pos = legend_token_modifiers()
        .iter()
        .position(|&m| m == "declaration")
        .expect("legend declares `declaration`");
    1 << pos
}

#[test]
fn tp_a_list_built_upvar_declares_its_local_like_the_literal_spelling() {
    let src = "proc f {disp} {\n    \
uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]\n    \
variable ::tk::Priv\n\
}\n";
    let toks = decode(src, "tcl8.6");
    let built = toks
        .iter()
        .find(|t| t.line == 1 && tok_text(src, t) == "::tk::Priv")
        .expect("the built `upvar`'s local name word is tokenised");
    let literal = toks
        .iter()
        .find(|t| t.line == 2 && tok_text(src, t) == "::tk::Priv")
        .expect("the `variable ::tk::Priv` word is tokenised");
    assert_eq!(
        (built.ttype.as_str(), built.mods & declaration_bit() != 0),
        ("variable", true),
        "the built `upvar`'s local is a variable declaration, not a namespace"
    );
    assert_eq!(
        (built.ttype.as_str(), built.mods),
        (literal.ttype.as_str(), literal.mods),
        "the two spellings of the same declaration must agree"
    );
}

#[test]
fn fp_a_list_in_a_value_slot_is_still_inert_data() {
    // `list` invokes nothing — `set x [list upvar 1 a b]` merely builds a
    // four-element list.  tclsh 9.0.4 / 8.6.16: `set x [list upvar 1 a b];
    // puts $x` prints `upvar 1 a b`, and no variable `b` is created
    // (`info exists b` -> 0).  Painting `b` as a declaration would be a lie.
    let src = "proc f {} {\n    set x [list upvar 1 a b]\n}\n";
    let toks = decode(src, "tcl8.6");
    let b = toks
        .iter()
        .find(|t| t.line == 1 && tok_text(src, t) == "b")
        .expect("the `b` word is tokenised");
    assert_ne!(
        (b.ttype.as_str(), b.mods & declaration_bit() != 0),
        ("variable", true),
        "a value slot defers nothing, so nothing is declared: {b:?}"
    );
}

#[test]
fn tn_a_dynamic_list_head_keeps_todays_behaviour() {
    // `[list $cb #0 a b]` names no statically known command, so nothing is
    // retagged — the abstain-on-doubt direction.
    let src = "proc f {cb} {\n    uplevel #0 [list $cb #0 a b]\n}\n";
    let toks = decode(src, "tcl8.6");
    let b = toks
        .iter()
        .find(|t| t.line == 1 && tok_text(src, t) == "b")
        .expect("the `b` word is tokenised");
    assert_ne!(
        (b.ttype.as_str(), b.mods & declaration_bit() != 0),
        ("variable", true),
        "an unresolvable head must not produce a declaration: {b:?}"
    );
}
