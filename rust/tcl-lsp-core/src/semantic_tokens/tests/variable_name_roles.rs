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

//! Argument words that name a variable or a command: writes, reads, array elements and procedure-declared roles.

use super::*;

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
