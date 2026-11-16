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

//! Command words that resolve to no procedure: the module's `unknown` handler and unenumerable dispatches.

use super::*;

/// A `TclOO` method's relative command resolves in the receiver object's
/// namespace.  The selected implementation may itself call any procedure
/// reachable from this source with arguments the lexical CFG never sees,
/// so cross-file evidence must withdraw an otherwise-uniform literal.
#[test]
fn runtime_selected_method_command_poisons_cross_file_param_evidence() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "::helper prod\noo::class create C { method run {} { hook } }\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &["::helper".to_owned()],
    );
    let helper = evidence.get("::helper").expect("direct call recorded");
    assert_eq!(
        helper.uniform_literal_at(0),
        None,
        "an object-local hook can call helper with arbitrary arguments: {evidence:?}",
    );
}

/// Absolute method commands do not depend on the receiver namespace and
/// therefore must not discard unrelated exact call-site evidence.
#[test]
fn absolute_only_method_keeps_cross_file_param_evidence_exact() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc ::helper {mode} {return $mode}\n::helper prod\noo::class create C { method run {} { ::puts ok } }\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &["::helper".to_owned()],
    );
    let helper = evidence.get("::helper").expect("direct call recorded");
    assert_eq!(helper.uniform_literal_at(0), Some("prod"));
}

// A module's own `unknown` handler. Tcl dispatches every
// unresolved command word to it, so its direct callers are never its
// complete caller set. tclsh8.6/9.0-confirmed: with `proc unknown {cmd
// args}` in scope, `bogus beta gamma` runs the handler with
// `cmd` = `bogus`, `args` = `beta gamma`.

#[test]
fn an_unresolved_word_is_a_call_site_of_the_modules_unknown_handler_1044() {
    // TP: `bogus` names nothing, so real Tcl calls the handler with
    // `bogus`. Seeing only the two `unknown alpha`
    // calls, the scan would bind `cmd` to the constant `"alpha"` and fold
    // `$cmd eq "alpha"` on a genuinely runtime-varying condition.
    let ev = evidence(
        "proc unknown {cmd args} { if {$cmd eq \"alpha\"} { return 1 } else { return 2 } }\nunknown alpha\nunknown alpha\nbogus beta\n",
    );
    assert_eq!(
        slot(&ev, "::unknown", 0),
        (vec!["alpha".into(), "bogus".into()], false),
        "the unresolved word itself is the handler's first argument",
    );
    assert_eq!(uniform(&ev, "::unknown", 0), None, "must not fold");
}

#[test]
fn the_unresolved_words_own_arguments_follow_it_into_the_handler_1044() {
    // TP: Tcl passes the failed call's arguments after the word, so
    // `args`' positions carry them — evidence the scan really can see,
    // recorded in full rather than merely poisoned.
    let ev = evidence("proc unknown {cmd args} { return $cmd }\nbogus beta gamma\n");
    assert_eq!(slot(&ev, "::unknown", 0), (vec!["bogus".into()], false));
    assert_eq!(slot(&ev, "::unknown", 1), (vec!["beta".into()], false));
    assert_eq!(slot(&ev, "::unknown", 2), (vec!["gamma".into()], false));
}

#[test]
fn a_module_with_no_unknown_handler_is_unaffected_1044() {
    // TN, the common case and the whole regression risk: an unresolved
    // word in a module that defines no handler must change nothing.
    let ev = evidence("proc helper {mode} { return $mode }\nhelper a\nbogus beta\n");
    assert_eq!(slot(&ev, "::helper", 0), (vec!["a".into()], false));
    assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("a"));
    assert!(
        ev.get("::unknown").is_none(),
        "no handler defined, so nothing may be attributed to one: {ev:?}",
    );
}

#[test]
fn a_handler_never_seeds_even_when_every_visible_caller_agrees_1044() {
    // The handler's caller set is unenumerable *by construction*, so
    // agreement among the callers a scan can see proves nothing.
    //
    // "With no unresolved word in the file the direct callers really are
    // all of them" is false: real Tcl routes to the handler
    // every word that resolves to nothing at the instant of the call:
    // an auto-loaded name, a name another sourced file introduces, a
    // name built by string arithmetic, a name typed at a prompt. None of
    // those appear in the source for any scan to find.
    //
    // On tclsh8.6, the seeded words are
    // wrong in *both* directions. `Dog new` after `oo::class create Dog`
    // and `worker` after `coroutine worker body` are recorded as
    // dispatches, yet neither ever reaches the handler. And a `bogus
    // beta` written *before* `proc unknown` is handled by the builtin
    // `::unknown` — it errors — so it is not a call site of this
    // handler either.
    let ev = evidence(
        "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "defining the handler is itself the unenumerable caller: {ev:?}",
    );
}

#[test]
fn a_class_command_never_seeds_the_handler_1044() {
    // On tclsh8.6, `oo::class create Dog` binds `Dog`, so `Dog
    // new` dispatches to the class command and the handler is never
    // called. The scan cannot resolve `Dog` and records it as an
    // unresolved-word dispatch anyway; the unconditional poison is what
    // stops that invented evidence becoming a fold.
    let ev = evidence(
        "proc unknown {cmd args} { if {$cmd eq \"Dog\"} { return 1 } else { return 2 } }\noo::class create Dog {\n    method bark {} { return woof }\n}\nDog new\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "a class command is not a handler call site: {ev:?}",
    );
}

#[test]
fn a_coroutine_command_never_seeds_the_handler_1044() {
    // On tclsh8.6, `coroutine worker body` binds `worker`, so
    // calling it resumes the coroutine and the handler is never called.
    let ev = evidence(
        "proc unknown {cmd args} { if {$cmd eq \"worker\"} { return 1 } else { return 2 } }\nproc body {} { yield ; return done }\ncoroutine worker body\nworker\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "a coroutine command is not a handler call site: {ev:?}",
    );
}

#[test]
fn a_word_written_before_the_handler_never_seeds_it_1044() {
    // On tclsh8.6, `bogus beta` on line 1, with `proc unknown`
    // defined only afterwards, is handled by the *builtin* `::unknown`
    // and errors with `invalid command name "bogus"`. The scan is
    // definition-order-insensitive, so it records the dispatch anyway.
    let ev = evidence(
        "bogus beta\nproc unknown {cmd args} { if {$cmd eq \"bogus\"} { return 1 } else { return 2 } }\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "an order-insensitive scan may not seed an order-sensitive dispatch: {ev:?}",
    );
}

#[test]
fn a_registry_builtin_is_not_an_unresolved_word_1044() {
    // FP guard: a builtin resolves to no *user proc*, but it is not an
    // unresolved word — Tcl never routes `puts`/`set` to the handler.
    let ev = evidence(
        "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\nset x 1\nincr x\n",
    );
    assert_eq!(
        slot(&ev, "::unknown", 0),
        (vec!["alpha".into()], false),
        "builtins must not appear as handler arguments: {ev:?}",
    );
}

#[test]
fn an_unenumerable_dispatch_word_still_poisons_rather_than_naming_the_handler_1044() {
    // FN guard: a dynamic word whose value set is unreadable is not an
    // *unresolved* word — the scan cannot say it resolves to nothing —
    // so it keeps withdrawing every seed, the handler's included.
    let ev = evidence(
        "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nset c [gets stdin]\n$c beta\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "an unreadable dispatch may name anything, handler included: {ev:?}",
    );
}

#[test]
fn a_cross_file_dispatch_never_seeds_another_files_handler_1044() {
    // The cross-file scan resolves against the *project's* names, so a
    // file that defines no handler still attributes its unresolved words
    // to one another file defines. That path needs the same poison, or
    // the miscompile simply moves across the file boundary.
    //
    // Shape: `h.tcl` holds `proc unknown`, `c.tcl` holds
    // `oo::class create Dog` + `Dog new`. The `Dog` dispatch is invented
    // (tclsh8.6: the class command answers, the handler is never called),
    // and it would be the handler's only recorded call site.
    let reg = registry();
    let evidence = scan_source_call_sites(
        "oo::class create Dog { method bark {} { return woof } }\nDog new\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::unknown"]),
        &[],
    );
    assert_eq!(
        evidence
            .get("::unknown")
            .and_then(|e| e.uniform_literal_at(0)),
        None,
        "a sibling file's dispatch may not seed the project's handler: {evidence:?}",
    );
}

#[test]
fn only_one_command_carries_the_unresolved_handler_trait() {
    // `unresolved_command_handler` takes the first match from a hash-map
    // walk, so more than one carrier would make the answer depend on
    // iteration order. Sorting makes it deterministic; this pins the
    // stronger property that there is nothing to choose between.
    let reg = registry();
    let carriers = reg.commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER);
    assert_eq!(
        carriers.len(),
        1,
        "a second carrier needs a resolution rule, not an arbitrary pick: {carriers:?}",
    );
}

#[test]
fn an_unenumerable_dispatch_marks_the_module_non_enumerable() {
    let ev = evidence("proc helper {mode} { return $mode }\nset cmd [gets stdin]\n$cmd dev\n");
    assert_eq!(uniform(&ev, "::helper", 0), None);
}

/// An unenumerable dispatch withdraws every seed in the unit that
/// contains it — expressed per callee, so it survives a merge.
#[test]
fn an_unenumerable_dispatch_withdraws_this_units_seeds() {
    let ev = evidence(
        "proc helper {mode} { return $mode }\nhelper prod\nhelper prod\nset cmd [gets stdin]\n$cmd dev\n",
    );
    assert_eq!(uniform(&ev, "::helper", 0), None);
}

/// ...but only that unit's.  A file declaring no linkage cannot dispatch
/// against a command table it never loaded, so merging its evidence into
/// a project must not disturb an unrelated file's sound seed.  Without
/// the reach bound, one `eval $script` anywhere in a workspace withdrew
/// every interprocedural seed in every file.
#[test]
fn an_unenumerable_dispatch_does_not_reach_an_unlinked_file() {
    let opaque = evidence("proc mine {x} { return $x }\nset cmd [gets stdin]\n$cmd dev\n");
    let mut project = evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
    project.merge_from(&opaque);
    assert_eq!(
        uniform(&project, "::theirs", 0).as_deref(),
        Some("prod"),
        "the dispatching file declares no `source`/`package require`, so it \
             cannot name `theirs` at all",
    );
    assert_eq!(
        uniform(&project, "::mine", 0),
        None,
        "its own procedure is still withdrawn",
    );
}

/// The host supplies the reach, so a file linked to the callee's — in
/// *either* direction — retracts its seed.
///
/// The inbound direction is the one a file-local bound gets wrong: a
/// library declaring no `source` of its own still runs inside its
/// sourcer's interpreter, so its unreadable dispatch can name the
/// sourcer's procedures. Bounding by what the *scanning* file loads
/// missed exactly that.
#[test]
fn an_unenumerable_dispatch_reaches_whatever_the_host_says_it_links_to() {
    let reg = registry();
    let known: HashSet<String> = ["::theirs".to_owned()].into_iter().collect();
    let linked = ["::theirs".to_owned()];
    for src in [
        // Outbound: this file sources the other.
        "source other.tcl\nset cmd [gets stdin]\n$cmd dev\n",
        // Inbound: this file declares no linkage at all, but the host put
        // it in the callee's component because something sources it.
        "set cmd [gets stdin]\n$cmd dev\n",
    ] {
        let opaque = scan_source_call_sites(
            src,
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &known,
            &linked,
        );
        let mut project =
            evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
        project.merge_from(&opaque);
        assert_eq!(
            uniform(&project, "::theirs", 0),
            None,
            "a linked file's unreadable dispatch can name ::theirs: {src}",
        );
    }
}

/// ...and an *unlinked* file cannot: the host leaves it out of the reach,
/// so its `$cmd` does not disturb a sound seed in a file it shares only a
/// workspace folder with.
#[test]
fn an_unenumerable_dispatch_does_not_reach_an_unlinked_project_file() {
    let reg = registry();
    let known: HashSet<String> = ["::theirs".to_owned()].into_iter().collect();
    let opaque = scan_source_call_sites(
        "proc mine {x} { return $x }\nset cmd [gets stdin]\n$cmd dev\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &known,
        // Not in the callee's component: its reach is its own procs.
        &["::mine".to_owned()],
    );
    let mut project = evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
    project.merge_from(&opaque);
    assert_eq!(
        uniform(&project, "::theirs", 0).as_deref(),
        Some("prod"),
        "an unlinked file shares no interpreter, so it cannot name ::theirs",
    );
}

#[test]
fn a_script_the_scan_cannot_read_makes_the_module_non_enumerable() {
    for src in [
        "proc helper {mode} { return $mode }\nset s [gets stdin]\neval $s\n",
        "proc helper {mode} { return $mode }\ncatch $body\n",
        "proc helper {mode} { return $mode }\napply $fn 1\n",
    ] {
        assert!(
            uniform(&evidence(src), "::helper", 0).is_none(),
            "a script received as a value may call anything: {src}",
        );
    }
}
