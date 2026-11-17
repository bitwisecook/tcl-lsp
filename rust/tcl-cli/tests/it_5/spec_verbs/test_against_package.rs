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

//! `tcl spec test`: a pack compared with the package it describes, in a real shell under the project's policy.

use super::*;

/// A pack whose arity is narrower than the package's, and one wider, are each
/// reported against the real shell: `demo::flex` takes one to three words and the
/// pack says two, so a call with one and a call with three are accepted where the
/// pack says they are errors; `demo::fixed` takes two and the pack says one or
/// two, so a call with one is refused where the pack says it is valid. A pack
/// that describes the package truthfully has no row and exits 0.
#[test]
fn spec_test_reports_an_arity_divergence() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let wrong = Described::new(
        "spec-test-arity",
        DEMO_PACKAGE,
        &demo_pack("2", "1..2"),
        &["demo"],
    );
    let (stdout, stderr, code) = wrong.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    for row in [
        "demo::flex: arity: declares at least 2 argument(s), but `demo::flex x` was not refused as `wrong # args`",
        "demo::flex: arity: declares at most 2 argument(s), but `demo::flex x x x` was not refused as `wrong # args`",
        "demo::fixed: arity: declares 1 argument(s) valid, but `demo::fixed x` was refused as `wrong # args`",
        "2 command(s) tested against 'demo', 3 divergence(s)",
    ] {
        assert!(stdout.contains(row), "missing {row:?} in:\n{stdout}");
    }
    assert_eq!(
        stdout.matches("arity:").count(),
        3,
        "one row per divergence: {stdout}"
    );

    let truthful = Described::new(
        "spec-test-truth",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = truthful.run(&tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// A command the pack declares and the package does not define is one row and no
/// other question, and the rest of the pack is still held to the package.
#[test]
fn spec_test_reports_a_command_the_package_does_not_define() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let pack = "speclib demo 2.0 {\n    command demo::fixed {\n        arity 2\n        \
                required_package demo\n    }\n    command demo::ghost {\n        arity 1\n        \
                required_package demo\n        effects -complete {package demo}\n        world_effects none -complete {package demo}\n        completion {Ok Error} -complete {package demo}\n        result_stability ReferentiallyTransparent -complete {package demo}\n        hover { example {demo::ghost 1} }\n    }\n}\n";
    let ghost = Described::new("spec-test-ghost", DEMO_PACKAGE, pack, &["demo"]);
    let (stdout, stderr, code) = ghost.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("demo::ghost: missing: declared, but the package does not define it"),
        "{stdout}"
    );
    assert_eq!(
        stdout.matches("demo::ghost:").count(),
        1,
        "one row, and no question asked of a command that is not there: {stdout}"
    );
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 1 divergence(s)"),
        "{stdout}"
    );
}

/// The policy that decides is the operator's: the project `tcl` is run in, whose
/// manifest is found from the working directory as `tcl pkg` finds it, and not the
/// directory the pack happens to be in.
#[test]
fn spec_test_reads_the_policy_of_the_project_it_is_run_in() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let held = Described::nested(
        "spec-test-project",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    // Run from a directory below the manifest, which is where the policy is.
    let mut command = held.command();
    command
        .current_dir(held.pack.parent().expect("the specs directory"))
        .arg("--tclsh")
        .arg(&tclsh);
    let (stdout, stderr, code) = finished(&mut command);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// A dependency vendored into the project has a manifest and a `tclpkg.toml` of
/// its own, and neither can be what lets it run: the policy is the operator's,
/// which says nothing of the package until the operator does.
#[test]
fn spec_test_ignores_the_policy_of_the_tree_the_pack_was_found_in() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let vendored = Described::vendored(
        "spec-test-vendored",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = vendored.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("running the package 'demo' is not permitted by policy"),
        "the dependency opted itself in: {stderr}"
    );
    assert!(stdout.is_empty(), "nothing ran: {stdout}");

    vendored.trust_in_the_project(&["demo"]);
    let (stdout, stderr, code) = vendored.run(&tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// The operator who stands inside a dependency vendored into the project is still
/// the project's operator. The dependency's manifest is then the nearest and its
/// `tclpkg.toml` would be the policy, so a `cd` would opt it in; the policy is the
/// outermost project's, which is the same whether the operator stands in the
/// project or in the dependency.
#[test]
fn spec_test_from_inside_a_vendored_dependency_takes_the_projects_policy() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let vendored = Described::vendored(
        "spec-test-vendored-inside",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let inside = vendored.dependency();
    let (stdout, stderr, code) = vendored.run_in(&inside, &tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("running the package 'demo' is not permitted by policy"),
        "standing inside the dependency opted it in: {stderr}"
    );
    assert!(stdout.is_empty(), "nothing ran: {stdout}");

    vendored.trust_in_the_project(&["demo"]);
    let (stdout, stderr, code) = vendored.run_in(&inside, &tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

const BEHAVING_PACKAGE: &str = "package provide demo 1.0\n\
puts -nonewline \"demo loaded, with no newline\"\n\
namespace eval demo {variable cache; variable count 0}\n\
proc demo::double {x} {expr {$x * 2}}\n\
proc demo::label {x} {return \"value $x\"}\n\
proc demo::remember {x} {set ::remembered $x; return $x}\n\
proc demo::raises {x} {error \"no such thing\"}\n\
proc demo::memo {x} {if {![info exists ::memoised]} {set ::memoised $x}; return $x}\n\
proc demo::tidy {x} {set local $x; return $local}\n\
proc demo::cached {x} {variable cache; if {![info exists cache($x)]} {set cache($x) [expr {$x + 1}]}; return $cache($x)}\n\
proc demo::counted {x} {variable count; incr count; return $x}\n";

const BEHAVING_PACK: &str = "speclib demo 2.0 {\n\
    command demo::double {\n\
        arity 1\n\
        required_package demo\n\
        return_type Int\n\
        runtime_backing tcl-body {-pack-text {proc demo::double {x} {expr {$x * 3}}}}\n\
        hover { example {demo::double 21} }\n\
    }\n\
    command demo::label {\n\
        arity 1\n\
        required_package demo\n\
        return_type Int\n\
        hover { example {demo::label 7} }\n\
    }\n\
    command demo::remember {\n\
        arity 1\n\
        required_package demo\n\
        effects -complete {package demo}\n        world_effects none -complete {package demo}\n        completion {Ok Error} -complete {package demo}\n        result_stability ReferentiallyTransparent -complete {package demo}\n\
        hover { example {demo::remember 5} }\n\
    }\n\
    command demo::raises {\n\
        arity 1\n\
        required_package demo\n\
        hover { example {demo::raises 1} }\n\
    }\n\
    command demo::memo {\n\
        arity 1\n\
        required_package demo\n\
        effects -complete {package demo}\n        world_effects none -complete {package demo}\n        completion {Ok Error} -complete {package demo}\n        result_stability ReferentiallyTransparent -complete {package demo}\n\
        hover { example {demo::memo 3} }\n\
    }\n\
    command demo::tidy {\n\
        arity 1\n\
        required_package demo\n\
        effects -complete {package demo}\n        world_effects none -complete {package demo}\n        completion {Ok Error} -complete {package demo}\n        result_stability ReferentiallyTransparent -complete {package demo}\n\
        hover { example {demo::tidy 4} }\n\
    }\n\
    command demo::cached {\n\
        arity 1\n\
        required_package demo\n\
        effects -complete {package demo}\n        world_effects none -complete {package demo}\n        completion {Ok Error} -complete {package demo}\n        result_stability ReferentiallyTransparent -complete {package demo}\n\
        hover { example {demo::cached 4} }\n\
    }\n\
    command demo::counted {\n\
        arity 1\n\
        required_package demo\n\
        effects -complete {package demo}\n        world_effects none -complete {package demo}\n        completion {Ok Error} -complete {package demo}\n        result_stability ReferentiallyTransparent -complete {package demo}\n\
        hover { example {demo::counted 4} }\n\
    }\n\
}\n";

/// The other questions: a `returns` type the answer is not a value of, an
/// `example` that raises, a command declared `pure` that writes a global or fills
/// a namespace variable its first call, and a Tcl-body reference body that answers
/// differently from the command it describes, each one row naming the command and
/// what the shell did.
#[test]
fn spec_test_reports_what_the_examples_the_purity_and_the_reference_body_disagree_on() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let behaving = Described::new(
        "spec-test-facts",
        BEHAVING_PACKAGE,
        BEHAVING_PACK,
        &["demo"],
    );
    let (stdout, stderr, code) = behaving.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    for row in [
        "demo::label: returns: declares it returns int, but `demo::label 7` answered `value 7`",
        "demo::remember: pure: declared pure, but `demo::remember 5` wrote the variable(s) remembered",
        "demo::raises: example: `demo::raises 1` raised: no such thing",
        "demo::memo: pure: declared pure, but running it created or changed the variable(s) memoised",
        "demo::cached: pure: declared pure, but running it created or changed the variable(s) ::demo::cache",
        "demo::counted: pure: declared pure, but `demo::counted 4` wrote the variable(s) ::demo::count",
        "demo::double: reference: `demo::double 21` answers `42` (0) as the command and `63` (0) as its reference body",
    ] {
        assert!(stdout.contains(row), "missing {row:?} in:\n{stdout}");
    }
    assert!(
        !stdout.contains("demo::double: returns")
            && !stdout.contains("demo::double: example")
            && !stdout.contains("demo::tidy"),
        "a command the pack describes truthfully has no row: {stdout}"
    );
    assert!(
        stdout.contains("8 command(s) tested against 'demo', 7 divergence(s)"),
        "{stdout}"
    );
}

#[test]
fn spec_test_detects_temporary_namespace_variables_without_rejecting_locals() {
    let package = r"package provide demo 1.0
namespace eval demo {}
proc demo::scratch {} {set ::scratch x; unset ::scratch; return ok}
proc demo::nested {} {namespace eval ::temporary {set scratch x; unset scratch}; return ok}
proc demo::once {} {
    set ::once_scratch x
    unset ::once_scratch
    proc ::demo::once {} {return ok}
    return ok
}
proc demo::arity_only {x} {set ::arity_scratch x; unset ::arity_scratch; return $x}
proc demo::local {} {set scratch x; unset scratch; return ok}
";
    let pack = r"speclib demo 2.0 {
    command demo::scratch {required_package demo; effects -complete {package demo}; world_effects none -complete {package demo}; completion {Ok Error} -complete {package demo}; result_stability ReferentiallyTransparent -complete {package demo}; hover {example {demo::scratch}}}
    command demo::nested {required_package demo; effects -complete {package demo}; world_effects none -complete {package demo}; completion {Ok Error} -complete {package demo}; result_stability ReferentiallyTransparent -complete {package demo}; hover {example {demo::nested}}}
    command demo::once {required_package demo; effects -complete {package demo}; world_effects none -complete {package demo}; completion {Ok Error} -complete {package demo}; result_stability ReferentiallyTransparent -complete {package demo}; hover {example {demo::once}}}
    command demo::arity_only {required_package demo; effects -complete {package demo}; world_effects none -complete {package demo}; completion {Ok Error} -complete {package demo}; result_stability ReferentiallyTransparent -complete {package demo}; arity 1}
    command demo::local {required_package demo; effects -complete {package demo}; world_effects none -complete {package demo}; completion {Ok Error} -complete {package demo}; result_stability ReferentiallyTransparent -complete {package demo}; hover {example {demo::local}}}
}
";
    let described = Described::new("spec-test-transient", package, pack, &["demo"]);
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let (stdout, stderr, code) = described.run(&tclsh.path);
        assert_eq!(code, 1, "{version:?}: stdout: {stdout}\nstderr: {stderr}");
        for (command, variable) in [
            ("scratch", "scratch"),
            ("nested", "::temporary::scratch"),
            ("once", "once_scratch"),
            ("arity_only", "arity_scratch"),
        ] {
            assert!(
                stdout.lines().any(|line| {
                    line.starts_with(&format!("demo::{command}: pure:")) && line.contains(variable)
                }),
                "{version:?}: missing {command}/{variable} in:\n{stdout}"
            );
        }
        assert!(!stdout.contains("demo::local:"), "{version:?}: {stdout}");
        assert!(
            stdout.contains("5 command(s) tested against 'demo', 4 divergence(s)"),
            "{version:?}: {stdout}"
        );
    }
}

/// A reference body a pack takes from a file of the package it ships is read when
/// the pack loads and held to the command as one written in the pack is.
#[test]
fn spec_test_compares_a_reference_body_the_pack_takes_from_a_package_file() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let pack = "speclib demo 2.0 {\n    command demo::double {\n        arity 1\n        \
                required_package demo\n        runtime_backing tcl-body {-package-source src/double.tcl}\n        \
                hover { example {demo::double 21} }\n    }\n}\n";
    let held = Described::nested(
        "spec-test-package-source",
        BEHAVING_PACKAGE,
        pack,
        &["demo"],
    );
    let project = held
        .pack
        .parent()
        .and_then(Path::parent)
        .expect("the project that holds the pack");
    std::fs::create_dir_all(project.join("src")).expect("src");
    std::fs::write(
        project.join("src").join("double.tcl"),
        "proc demo::double {x} {expr {$x * 3}}\n",
    )
    .expect("write the body");
    let (stdout, stderr, code) = held.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains(
            "demo::double: reference: `demo::double 21` answers `42` (0) as the command and `63` (0) as its reference body"
        ),
        "{stdout}"
    );
}

/// Requiring a package runs its Tcl, so the verb runs only for a package the
/// project's policy opts in; nothing runs and the exit is 1 when it does not.
#[test]
fn spec_test_runs_only_a_package_the_policy_opts_in() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let unlisted = Described::new(
        "spec-test-policy",
        DEMO_PACKAGE,
        &demo_pack("2", "1..2"),
        &[],
    );
    let (stdout, stderr, code) = unlisted.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("running the package 'demo' is not permitted by policy"),
        "{stderr}"
    );
    assert!(stderr.contains("tcl pkg trust demo"), "{stderr}");
    assert!(stdout.is_empty(), "nothing ran: {stdout}");
}

/// A package the shell cannot require is one row and exit 1, and a pack with
/// no `required_package` agreement needs the package named.
#[test]
fn spec_test_says_when_the_package_cannot_be_required() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let missing = Described::new(
        "spec-test-missing",
        "package provide demo 1.0\n",
        &demo_pack("2", "2").replace("required_package demo", "required_package absent"),
        &["absent"],
    );
    let (stdout, stderr, code) = missing.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("-: load: can't find package absent"),
        "{stdout}"
    );
    assert!(
        stdout.contains("0 command(s) tested against 'absent', 1 divergence(s)"),
        "the package is not there, so nothing is asked: {stdout}"
    );

    let unnamed = Described::new(
        "spec-test-unnamed",
        DEMO_PACKAGE,
        "speclib demo 2.0 {\n    command demo::flex {\n        arity 1\n    }\n}\n",
        &["demo"],
    );
    let (_stdout, stderr, code) = unnamed.run(&tclsh);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("name the Tcl package with --package"),
        "{stderr}"
    );
}

/// A pack that declares no command has nothing to ask a shell, and a file that is
/// not there is an error, not an empty pack.
#[test]
fn spec_test_says_when_there_is_nothing_to_test() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let empty = Described::new(
        "spec-test-empty",
        DEMO_PACKAGE,
        "speclib demo 2.0 {\n}\n",
        &["demo"],
    );
    let (stdout, stderr, code) = empty.run(&tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("the pack declares no command to test"),
        "{stdout}"
    );

    let absent = empty.pack.with_file_name("absent.tclspec");
    let (stdout, stderr, code) = run(&[
        "spec",
        "test",
        &absent.to_string_lossy(),
        "--tclsh",
        &tclsh.to_string_lossy(),
    ]);
    assert_eq!(code, 2, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stderr.contains("not a file"), "{stderr}");
}

#[test]
fn spec_test_rejects_failed_pack_loads_before_starting_a_shell() {
    let tree = Tree::new("spec-test-invalid");
    let pack = tree.path().join("invalid.tclspec");
    let absent_shell = tree.path().join("no-tclsh");
    for (source, reason) in [
        ("speclib demo 99.0 {}", "SpecTcl vocabulary 99.0"),
        (
            "speclib demo 2.0 {command demo::ok {arity 0}; error broken}",
            "pack evaluation failed",
        ),
        ("set unrelated 1", "no speclib declaration was loaded"),
    ] {
        std::fs::write(&pack, source).expect("write invalid pack");
        let (stdout, stderr, code) = run(&[
            "spec",
            "test",
            &pack.to_string_lossy(),
            "--tclsh",
            &absent_shell.to_string_lossy(),
        ]);
        assert_eq!(code, 2, "{source}: stdout: {stdout}\nstderr: {stderr}");
        assert!(stderr.contains(reason), "{source}: {stderr}");
        assert!(!stdout.contains("no command to test"), "{stdout}");
    }
    std::fs::write(&pack, "speclib demo 2.0 {display_name {Empty}}")
        .expect("write valid empty pack");
    let (stdout, stderr, code) = run(&[
        "spec",
        "test",
        &pack.to_string_lossy(),
        "--tclsh",
        &absent_shell.to_string_lossy(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("no command to test"), "{stdout}");
}

/// A shell that stops before it has asked every command is not a pass, whatever
/// status it stops with: the verb says what the shell said and which commands it
/// never asked, and exits 1.
#[test]
fn spec_test_says_when_the_shell_stops_before_it_has_asked_every_command() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let stopping = Described::new(
        "spec-test-stops",
        "package provide demo 1.0\nputs stderr {demo is leaving}\nexit 3\n",
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = stopping.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("the shell exited with status 3 before it had asked every command")
            && stderr.contains("0 of 2 asked; not asked: demo::flex, demo::fixed")
            && stderr.contains("demo is leaving"),
        "{stderr}"
    );
    assert!(
        stdout.contains("0 of 2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// A package that ends the shell with status 0 — when it is required, or when one
/// of its commands is asked — has not passed, and the commands it never let the
/// verb ask are named, not counted as tested.
#[test]
fn spec_test_does_not_pass_a_package_that_exits_the_shell_with_status_0() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let on_require = Described::new(
        "spec-test-exit-require",
        "package provide demo 1.0\nexit 0\n",
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = on_require.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("the shell exited with status 0 before it had asked every command")
            && stderr.contains("not asked: demo::flex, demo::fixed"),
        "{stderr}"
    );

    // The second command ends the shell when it is asked, so the first was asked
    // and its row is there, and the second and the third were not.
    let package = "package provide demo 1.0\nnamespace eval demo {}\n\
                   proc demo::flex {a {b x} {c y}} {return \"$a$b$c\"}\n\
                   proc demo::fixed {a b} {exit 0}\n\
                   proc demo::last {a} {return $a}\n";
    let pack = "speclib demo 2.0 {\n    command demo::flex {\n        arity 2\n        \
                required_package demo\n    }\n    command demo::fixed {\n        arity 2\n        \
                required_package demo\n    }\n    command demo::last {\n        arity 1\n        \
                required_package demo\n    }\n}\n";
    let part_way = Described::new("spec-test-exit-part-way", package, pack, &["demo"]);
    let (stdout, stderr, code) = part_way.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("demo::flex: arity: declares at least 2 argument(s)"),
        "the row found before the shell stopped is kept: {stdout}"
    );
    assert!(
        stdout.contains("1 of 3 command(s) tested against 'demo'"),
        "{stdout}"
    );
    assert!(
        stderr.contains("1 of 3 asked; not asked: demo::fixed, demo::last"),
        "{stderr}"
    );
}

/// With no shell named, the verb runs the one in the active virtual environment
/// (`TCL_VENV`), which here is a wrapper around the shell on `PATH`; the audit
/// log records which program ran.
#[cfg(unix)]
#[test]
fn spec_test_uses_the_shell_of_the_active_venv_when_none_is_named() {
    use std::os::unix::fs::PermissionsExt;

    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let described = Described::new(
        "spec-test-venv",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let venv = described.tree.path().join("venv");
    let bin = venv.join("bin");
    std::fs::create_dir_all(&bin).expect("venv bin");
    let wrapper = bin.join("tclsh");
    std::fs::write(
        &wrapper,
        format!("#!/bin/sh\nexec '{}' \"$@\"\n", tclsh.display()),
    )
    .expect("write the wrapper");
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))
        .expect("make the wrapper executable");
    let mut command = described.command();
    command.env("TCL_VENV", &venv);
    let (stdout, stderr, code) = finished(&mut command);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
    let audit = std::fs::read_to_string(
        described
            .tree
            .path()
            .join("home/state/tcl-lsp/pkg-audit.log"),
    )
    .expect("the run was audited");
    let program = format!("\"program\":\"{}\"", wrapper.display());
    assert!(
        audit
            .lines()
            .any(|line| line.contains("\"label\":\"spec-test\"") && line.contains(&program)),
        "the venv's shell ran: {audit}"
    );
}

/// `--package` names the package to require, and what it names wins over the
/// `required_package` the pack's commands declare.
#[test]
fn spec_test_takes_the_package_from_the_flag() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let flagged = Described::new(
        "spec-test-flag",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2").replace("required_package demo", "required_package absent"),
        &["demo"],
    );
    let mut command = flagged.command();
    command.args(["--package", "demo", "--tclsh"]).arg(&tclsh);
    let (stdout, stderr, code) = finished(&mut command);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// The shell runs under the package manager's timeout, so a package that does
/// not finish is stopped and said so, and is not a pass.
#[test]
fn spec_test_stops_a_package_that_does_not_finish_in_time() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let slow = Described::new(
        "spec-test-timeout",
        "package provide demo 1.0\nafter 20000\n",
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let policy = slow.pack.parent().expect("the project").join("tclpkg.toml");
    let mut text = std::fs::read_to_string(&policy).expect("the policy");
    text.push_str("[sandbox]\nmax-timeout-secs = 1\n");
    std::fs::write(&policy, text).expect("write the policy");
    let started = std::time::Instant::now();
    let (stdout, stderr, code) = slow.run(&tclsh);
    assert_eq!(code, 2, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("did not finish testing 'demo' in time"),
        "{stderr}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

#[test]
fn spec_test_presence_keeps_literal_colons_and_wildcards() {
    let described = Described::new(
        "spec-test-literal-presence",
        r"package provide demo 1.0
proc :literal {} {}
proc ghostOther {} {}
proc querX {} {}
",
        r"speclib demo 2.0 {
    command {:literal} {required_package demo}
    command {ghost*} {required_package demo}
    command {quer?} {required_package demo}
}",
        &["demo"],
    );
    let mut interpreters: Vec<PathBuf> = tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter_map(tcl_test_support::witness_tclsh)
        .map(|interpreter| interpreter.path)
        .collect();
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required Jim oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("Jim oracle discovery")
    };
    if let Some(jim) = jim {
        interpreters.push(jim.path);
    }
    for interpreter in interpreters {
        let (stdout, stderr, code) = described.run(&interpreter);
        assert_eq!(code, 1, "{interpreter:?}: {stdout}\n{stderr}");
        assert!(
            stdout.contains("ghost*: missing:"),
            "{interpreter:?}: {stdout}"
        );
        assert!(
            stdout.contains("quer?: missing:"),
            "{interpreter:?}: {stdout}"
        );
        assert!(
            !stdout.contains(":literal: missing:"),
            "{interpreter:?}: {stdout}"
        );
        assert!(
            stdout.contains("3 command(s) tested against 'demo', 2 divergence(s)"),
            "{interpreter:?}: {stdout}"
        );
    }
}
