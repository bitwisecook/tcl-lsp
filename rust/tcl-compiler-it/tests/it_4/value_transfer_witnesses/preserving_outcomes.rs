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

//! Outcomes that may preserve their target: no-match and partial scans, templates, constants and repeated targets.

use super::*;

/// Workspace commands whose one target may keep its prior value, declared
/// by outcome alone: no `READS_BEFORE_WRITE`, no write-class trait.
const MAY_PRESERVE_PACK: &str = "speclib kv 2.2 {
    command kv::maybe {
        arity 2
        arg 1 -role VarWrite
        semantics {
            stores -targets {1} -outcome may_write
        }
        evaluate none
    }
    command kv::wp {
        arity 2
        arg 1 -role VarWrite
        semantics {
            stores -targets {1} -outcome write_or_preserve
        }
        evaluate none
    }
    command kv::split3 {
        arity 4
        arg_role_resolver {words ctx} { role 1 VarWrite; role 2 VarWrite; role 3 VarWrite }
        arg_role_resolver_roles {VarWrite}
        semantics {
            stores -targets {1 2 3} -outcome write_or_preserve
            result -semantic int
        }
        evaluate -implementation kv.split3.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            body {s} {
                set parts [split $s :]
                if {[llength $parts] != 3} {
                    preserve 1; preserve 2; preserve 3
                    fold 0
                    return
                }
                lassign $parts a b c
                write 1 $a; write 2 $b; write 3 $c
                fold 3
            }
        }
    }
}
";

/// Whether the analysis of `source` reports W220 on `name`, and whether
/// the optimiser's single pass removes a store, under `dialect` with the
/// packs `key` names installed in `registry`.
fn dead_store_findings(
    registry: &tcl_registry::CommandRegistry,
    key: u64,
    source: &str,
    dialect: &str,
    name: &str,
) -> (bool, bool) {
    use tcl_compiler::analyser::Analyser;
    let named = format!("'{name}'");
    let reported = Analyser::new()
        .with_pack_overlay(key)
        .analyse(source, dialect)
        .diagnostics
        .into_iter()
        .any(|d| d.code == DiagCode::W220 && d.message.contains(&named));
    let profile = resolve_environment(dialect).analyser_profile();
    let removed = optimise_with_dialect(source, registry, Some(profile))
        .iter()
        .any(|rewrite| matches!(rewrite.code, DiagCode::O109 | DiagCode::O126));
    (reported, removed)
}

/// A store a later call only may overwrite is live: a pack target declared
/// `may_write` or `write_or_preserve` may keep the value the store gave it,
/// so the store ahead of the call — as a statement, in a condition, or in a
/// word — draws no W220 and no O109, as the store ahead of a `regexp` does,
/// whether or not the pack spells `READS_BEFORE_WRITE`. The shipped
/// `append_to_collection` (`may_write` in `specs/sdc_base.tclspec`) is the
/// first program: O109 had deleted `set c {}`, and the procedure then
/// returned an unset variable whenever the call kept the collection as it
/// was.
#[test]
fn a_store_a_declared_outcome_may_preserve_is_live() {
    let shipped = tcl_spectcl::bundled::load_from(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs"),
    );
    let eda = "synopsys-eda-tcl";
    let registry = tcl_spectcl::bundled::registry_for_dialect_from(eda, &shipped);
    let source =
        "proc p {objs} {\n    set c {}\n    append_to_collection c $objs\n    return $c\n}\n";
    assert_eq!(
        dead_store_findings(&registry, shipped.key, source, eda, "c"),
        (false, false),
        "{source}"
    );

    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("kv", MAY_PRESERVE_PACK);
    let dialect = "tcl9.0";
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
    for command in ["kv::maybe", "kv::wp"] {
        for call in [
            format!("    {command} $s v\n"),
            format!("    if {{[{command} $s v]}} {{ puts hit }}\n"),
            format!("    set r [{command} $s v]\n    puts $r\n"),
        ] {
            let source = format!("proc p {{s}} {{\n    set v old\n{call}    return $v\n}}\n");
            assert_eq!(
                dead_store_findings(&registry, packs.key, &source, dialect, "v"),
                (false, false),
                "{source}"
            );
        }
    }

    // The no-match path's `preserve` keeps the prior value: `x` holds `old`
    // after the call, and the store stays.
    let source = "proc p {} {\n    set x old\n    kv::split3 nocolons x y z\n    return $x\n}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, dialect);
    assert_eq!(value_at(&unit, "::p", "x", 2), Some(text("old")));
    assert_eq!(answers_for(&unit, "::p", "kv::split3"), ["evaluated"]);
    assert_eq!(
        dead_store_findings(&registry, packs.key, source, dialect, "x"),
        (false, false)
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A materialised child carries an authenticated factory source span (#2143):
/// `Configure port 8080 {the port}` installs `proc port {x} {return 8080}`.
/// The child's W214 for `x` may anchor at the original formal or the factory
/// call, rather than at 1:1. A directly read formal and an `eval` script
/// with unknown caller-frame reads remain silent. The
/// factory's template is read through its template-word plan, and `port
/// ignored` is `8080` under tclsh 8.4 to 9.1 before and after `tcl opt`.
#[test]
fn a_materialised_child_carries_its_factory_call_span() {
    use tcl_compiler::analyser::Analyser;
    let source = "proc Configure {name default description} {\n    \
                  proc $name {x} [subst -nocommands {return $default}]\n}\n\
                  Configure port 8080 {the port}\n";
    let call_start = source.find("Configure port 8080 {the port}").unwrap();
    let call_end = call_start + "Configure port 8080 {the port}".len();
    let formal_start = source.find("{x}").unwrap() + 1;
    let formal_span = tcl_lexer::Span::new(
        u32::try_from(formal_start).unwrap(),
        u32::try_from(formal_start).unwrap() + 1,
    );
    let call_span = tcl_lexer::Span::new(
        u32::try_from(call_start).unwrap(),
        u32::try_from(call_end).unwrap(),
    );
    let read_source = "proc Configure {name} {proc $name {x} {return $x}}\nConfigure used\n";
    let dynamic_source =
        "proc Configure {name} {proc $name {x} {eval $::code}}\nConfigure delegated\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let reported = Analyser::new().analyse(source, dialect).diagnostics;
        let spans: Vec<_> = reported
            .iter()
            .filter(|d| d.code == DiagCode::W214 && d.message.contains("'::port'"))
            .map(|d| d.span)
            .collect();
        assert!(
            spans == [formal_span] || spans == [call_span],
            "{dialect}: {reported:?}"
        );
        for (label, control) in [("read", read_source), ("dynamic eval", dynamic_source)] {
            let reported = Analyser::new().analyse(control, dialect).diagnostics;
            assert!(
                !reported.iter().any(|d| {
                    d.code == DiagCode::W214
                        && (d.message.contains("'::used'") || d.message.contains("'::delegated'"))
                }),
                "{dialect}: {label}: {reported:?}"
            );
        }
    }
    prints_under_every_release(&format!("{source}puts [port ignored]\n"), "8080\n");
}

/// A computed template that runs commands can read any variable:
/// `subst -novariables $t` over `[set x]` reads `x`, so the store
/// before it stays — the original and optimised programs print `1` under
/// tclsh 8.4 to 9.1, where dropping `set x 1` as unused made them raise.
#[test]
fn a_computed_template_that_runs_commands_keeps_the_stores_it_reads() {
    let source = "proc f {t} {\n    set x 1\n    return [subst -novariables $t]\n}\n\
                  puts [f {[set x]}]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let (rewritten, _) = optimised(source, dialect);
        assert!(rewritten.contains("set x 1"), "{dialect}: {rewritten}");
    }
    prints_under_every_release(source, "1\n");
}

// The interface page's exit witnesses (its § *Test
// anchors*, "fixed witnesses to add"), each read off the shared lattice
// through the memoised unit and checked against `tcl opt`'s rewritten
// program, printed under every release on `PATH`.

/// The no-match preserve witness: a `regexp` that cannot match leaves its
/// match variables exactly as they were — the lattice holds `a`'s prior
/// value at the call's own definition (a `Preserve`, not a manufactured
/// one), and `zz` never matches `(x)(y)`, so both the original and the
/// `tcl opt`-rewritten program print `before` on every release.
#[test]
fn the_no_match_preserve_witness() {
    let source = "proc p {} {\n    set a before\n    regexp {(x)(y)} zz a b\n    puts $a\n}\np\n";
    for dialect in TCL_CORE_DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "a"),
            text("before"),
            "{dialect}: the no-match leaves $a at its prior value"
        );
    }
    prints_under_every_release(source, "before\n");
}

/// The partial-scan witness: `scan "12" "%d %d" a b` converts one field
/// from the two-integer format before the input is exhausted — `a` takes
/// the converted `12`, `b` is left exactly as it was (a `Preserve`, the
/// same outcome kind the no-match witness reads) — and both agree with
/// `tclsh` before and after `tcl opt`. Run as its own statement, not
/// nested in a `[…]` value position: a nested invocation's stores answer
/// under the effect-free policy `fold_cmd_subst_routes` documents, which
/// has no definition for them to land on, so `a` and `b` would be
/// `Overdefined` for a reason unrelated to what this witness proves.
#[test]
fn the_partial_scan_witness() {
    let source = "proc p {} {\n    set a before\n    set b before\n    \
                  scan \"12\" \"%d %d\" a b\n    puts \"$a $b\"\n}\np\n";
    for dialect in TCL_CORE_DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "a"),
            LatticeValue::Const(ConstValue::Int(12)),
            "{dialect}: the converted field takes the scanned value, typed as %d built it"
        );
        assert_eq!(
            last_value(source, dialect, "::p", "b"),
            text("before"),
            "{dialect}: the field past the exhausted input is preserved"
        );
    }
    prints_under_every_release(source, "12 before\n");
}

/// The conversion-count witness: `%n` is a
/// conversion for `scan`'s underflow, as C's `nconversions` counts it, so
/// `scan "" %n%d n a` is 1 and writes `n` the characters consumed, 0, on
/// every release. The route had answered the underflow's -1 and preserved
/// `n`, so `tcl opt` rewrote `puts $n` to `puts 5` and W210 reported the
/// read of an `n` the call binds.
#[test]
fn the_percent_n_count_witness() {
    use tcl_compiler::analyser::Analyser;
    let kept = "proc p {} {\n    set n 5\n    scan \"\" %n%d n a\n    puts $n\n}\np\n";
    let bound = "proc q {} {\n    scan \"\" %n%d n a\n    puts $n\n}\nq\n";
    for dialect in TCL_CORE_DIALECTS {
        assert_eq!(
            last_value(kept, dialect, "::p", "n"),
            LatticeValue::Const(ConstValue::Int(0)),
            "{dialect}: `%n` writes the characters consumed"
        );
        let (rewritten, rewrites) = optimised(kept, dialect);
        assert!(
            !rewritten.contains("puts 5"),
            "{dialect}: the scan wrote `n`\n{rewritten}\n{rewrites:#?}"
        );
        for source in [kept, bound] {
            let reported: Vec<String> = Analyser::new()
                .analyse(source, dialect)
                .diagnostics
                .into_iter()
                .filter(|d| d.code == DiagCode::W210 && d.message.contains("'n'"))
                .map(|d| d.message)
                .collect();
            assert!(reported.is_empty(), "{dialect}: {source}{reported:?}");
        }
    }
    prints_under_every_release(kept, "0\n");
    prints_under_every_release(bound, "0\n");
}

/// `const` writes only an absent place: the first `const c 5`
/// writes 5 into the absent `c` and the second keeps it — tclsh 9.0 and
/// 9.1 print 5 twice — so the lattice holds 5 after the first and no value
/// after the second, and `tcl opt` never prints 7. `set x 1; const x 2`
/// raises on both releases, and the route declines it.
#[test]
fn const_writes_only_an_absent_place() {
    let source = "proc p {} {\n    const c 5\n    puts $c\n    const c 7\n    puts $c\n}\np\n";
    let raises = "proc q {} {\n    set x 1\n    const x 2\n}\nq\n";
    for dialect in ["tcl9.0", "tcl9.1"] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "c", 1).and_then(lattice_text),
            Some("5".to_owned()),
            "{dialect}: the absent place takes the value"
        );
        assert_eq!(
            value_at(&unit, "::p", "c", 2).and_then(lattice_text),
            None,
            "{dialect}: an existing constant keeps a value the route does not model"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts 7"),
            "{dialect}: {rewritten}\n{rewrites:#?}"
        );
        assert!(
            answers_for(&unit_of(raises, dialect), "::q", "const")
                .iter()
                .all(|answer| answer.starts_with("declined")),
            "{dialect}: an existing variable declines"
        );
    }
    for (series, tclsh) in releases_on_path() {
        if !series.starts_with('9') {
            continue;
        }
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, "5\n5\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
        assert_eq!(
            run_script(&tclsh, raises).map(|(ok, _)| ok),
            Some(false),
            "tclsh{series}: const over an existing variable raises"
        );
    }
}

/// The repeated-target witness: a call whose declared targets name the
/// same place twice composes in execution order, so the last position's
/// value wins. `lassign`'s repeated-target form (the interface page's own
/// "`lassign … a a`") is 8.5+ and raises under 8.4, so this reads the same
/// fact through `regexp`'s match-variable list, which every release
/// shares: `(a)(b)` against `ab` captures `a` into `x` and then `b` into
/// `x` again, and `x` ends the call as `b`.
#[test]
fn the_repeated_target_witness() {
    let source = "proc p {} {\n    regexp {(a)(b)} ab -> x x\n    puts $x\n}\np\n";
    for dialect in TCL_CORE_DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "x"),
            text("b"),
            "{dialect}: the second position's capture is the one that survives"
        );
    }
    prints_under_every_release(source, "b\n");
}
