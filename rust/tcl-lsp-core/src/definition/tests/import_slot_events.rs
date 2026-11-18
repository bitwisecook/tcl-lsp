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

//! The life of an import alias: `namespace forget`, ordering, `-force` conflicts, the export oracle, source changes and chains.

use super::*;

// Behaviour 1 — `namespace forget`.

#[test]
fn a_forget_after_the_import_stops_resolving_the_alias() {
    // TN (the bug): oracle
    //   namespace eval ::src { proc p {} {return P}; namespace export p }
    //   namespace eval ::dst { namespace import ::src::* }
    //   namespace eval ::dst { p }                        → P
    //   namespace eval ::dst { namespace forget ::src::p }
    //   namespace eval ::dst { p }                        → invalid command name "p"
    //   info commands ::dst::*                            → (empty)
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "a forgotten alias must stop resolving after the forget: {hit:?}"
    );
}

#[test]
fn a_call_before_the_forget_still_resolves() {
    // TP — the forget is order-gated, not file-wide: a call written
    // between the import and the forget still reaches `::src::p`.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a call before the forget is unaffected by it: {hit:?}"
    );
}

#[test]
fn a_glob_forget_removes_every_matching_alias() {
    // TN — oracle: `namespace forget ::src::*` empties `info commands
    // ::dst::*` entirely.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    proc q {} { return Q }\n    namespace export *\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::*\n}\n";
    let analysis = analyse(src);
    for name in ["p", "q"] {
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX).is_none(),
            "`{name}` must be forgotten by the glob pattern"
        );
    }
}

/// FN guard: a **dynamic** forget pattern
/// revokes nothing, so the alias keeps resolving.
///
/// `namespace forget ::src::$name` really does remove the alias when
/// `$name` is `p` (oracle for the literal spelling is on
/// `a_forget_after_the_import_stops_resolving_the_alias`) — but
/// nothing static says what `$name` holds, and the pattern is a *glob*, so
/// a guess would have to be either "revokes `p`" or "revokes everything".
/// Both silently drop references the program still has, which is the one
/// direction this family never takes: a removal that cannot be proved is
/// not applied. The literal prefix (`::src::`) narrows the *source* but
/// not the name, so it decides nothing on its own.
///
/// The scanner-side twin of this abstention is
/// `signature_scan::handlers::handle_namespace_forget_skips_dynamic_patterns`;
/// this pins the consequence a consumer actually sees.
#[test]
fn a_dynamic_forget_pattern_revokes_nothing() {
    let dynamic = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::$name\n}\n";
    let analysis = analyse(dynamic);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a dynamic forget pattern must not revoke on a guess: {hit:?}"
    );
    // Control — the same source with the pattern written literally does
    // revoke, so the difference is the substitution and nothing else.
    let literal = dynamic.replace("::src::$name", "::src::p");
    let analysis = analyse(&literal);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "the literal spelling of the same forget still revokes"
    );
}

/// FN guard for a dynamic export pattern — the export-side twin of
/// [`a_dynamic_forget_pattern_revokes_nothing`], and the same sign
/// argument.
///
/// `namespace export $pat` really does export whatever `$pat` expands to,
/// but nothing static says what that is and the word is a *glob*. The
/// scanner therefore records no pattern at all
/// (`Analyser::handle_namespace_export` skips a word containing `$` or
/// `[`), so the import takes a snapshot that does not cover the name and
/// the resolver abstains. That is the safe direction here precisely
/// because the sign is inverted from the forget case: a *guessed export*
/// can only add names an import never took, inventing a definition the
/// program has no path to.
///
/// The abstention costs a real answer, which is why it is pinned rather
/// than merely documented — a future change that starts guessing here
/// will fail this test, not silently start resolving.
#[test]
fn a_dynamic_export_pattern_exports_nothing() {
    let dynamic = "set pat p\nnamespace eval src {\n    proc p {} { return P }\n    namespace export $pat\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(dynamic);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "a dynamic export pattern must not be guessed into a covering export: {hit:?}"
    );
    // Control — the same source with the pattern written literally does
    // export, so the difference is the substitution and nothing else.
    let literal = dynamic.replace("namespace export $pat", "namespace export p");
    let analysis = analyse(&literal);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX)
            .is_some_and(|p| p.qualified_name == "::src::p"),
        "the literal spelling of the same export does resolve"
    );
}

/// TN for the interaction the dynamic-export abstention has with a
/// tombstone: a `namespace export -clear` written
/// after a dynamic pattern leaves the snapshot empty either way, so the
/// unrecorded pattern costs nothing there — the abstention is only ever
/// visible *before* a clear, which the test above covers.
#[test]
fn a_clear_after_a_dynamic_export_is_still_empty() {
    let src = "set pat p\nnamespace eval src {\n    proc p {} { return P }\n    namespace export $pat\n    namespace export -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "a `-clear` after any export leaves nothing exported"
    );
}

/// A load-level forget precedes a body-local import, so the call resolves.
/// C Tcl 8.6.14/9.0.4 return `P` for this original ordering.
#[test]
fn a_load_level_forget_before_a_body_local_import_revokes_nothing() {
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; p }\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\n";
    let analysis = analyse(src);
    let call = u32::try_from(src.rfind("; p }").expect("call in source") + 2).expect("tiny source");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "the body-local import runs after the load-level forget and reinstalls: {hit:?}"
    );
    // FN guard — a forget written *inside that same body*, after the
    // import, is an ordinary later statement and does revoke.
    let inner = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; namespace forget ::src::p ; p }\n}\n";
    let analysis = analyse(inner);
    let call =
        u32::try_from(inner.rfind("; p }").expect("call in source") + 2).expect("tiny source");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_none(),
        "a forget in the same body before the call does revoke"
    );
}

#[test]
fn a_simple_forget_pattern_removes_the_alias_too() {
    // TN — `Tcl_ForgetImport`'s unqualified branch: the pattern is matched
    // against this namespace's own imported command names, whatever their
    // origin. Oracle: `namespace eval ::dst {namespace forget p}` leaves
    // `info commands ::dst::*` empty and raises no error.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget p\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "an unqualified forget pattern removes the alias too"
    );
}

#[test]
fn a_forget_of_an_unrelated_source_leaves_the_alias_alone() {
    // FP guard — a qualified forget names the source whose imports it
    // removes; one naming a *different* namespace must not touch this
    // alias.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval other {\n    proc p {} { return O }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::other::p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a forget aimed at another source namespace revokes nothing: {hit:?}"
    );
}

#[test]
fn a_re_import_after_a_forget_resolves_again() {
    // TP — the log is ordered, not a one-way tombstone: re-running the
    // import after the forget installs the alias again (oracle: the bare
    // call works once more).
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a later re-import reinstates the alias: {hit:?}"
    );
}

// Behaviour 1b — the install is order-gated too.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
//
//   namespace eval ::src { proc p {} {return P}; namespace export p }
//   namespace eval ::dst { p }                    → invalid command name "p"
//   namespace eval ::dst { namespace import ::src::* }
//   namespace eval ::dst { p }                    → P
//
//   namespace eval ::app { proc run {} { helper } }
//   namespace eval ::app { namespace import ::src::* }
//   ::app::run                                    → HELP
//
//   namespace eval ::app2 { proc run2 {} {
//       catch {helper} e ; namespace import ::src::* ; return "$e [helper]" } }
//   ::app2::run2       → {invalid command name "helper"} HELP

#[test]
fn a_top_level_call_before_its_own_import_resolves_nothing() {
    // TN: both statements are top level, so
    // the offsets are in genuine execution order.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\np\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_none(),
        "a call written before its own import reaches nothing: {hit:?}"
    );
    // TP — the same call after the import does resolve.
    let after = after_last(src, "namespace import ::src::*\n}\n");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after)
            .is_some_and(|p| p.qualified_name == "::src::p"),
    );
}

#[test]
fn a_body_local_call_sees_an_import_written_later_in_the_file() {
    // TP (CRITICAL) — the gate is `in_effect`, not `at < call_off`: the
    // whole file loads, imports included, before any body runs. This is
    // the shape `tcllib`'s `modules/uev/uevent.tcl` is written in (procs
    // first, `namespace import` at the bottom); a plain-offset gate broke
    // every one of its call sites.
    let src = "namespace eval src {\n    proc helper {} { return HELP }\n    namespace export helper\n}\nnamespace eval app {\n    proc run {} { helper }\n}\nnamespace eval app {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "helper }");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::helper"),
        "a body-local call observes its own file's later top-level import: {hit:?}"
    );
}

#[test]
fn a_call_before_an_import_in_that_same_body_resolves_nothing() {
    // TN — the FN guard: an import written in the *same* body after the
    // call is an ordinary later statement of the running script, exactly
    // as the export snapshot already treats one.
    let src = "namespace eval src {\n    proc helper {} { return HELP }\n    namespace export helper\n}\nnamespace eval app {\n    proc run {} { helper ; namespace import ::src::* ; helper }\n}\n";
    let analysis = analyse(src);
    let first = at_first(src, "helper ;");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", first).is_none(),
        "the first call runs before the same body's import",
    );
    let second = after_last(src, "namespace import ::src::* ; ");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "app", "helper", second)
            .is_some_and(|p| p.qualified_name == "::src::helper"),
        "the second call runs after it",
    );
}

#[test]
fn a_call_before_a_forced_import_is_not_shadowed_yet() {
    // TP — the same gate through `ImportQuery::ForcedShadow`: a `-force`
    // import has not replaced the local command until it runs, so a call
    // written before it still reaches the local definition (oracle:
    // `::dst::p` → LOCAL before the forced import, → SRC after).
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\np\nnamespace eval dst {\n    namespace import -force ::src::*\n}\n";
    let analysis = analyse(src);
    let before = at_first(src, "\np\n") + 1;
    assert!(
        !forced_import_shadows(&analysis, DOC, "dst", "p", before),
        "the shadow is not in effect before the forced import runs",
    );
    let after = after_last(src, "namespace import -force ::src::*\n}\n");
    assert!(forced_import_shadows(&analysis, DOC, "dst", "p", after));
}

#[test]
fn a_later_conflicting_import_does_not_decide_an_earlier_call() {
    // TP — the winning source is chosen from the installs already in
    // effect, so a `-force` re-import from a different namespace written
    // *after* the call cannot retarget it (oracle: with `::A` imported
    // first, a call between the two imports runs `::A::p`).
    let src = "namespace eval A {\n    proc p {} { return A }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return B }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\np\nnamespace eval dst {\n    namespace import -force ::B::*\n}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the call between the two imports takes the first edge: {hit:?}"
    );
    let after = after_last(src, "namespace import -force ::B::*\n}\n");
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after)
            .is_some_and(|p| p.qualified_name == "::B::p"),
        "…and a call after the forced re-import takes the second",
    );
}

// Behaviour 2 — `-force` / import-conflict semantics.

#[test]
fn a_forced_import_shadows_the_local_command() {
    // TP (the bug): oracle
    //   namespace eval ::src { proc p {} {return SRC}; namespace export p }
    //   namespace eval ::dst { proc p {} {return LOCAL} }
    //   ::dst::p                                   → LOCAL
    //   namespace eval ::dst { namespace import -force ::src::* }
    //   ::dst::p                                   → SRC
    //   namespace origin ::dst::p                  → ::src::p
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "p");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a `-force` import replaces the local command: {hit:?}"
    );
}

#[test]
fn an_unforced_import_onto_an_existing_command_installs_nothing() {
    // TN (the other bug): the same program without `-force` raises `can't
    // import command "p": already exists`, the local survives, and
    // `namespace origin ::dst::p` answers `::dst::p`. The import must not
    // make the call a reference to `::src::p`.
    // tclsh 8.4.20-9.1.0: the caught import fails, `p` in dst -> LOCAL and
    // namespace origin ::dst::p -> ::dst::p.
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    catch {namespace import ::src::*}\n}\nnamespace eval dst {p}\n";
    let analysis = analyse(src);
    let call =
        u32::try_from(src.rfind("namespace eval dst {p}").unwrap() + "namespace eval dst {".len())
            .unwrap();
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_none(),
        "a conflicting non-`-force` import installs nothing"
    );
    // …and the ordinary namespace lookup still answers the local.
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::dst::p"),
        "the local definition survives the failed import: {hit:?}"
    );
}

#[test]
fn a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias() {
    // The three-phase ordering. The conflict is
    // judged at the *import's* position, so a `proc` written after it does
    // not retroactively make the import a no-op — a call between the two
    // still reaches `::src::p`. But the `proc` *does* recreate
    // `::dst::p` as an ordinary command, silently, so a call after it
    // reaches the local one. Oracle (9.0.4 / 8.6.14, byte-identical):
    //   namespace eval ::dst {namespace import ::src::*}
    //   ::dst::p                    → SRC   ; origin → ::src::p
    //   namespace eval ::dst {proc p {} {return LOCAL}}    (rc 0, silent)
    //   ::dst::p                    → LOCAL ; origin → ::dst::p
    //   ::src::p                    → SRC   (untouched)
    // tclsh 8.4.20-9.1.0: the calls run in dst (SRC, then LOCAL).
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {p}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {p}\n";
    let analysis = analyse(src);
    // TP — between the import and the redefinition, the alias is live.
    let between = at_first(src, "namespace eval dst {p}")
        + u32::try_from("namespace eval dst {".len()).unwrap();
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", between);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a later local definition is not a conflict at the import: {hit:?}"
    );
    // TN — after it, the alias is gone and the local command is what the
    // ordinary namespace lookup answers.
    let after =
        u32::try_from(src.rfind("namespace eval dst {p}").unwrap() + "namespace eval dst {".len())
            .unwrap();
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", after).is_none(),
        "redefining the imported name ends the alias"
    );
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", after, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::dst::p"),
        "after the redefinition the call reaches the local proc: {hit:?}"
    );
}

#[test]
fn a_redefinition_ends_a_forced_import_shadow() {
    // TN — the same rule reached through `forced_import_shadows`: with the
    // shadow stuck on, `definition` / `hover` / `signature_help` would skip
    // the valid local definition and answer the import source forever.
    // Oracle: `proc ::dst::p` after `namespace import -force ::src::*`
    // returns LOCAL2 with `namespace origin ::dst::p` → `::dst::p`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    proc p {} { return LOCAL2 }\n}\n";
    let analysis = analyse(src);
    let between = at_first(src, "namespace import -force")
        + u32::try_from("namespace import -force ::src::*".len()).expect("tiny");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", between, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "between the forced import and the redefinition the source wins: {hit:?}"
    );
    let after = after_last(src, "return LOCAL2 }");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", after, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::dst::p"),
        "the redefinition ends the forced shadow: {hit:?}"
    );
}

#[test]
fn a_live_alias_is_an_import_conflict_for_a_different_source() {
    // TN (CRITICAL) — with `::dst` already
    // importing `::A::*`, a later *unforced* `namespace import ::B::*`
    // fails and changes nothing. Oracle (9.0.4 / 8.6.14):
    //   namespace eval ::dst {namespace import ::A::*}   ; origin → ::A::p
    //   namespace eval ::dst {namespace import ::B::*}
    //     → can't import command "p": already exists
    //   ::dst::p → AP ; origin → ::A::p
    // Before this, both imports entered the install set and the *newest*
    // won, resolving later calls to `::B::p`.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the failed second import leaves the first alias in place: {hit:?}"
    );
}

#[test]
fn a_forced_import_from_a_second_source_replaces_the_first_alias() {
    // TP, the other half of the conflict rule — with `-force` the second import
    // wins (oracle: `::dst::p` → BP, `namespace origin` → `::B::p`).
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import -force ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::B::p"),
        "a `-force` import replaces a live alias too: {hit:?}"
    );
}

#[test]
fn a_body_local_import_conflicts_with_a_later_top_level_import() {
    // TN (CRITICAL) — the in-document twin of the workspace tier's own
    // check: which import installs is decided in *execution* order, not
    // written order. Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
    //
    //   namespace eval ::dst { proc p {} { namespace import ::B::x } }
    //   namespace eval ::dst { namespace import ::A::* }
    //   namespace origin ::dst::x  -> ::A::x
    //   ::dst::p                   -> can't import command "x": already exists
    //   namespace origin ::dst::x  -> ::A::x   (unchanged)
    //
    // The whole file loads before any body runs, so the top-level `::A`
    // import owns the name and the body-local `::B` one installs nothing.
    // Folding the slot log in raw offset order got both halves wrong: it
    // installed `::B` and then conflicted `::A` away.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    proc runner {} { namespace import ::B::p }\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the top-level import runs first and the body-local one installs \
             nothing: {hit:?}"
    );
}

#[test]
fn a_later_import_in_the_same_body_still_loses_to_the_earlier_one() {
    // TN guard — inside one body the offsets *are* execution order again.
    // Oracle: `proc q {} { namespace import ::A::* ; namespace import
    // ::B::p }` → the first succeeds, the second raises `already exists`,
    // `namespace origin` stays `::A::p`.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    proc q {} { namespace import ::A::* ; namespace import ::B::p }\n}\n";
    let analysis = analyse(src);
    let call = after_last(src, "namespace import ::B::p");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "the second import of the same body installs nothing: {hit:?}"
    );
}

#[test]
fn a_forget_lets_the_next_unforced_import_install() {
    // FN guard for the conflict rule — the conflict is the *live* alias, not the
    // fact that one was ever installed. Oracle: forgetting `::A::p` first
    // makes the unforced `::B::*` import succeed (`origin` → `::B::p`).
    let src = "rename auto_import savedPrelude; namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval B {\n    proc p {} { return BP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace forget ::A::p\n}\nnamespace eval dst {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::B::p"),
        "after the forget the next unforced import installs: {hit:?}"
    );
}

#[test]
fn a_re_import_from_the_same_source_is_not_a_conflict() {
    // FN guard — re-importing the *same* thing is a silent no-op, not the
    // `already exists` error (oracle: rc 0, `origin` still `::A::p`), so
    // the second import must not be treated as conflicting with the alias
    // it would reinstall.
    let src = "namespace eval A {\n    proc p {} { return AP }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\nnamespace eval dst {\n    namespace import ::A::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::A::p"),
        "a same-source re-import stays live: {hit:?}"
    );
}

#[test]
fn an_unforced_import_without_a_conflict_still_installs() {
    // FN guard — the conflict rule must not fire when the importing
    // namespace holds nothing of that name (the overwhelmingly common
    // case, oracle `H` row: `::dst::p` → `SRC`).
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
        "an unconflicted import still installs"
    );
}

#[test]
fn a_forced_import_before_the_call_does_not_shadow_a_later_call_site() {
    // The forced import replaces the local `::dst::p`, and forgetting it
    // removes the only `::dst::p` left, so the local definition does not
    // come back. Oracle (tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0):
    // the forced import makes `namespace origin ::dst::p` `::src::p`, the
    // forget leaves no `::dst::p`, and the call raises
    // `invalid command name "p"`. The query sits on the call's own head
    // word, which is what the original declaration selection reads.
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n}\nnamespace eval dst {\n    namespace import -force ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "p");
    let hit = resolve_called_proc(&analysis, src, "::dst", "p", call, DOC);
    assert!(
        hit.is_none(),
        "after the forget no `p` is left for the call to reach: {hit:?}"
    );
}

// Behaviour 2b — what an *absent* export means for `-force`.

#[test]
fn a_forced_import_shadows_when_this_file_holds_no_export_for_the_source() {
    // TP (CRITICAL) — the partly-observable source. `::src`'s proc is in
    // this document, its `namespace export` is not, so a
    // "does this file hold *anything* of `::src`" test reads the gap as
    // "not exported" and single-file go-to-definition kept answering the
    // local `::app::helper`. Oracle (8.6.14 / 9.0.4) with the export
    // present anywhere in the program: `::app::helper` → SRC and
    // `namespace origin ::app::helper` → `::src::helper`.
    let src = "namespace eval src {\n    proc helper {} { return SRC }\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "helper");
    assert!(
        forced_import_shadows(&analysis, DOC, "::app", "helper", call),
        "a `-force` import of a namespace whose exports this file cannot \
             see must presume the shadow",
    );
    assert!(
        resolve_called_proc(&analysis, src, "::app", "helper", call, DOC).is_none(),
        "…and the local definition the import deleted must not be the answer",
    );
}

#[test]
fn a_forced_import_does_not_shadow_when_this_file_holds_the_export_list() {
    // TN / FP guard — with an export declaration for `::src` in this
    // document the gap *is* evidence: `helper` is not among what `::src`
    // exports, so the forced import installs nothing and the local
    // definition is still what the call reaches (oracle: with only
    // `namespace export other`, `::app::helper` → LOCAL).
    let src = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "helper");
    assert!(!forced_import_shadows(
        &analysis, DOC, "::app", "helper", call
    ));
    let hit = resolve_called_proc(&analysis, src, "::app", "helper", call, DOC);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::app::helper"),
        "the local definition survives an import that binds nothing: {hit:?}"
    );
}

#[test]
fn a_forced_import_shadows_when_another_file_holds_the_covering_export() {
    // TP (the bug, CRITICAL) — `::src` exports `helper` somewhere else in
    // the program. The `-force` import therefore *did* delete
    // `::app::helper`, and the call reaches `::src::helper` — which this
    // document happens to hold, so the in-document tier can answer it
    // outright. The document-only rule reads "this file has an export
    // record for ::src and none of them covers helper" as a fact and keeps
    // answering the deleted local definition.
    assert_eq!(
        partly_observable_target(&FakeExports(&[("src", &["other", "helper"])])),
        Some("::src::helper".to_owned()),
    );
}

#[test]
fn a_forced_import_does_not_shadow_when_the_whole_program_exports_nothing() {
    // TN, byte-identical input to the test above — nothing anywhere in the
    // program exports `helper`, so the `-force` import binds only `other`
    // and the local definition survives (oracle: LOCAL / ::app::helper).
    // The document-only rule gets this one right too; it is the pinned
    // true negative
    // `a_forced_import_does_not_shadow_when_this_file_holds_the_export_list`
    // covers without an oracle.
    assert_eq!(
        partly_observable_target(&FakeExports(&[("src", &["other"])])),
        Some("::app::helper".to_owned()),
    );
}

#[test]
fn a_forced_import_from_a_namespace_the_program_cannot_see_still_shadows() {
    // TP / abstention — `::src` is in no indexed file (an installed
    // package, a document outside the workspace), so the oracle answers
    // `Unknown`. `-force` deletes the local command whether or not
    // anything here can prove the export, and answering with a command the
    // import may have removed is worse than answering nothing: the
    // resolver abstains and the cross-document tier takes over.
    assert_eq!(partly_observable_target(&FakeExports(&[])), None);
}

#[test]
fn the_same_document_without_an_oracle_keeps_the_document_only_answer() {
    // FP guard on the plumbing itself: the oracle is *optional*. A caller
    // with no workspace index — a single-document unit test, the `tcl`
    // CLI, a buffer the workspace has not indexed — must get exactly the
    // document-only answer rather than a panic or the abstention above.
    let analysis = analyse(PARTLY_OBSERVABLE);
    let call = last_call_head(PARTLY_OBSERVABLE, "helper");
    assert!(!forced_import_shadows(
        &analysis, DOC, "::app", "helper", call
    ));
    let hit = resolve_called_proc(&analysis, PARTLY_OBSERVABLE, "::app", "helper", call, DOC);
    assert_eq!(
        hit.map(|p| p.qualified_name.clone()),
        Some("::app::helper".to_owned()),
    );
}

#[test]
pub(super) fn the_real_oracle_does_not_resurrect_an_export_written_after_the_import() {
    // FP guard, an export written after its import under the oracle, and the reason
    // [`ProgramExports`] pairs the oracle with the document's **real**
    // URI. The export covering the name is in this very document but
    // written *after* the import, so it is not retroactive (oracle:
    // `invalid command name "p"`). The workspace oracle holds that same
    // row, tagged with the same URI, so it ranks it against the import by
    // offset and answers `NotExported`. Handed a placeholder URI instead
    // it could not rank the row at all, and
    // `exported_at_import_site`'s abstain-toward-exported rule would have
    // installed an import Tcl never made.
    //
    // Deliberately run against the real `WorkspaceIndex` snapshot rather
    // than [`FakeExports`], which is position-blind by design.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return SRC }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let call = last_call_head(src, "p");
    let index =
        crate::workspace_index::WorkspaceIndex::from_documents([("file:///main.tcl", &analysis)]);
    let exports = index.export_snapshot();
    let ctx = DOC.in_program(ProgramExports {
        uri: "file:///main.tcl",
        oracle: exports.as_ref(),
    });
    assert!(
        proc_visible_via_wildcard_import(&analysis, ctx, "::dst", "p", call).is_none(),
        "an export written after the import must not install it retroactively",
    );
    // …and a *second* import written after the export does pick it up,
    // so the guard above is the ordering rule and not a blanket refusal.
    let src2 = format!("{src}namespace eval dst {{\n    namespace import ::src::*\n}}\n");
    let analysis2 = analyse(&src2);
    let index2 =
        crate::workspace_index::WorkspaceIndex::from_documents([("file:///main.tcl", &analysis2)]);
    let exports2 = index2.export_snapshot();
    let ctx2 = DOC.in_program(ProgramExports {
        uri: "file:///main.tcl",
        oracle: exports2.as_ref(),
    });
    let after = u32::try_from(src2.len()).expect("tiny test source");
    assert!(
        proc_visible_via_wildcard_import(&analysis2, ctx2, "::dst", "p", after)
            .is_some_and(|p| p.qualified_name == "::src::p"),
        "each import site takes its own export snapshot",
    );
}

// Behaviour 3 — source rename / delete / redefine after the import.

#[test]
fn deleting_the_source_command_kills_the_alias() {
    // TN (the bug): oracle
    //   namespace eval ::src { proc p {} {return P}; namespace export p }
    //   namespace eval ::dst { namespace import ::src::* }
    //   ::dst::p              → P
    //   rename ::src::p {}
    //   ::dst::p              → invalid command name "::dst::p"
    //   info commands ::dst::* → (empty)
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nrename ::src::p {}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "the alias holds the command object, so deleting it kills the alias: {hit:?}"
    );
}

#[test]
fn renaming_the_source_command_leaves_the_alias_alive() {
    // TP — the crucial asymmetry: oracle
    //   rename ::src::p ::src::pp
    //   ::dst::p                  → P          (still works)
    //   namespace origin ::dst::p → ::src::pp  (only the origin moved)
    //   info commands ::dst::*    → ::dst::p
    // A model that treated any `rename` of the source as a removal would
    // lose a genuinely-live command here.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nrename ::src::p ::src::pp\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a rename moves the origin but keeps the alias: {hit:?}"
    );
}

#[test]
fn a_call_before_the_source_deletion_still_resolves() {
    // FP guard — the deletion is order-gated like every other lifecycle
    // event, so a call written before it is unaffected.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\np\nrename ::src::p {}\n";
    let analysis = analyse(src);
    let call = at_first(src, "\np\n") + 1;
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).is_some(),
        "a call before the deletion still reaches the source"
    );
}

#[test]
fn a_redefinition_of_the_source_is_seen_through_the_link() {
    // TP — oracle: redefining `::src::p` after the import makes `::dst::p`
    // run the *new* body (`SECOND`), with `namespace origin ::dst::p`
    // still `::src::p`. The alias re-reads the source at call time, so the
    // definition a call reaches is the one in effect at the *call*, not
    // the one in effect at the import.
    let src = "namespace eval src {\n    proc p {} { return FIRST }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    proc p {} { return SECOND }\n}\ndst::p\n";
    let analysis = analyse(src);
    let call = after_last(src, "dst::p");
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", call).expect("alias");
    let captured = analysis
        .proc_def_in_effect_at(&hit.qualified_name, call)
        .expect("definition in effect at the call");
    assert_eq!(
        captured.name_span.start(),
        at_first(src, "p {} { return SECOND }"),
        "the redefinition is seen through the link"
    );
}

// Behaviour 4 — import chains.

#[test]
fn an_import_chain_follows_to_the_original_source() {
    // TP (the bug): oracle
    //   namespace eval ::C { proc p {} {return CP}; namespace export p }
    //   namespace eval ::B { namespace import ::C::*; namespace export p }
    //   namespace eval ::A { namespace import ::B::* }
    //   ::A::p                    → CP
    //   namespace origin ::A::p   → ::C::p
    // Statically `::B::p` is in no `all_procs`, so a single-hop walk found
    // nothing and navigation abstained.
    let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::C::p"),
        "the chain must follow ::A → ::B → ::C: {hit:?}"
    );
}

#[test]
fn a_forget_in_the_middle_of_a_chain_kills_the_whole_chain() {
    // TN — oracle: `namespace eval ::B {namespace forget ::C::p}` makes
    // `::A::p` an `invalid command name` too, because deleting an imported
    // command deletes the commands imported *from* it. Both `info commands
    // ::A::*` and `::B::*` come back empty.
    let src = "rename auto_import savedPrelude; namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n}\nnamespace eval B {\n    namespace forget ::C::p\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none(),
        "a forget in the middle hop kills the outer alias too"
    );
}

#[test]
fn a_chain_hop_that_was_never_exported_does_not_resolve() {
    // FN guard — each hop keeps its own export gate: with `::B` importing
    // `::C::*` but never re-exporting `p`, `::A`'s import of `::B::*`
    // binds nothing (oracle: `::A::p` → invalid command name).
    let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n}\nnamespace eval A {\n    namespace import ::B::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none(),
        "the middle hop must have exported the name for the chain to run"
    );
}

#[test]
fn a_mutually_importing_pair_terminates() {
    // FP/hang guard — two namespaces importing each other's `*` form a
    // cycle with no definition at either end; the walk is bounded by
    // `indirection::MAX_COMMAND_NAME_HOPS` and must simply abstain.
    let src = "namespace eval A {\n    namespace import ::B::*\n    namespace export *\n}\nnamespace eval B {\n    namespace import ::A::*\n    namespace export *\n}\n";
    let analysis = analyse(src);
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "A", "p", u32::MAX).is_none());
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "B", "p", u32::MAX).is_none());
}
