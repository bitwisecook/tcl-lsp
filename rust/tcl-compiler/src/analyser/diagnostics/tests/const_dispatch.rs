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

//! Command heads that evaluate to a constant: the references they record and the shapes that abstain.

use super::*;

// Positioned computed heads retain their original written syntax. Navigation
// uses the separately retained current lookup slot, so deleted commands do not
// acquire a target from historical declarations or value contributors.

fn const_dispatch_target(src: &str) -> Option<(String, Option<String>)> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    r.command_invocations
        .into_iter()
        .find(|i| i.range.start() == dispatch && i.indirect)
        .and_then(|i| {
            let reference = i.resolved_command_reference.as_ref()?;
            assert_eq!(i.name, "${cmd}", "preserve the authored variable head");
            let reporting = reference.slot()?;
            Some((
                reporting.strip_prefix("::").unwrap_or(reporting).to_owned(),
                i.resolved_qualified_name,
            ))
        })
}

#[test]
fn const_dispatch_evaluated_literal_command_names_preserve_navigation() {
    for (name, body, slot) in [
        (
            "$target",
            "proc {$target} {} {}; set cmd {$target}; $cmd",
            "::$target",
        ),
        (
            "has space",
            "proc {has space} {} {}; set cmd {has space}; $cmd",
            "::has space",
        ),
        ("", "proc {} {} {}; set cmd {}; $cmd", "::"),
    ] {
        assert_eq!(
            const_dispatch_target(body),
            Some((name.to_owned(), Some(slot.to_owned()))),
            "{body}"
        );
    }
}

#[test]
fn const_dispatch_tp_issue_1009_proc_deleted_no_reestablishment_does_not_resolve() {
    let src = "proc target {} {}\nrename target {}\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        None,
        "a $cmd dispatch to a deleted proc with no re-establishment must not resolve"
    );
}

#[test]
fn const_dispatch_fp_issue_1009_proc_reestablished_after_deletion_resolves() {
    let src = "proc target {} {}\nrename target {}\nproc target {} {}\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("target".to_owned(), Some("::target".to_owned())))
    );
}

#[test]
fn const_dispatch_tn_issue_1009_proc_no_deletion_still_resolves() {
    let src = "proc target {} {}\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("target".to_owned(), Some("::target".to_owned())))
    );
}

#[test]
fn const_dispatch_tp_issue_1009_class_deleted_no_reestablishment_does_not_resolve() {
    let src = "oo::class create Target\nrename Target {}\nset cmd Target\n$cmd\n";
    assert_eq!(const_dispatch_target(src), None);
}

#[test]
fn const_dispatch_tp_issue_1009_rename_target_deleted_no_reestablishment_does_not_resolve() {
    let src = "proc helper {} {}\nrename helper ha2\nrename ha2 {}\nset cmd ha2\n$cmd\n";
    assert_eq!(const_dispatch_target(src), None);
}

#[test]
fn const_dispatch_fp_issue_1009_rename_target_no_deletion_still_resolves() {
    let src = "proc helper {} {}\nrename helper ha2\nset cmd ha2\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("ha2".to_owned(), Some("::ha2".to_owned())))
    );
}

#[test]
fn const_dispatch_tp_issue_1009_alias_deleted_no_reestablishment_does_not_resolve() {
    let src = "proc target {} {}\ninterp alias {} short {} target\ninterp alias {} short {}\nset cmd short\n$cmd\n";
    assert_eq!(const_dispatch_target(src), None);
}

#[test]
fn const_dispatch_fp_issue_1009_alias_no_deletion_still_resolves() {
    let src = "proc target {} {}\ninterp alias {} short {} target\nset cmd short\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("short".to_owned(), Some("::short".to_owned())))
    );
}

#[test]
fn const_dispatch_fp_issue_1009_deletion_inside_never_triggered_body_still_resolves() {
    // The same conditional-body guard applies — a deletion
    // recorded inside a proc that's never called must not disqualify the
    // dispatch (tclsh: calling nothing that invokes `maybeDelete` leaves
    // `target` live).
    let src = "proc target {} {}\nproc maybeDelete {} { rename target {} }\nset cmd target\n$cmd\n";
    assert_eq!(
        const_dispatch_target(src),
        Some(("target".to_owned(), Some("::target".to_owned())))
    );
}

#[test]
fn const_cmd_head_records_a_reference_to_the_dispatched_proc_m7() {
    let mut a = Analyser::new();
    let src = "proc target {} {}\nset cmd target\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.find("$cmd").unwrap()).unwrap();
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.range.start() == dispatch && i.indirect);
    assert!(
        inv.is_some_and(|i| i.resolved_qualified_name.as_deref() == Some("::target")
            && i.resolved_command_reference
                .as_ref()
                .is_some_and(|reference| reference.slot() == Some("::target")
                    && reference.definition().is_some())
            && i.indirect),
        "const $cmd dispatch must reference ::target (indirect): {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start(), i.indirect))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn const_cmd_head_resolves_in_the_dispatch_namespace_m7() {
    let mut a = Analyser::new();
    let src = "namespace eval ns { proc target {} {} }\n\
               namespace eval ns { set cmd target; $cmd }\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::ns::target")),
        "{:?}",
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .map(|i| (&i.name, &i.resolved_qualified_name))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn const_cmd_head_abstains_on_unknown_or_dynamic_values_m7() {
    // Preserve the original indirect invocation without inventing a definition.
    let mut a = Analyser::new();
    let r = a.analyse("set cmd nosuchcmd\n$cmd\n", "tcl8.6");
    assert!(
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .all(|i| { i.resolved_command_reference.is_none() })
    );
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("proc target {} {}\nset cmd [pick]\n$cmd\n", "tcl8.6");
    assert!(
        r2.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .all(|i| { i.resolved_command_reference.is_none() })
    );
    // Frozen interpolation can resolve the real slot without making its
    // contributing text an editable command literal.
    let mut a4 = Analyser::new();
    let r4 = a4.analyse(
        "proc target {} {}\nset suffix arget\nset cmd t$suffix\n$cmd\n",
        "tcl8.6",
    );
    assert!(
        r4.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .any(|i| {
                i.resolved_command_reference
                    .as_ref()
                    .is_some_and(|reference| {
                        reference.slot() == Some("::target") && reference.definition().is_some()
                    })
            })
    );
    // An actual builtin slot has no source-owned procedure definition.
    let mut a3 = Analyser::new();
    let r3 = a3.analyse("set cmd puts\n$cmd hi\n", "tcl8.6");
    assert!(
        r3.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .any(|i| {
                i.resolved_command_reference
                    .as_ref()
                    .is_some_and(|reference| {
                        reference.slot() == Some("::puts") && reference.definition().is_none()
                    })
            })
    );
}

#[test]
fn const_cmd_head_resolves_through_a_pure_copy_chain_m7() {
    // `set x target; set cmd $x; $cmd` dispatches ::target — the copy
    // chain preserves provenance, so the *ultimate* literal (`target` in
    // `set x target`) is the writable reference: renaming must rewrite that
    // literal, keeping the dispatch alive.
    let mut a = Analyser::new();
    let src = "proc target {} {}\nset x target\nset cmd $x\n$cmd\n";
    let r = a.analyse(src, "tcl8.6");
    let dispatch = u32::try_from(src.find("$cmd").unwrap()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.indirect
            && i.range.start() == dispatch
            && i.resolved_qualified_name.as_deref() == Some("::target")
            && i.rename_safe),
        "copy-chained const dispatch must reference ::target: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start(), i.indirect))
            .collect::<Vec<_>>(),
    );
    let lit = u32::try_from(src.find("set x target").unwrap() + "set x ".len()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| !i.indirect
            && i.range.start() == lit
            && i.resolved_qualified_name.as_deref() == Some("::target")),
        "the ultimate literal must be the writable reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start(), i.indirect))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn branch_joined_const_dispatch_records_every_may_target_945() {
    // Branch-joined dispatch: `set cmd foo; if {$runtime} { set cmd bar };
    // $cmd` dispatches ::foo when the branch is not taken and ::bar when
    // it is (tclsh 9.0.4: runtime=0 → foo, runtime=1 → bar).  The value
    // at the dispatch point is the SSA φ-join {foo, bar} — never the
    // lexically last write — so BOTH procs are referenced, each with its
    // own writable literal.
    let mut a = Analyser::new();
    // `runtime` is bound first: unset, its read raises (tclsh 8.4.20 to
    // 9.1.0: can't read "runtime") and the dispatch is never reached.
    let src = "proc foo {} {return foo}\nproc bar {} {return bar}\n\
               set runtime [gets stdin]\nset cmd foo\nif {$runtime} {\n    set cmd bar\n}\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    let heads: Vec<&str> = r
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads.contains(&"::foo") && heads.contains(&"::bar"),
        "both may-targets must be referenced at the join: {heads:?}",
    );
    // Each contributing literal is a writable reference to its own target.
    let foo_lit = u32::try_from(src.find("set cmd foo").unwrap() + "set cmd ".len()).unwrap();
    let bar_lit = u32::try_from(src.find("set cmd bar").unwrap() + "set cmd ".len()).unwrap();
    let lit_of = |target: &str| {
        r.command_invocations
            .iter()
            .find(|i| !i.indirect && i.resolved_qualified_name.as_deref() == Some(target))
            .map(|i| i.range.start())
    };
    assert_eq!(lit_of("::foo"), Some(foo_lit), "foo literal is writable");
    assert_eq!(lit_of("::bar"), Some(bar_lit), "bar literal is writable");
}

#[test]
fn switch_and_loop_joined_const_dispatch_records_every_may_target_945() {
    // The same φ-join soundness across `switch` arms…
    let mut a = Analyser::new();
    let src = "proc left {} {}\nproc right {} {}\nset x [gets stdin]\nset cmd left\n\
               switch -- $x {\n  a { set cmd right }\n  default {}\n}\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    let heads: Vec<&str> = r
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads.contains(&"::left") && heads.contains(&"::right"),
        "switch join must keep both may-targets: {heads:?}",
    );
    // …and across a `while` back-edge (the loop-carried φ folds the
    // pre-loop literal with the in-loop reassignment).
    let mut a2 = Analyser::new();
    let src2 = "proc first {} {}\nproc next {} {}\nset go [gets stdin]\nset cmd first\n\
                while {$go} {\n    $cmd\n    set cmd next\n}\n";
    let r2 = a2.analyse(src2, "tcl");
    let dispatch2 = u32::try_from(src2.find("$cmd").unwrap()).unwrap();
    let heads2: Vec<&str> = r2
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch2)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads2.contains(&"::first") && heads2.contains(&"::next"),
        "loop-carried join must keep both may-targets: {heads2:?}",
    );
}

#[test]
fn catch_and_try_body_writes_join_never_last_write_945() {
    // Two shapes, because `catch` is only inlined where its result variable
    // could have a slot — inside a procedure (#2207).
    //
    // At the top level it stays the opaque call with summarised defs, so the
    // provenance walk still sees a non-literal defining statement and
    // **abstains**: no indirect reference at all, and never a lexical map's
    // answer (the body's `set cmd risky` presented as the unconditional
    // value).
    let mut a = Analyser::new();
    let src = "proc safe {} {}\nproc risky {} {}\nset cmd safe\n\
               catch {\n    set cmd risky\n}\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.indirect && i.range.start() == dispatch),
        "an opaque top-level catch write must abstain, not settle to a single target",
    );

    // Inside a procedure the body is real CFG blocks, so the φ-join keeps
    // BOTH may-targets — the body may fail before the write (`safe` survives)
    // or complete (`risky`). Same contract, met by precision rather than by
    // abstaining.
    //
    // The join is only sound because `lower_catch` records an exception edge
    // from the *pre-catch* block: the body can fail at its first command.
    // Without that edge this settles to `risky` alone, which is exactly the
    // false single-target this test's name warns about.
    let mut in_proc_analyser = Analyser::new();
    let proc_src = "proc safe {} {}\nproc risky {} {}\nproc p {} {\n\
                set cmd safe\ncatch {\n    set cmd risky\n}\n$cmd\n}\n";
    let proc_result = in_proc_analyser.analyse(proc_src, "tcl");
    let proc_dispatch = u32::try_from(proc_src.rfind("$cmd").unwrap()).unwrap();
    let proc_heads: Vec<&str> = proc_result
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == proc_dispatch)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        proc_heads.contains(&"::safe") && proc_heads.contains(&"::risky"),
        "an inlined catch body keeps both may-targets, never one: {proc_heads:?}",
    );

    // A `try` body, by contrast, inlines with real CFG structure in
    // analysis builds, so its φ-join keeps BOTH may-targets — the body
    // may error before the write (`safe` survives) or complete
    // (`risky`) — and each side keeps its writable literal.
    let mut a2 = Analyser::new();
    let src2 = "proc safe {} {}\nproc risky {} {}\nset cmd safe\n\
                try {\n    set cmd risky\n} on error {} {}\n$cmd\n";
    let r2 = a2.analyse(src2, "tcl");
    let dispatch2 = u32::try_from(src2.rfind("$cmd").unwrap()).unwrap();
    let heads2: Vec<&str> = r2
        .command_invocations
        .iter()
        .filter(|i| i.indirect && i.range.start() == dispatch2)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads2.contains(&"::safe") && heads2.contains(&"::risky"),
        "try join must keep both may-targets: {heads2:?}",
    );
}

#[test]
fn unprovable_const_dispatch_shapes_abstain_945() {
    // A proc parameter: the value flows in from the caller — abstain.
    let mut a = Analyser::new();
    let r = a.analyse("proc run {cmd} {\n    $cmd\n}\n", "tcl");
    assert!(
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .all(|i| { i.resolved_command_reference.is_none() })
    );
    // A write reachable through `upvar`: the alias write is not a local
    // literal definition — abstain.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "proc target {} {}\nproc setit {v} {upvar 1 $v x; set x target}\n\
         proc go {} {\n    set cmd other\n    setit cmd\n    $cmd\n}\n",
        "tcl",
    );
    let dispatch = 0; // any indirect site inside `go` would be unsound
    let _ = dispatch;
    assert!(
        !r2.command_invocations
            .iter()
            .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::target")),
        "an upvar-mutated head must not settle to the callee's write",
    );
}

#[test]
fn namespace_qualified_const_value_resolves_absolutely_945() {
    let mut a = Analyser::new();
    let src = "namespace eval ns { proc target {} {} }\nset cmd ::ns::target\n$cmd\n";
    let r = a.analyse(src, "tcl");
    let dispatch = u32::try_from(src.rfind("$cmd").unwrap()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.indirect
            && i.range.start() == dispatch
            && i.resolved_qualified_name.as_deref() == Some("::ns::target")),
        "an absolute value resolves to itself: {:?}",
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .map(|i| (&i.name, &i.resolved_qualified_name))
            .collect::<Vec<_>>(),
    );
    // The literal is the writable component.
    let lit = u32::try_from(src.find("::ns::target\n").unwrap()).unwrap();
    assert!(
        r.command_invocations
            .iter()
            .any(|i| !i.indirect && i.range.start() == lit),
        "the qualified literal is a writable reference",
    );
}

#[test]
fn traced_head_variable_abstains_945() {
    // A write trace can rewrite `cmd` at any moment (tclsh 9.0.4: the
    // callback fires on every write, and reads via `trace add variable …
    // read` can substitute values) — the reaching-definition walk cannot
    // see those writes, so a traced dispatch head abstains entirely.
    let mut a = Analyser::new();
    let src = "proc target {} {}\nproc redirect {n1 n2 op} {}\n\
               set cmd target\ntrace add variable cmd write redirect\n$cmd\n";
    let r = a.analyse(src, "tcl");
    assert!(
        !r.command_invocations.iter().any(|i| i.indirect),
        "a traced head must abstain: {:?}",
        r.command_invocations
            .iter()
            .filter(|i| i.indirect)
            .map(|i| &i.name)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn expanded_head_list_prefix_keeps_the_exact_writable_component_945() {
    // `{*}$cmd` expands the value as a command *prefix* — tclsh 9.0.4
    // dispatches its first list element with the rest appended as
    // arguments.  The writable provenance narrows to that element's
    // sub-span inside the defining literal (`helper` within
    // `{helper arg}`), so a rename rewrites exactly the command
    // component and keeps the baked argument.
    let mut a = Analyser::new();
    let src = "proc helper {a} {}\nset cmd {helper arg}\n{*}$cmd\n";
    let r = a.analyse(src, "tcl");
    let heads: Vec<&str> = r
        .command_invocations
        .iter()
        .filter(|i| i.indirect)
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        heads.contains(&"::helper"),
        "the prefix's first element dispatches ::helper: {heads:?}",
    );
    let lit_start = u32::try_from(src.find("helper arg").unwrap()).unwrap();
    let lit_end = lit_start + u32::try_from("helper".len()).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| !i.indirect
            && i.range.start() == lit_start
            && i.range.end() == lit_end
            && i.resolved_qualified_name.as_deref() == Some("::helper")),
        "the writable span covers exactly the command component: {:?}",
        r.command_invocations
            .iter()
            .filter(|i| !i.indirect && i.name.contains("helper"))
            .map(|i| (i.range.start(), i.range.end()))
            .collect::<Vec<_>>(),
    );
    // A plain (non-expanded) `$cmd` head treats the whole value as the
    // command name — a two-element value names no known command, so the
    // site abstains rather than resolving the first element.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("proc helper {a} {}\nset cmd {helper arg}\n$cmd\n", "tcl");
    assert!(
        !r2.command_invocations.iter().any(|i| i.indirect),
        "a non-expanded multi-word value is not a prefix dispatch",
    );
}
