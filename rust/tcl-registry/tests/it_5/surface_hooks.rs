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

//! The Tcl surface's hook table, through the installed set: the registrations
//! generic core modules read in place of naming `commands::tcl`
//! (`docs/design/registry/registry-split/02-behaviour-to-surfaces.md`).

use tcl_dialect::TclVersion;
use tcl_registry::native_compilation::NativeCompilationGrammar;
use tcl_registry::source_navigation::static_tcl_source_display;
use tcl_registry::{
    ClauseShapeIssue, InvocationArguments, InvocationDialect, Traits, TryClauseKind,
    TryCompletionSelector, default_registry, installed, selected_try_control_invocation,
};

#[test]
fn the_try_compiler_walks_the_try_specs_own_clause_grammar() {
    let clauses = installed()
        .try_clauses()
        .expect("the Tcl surface registers it");
    let specs: Vec<_> = installed()
        .surfaces()
        .iter()
        .flat_map(|surface| surface.specs())
        .filter(|spec| {
            spec.native_compilation
                .is_some_and(|native| native.grammar == NativeCompilationGrammar::Try)
        })
        .collect();
    assert!(!specs.is_empty());
    for spec in specs {
        assert_eq!(spec.clause_grammar, Some(clauses), "{}", spec.name);
    }
    let words = [
        "body",
        "on",
        "error",
        "{msg opts}",
        "handler",
        "finally",
        "cleanup",
    ];
    let arguments = InvocationArguments::literals(&words)
        .with_dialect(InvocationDialect::for_version(TclVersion::V8_6));
    let selected = selected_try_control_invocation(arguments, 0).expect("a closed try");
    assert_eq!(selected.body_index, 0);
    let clauses: Vec<_> = selected
        .clauses
        .iter()
        .map(|clause| (clause.kind, clause.body_index))
        .collect();
    assert_eq!(
        clauses,
        [
            (TryClauseKind::On(TryCompletionSelector::Error), 4),
            (TryClauseKind::Finally, 6)
        ]
    );
}

#[test]
fn the_if_compiler_checks_agree_with_the_if_specs_clause_check() {
    let hooks = installed()
        .conditional_clauses()
        .expect("the Tcl surface registers them");
    let spec = default_registry().get("if").expect("if");
    let check = spec.clause_shape_check.expect("if checks its clause shape");
    for words in [
        &["c"][..],
        &["c", "a"],
        &["c", "then"],
        &["c", "a", "elseif"],
        &["c", "a", "elseif", "d"],
        &["c", "a", "else"],
        &["c", "a", "else", "b"],
        &["c", "a", "else", "b", "extra"],
        &["c", "a", "b", "extra"],
    ] {
        assert_eq!(
            (hooks.shape_error)(words),
            check(InvocationArguments::literals(words)).map(ClauseShapeIssue::error),
            "{words:?}"
        );
    }
    assert_eq!(
        (hooks.compile_message)(&["c", "a", "else"], "if", &["c", "a", "else"]).as_deref(),
        Some("wrong # args: no script following \"else\" argument")
    );
    assert_eq!(
        default_registry().bpf_conditional_operands("if", &["c", "A", "elseif", "d", "B", "C"]),
        Some(vec![(Some(0), 1), (Some(3), 4), (None, 5)])
    );
    assert_eq!(
        default_registry().bpf_conditional_operands("if", &["c", "then"]),
        None
    );
}

#[test]
fn a_native_id_resolves_to_the_shipped_implementation() {
    let set = installed();
    let llength = set
        .native_hook("llength::const_fold", |table| table.const_fold)
        .expect("shipped");
    assert_eq!(llength(&["a b c"]).as_deref(), Some("3"));
    let range = set
        .native_hook("string::range::const_fold_versioned", |table| {
            table.const_fold_versioned
        })
        .expect("shipped");
    assert_eq!(
        range(&["abcdef", "1", "3"], Some(TclVersion::V9_0)).as_deref(),
        Some("bcd")
    );
    assert!(
        set.native_hook("proc::state_transitions.resolver", |table| {
            table.state_transition_resolver
        })
        .is_some()
    );
    assert!(
        set.native_hook("llength", |table| table.const_fold)
            .is_none()
    );
    assert!(
        set.native_hook("llength::const_fold", |table| table.const_fold_versioned)
            .is_none()
    );
    for ids in [
        set.native_hook_ids(|table| table.const_fold),
        set.native_hook_ids(|table| table.const_fold_versioned),
    ] {
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "an id is registered twice");
    }
}

#[test]
fn the_static_display_reads_the_registered_recipe() {
    let display = static_tcl_source_display(default_registry()).expect("authored display");
    assert_eq!(display.named_callables, ["proc", "method"]);
    assert_eq!(display.constructors, ["constructor"]);
    assert_eq!(display.regex_commands, ["regexp", "regsub"]);
    assert_eq!(display.regex_options.first(), Some(&"-nocase"));
    assert!(!display.regex_options.contains(&"--"));
}

#[test]
fn thread_commands_are_thread_primitives_in_any_surface() {
    let set = installed();
    for name in [
        "thread::send",
        "::thread::create",
        "tsv::set",
        "tpool::post",
    ] {
        assert!(
            set.traits_in_any_surface(name)
                .contains(Traits::THREAD_PRIMITIVE),
            "{name}"
        );
    }
    for name in ["puts", "coroutine", "no::such::command"] {
        assert!(
            !set.traits_in_any_surface(name)
                .contains(Traits::THREAD_PRIMITIVE),
            "{name}"
        );
    }
}
