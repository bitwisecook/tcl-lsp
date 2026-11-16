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

//! `defs_of`: assignments, calls, dynamic targets and the barrier forms the registry describes.

use super::*;

// defs_of tests

#[test]
fn defs_of_assign_const() {
    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    assert_eq!(defs_of(&stmt), vec!["x"]);
}

#[test]
fn defs_of_dynamic_target_name_has_no_static_def() {
    // `set $p 1` / `set ${p} 1` write the variable *named by* `$p`, not
    // `p` itself — an opaque place, so there is no static def.
    for name in ["$p", "${p}", "a$b", "[gen]"] {
        let stmt = Statement::AssignValue {
            span: Span::new(0, 10),
            name: name.into(),
            name_braced: false,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(
            defs_of(&stmt).len(),
            0,
            "dynamic target {name:?} must not be a static def"
        );
    }
    // A constant-keyed array element defs its own per-element variable
    // (the rename walk adds the base def alongside); a dynamic key
    // stays on the base.
    let arr = Statement::AssignValue {
        span: Span::new(0, 10),
        name: "arr(idx)".into(),
        name_braced: false,
        value: "1".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    assert_eq!(defs_of(&arr), vec!["arr(idx)"]);
    let dyn_key = Statement::AssignValue {
        span: Span::new(0, 10),
        name: "arr($i)".into(),
        name_braced: false,
        value: "1".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    assert_eq!(defs_of(&dyn_key), vec!["arr"]);
    // Braces suppress substitution: the key is the literal text `$i`.
    let braced = Statement::AssignValue {
        span: Span::new(0, 10),
        name: "arr($i)".into(),
        name_braced: true,
        value: "1".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    assert_eq!(defs_of(&braced), vec!["arr($i)"]);
}

#[test]
fn is_dynamic_write_target_classifies_names() {
    assert!(is_dynamic_write_target("$p", false));
    assert!(is_dynamic_write_target("${p}", false));
    assert!(is_dynamic_write_target("a$b", false));
    assert!(is_dynamic_write_target("[gen]", false));
    assert!(!is_dynamic_write_target("p", false));
    assert!(!is_dynamic_write_target("arr(idx)", false));
    assert!(!is_dynamic_write_target("ns::var", false));
}

#[test]
fn brace_quoted_write_target_is_not_dynamic() {
    // `set {$n} 1` names the literal variable `$n`; the braces suppress
    // every substitution, so nothing about the target is computed.
    // tclsh 9.0.4 / 8.6.14 (identical):
    //   set {$n} v; info exists {$n} → 1 ; info exists n → 0
    assert!(!is_dynamic_write_target("$n", true));
    assert!(!is_dynamic_write_target("${n}", true));
    assert!(!is_dynamic_write_target("[gen]", true));
    assert!(!is_dynamic_write_target("$a(k)", true));
    // TN control: the same spellings *unbraced* are still dynamic.
    assert!(is_dynamic_write_target("$n", false));
    assert!(is_dynamic_write_target("[gen]", false));
}

#[test]
fn defs_of_incr() {
    let stmt = Statement::Incr {
        span: Span::new(0, 10),
        name: "i".into(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    assert_eq!(defs_of(&stmt), vec!["i"]);
}

#[test]
fn defs_of_call_with_defs() {
    let stmt = Statement::Call {
        span: Span::new(0, 20),
        command: "lappend".into(),
        canonical_command: None,
        args: vec!["list".into(), "item".into()],
        defs: vec!["list".into()],
        reads: vec![],
        reads_own_defs: true,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(defs_of(&stmt), vec!["list"]);
}

#[test]
fn defs_of_call_no_defs() {
    let stmt = Statement::Call {
        span: Span::new(0, 10),
        command: "puts".into(),
        canonical_command: None,
        args: vec!["hello".into()],
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(defs_of(&stmt), [] as [std::string::String; 0]);
}

#[test]
fn defs_of_return() {
    let stmt = Statement::Return {
        tokens: None,
        span: Span::new(0, 10),
        value: Some("1".into()),
        value_word: None,
        expr: None,
        expr_base: None,
        command_binding: None,
        braced: false,
    };
    assert_eq!(defs_of(&stmt), [] as [std::string::String; 0]);
}

#[test]
fn defs_of_barrier_trace() {
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "trace".into(),
        command: "trace".into(),
        canonical_command: None,
        args: vec!["add".into(), "variable".into(), "$x".into()],
        tokens: None,
    };
    assert_eq!(defs_of(&stmt), vec!["x"]);
}

#[test]
fn defs_of_barrier_dict_for() {
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "dict for".into(),
        command: "dict::for".into(),
        canonical_command: None,
        args: vec!["k v".into(), "$d".into()],
        tokens: None,
    };
    assert_eq!(defs_of(&stmt), vec!["k", "v"]);
}

/// Loop-header barriers resolve via the registry's `loop_list_header`
/// flag — the ensemble-rewritten spellings work, and a user proc that
/// merely ENDS in `::for` does not have its first argument misread as
/// loop variables (the old suffix match did exactly that).
#[test]
fn defs_of_barrier_loop_header_via_registry() {
    let reg = CommandRegistry::build_default();
    let barrier = |command: &str| Statement::Barrier {
        span: Span::new(0, 30),
        reason: "loop".into(),
        command: command.into(),
        canonical_command: None,
        args: vec!["k v".into(), "$d".into()],
        tokens: None,
    };
    for cmd in ["::tcl::dict::for", "dict::for", "::tcl::dict::map"] {
        assert_eq!(
            defs_of_with_registry(&barrier(cmd), Some(&reg)),
            vec!["k", "v"],
            "{cmd} is a registry loop-list-header subcommand"
        );
    }
    assert_eq!(
        defs_of_with_registry(&barrier("::tcl::array::for"), Some(&reg)),
        vec!["k", "v"],
        "array for shares the loop-list-header shape"
    );
    assert!(
        !defs_of_with_registry(&barrier("my::for"), Some(&reg))
            .iter()
            .any(|d| d == "k" || d == "v"),
        "a user proc ending in ::for must not be misread as a loop header"
    );
}

#[test]
fn defs_of_barrier_loop_header_uses_tcl_list_grammar() {
    let reg = CommandRegistry::build_default();
    let barrier = |var_list: &str| Statement::Barrier {
        span: Span::new(0, 30),
        reason: "loop".into(),
        command: "::tcl::dict::for".into(),
        canonical_command: None,
        args: vec![var_list.into(), "$d".into()],
        tokens: None,
    };

    assert_eq!(
        defs_of_with_registry(
            &barrier(r#"{one name} "two name" three\ name plain"#),
            Some(&reg),
        ),
        vec!["one name", "two name", "three name", "plain"],
    );
    assert_eq!(
        defs_of_with_registry(&barrier("{unterminated"), Some(&reg)).len(),
        0,
        "a malformed var-list fails before defining loop variables"
    );
}

/// `tcltest::test` body indices come from the spec's arg-role resolver
/// (option-keyed `-setup`/`-body`/`-cleanup` values plus the legacy
/// positional body) via the generic `ArgRole::Body` walk, not a
/// `command == "test"` special case.
#[test]
fn structural_body_indices_tcltest_via_registry() {
    use tcl_registry::ArgRole;
    // This is a role-metadata query, not proof of a loaded source command.
    let mut reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let unprovided = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    reg.insert_ambient_package("tcltest", "2.5.8");
    // Option form: -setup and -body values are Body roles; -result's is
    // not. Both the qualified and the exported bare spelling resolve.
    let option_form = [
        "n",
        "d",
        "-setup",
        "{set x 1}",
        "-body",
        "{incr x}",
        "-result",
        "2",
    ];
    let got = reg.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body);
    assert_eq!(got, vec![3, 5], "option-form bodies via the option model");
    assert_eq!(
        unprovided.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body),
        vec![3, 5],
        "package assistance advertises roles without proving a loaded handler"
    );
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "tcltest::test n d -body {set package_only 1}",
        &unprovided,
        false,
    );
    assert!(
        unit.top_level.ssa.blocks.values().all(|block| block
            .statements
            .iter()
            .all(|statement| statement.defs.is_empty())),
        "unprovided package role inventory cannot donate physical definitions"
    );
    // The bare `test` spelling is NOT a registry key: it resolves only
    // through the lowering's namespace-import canonicalisation (which
    // stamps `canonical_command` — threaded into the SSA lookups). A
    // user proc that merely happens to be called `test` must not pick
    // up tcltest body semantics, which the old string match caused.
    assert_eq!(
        reg.arg_indices_for_role("test", &option_form, ArgRole::Body)
            .len(),
        0
    );
    // Legacy positional form: body is the penultimate argument.
    let legacy = ["n", "d", "{incr x}", "1"];
    let got = reg.arg_indices_for_role("tcltest::test", &legacy, ArgRole::Body);
    assert_eq!(got, vec![2], "legacy positional body");
    // Option form with NO body options marks nothing (and must not fall
    // back to the positional branch: `-result` is not a body).
    let no_body = ["n", "d", "-result", "2"];
    let got = reg.arg_indices_for_role("tcltest::test", &no_body, ArgRole::Body);
    assert_eq!(got.len(), 0, "no body options → no body roles: {got:?}");
    // The generic structural walk consumes the same roles (braced-token
    // filtering applies on real token streams; `None` tokens filter all,
    // so only the empty expectation is assertable here).
    let no_body_args: Vec<String> = no_body.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        structural_body_indices("tcltest::test", &no_body_args, None, &reg).len(),
        0
    );
}

/// `trace add variable` defs route through the registry's
/// `ArgRole::VarWrite` query rather than a string match.
#[test]
fn defs_of_barrier_trace_via_registry() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "trace".into(),
        command: "trace".into(),
        canonical_command: None,
        args: vec!["add".into(), "variable".into(), "$x".into()],
        tokens: None,
    };
    assert_eq!(defs_of_with_registry(&stmt, Some(&reg)), vec!["x"]);
}

/// `trace add execution` does NOT define a variable — the
/// command name being traced is not a `VarWrite` target.
#[test]
fn defs_of_barrier_trace_add_execution_no_def() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "trace".into(),
        command: "trace".into(),
        canonical_command: None,
        args: vec!["add".into(), "execution".into(), "foo".into()],
        tokens: None,
    };
    assert_eq!(
        defs_of_with_registry(&stmt, Some(&reg)),
        [] as [std::string::String; 0]
    );
}

/// `global x y z` produces NO defs from the registry path
/// (`var_scoping` handles the per-arg list).  Without the
/// `CREATES_DYNAMIC_BARRIER` skip, the role-driven walk would
/// only mark `x` (partial defs) and miss `y` / `z`.
#[test]
fn defs_of_barrier_global_vararg_no_partial_defs() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "global".into(),
        command: "global".into(),
        canonical_command: None,
        args: vec!["x".into(), "y".into(), "z".into()],
        tokens: None,
    };
    assert_eq!(
        defs_of_with_registry(&stmt, Some(&reg)),
        [] as [std::string::String; 0]
    );
}

/// `variable a b c` — same vararg-list shape as
/// `global`, same skip.
#[test]
fn defs_of_barrier_variable_vararg_no_partial_defs() {
    let reg = CommandRegistry::build_default();
    let stmt = Statement::Barrier {
        span: Span::new(0, 30),
        reason: "variable".into(),
        command: "variable".into(),
        canonical_command: None,
        args: vec!["a".into(), "b".into(), "c".into()],
        tokens: None,
    };
    assert_eq!(
        defs_of_with_registry(&stmt, Some(&reg)),
        [] as [std::string::String; 0]
    );
}
