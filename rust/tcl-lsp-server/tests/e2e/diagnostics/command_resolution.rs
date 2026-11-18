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

//! Unknown commands (W123, W143) and what resolves one: created objects, scoped command environments, `tclIndex` autoload, package indexes and renames.

use super::*;

// `CLASS create NAME` binds a command NAME.

#[test]
fn w307_silent_for_create_named_objects_iterated() {
    // Object commands are bound by `C create c1`, then iterated via
    // `foreach elem [list c1 l1 …]` and dispatched through `$elem`. The
    // created names are known commands, so W307 must not fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
C create c1 1 out 0 -c 1e-9
L create l1 1 out 0 -l 10e-6
C create c2 2 n002 0 -c 1e-9
foreach elem [list c1 l1 c2] {
    $elem actOnParam -set 1
}
";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W307"),
        "dispatch over created object names must not fire W307: {:?}",
        codes(&diags),
    );
}

#[test]
fn w123_silent_for_create_named_object_literal_dispatch() {
    // A known class's `create NAME` binds `NAME`; a literal `NAME method` call
    // is a real command, not an unknown (W123).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C { method configure args {} }\nC create c1\nc1 configure -x 2\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !on_line(&diags, "W123").contains(&2),
        "`c1 configure` on line 2 must not be an unknown command: {:?}",
        codes(&diags),
    );
}

// `report::defstyle` scoped command environment.

#[test]
fn defstyle_body_scoped_commands_no_w123() {
    // The report configuration methods (top/data/columns/…) are scoped
    // commands inside the style script — no unknown-command diagnostic.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "::report::defstyle simpletable {} {\n\
               \x20   top set [split \"x\"]\n\
               \x20   data set [split \"y\"]\n\
               \x20   bottom enable\n\
               \x20   topdatasep enable\n\
               \x20   columns\n\
               }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W123"),
        "no W123 in a valid style body: {diags:?}"
    );
}

#[test]
fn defstyle_body_typo_still_w123() {
    // A genuine typo inside the body is still an unknown command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::report::defstyle st {} {\n    toop set x\n}\n");
    assert!(has_code(&diags, "W123"), "typo `toop` flagged: {diags:?}");
    assert!(
        diags.iter().any(|d| message(d).contains("toop")),
        "message names the typo: {diags:?}",
    );
}

#[test]
fn w123_call_chain_reaching_a_command_before_its_deletion_issue_1015() {
    // `inner` is never invoked at the top level — only `outer` is — but
    // `outer` calls `inner`, which calls `helper`, all before the `rename
    // helper {}`. tclsh 8.6/9.0 both run this clean (exit 0, no error), so
    // the analyser must not draw W123 on `helper`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc helper {} { return hi }\nproc inner {} { helper }\nproc outer {} { inner }\nouter\nrename helper {}\n",
    );
    assert!(
        !has_code(&diags, "W123"),
        "outer's top-level call reaches helper two bodies deep, before the rename: {diags:?}"
    );
}

#[test]
fn w123_unreached_mutual_recursion_cycle_still_flags_issue_1015() {
    // TP control for the same fix: neither cycle member is ever called, so
    // `helper` is genuinely unreachable before the rename and the call must
    // still be flagged — the reachability query must terminate on the
    // cycle, not treat it as proof of reachability.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc helper {} {}\nproc pingCaller {} { helper\n pongCaller }\nproc pongCaller {} { pingCaller }\nrename helper {}\n",
    );
    assert!(
        has_code(&diags, "W123"),
        "an unreached cycle proves nothing; helper is deleted: {diags:?}"
    );
}

#[test]
fn defstyle_bad_operation_is_w001() {
    // `top bogus` — unknown ensemble operation of a scoped command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::report::defstyle st {} {\n    top bogus\n}\n");
    assert!(
        has_code(&diags, "W001"),
        "unknown scoped op → W001: {diags:?}"
    );
}

#[test]
fn report_object_methods_have_no_unknown_command() {
    // `report::report` binds `r`; `r <method>` resolves via the object class.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require report\n::report::report r 3\nr data set x\nr printmatrix m\n",
    );
    assert!(
        !has_code(&diags, "W123"),
        "report object methods resolve: {diags:?}"
    );
}

// A command a `tclIndex` on the configured `libraryPaths` auto-loads (the
// BLT/Rbc idiom: `Rbc_ZoomStack` / `Rbc_ActiveLegend`) must not be flagged
// "Unknown command" (W123), end-to-end against the packaged server, with
// `xcDiagnostics` left at its default (off). The package database resolves
// the command exactly as go-to-definition does; the diagnostic must agree.

/// Per-call counter so repeat runs don't collide on the temp dir.
static RBC_LIB_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn autoload_library_command_not_unknown_issue_832() {
    use std::sync::atomic::Ordering;

    // A library dir with a `tclIndex` that auto-loads two global procs by bare
    // name — no `package require` needed, the exact shape from the issue.
    let libdir = crate::common::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-rbc-{}-{}",
        std::process::id(),
        RBC_LIB_N.fetch_add(1, Ordering::Relaxed)
    )));
    let pkgdir = libdir.join("rbc");
    std::fs::create_dir_all(&pkgdir).expect("mk rbc lib dir");
    std::fs::write(
        pkgdir.join("graph.tcl"),
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_ZoomStack {graph args} {}\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        pkgdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");

    // Point the server's auto_path (`libraryPaths`) at the library's parent dir;
    // `scan_path` descends into the `rbc/` subdir (C-Tcl auto_path rule) at
    // startup, so its `tclIndex` enters the package database.
    let mut lsp = Lsp::with_config(serde_json::json!({
        "features": { "linkedEditingRange": true },
        "libraryPaths": [ libdir.to_str().expect("libraryPaths requires a UTF-8 path") ],
    }));

    let uri = unique_uri("tcl");
    let src = "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n";
    lsp.open_document(&uri, src);

    // The package database is (re)built asynchronously at startup; poll the
    // deterministic pull path until it is live (W123 clears) or the deadline.
    let mut diags = lsp.pull_diagnostics(&uri);
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    while has_code(&diags, "W123") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }
    assert!(
        !has_code(&diags, "W123"),
        "auto_path library commands must not be W123 (#832), got: {:?}",
        codes(&diags),
    );

    // Control: a typo the `tclIndex` does not declare still fires W123 — the
    // database is loaded and the check is data-driven, not disabled wholesale.
    let typo = unique_uri("tcl");
    let tdiags = lsp.open_ready(&typo, "Rbc_ActveLegend .g\n");
    assert!(
        has_code(&tdiags, "W123"),
        "a command no index declares must still be W123, got: {:?}",
        codes(&tdiags),
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

/// Per-call counter for the autoload go-to-definition fixture dir.
static AUTOLOAD_DEF_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn autoload_library_command_go_to_definition_m8() {
    use std::sync::atomic::Ordering;

    // A library dir whose `tclIndex` auto-loads a global proc
    // `Rbc_ActiveLegend`, defined on line 0 of `graph.tcl`. The command is
    // defined nowhere in the open workspace, so go-to-definition must fall
    // through to the autoload tier (M8) and jump into the library file.
    let libdir = crate::common::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-autoload-def-{}-{}",
        std::process::id(),
        AUTOLOAD_DEF_N.fetch_add(1, Ordering::Relaxed)
    )));
    let pkgdir = libdir.join("rbc");
    std::fs::create_dir_all(&pkgdir).expect("mk lib dir");
    std::fs::write(
        pkgdir.join("graph.tcl"),
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_ZoomStack {graph args} {}\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        pkgdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");

    let mut lsp = Lsp::with_config(serde_json::json!({
        "libraryPaths": [ libdir.to_str().expect("libraryPaths requires a UTF-8 path") ],
    }));

    let uri = unique_uri("tcl");
    let src = "Rbc_ActiveLegend .g\n";
    lsp.open_document(&uri, src);

    // The package database loads asynchronously at startup; poll (via the W123
    // clearing on the pull path) until it is live, then go-to-definition.
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    let mut diags = lsp.pull_diagnostics(&uri);
    while has_code(&diags, "W123") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }

    // Cursor on the `Rbc_ActiveLegend` call head (line 0, col 2).
    let locs = crate::common::helpers::locations(&lsp.definition(&uri, 0, 2));
    assert!(
        !locs.is_empty(),
        "autoload go-to-definition must resolve the library proc (M8); diags {:?}",
        codes(&diags),
    );
    assert!(
        locs[0].uri.ends_with("graph.tcl"),
        "must jump into the library file graph.tcl, got {}",
        locs[0].uri,
    );
    let line = locs[0]
        .range
        .get("start")
        .and_then(|s| s.get("line"))
        .and_then(Value::as_i64);
    assert_eq!(
        line,
        Some(0),
        "Rbc_ActiveLegend is declared on line 0 of graph.tcl",
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

// A `pkgIndex.tcl` that gates its `package ifneeded` behind
// `package vsatisfies [package provide Tcl] …` supplies its commands only on
// the releases the guard admits, so whether a call is "unknown" depends on the
// dialect the workspace targets.

/// Per-call counter for the guarded-pkgIndex fixture dir.
static GUARDED_PKG_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Build a library dir whose `pkgIndex.tcl` carries the tcllib `file::home`
/// gate — `if {[package vsatisfies [package provide Tcl] 9-]} { return }` —
/// in front of its only `package ifneeded`.
///
/// Oracle (both shapes verified by hand): `tclsh8.6` loads `mypkg` and runs
/// `mypkgHello`; `tclsh9.0` fails with `can't find package mypkg`, so the call
/// is a genuine `invalid command name` error there.
fn guarded_pkg_libdir() -> crate::common::ScratchDir {
    pkg_libdir_with_index(
        "if {[package vsatisfies [package provide Tcl] 9-]} { return }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
    )
}

/// Build a `mypkg` library dir whose `pkgIndex.tcl` is exactly `index`, next to
/// a `mypkg.tcl` defining `mypkgHello`.
fn pkg_libdir_with_index(index: &str) -> crate::common::ScratchDir {
    use std::sync::atomic::Ordering;
    let libdir = crate::common::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-guardedpkg-{}-{}",
        std::process::id(),
        GUARDED_PKG_N.fetch_add(1, Ordering::Relaxed)
    )));
    let pkgdir = libdir.join("mypkg");
    std::fs::create_dir_all(&pkgdir).expect("mk mypkg lib dir");
    std::fs::write(pkgdir.join("pkgIndex.tcl"), index).expect("write pkgIndex.tcl");
    std::fs::write(
        pkgdir.join("mypkg.tcl"),
        "proc mypkgHello {} { return hi }\n",
    )
    .expect("write mypkg.tcl");
    libdir
}

/// Open `package require mypkg` + `source <child>` in one document and the
/// bare `mypkgHello` call in another, then poll the child's diagnostics until
/// the asynchronously-built package database has settled.
///
/// The call must live in a file with no `package require` of its own — the
/// analyser drops every W123 in a file that has one — so the requirement
/// reaches it through the `source` ancestor.
fn guarded_pkg_child_diagnostics(dialect: &str, libdir: &std::path::Path) -> Vec<Value> {
    let mut lsp = Lsp::with_config(serde_json::json!({
        "dialect": dialect,
        "libraryPaths": [ libdir.to_str().expect("libraryPaths requires a UTF-8 path") ],
    }));
    let child = unique_uri("tcl");
    let child_name = child.rsplit('/').next().expect("child basename").to_owned();
    let entry = unique_uri("tcl");
    lsp.open_ready(
        &entry,
        &format!("package require mypkg\nsource {child_name}\n"),
    );
    let child_source = "mypkgHello\n";
    lsp.open_document(&child, child_source);

    lsp.current_complete_diagnostics(&child, 1, child_source, Duration::from_secs(15))
}

/// TP: under a 9.x target the guard's `return` fires, `mypkg`
/// never registers, and the call really is an unknown command.
#[test]
fn guarded_pkgindex_unavailable_on_target_still_fires_w123_issue_1017() {
    let libdir = guarded_pkg_libdir();
    let diags = guarded_pkg_child_diagnostics("tcl9.0", &libdir);
    assert!(
        has_code(&diags, "W123"),
        "a package its pkgIndex guard excludes on 9.x must not suppress W123 (#1017), got: {:?}",
        codes(&diags),
    );
    let _ = std::fs::remove_dir_all(&libdir);
}

/// TN: under an 8.6 target the same guard does
/// not fire, the package registers, and flagging the call would be a false
/// positive.
#[test]
fn guarded_pkgindex_available_on_target_suppresses_w123_issue_1017() {
    let libdir = guarded_pkg_libdir();
    let diags = guarded_pkg_child_diagnostics("tcl8.6", &libdir);
    assert!(
        !has_code(&diags, "W123"),
        "a package the guard admits on 8.6 must suppress W123, got: {:?}",
        codes(&diags),
    );
    let _ = std::fs::remove_dir_all(&libdir);
}

/// TN — Tcl 9's own core-package index shape: the `package ifneeded` lives
/// inside `apply {{dir} { … foreach … }} $dir` (the zipfs
/// `tcl_library/pkgIndex.tcl`).  The declaration is only *conditional* — this
/// analysis models neither the `apply` frame nor the loop — and a conditional
/// registration counts as loadable, so flagging the call would be a false
/// positive.
///
/// Oracle: `tclsh9.0` sources exactly this shape at startup and
/// `package require http` works, which is what makes an "unknown command"
/// verdict on such a package wrong.
#[test]
fn apply_wrapped_pkgindex_declaration_suppresses_w123() {
    let libdir = pkg_libdir_with_index(
        "apply {{dir} {\n\
         \x20 foreach f {mypkg.tcl} {\n\
         \x20   package ifneeded mypkg 1.0 [list source [file join $dir $f]]\n\
         \x20 }\n\
         }} $dir\n",
    );
    let diags = guarded_pkg_child_diagnostics("tcl9.0", &libdir);
    assert!(
        !has_code(&diags, "W123"),
        "a package declared inside an apply/foreach body must suppress W123, got: {:?}",
        codes(&diags),
    );
    let _ = std::fs::remove_dir_all(&libdir);
}

/// TN — a guard naming a **patch level** cannot be decided against the
/// `major.minor` release model, so it must not decide the package away.
///
/// Oracle (`tclsh9.0`, `[package provide Tcl]` = 9.0.4):
/// `package vsatisfies 9.0.4 9.0.1-` = 1, so the real interpreter takes the
/// guard's true arm, `mypkg` registers, and `mypkgHello` resolves.
#[test]
fn patch_level_guarded_pkgindex_suppresses_w123() {
    let libdir = pkg_libdir_with_index(
        "if {![package vsatisfies [package provide Tcl] 9.0.1-]} { return }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
    );
    let diags = guarded_pkg_child_diagnostics("tcl9.0", &libdir);
    assert!(
        !has_code(&diags, "W123"),
        "a patch-level guard must abstain rather than exclude the package, got: {:?}",
        codes(&diags),
    );
    let _ = std::fs::remove_dir_all(&libdir);
}

/// TN — the shape `georgtree/tclopt`'s real `pkgIndex.tcl.in` has: a
/// TEA-style version branch whose **both** arms declare the same `package
/// ifneeded`. The package is available whichever arm runs, so flagging its
/// commands is a false positive.
///
/// This e2e test exists because the both-arms shape is otherwise pinned
/// only by a `package_resolver::reachability` unit test, which would not
/// catch a regression in how the server's diagnostics pipeline consumes
/// that module.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, `TCLLIBPATH` pointing at the fixture):
/// `package require mypkg` succeeds and `mypkgHello` returns `hi` on both,
/// exactly as for a flat unguarded index.
#[test]
fn both_arms_of_a_version_branch_declaring_the_package_suppress_w123_923_idx42() {
    let libdir = pkg_libdir_with_index(
        "if {[package vsatisfies [package provide Tcl] 9-]} {\n\
         \x20   package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n\
         } else {\n\
         \x20   package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n\
         }\n",
    );
    for dialect in ["tcl8.6", "tcl9.0"] {
        let diags = guarded_pkg_child_diagnostics(dialect, &libdir);
        assert!(
            !has_code(&diags, "W123"),
            "{dialect}: a package declared in every arm of a version branch is \
             always available and must suppress W123, got: {:?}",
            codes(&diags),
        );
    }
    let _ = std::fs::remove_dir_all(&libdir);
}

/// TP control for the above — the same both-arms index, but the sourced file
/// does not define the called command. The suppression must be
/// *command-specific* (the package resolved, this command genuinely is not in
/// it), not a blanket "package involved, stay quiet".
#[test]
fn a_command_absent_from_a_branch_declared_package_still_fires_w123_923_idx42() {
    let libdir = pkg_libdir_with_index(
        "if {[package vsatisfies [package provide Tcl] 9-]} {\n\
         \x20   package ifneeded mypkg 1.0 [list source [file join $dir other.tcl]]\n\
         } else {\n\
         \x20   package ifneeded mypkg 1.0 [list source [file join $dir other.tcl]]\n\
         }\n",
    );
    // `other.tcl` exists but defines something else entirely.
    std::fs::write(
        libdir.join("mypkg").join("other.tcl"),
        "proc somethingElse {} { return hi }\n",
    )
    .expect("write other.tcl");
    let diags = guarded_pkg_child_diagnostics("tcl9.0", &libdir);
    assert!(
        has_code(&diags, "W123"),
        "a command the resolved package does not provide must still be flagged, got: {:?}",
        codes(&diags),
    );
    let _ = std::fs::remove_dir_all(&libdir);
}

/// TN — a package whose `pkgIndex.tcl` only `load`s a binary extension has
/// **no** companion `.tcl` file at all (`pix`'s real shape). Such a
/// declaration is still a real, known package: treating it as unknowable
/// made the whole document's W120
/// "requires `package require …`" diagnostics vanish, including ones about
/// entirely unrelated packages.
///
/// The two committed regression tests for this call `parse_pkg_index` /
/// `refine_w120_diagnostics` directly; this drives the same fact through the
/// real server, a real on-disk library scan and the pull-diagnostics path.
#[test]
fn a_load_only_package_does_not_suppress_unrelated_w120s_923_idx72() {
    use std::sync::atomic::Ordering;
    let libdir = crate::common::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-loadonlypkg-{}-{}",
        std::process::id(),
        GUARDED_PKG_N.fetch_add(1, Ordering::Relaxed)
    )));
    let pkgdir = libdir.join("testpix");
    std::fs::create_dir_all(&pkgdir).expect("mk testpix lib dir");
    // Deliberately the *only* file in the directory — no companion `.tcl`.
    std::fs::write(
        pkgdir.join("pkgIndex.tcl"),
        "package ifneeded testpix 0.8 \
         [list apply {dir {load [file join $dir libtestpix.so] Pix}} $dir]\n",
    )
    .expect("write pkgIndex.tcl");

    let mut lsp = Lsp::with_config(serde_json::json!({
        "libraryPaths": [ libdir.to_str().expect("libraryPaths requires a UTF-8 path") ],
    }));
    let baseline_uri = unique_uri("tcl");
    lsp.open_ready(&baseline_uri, "http::register https 443 ::tls::socket\n");
    let baseline = lsp.pull_diagnostics(&baseline_uri);
    assert!(
        has_code(&baseline, "W120"),
        "baseline: an `http::register` with no `package require http` must draw \
         W120, got: {:?}",
        codes(&baseline),
    );

    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require testpix\nhttp::register https 443 ::tls::socket\n",
    );
    // The package database loads asynchronously; poll until the answer settles.
    let mut diags = lsp.pull_diagnostics(&uri);
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        let next = lsp.pull_diagnostics(&uri);
        if has_code(&next, "W120") {
            diags = next;
            break;
        }
        diags = next;
    }
    assert!(
        has_code(&diags, "W120"),
        "requiring a load-only package must not silence an unrelated W120, got: {:?}",
        codes(&diags),
    );
    let _ = std::fs::remove_dir_all(&libdir);
}

/// Per-call counter for the autoload references/rename fixture dir.
static AUTOLOAD_REFS_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// M8's second half: once the autoload tier resolves a library command, the
/// defining library file is merged into the workspace index, so
/// **find-references** reaches the library declaration and the library's own
/// call sites, and **rename** rewrites them alongside the workspace call.
#[test]
fn autoload_library_command_references_and_rename_m8() {
    use std::sync::atomic::Ordering;

    let libdir = crate::common::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-autoload-refs-{}-{}",
        std::process::id(),
        AUTOLOAD_REFS_N.fetch_add(1, Ordering::Relaxed)
    )));
    let pkgdir = libdir.join("rbc");
    std::fs::create_dir_all(&pkgdir).expect("mk lib dir");
    // `Rbc_Wire` calls `Rbc_ActiveLegend` inside the library, so the library
    // contributes an internal call site of its own (line 1).
    std::fs::write(
        pkgdir.join("graph.tcl"),
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_Wire {} { Rbc_ActiveLegend .g }\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        pkgdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_Wire) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");

    let mut lsp = Lsp::with_config(serde_json::json!({
        "libraryPaths": [ libdir.to_str().expect("libraryPaths requires a UTF-8 path") ],
    }));

    let uri = unique_uri("tcl");
    let src = "Rbc_ActiveLegend .g\n";
    lsp.open_document(&uri, src);

    // The package database loads asynchronously at startup; poll (via the W123
    // clearing on the pull path) until it is live.
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    let mut diags = lsp.pull_diagnostics(&uri);
    while has_code(&diags, "W123") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }

    // References from the workspace call site (cursor on the head, incl. the
    // declaration): the library's declaration (line 0) and its internal call
    // site (line 1) must both surface.
    let refs = crate::common::helpers::locations(&lsp.references(&uri, 0, 2, true));
    let lib_lines: Vec<i64> = refs
        .iter()
        .filter(|l| l.uri.ends_with("graph.tcl"))
        .filter_map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect();
    assert!(
        lib_lines.contains(&0),
        "library declaration must be a reference target, got {refs:?}",
    );
    assert!(
        lib_lines.contains(&1),
        "library-internal call site must be a reference, got {refs:?}",
    );

    // Rename from the same cursor rewrites the workspace call *and* both
    // library sites, so the family stays consistent.
    let edits = crate::common::helpers::rename_edits(&lsp.rename(&uri, 0, 2, "Rbc_Shiny"));
    let lib_edit_count = edits
        .iter()
        .filter(|(u, _)| u.ends_with("graph.tcl"))
        .map(|(_, es)| es.len())
        .next()
        .unwrap_or(0);
    assert_eq!(
        lib_edit_count, 2,
        "library declaration + internal call must be rewritten, got {edits:?}",
    );
    assert!(
        edits.iter().any(|(u, _)| *u == uri),
        "the workspace call site is rewritten too: {edits:?}",
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

/// Per-call counter for the workspace-folder consumer-rename fixture dir.
static WS_FOLDER_RENAME_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The VS Code fixture shape for M8's consumer rename, driven end-to-end
/// through a REAL workspace folder (no `libraryPaths` config): the library
/// lives in a `rbclib/` subdirectory of the folder (with a `tclIndex`), the
/// consumer at the folder root calls `Rbc_ActiveLegend` bare, and a rename at
/// the consumer's call site must rewrite the library declaration.  Mirrors
/// `editors/vscode/src/test/renameSymbol.test.ts`'s M8 test so the
/// environment-specific path (workspace-scan indexing, not the
/// `libraryPaths` autoload tier) is pinned in the fast harness too.
#[test]
fn consumer_rename_resolves_through_a_real_workspace_folder_m8() {
    use std::sync::atomic::Ordering;

    let root = crate::common::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-ws-rename-{}-{}",
        std::process::id(),
        WS_FOLDER_RENAME_N.fetch_add(1, Ordering::Relaxed)
    )));
    let libdir = root.join("rbclib");
    std::fs::create_dir_all(&libdir).expect("mk lib dir");
    std::fs::write(
        libdir.join("graph.tcl"),
        "# Auto-loaded library procs (BLT/Rbc idiom) — reachable by bare name via the\n\
         # sibling tclIndex, with no `package require` in the caller.\n\
         proc Rbc_ActiveLegend {graph} {}\n\
         proc Rbc_ZoomStack {graph args} {}\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        libdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");
    let consumer_path = root.join("autoloadLibrary.tcl");
    std::fs::write(&consumer_path, "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n")
        .expect("write consumer");

    let mut lsp = Lsp::tcl();
    // Attach the real folder (the fake `initialize` root indexes nothing) and
    // let the rescan index rbclib/graph.tcl from disk.
    let folder_uri = tcl_lsp_server::canonical_file_uri(&root)
        .expect("workspace fixture has a representable file URI")
        .to_string();
    lsp.notify(
        "workspace/didChangeWorkspaceFolders",
        serde_json::json!({
            "event": {
                "added": [{ "uri": folder_uri, "name": "fixture" }],
                "removed": []
            }
        }),
    );

    let uri = tcl_lsp_server::canonical_file_uri(&consumer_path)
        .expect("consumer fixture has a representable file URI")
        .to_string();
    lsp.open_document(&uri, "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n");

    // The folder scan runs asynchronously; poll the rename until the library
    // edit appears (or the deadline), exactly like the editor test.
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(15));
    let mut lib_lines: Vec<i64> = Vec::new();
    while std::time::Instant::now() < deadline && lib_lines.is_empty() {
        let resp = lsp.request(
            "textDocument/rename",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 },
                "newName": "Rbc_ShinyLegend"
            }),
        );
        if let Some(changes) = resp.get("changes").and_then(|c| c.as_object()) {
            for (target, edits) in changes {
                if target.ends_with("graph.tcl") {
                    lib_lines = edits
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.pointer("/range/start/line").and_then(Value::as_i64))
                        .collect();
                }
            }
        }
        if lib_lines.is_empty() {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    assert!(
        lib_lines.contains(&2),
        "the library declaration (graph.tcl line 2) must be rewritten: {lib_lines:?}"
    );

    // VS Code gates every rename behind `textDocument/prepareRename` — the
    // consumer document has no local declaration, so prepare must take the
    // workspace fall-through and accept with the call-site word's own range
    // (a `null` here made the editor abort with "The element can't be
    // renamed." before ever sending the rename).
    let prep = lsp.request(
        "textDocument/prepareRename",
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        }),
    );
    assert_eq!(
        prep.pointer("/placeholder").and_then(Value::as_str),
        Some("Rbc_ActiveLegend"),
        "prepare accepts the consumer call site: {prep:?}"
    );
    assert_eq!(
        prep.pointer("/range/start/character")
            .and_then(Value::as_i64),
        Some(0),
        "the highlighted range is the call-site word: {prep:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A direct call into a private `::tcl::` implementation
/// namespace is flagged W143 with a concrete public-command suggestion.
#[test]
fn direct_private_tcl_namespace_call_is_w143_with_suggestion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::tcl::dict::create a 1\n");
    let hits = with_code(&diags, "W143");
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one W143: {:?}",
        codes(&diags)
    );
    assert!(
        message(&hits[0]).contains("dict create"),
        "expected the concrete 'dict create' suggestion in the message, got: {:?}",
        message(&hits[0])
    );
}

/// The ordinary public ensemble call is never flagged, nor is a user's own
/// namespace nested under `tcl::`, nor Tcl's own public `tcl::`-rooted
/// namespaces (`tcl::mathop`/`tcl::mathfunc`).
#[test]
fn public_and_user_owned_tcl_namespaces_are_not_w143() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "dict create a 1\n\
         namespace eval ::tcl::mycustom { proc foo {} {} }\n\
         ::tcl::mycustom::foo\n\
         ::tcl::mathop::+ 1 2\n\
         tcl::mathop::* 2 3\n\
         ::tcl::mathfunc::sin 0\n\
         tcl::mathfunc::max 1 2\n",
    );
    assert!(
        !has_code(&diags, "W143"),
        "no public/user-owned namespace call should draw W143: {:?}",
        codes(&diags)
    );
}

/// tcllib publishes public packages *inside* `::tcl::chan` (the whole
/// `virtchannel_base` module), so W143's namespace claim there is
/// tail-restricted to real `chan` ensemble subcommands — a
/// `tcl::chan::memchan` call must never be flagged, and must never draw
/// W143 alongside the W120 that asks for its `package require`.
#[test]
fn public_tcllib_channel_packages_are_not_w143() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require tcl::chan::memchan\n\
         set c [tcl::chan::memchan]\n\
         set f [tcl::chan::fifo]\n",
    );
    assert!(
        !has_code(&diags, "W143"),
        "public tcllib virtchannel packages must not draw W143: {:?}",
        codes(&diags)
    );
}

/// A file that defines the command itself owns that name — a user's own
/// `proc ::tcl::dict::mine` is not a call into Tcl's internals.
#[test]
fn own_proc_in_a_private_namespace_is_not_w143() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace eval ::tcl::dict { proc mine {d} { return $d } }\n\
         ::tcl::dict::mine {a 1}\n",
    );
    assert!(
        !has_code(&diags, "W143"),
        "a self-defined private-namespace command must not draw W143: {:?}",
        codes(&diags)
    );
}

/// `rename OLD NEW` must validate that `OLD`
/// resolves to a known command.
///
/// tclsh 9.0.4 and 8.6.16 both abort the script:
/// `can't rename "definitelyNotDefinedAnywhere": command doesn't exist`
/// (exit 1). The whole rename statement is a guaranteed runtime error, so
/// the source word draws W123 end to end.
#[test]
fn w123_rename_of_a_nonexistent_command_is_published() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "rename definitelyNotDefinedAnywhere someAlias\n");
    assert!(
        has_code(&diags, "W123"),
        "the rename source must be resolved like any other command word: {diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| message(d).contains("definitelyNotDefinedAnywhere")),
        "the message names the missing command: {diags:?}"
    );
}

/// The delete form of the same statement. tclsh: `can't delete
/// "totallyBogusCommand": command doesn't exist` (exit 1) — `rename X {}`
/// was the form the original fix nearly missed.
#[test]
fn w123_deleting_rename_of_a_nonexistent_command_is_published() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "rename totallyBogusCommand {}\nputs done\n");
    assert!(
        has_code(&diags, "W123"),
        "`rename X {{}}` deletes X, so X must exist: {diags:?}"
    );
    assert_eq!(
        on_line(&diags, "W123"),
        [0].into_iter().collect::<BTreeSet<i64>>(),
        "flagged at the rename statement, nowhere else: {diags:?}"
    );
}

/// TN control for the two above: renaming a command that really does exist
/// is silent, so the check is a resolution, not a blanket flag on `rename`.
#[test]
fn w123_rename_of_an_existing_command_is_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc greet {} { return hi }\nrename greet hello\nrename puts _oldputs\n",
    );
    assert!(
        !has_code(&diags, "W123"),
        "a user proc and a builtin are both renameable: {diags:?}"
    );
}
