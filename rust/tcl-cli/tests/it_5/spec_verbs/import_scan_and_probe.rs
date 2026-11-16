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

//! `tcl spec import` describing an extension: the C source scan, the shell probe and their merge.

use super::*;

// ───────────────── describing a C extension: `tcl spec import` ─────────────────

/// The directory of the shim's C test extensions: `pkga.c`, entered at
/// `Pkga_Init`, and `doors.c`, entered at `Doors_Init` (and `layout.c`, which
/// defines no entry point). Two extensions, so describing one names its entry
/// point.
fn pkga_source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tcl-cshim/tests/c")
}

/// `--c-source` reads the real test extension `pkga`, named by its entry point
/// in a directory that holds another: its five commands and its package, each
/// a row at the conservative default for a command native code registers,
/// `host-native`, with the arity its usage message states and the provenance
/// `c-scan` beside the evidence, and none of the other extension's commands.
#[test]
fn spec_import_describes_a_c_extension_from_its_source() {
    let dir = pkga_source_dir();
    let (stdout, stderr, code) = run(&[
        "spec",
        "import",
        "--c-source",
        &dir.to_string_lossy(),
        "--entry",
        "Pkga",
    ]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("speclib pkga"), "{stdout}");
    assert!(
        !stdout.contains("doors_"),
        "the other extension's commands are not pkga's:\n{stdout}"
    );
    assert!(
        stdout.contains("c-scan: the extension entered at Pkga_Init (pkga.c:303)"),
        "the pack says which entry point it describes:\n{stdout}"
    );
    for name in [
        "pkga_calc",
        "pkga_count",
        "pkga_eq",
        "pkga_forget",
        "pkga_quote",
    ] {
        let block = command_block(&stdout, name);
        assert!(
            block.contains("runtime_backing host-native"),
            "{name}: {block}"
        );
        assert!(block.contains("required_package pkga"), "{name}: {block}");
        assert!(
            stdout.contains(&format!("{name} (c-scan)")),
            "{name} carries its provenance:\n{stdout}"
        );
    }
    assert!(
        command_block(&stdout, "pkga_eq").contains("arity 2"),
        "{stdout}"
    );
    assert!(
        stdout.contains("c-scan: usage `string1 string2`"),
        "the evidence names the usage message it read:\n{stdout}"
    );
    assert!(
        stdout.contains("c-scan: proposes arity exactly 2"),
        "{stdout}"
    );
    assert!(
        stderr.contains("pkga: 5 command(s) described (5 from the C source, 0 from the probe)"),
        "{stderr}"
    );
}

/// A source directory holding several extensions is several descriptions, one
/// per entry point (the C-shim fixture directory holds three: `doors`, `pkga`
/// and the index-cache probe): with none named the import describes nothing and says which
/// entry points there are, rather than describe one of them, or a mixture, as
/// though it were the whole (negative); naming the other describes its own
/// commands and package; and a prefix the sources do not define is refused
/// with the ones they do.
#[test]
fn spec_import_describes_one_extension_per_entry_point() {
    let dir = pkga_source_dir();
    let dir = dir.to_string_lossy();
    let (stdout, stderr, code) = run(&["spec", "import", "--c-source", &dir]);
    assert_ne!(code, 0, "stdout: {stdout}");
    assert!(stdout.is_empty(), "nothing is described:\n{stdout}");
    assert!(
        stderr.contains(
            "the sources hold 3 extensions, one per entry point: Doors_Init (doors.c:261), \
             IndexCache_Init (index_cache.c:18), Pkga_Init (pkga.c:303); describe one at a \
             time with --entry PREFIX"
        ),
        "{stderr}"
    );

    let (stdout, stderr, code) = run(&["spec", "import", "--c-source", &dir, "--entry", "Doors"]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("speclib doors"), "{stdout}");
    assert!(!stdout.contains("pkga_"), "{stdout}");
    assert!(
        command_block(&stdout, "doors_eval").contains("required_package doors"),
        "{stdout}"
    );
    assert!(
        stderr.contains("doors: 11 command(s) described (11 from the C source, 0 from the probe)"),
        "{stderr}"
    );

    let (_stdout, stderr, code) = run(&["spec", "import", "--c-source", &dir, "--entry", "Nosuch"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains(
            "--entry Nosuch: the sources define no Nosuch_Init; their entry points are \
             Doors_Init (doors.c:261), IndexCache_Init (index_cache.c:18), Pkga_Init \
             (pkga.c:303)"
        ),
        "{stderr}"
    );
}

/// A registration whose name the scan cannot read is a row marked dynamic in
/// the JSON, and a line in the pack that says to declare it by hand; it is
/// never given a name, and the commands around it are unchanged.
#[test]
fn spec_import_marks_a_computed_registration_dynamic() {
    let tree = Tree::new("spec-import-dynamic");
    std::fs::write(
        tree.path().join("factory.c"),
        "int Init(Tcl_Interp *interp) {\n    Tcl_CreateObjCommand(interp, \"fixed\", P, NULL, NULL);\n    \
         Tcl_CreateObjCommand(interp, names[i], P, NULL, NULL);\n    Tcl_NewMethod(interp, c, n, 1, &t, 0);\n    return 0;\n}\n",
    )
    .expect("write source");
    let dir = tree.path().to_string_lossy().into_owned();
    let (stdout, stderr, code) = run(&["spec", "import", "--c-source", &dir, "--json"]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(report["commands"][0]["name"], "fixed");
    assert_eq!(
        report["commands"][0]["provenance"],
        serde_json::json!(["c-scan"])
    );
    let dynamic = &report["dynamic"][0];
    assert_eq!(dynamic["dynamic"], true);
    assert_eq!(dynamic["name_expression"], "names[i]");
    assert_eq!(dynamic["file"], "factory.c");
    assert_eq!(dynamic["line"], 3);
    assert!(
        report["blind"][0]
            .as_str()
            .is_some_and(|b| b.contains("Tcl_NewMethod") && b.contains("TclOO")),
        "{report}"
    );

    let (pack, _stderr, code) = run(&["spec", "import", "--c-source", &dir]);
    assert_eq!(code, 0);
    assert_eq!(
        pack.matches("\ncommand ").count(),
        1,
        "only the named command: {pack}"
    );
    assert!(pack.contains("declare it by hand"), "{pack}");
}

#[test]
fn spec_import_c_source_refuses_what_it_cannot_scan() {
    let tree = Tree::new("spec-import-nothing");
    let dir = tree.path().to_string_lossy().into_owned();
    let (_stdout, stderr, code) = run(&["spec", "import", "--c-source", &dir]);
    assert_ne!(code, 0);
    assert!(stderr.contains("no C sources"), "{stderr}");
    let (_stdout, stderr, code) = run(&["spec", "import", "--c-source", "/no/such/dir"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("is not a directory"), "{stderr}");
    let (_stdout, stderr, code) =
        run(&["spec", "import", "--c-source", &dir, "--snapshot", "1.0=/x"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}

/// The probe requires the package in a real shell and lists what it added. The
/// package runs, so only one the policy opts in does; nothing runs and the exit
/// is 1 when it does not.
#[test]
fn spec_import_probe_lists_the_commands_a_package_adds() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let probed = Described::new(
        "spec-import-probe",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = probed.probe(&tclsh, &[]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    for name in ["demo::fixed", "demo::flex"] {
        assert!(
            stdout.contains(&format!("{name} (probe)")),
            "{name} carries its provenance:\n{stdout}"
        );
        assert!(
            command_block(&stdout, name).contains("runtime_backing host-native"),
            "{stdout}"
        );
    }
    assert!(
        stdout.contains("probe: `package require demo` added `demo::flex` to the shell"),
        "{stdout}"
    );
    assert!(
        stderr.contains("demo: 2 command(s) described (0 from the C source, 2 from the probe)"),
        "{stderr}"
    );

    let unlisted = Described::new(
        "spec-import-probe-policy",
        DEMO_PACKAGE,
        &demo_pack("2", "2"),
        &[],
    );
    let (stdout, stderr, code) = unlisted.probe(&tclsh, &[]);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("running the package 'demo' is not permitted by policy"),
        "{stderr}"
    );
    assert!(stdout.is_empty(), "nothing ran: {stdout}");
}

/// A command both sources found carries both provenances, and the probe adds the
/// commands the scan could not name.
#[test]
fn spec_import_merges_the_scan_and_the_probe_by_command() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let both = Described::new(
        "spec-import-merge",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let sources = both.tree.path().join("csrc");
    std::fs::create_dir_all(&sources).expect("source dir");
    std::fs::write(
        sources.join("demo.c"),
        "int Init(Tcl_Interp *interp) {\n    Tcl_PkgProvide(interp, \"demo\", \"1.0\");\n    \
         Tcl_CreateObjCommand(interp, \"demo::fixed\", Fixed, NULL, NULL);\n    return 0;\n}\n\
         static int Fixed(void *d, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {\n    \
         Tcl_WrongNumArgs(interp, 1, objv, \"a b\");\n    return 0;\n}\n",
    )
    .expect("write source");
    let source_dir = sources.to_string_lossy().into_owned();
    let (stdout, stderr, code) = both.probe(&tclsh, &["--c-source", &source_dir]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("demo::fixed (c-scan, probe)"), "{stdout}");
    assert!(stdout.contains("demo::flex (probe)"), "{stdout}");
    assert!(
        command_block(&stdout, "demo::fixed").contains("arity 2"),
        "{stdout}"
    );
}

#[test]
fn spec_import_probe_says_when_the_package_cannot_be_required() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let missing = Described::new(
        "spec-import-probe-missing",
        "package provide demo 1.0\n",
        &demo_pack("2", "2"),
        &["absent"],
    );
    let mut command = missing.import_command(&tclsh, "absent", &[]);
    let (stdout, stderr, code) = finished(&mut command);
    assert_ne!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stderr.contains("could not require 'absent'"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    let mut command = missing.import_command(&tclsh, "a}b", &[]);
    let (_stdout, stderr, code) = finished(&mut command);
    assert_ne!(code, 0);
    assert!(stderr.contains("is not a package name"), "{stderr}");
}

/// The JSON of a probe says the package and the version the shell was told it
/// provides, and each command it added with the provenance `probe`.
#[test]
fn spec_import_probe_reports_the_version_the_package_provided() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let probed = Described::new(
        "spec-import-probe-json",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = probed.probe(&tclsh, &["--json"]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(report["package"], "demo");
    assert_eq!(report["version"], "1.0");
    assert_eq!(report["commands"][0]["name"], "demo::fixed");
    assert_eq!(
        report["commands"][0]["provenance"],
        serde_json::json!(["probe"])
    );
}

/// The shell runs under the package manager's timeout, so a package that does not
/// finish is stopped and said so, and the probe lists nothing.
#[test]
fn spec_import_probe_stops_a_package_that_does_not_finish_in_time() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let slow = Described::new(
        "spec-import-probe-timeout",
        "package provide demo 1.0\nafter 20000\n",
        &demo_pack("2", "2"),
        &["demo"],
    );
    let policy = slow.pack.parent().expect("the project").join("tclpkg.toml");
    let mut text = std::fs::read_to_string(&policy).expect("the policy");
    text.push_str("[sandbox]\nmax-timeout-secs = 1\n");
    std::fs::write(&policy, text).expect("write the policy");
    let started = std::time::Instant::now();
    let (stdout, stderr, code) = slow.probe(&tclsh, &[]);
    assert_eq!(code, 2, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("did not finish probing 'demo' in time"),
        "{stderr}"
    );
    assert!(stdout.is_empty(), "{stdout}");
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

/// A package that ends the shell while it is required has not been probed: the
/// shell never said what it added, whatever status it left with.
#[test]
fn spec_import_probe_is_not_finished_by_a_package_that_exits_the_shell() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let leaving = Described::new(
        "spec-import-probe-exit",
        "package provide demo 1.0\nexit 0\n",
        &demo_pack("2", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = leaving.probe(&tclsh, &[]);
    assert_eq!(code, 2, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("the shell stopped before it had listed what 'demo' added (status 0)"),
        "{stderr}"
    );
    assert!(stdout.is_empty(), "{stdout}");
}

/// A command whose name is not plain text is skipped with a warning, so a package
/// cannot print a line of its own into the pack.
#[test]
fn spec_import_probe_skips_a_command_whose_name_is_not_plain_text() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let odd = Described::new(
        "spec-import-probe-odd-name",
        "package provide demo 1.0\nproc \"two words\" {} {}\nproc demo_ok {} {}\n",
        &demo_pack("2", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = odd.probe(&tclsh, &[]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("demo_ok (probe)"), "{stdout}");
    assert!(!stdout.contains("two words"), "{stdout}");
    assert!(
        stderr.contains("the probe skipped a command whose name is not plain text"),
        "{stderr}"
    );
}

/// Several `--c-source` directories are read as one extension, and each file is
/// named by its directory so two files of one name stay two.
#[test]
fn spec_import_names_each_file_by_its_directory_when_several_are_given() {
    let tree = Tree::new("spec-import-two-dirs");
    for name in ["one", "two"] {
        std::fs::create_dir_all(tree.path().join(name)).expect("dir");
        std::fs::write(
            tree.path().join(name).join("f.c"),
            "int Init(Tcl_Interp *i) {\n    Tcl_CreateObjCommand(i, names[k], P, 0, 0);\n    return 0;\n}\n",
        )
        .expect("write source");
    }
    let one = tree.path().join("one").to_string_lossy().into_owned();
    let two = tree.path().join("two").to_string_lossy().into_owned();
    let (stdout, stderr, code) = run(&[
        "spec",
        "import",
        "--c-source",
        &one,
        "--c-source",
        &two,
        "--json",
    ]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let files: Vec<String> = report["dynamic"]
        .as_array()
        .expect("dynamic rows")
        .iter()
        .filter_map(|row| row["file"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(files, [format!("{one}/f.c"), format!("{two}/f.c")]);
}

/// `--package` names the pack, over the package the source provides.
#[test]
fn spec_import_names_the_pack_by_the_package_flag() {
    let dir = pkga_source_dir();
    let (stdout, stderr, code) = run(&[
        "spec",
        "import",
        "--c-source",
        &dir.to_string_lossy(),
        "--entry",
        "Pkga",
        "--package",
        "renamed",
    ]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("speclib renamed"), "{stdout}");
    assert!(!stdout.contains("speclib pkga"), "{stdout}");
}

impl Described {
    /// `tcl spec import --probe PACKAGE` in the project, the shell named.
    fn import_command(&self, tclsh: &Path, package: &str, extra: &[&str]) -> Command {
        let mut command = self.environment();
        command
            .args(["spec", "import", "--probe", package, "--tclsh"])
            .arg(tclsh)
            .args(extra);
        command
    }

    /// Run `tcl spec import --probe demo` and return what it printed.
    pub(super) fn probe(&self, tclsh: &Path, extra: &[&str]) -> (String, String, i32) {
        finished(&mut self.import_command(tclsh, "demo", extra))
    }
}
