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

//! Variables written into the caller's frame through `upvar` and `uplevel`: no false W210 or W212, and the controls that still fire.

use super::*;

// Caller-frame injection through `upvar` / `uplevel`.
//
// The compiler-unit twins live in `tcl-compiler/tests/it_4/caller_frame_effects.rs`.
// These pin the same facts through the **live server**, whose incremental
// per-item analysis is a different code path from the whole-file walk those
// tests drive — a regression confined to the diagnostic-publishing pipeline
// would be invisible to them.

/// The `setdef` / `estruct` out-parameter shape: `upvar 1 $name local`
/// plus a write through the alias, called with a literal name. tclsh
/// 9.0.4 / 8.6.16: `things::estruct itemLegend1 tree1` leaves
/// `itemLegend1` set in the caller.
#[test]
fn an_upvar_out_parameter_draws_no_false_read_before_set_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace eval things {\n    proc estruct {name value} {\n        upvar 1 $name obj\n        set obj [dict create v $value]\n    }\n}\nproc build {} {\n    things::estruct itemLegend1 tree1\n    return [dict get $itemLegend1 v]\n}\n",
    );
    assert!(
        !has_code(&diags, "W210"),
        "the helper's upvar creates `itemLegend1` in build's frame: {:?}",
        codes(&diags)
    );
}

/// TP control for the test above — the suppression is per-name, so a local
/// the helper never touches is still read-before-set (tclsh 9.0.4 / 8.6.16
/// raise `can't read "neverSet"`).
#[test]
fn an_unrelated_local_still_draws_read_before_set_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc estruct {name value} {\n    upvar 1 $name obj\n    set obj $value\n}\nproc build {} {\n    estruct itemLegend1 tree1\n    return $neverSet\n}\n",
    );
    let hits = with_code(&diags, "W210");
    assert_eq!(hits.len(), 1, "exactly one W210: {:?}", codes(&diags));
    assert!(
        message(&hits[0]).contains("neverSet"),
        "and it must be the unrelated local: {:?}",
        message(&hits[0])
    );
}

/// `argparse` injects caller-frame locals from its own definition
/// list, none of which the proc's own text assigns. tclsh 9.0.4 / 8.6.16
/// (argparse 0.5): `upvarProc p 1 2` prints `a=… b=1 c=2`.
#[test]
fn argparse_injected_locals_draw_no_false_read_before_set_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require argparse\nproc upvarProc {args} {\n    argparse {\n        {a -upvar}\n        b\n        c\n    }\n    puts \"a=$a b=$b c=$c\"\n}\n",
    );
    assert!(
        !has_code(&diags, "W210"),
        "argparse's DSL creates a/b/c in this very frame: {:?}",
        codes(&diags)
    );
}

/// The **cross-file** half:
/// `setdef` lives in `utils.tcl`, the caller in `options.tcl`, and only a
/// `pkgIndex.tcl` ties them together, so the caller's compilation unit
/// holds no definition of `setdef` at all.  tclsh 9.0.4 / 8.6.16 running
/// the real four-file layout: `demo::build {a b c}` returns
/// `{nothing str no} {nothing str no} {nothing str no}`.
#[test]
fn a_cross_file_upvar_helper_draws_no_false_read_before_set_end_to_end() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-xfile-upvar-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).expect("create workspace root");
    std::fs::write(
        root.join("utils.tcl"),
        "namespace eval demo {}\nproc demo::setdef {d key args} {\n    upvar 1 $d _dict\n    dict set _dict $key $args\n}\n",
    )
    .expect("write utils.tcl");
    let options = root.join("options.tcl");
    let text = "proc demo::build {items} {\n    set opts {}\n    foreach item $items {\n        setdef options name -default nothing\n        lappend opts [dict get $options name]\n        set options {}\n    }\n    return $opts\n}\n";
    std::fs::write(&options, text).expect("write options.tcl");

    let mut lsp = Lsp::at_workspace_root(&root);
    let uri = tcl_lsp_server::canonical_file_uri(&options)
        .expect("options fixture has a representable file URI")
        .to_string();
    let diags = lsp.open_ready(&uri, text);
    assert!(
        !has_code(&diags, "W210"),
        "a helper this file cannot see may create `options` through upvar: {:?}",
        codes(&diags)
    );

    // TP control, same session: a name the opaque call never spells is
    // still read-before-set.
    let control = root.join("control.tcl");
    let control_text = "proc demo::build2 {} {\n    setdef options name -default nothing\n    return $neverPassed\n}\n";
    std::fs::write(&control, control_text).expect("write control.tcl");
    let control_uri = tcl_lsp_server::canonical_file_uri(&control)
        .expect("control fixture has a representable file URI")
        .to_string();
    let control_diags = lsp.open_ready(&control_uri, control_text);
    let hits = with_code(&control_diags, "W210");
    assert_eq!(
        hits.len(),
        1,
        "exactly one W210: {:?}",
        codes(&control_diags)
    );
    assert!(
        message(&hits[0]).contains("neverPassed"),
        "and it must be the name the call never spells: {:?}",
        message(&hits[0])
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A variable assigned indirectly through
/// `uplevel` in an outer frame is not read-before-set, however many ordinary
/// call frames sit between the writer and the frame it writes.
///
/// Oracle (tclsh 9.0.4 and 8.6.16, byte-identical): the script below prints
/// `99` — `answer` really is set in `outer`'s frame by a proc `outer` never
/// calls directly.
#[test]
fn w210_not_published_for_a_multi_frame_uplevel_assignment() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc setUp2 {var} {\n    uplevel 2 [list set $var 99]\n}\n\
         proc middle {} {\n    setUp2 answer\n}\n\
         proc outer {} {\n    middle\n    return $answer\n}\n",
    );
    assert!(
        !has_code(&diags, "W210"),
        "`uplevel 2` writes the plain caller's caller: {diags:?}"
    );
}

/// The one-hop spellings of the same idiom, both `[list …]`-built and
/// word-concatenated, must also stay silent.
#[test]
fn w210_not_published_for_a_single_hop_uplevel_assignment() {
    for body in [
        "uplevel 1 [list set $var 99]",
        "uplevel 1 set $var 99",
        "uplevel [expr {[info level] - 1}] [list set $var 99]",
    ] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(
            &uri,
            &format!(
                "proc setInCaller {{var}} {{\n    {body}\n}}\n\
                 proc useIt {{}} {{\n    setInCaller answer\n    return $answer\n}}\n"
            ),
        );
        assert!(
            !has_code(&diags, "W210"),
            "`{body}` defines the caller's variable: {diags:?}"
        );
    }
}

/// TP control for both of the above: with the `uplevel` replaced by a
/// callee-local `set`, nothing reaches the caller's frame and W210 must
/// still be published.
#[test]
fn w210_still_published_when_nothing_writes_the_outer_frame() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc setInCaller {var} {\n    set $var 99\n}\n\
         proc useIt {} {\n    setInCaller answer\n    return $answer\n}\n",
    );
    assert!(
        has_code(&diags, "W210"),
        "a callee-local `set` never reaches the caller: {diags:?}"
    );
}

/// W212 must not read the literal word `set` inside a
/// `[list …]`-built script as a directly-written `set $var` confusion — the
/// substitution happens in the building frame and is the idiom itself.
#[test]
fn w212_not_published_for_a_list_built_variable_name_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc setInCaller {var} {\n    uplevel 1 [list set $var 99]\n}\n",
    );
    assert!(
        !has_code(&diags, "W212"),
        "a list-built name word is pre-substituted, not a confusion: {diags:?}"
    );
}

/// TP control: the directly-written spelling is exactly what W212 is for.
#[test]
fn w212_still_published_for_a_directly_written_name_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc setInCaller {var} {\n    set $var 99\n}\n");
    assert!(
        has_code(&diags, "W212"),
        "a written `$` in a name position is still W212: {diags:?}"
    );
}

/// The canonical caller-frame-injection idiom
/// the `tclopt` corpus is built from — a `[list set $varName $value]` built
/// inside a `foreach`, with both words substituted.
///
/// Oracle (tclsh 9.0.4 and 8.6.16): the script prints `1 1 1`, so all three
/// names really are materialised in `Qfrac2`'s frame. Neither the
/// read-before-set nor the name/value check may fire.
#[test]
fn neither_w210_nor_w212_is_published_for_a_looped_list_built_injection() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc NewArrays {varNames value} {\n\
         \x20   foreach varName $varNames {\n\
         \x20       uplevel 1 [list set $varName $value]\n\
         \x20   }\n\
         }\n\
         proc Qfrac2 {} {\n\
         \x20   NewArrays {rdiagArray acnormArray waArray} 1\n\
         \x20   return [list $rdiagArray $acnormArray $waArray]\n\
         }\n",
    );
    assert!(
        !has_code(&diags, "W212"),
        "a list-built name word is the idiom, not a confusion: {diags:?}"
    );
    assert!(
        !has_code(&diags, "W210"),
        "the three names are materialised in the caller's frame: {diags:?}"
    );
}

/// TP control: with the injecting call removed, nothing writes those names
/// and all three reads are genuinely unset (tclsh: `can't read
/// "rdiagArray": no such variable`).
#[test]
fn w210_still_published_without_the_injecting_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc Qfrac2 {} {\n\
         \x20   return [list $rdiagArray $acnormArray $waArray]\n\
         }\n",
    );
    assert!(
        has_code(&diags, "W210"),
        "nothing writes those names: {diags:?}"
    );
}

/// TP control for the W212 carve-out in the same proc shape: the directly
/// written `set $varName $value` is exactly the confusion the code is for.
#[test]
fn w212_still_published_for_a_directly_written_looped_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc NewArrays {varNames value} {\n\
         \x20   foreach varName $varNames {\n\
         \x20       set $varName $value\n\
         \x20   }\n\
         }\n",
    );
    assert!(
        has_code(&diags, "W212"),
        "a written `$` in a name position is still W212: {diags:?}"
    );
}
