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

//! `TclOO`, snit and itcl: body members, declared instance variables, call sites and references.

use super::*;

#[test]
fn test_tcloo_body_variable_declares_every_name() {
    // `variable a b c` in a TclOO class body declares every name as
    // an instance variable, not just the first.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "oo::class create Foo {\n    variable width height depth\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    for name in ["width", "height", "depth"] {
        let t = tokens
            .iter()
            .find(|t| covered(src, t) == name)
            .unwrap_or_else(|| panic!("no token covers {name:?}: {tokens:?}"));
        assert_eq!(
            t.ttype, "variable",
            "`{name}` must be a variable: {tokens:?}"
        );
    }
}

#[test]
fn test_snit_type_body_members_highlight() {
    // snit definition bodies are registry data (definition_body grammar), so
    // their members recurse + highlight with no snit-specific walker code.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "snit::type Dog {\n\
               \x20   variable barks\n\
               \x20   method bark {volume} { return $barks }\n\
               \x20   typemethod count {} { return 1 }\n\
               }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    // `barks` is a declared variable; `volume` is a method *parameter* and
    // carries the standard LSP `parameter` type.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "barks" && t.ttype == "variable"),
        "snit `barks` must be a variable: {tokens:?}",
    );
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "volume" && t.ttype == "parameter"),
        "snit `volume` must be a parameter: {tokens:?}",
    );
    // The type name and the member names are `class` / `method`, not strings.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "Dog" && t.ttype == "class"),
        "snit `Dog` must be a class: {tokens:?}",
    );
    for m in ["bark", "count"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == m && t.ttype == "method"),
            "snit `{m}` must be a method: {tokens:?}",
        );
    }
    // `typemethod` is a member keyword; the method body's `return` is recursed.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "typemethod" && t.ttype == "keyword"),
        "`typemethod` must be a keyword: {tokens:?}",
    );
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "return" && t.ttype == "keyword"),
        "a snit method body must be recursed: {tokens:?}",
    );
}

#[test]
fn test_itcl_class_body_members_highlight() {
    // [incr Tcl] class bodies are registry data (definition_body grammar) too,
    // including the public/protected/private access-modifier wrappers.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "itcl::class Dog {\n\
               \x20   inherit Animal\n\
               \x20   variable barks\n\
               \x20   public method bark {volume} { set barks $volume }\n\
               \x20   private variable secret 0\n\
               }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    // Declared variables (incl. the access-modifier-wrapped ones) are variables.
    for name in ["barks", "secret"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == name && t.ttype == "variable"),
            "itcl `{name}` must be a variable: {tokens:?}",
        );
    }
    // A method's *parameter* carries the standard LSP `parameter` type, so a
    // theme can tell an argument from an ordinary local.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "volume" && t.ttype == "parameter"),
        "itcl `volume` must be a parameter: {tokens:?}",
    );
    // The class name is a `class`, not a bare string.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "Dog" && t.ttype == "class"),
        "itcl `Dog` must be a class: {tokens:?}",
    );
    // The method's declared name is a `method`, not a bare string.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "bark" && t.ttype == "method"),
        "itcl `bark` must be a method: {tokens:?}",
    );
    // The access modifier and the inner wrapped keyword are both keywords.
    for kw in ["inherit", "public", "method", "private"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == kw && t.ttype == "keyword"),
            "itcl `{kw}` must be a keyword: {tokens:?}",
        );
    }
    // The wrapped method body is recursed (`set` is a command there).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "set" && t.ttype == "function"),
        "an itcl method body must be recursed: {tokens:?}",
    );
}

/// `TclOO`, end to end: declarations, call sites, references, the one-liner
/// definer form, and `property`.
///
/// A method is the same entity at its declaration and at its call site, and a
/// `superclass` / `mixin` argument names a class rather than being a free
/// string — the registry's definition-body grammar carries both facts, so no
/// command name appears in the walker.
#[test]
fn test_tcloo_members_references_and_call_sites() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "oo::class create Base {\n",
        "    superclass Shape\n",
        "    mixin Loggable\n",
        "    variable count\n",
        "    constructor {n} { set count $n }\n",
        "    method bump {by} { my Recalc }\n",
        "    method Recalc {} { return $count }\n",
        "    forward len llength\n",
        "    export bump\n",
        "    unexport Recalc\n",
        "}\n",
        "oo::define Base method inline {z} { return $z }\n",
        "set o [Base new 1]\n",
        "$o bump 2\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    // Declarations.
    assert!(has("Base", "class"), "class name: {tokens:?}");
    assert!(has("bump", "method"), "method decl: {tokens:?}");
    assert!(has("by", "parameter"), "method param: {tokens:?}");
    assert!(has("n", "parameter"), "constructor param: {tokens:?}");
    assert!(has("count", "variable"), "declared variable: {tokens:?}");

    // References: a `superclass` / `mixin` argument names a *class*; an
    // `export` / `unexport` argument names a *method*.
    assert!(has("Shape", "class"), "superclass ref: {tokens:?}");
    assert!(has("Loggable", "class"), "mixin ref: {tokens:?}");
    assert!(has("Recalc", "method"), "unexport ref: {tokens:?}");
    // `forward NAME cmd` declares NAME as a method.
    assert!(has("len", "method"), "forward name: {tokens:?}");

    // The one-liner definer form gets the same treatment as the body form —
    // name, parameter and body — not just its keyword.
    assert!(has("inline", "method"), "one-liner method name: {tokens:?}");
    assert!(has("z", "parameter"), "one-liner method param: {tokens:?}");

    // Call sites are the same entity as the declaration.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "bump" && t.ttype == "method"),
        "`$o bump` call site must be a method: {tokens:?}",
    );
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == "Recalc" && t.ttype == "function"),
        "`my Recalc` must be a method, not a function: {tokens:?}",
    );
}
