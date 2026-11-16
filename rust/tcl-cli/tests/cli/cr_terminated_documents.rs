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

//! A document terminated with lone carriage returns reads, under every verb, the way the editor reads it.

use super::*;

/// A `.sslictcl` document terminated with lone `\r` must draw the same loader
/// findings as the `\n` form. `tclsh` ends a command at a bare CR, but the
/// lexer treats one as horizontal whitespace, so the loader has to read the
/// normalised analysis text — exactly as the server does before publishing
/// `SSLIC1xxx`. Without that, every declaration collapses into one command and
/// the CLI disagrees with the editor on a file the editor handles correctly.
#[test]
fn diag_reads_a_cr_terminated_sslictcl_document_the_way_the_editor_does() {
    let lf = "sslictcl 1\nsite-owner {a}\nrenewal-window {b}\ndeployment-note {c}\n";
    let cr = lf.replace('\n', "\r");

    let lf_codes = sslictcl_diag_rows("lf", lf);
    let cr_codes = sslictcl_diag_rows("cr", &cr);

    // Three unknown top-level words, each preserved as an extension on its own
    // line — the whole point of the vocabulary's forwards-compatibility rule.
    assert_eq!(
        lf_codes,
        vec![
            ("SSLIC1101".to_owned(), 2),
            ("SSLIC1101".to_owned(), 3),
            ("SSLIC1101".to_owned(), 4),
        ],
        "the `\\n` form is the reference reading"
    );
    assert_eq!(
        cr_codes, lf_codes,
        "a lone-CR document must read identically to the `\\n` one"
    );
}

/// The rows a lone-CR document and its `\n` twin must agree on: everything
/// except `W118`, the one lint whose subject *is* the line terminators.
fn without_line_ending_lint(rows: &[(String, u64)]) -> Vec<(String, u64)> {
    rows.iter()
        .filter(|(code, _)| code != "W118")
        .cloned()
        .collect()
}

/// Every code on the `tcl diag` path, not only `SSLIC1xxx`, must read the
/// analysis form of a lone-CR document.
///
/// If the analyser and compiler-checks passes read the raw bytes instead
/// (even with the loader branch normalising for itself), that diverges from
/// the editor twice over: the lexer treats a bare `\r` as horizontal
/// whitespace, so the whole file parses as one command — inventing findings
/// and hiding real ones — and `LineIndex` starts a line only after a `\n`,
/// so whatever survives is reported at line 1.
///
/// The reproducer is an unclosed bracket on the second line.
#[test]
fn diag_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let lf = "set a 1\nset b [\nputs $a\n";
    let cr = lf.replace('\n', "\r");

    let lf_rows = tcl_diag_rows("lf", lf);
    let cr_rows = tcl_diag_rows("cr", &cr);

    assert!(
        lf_rows.contains(&("E201".to_owned(), 2)),
        "the `\\n` form is the reference reading: {lf_rows:?}"
    );
    assert_eq!(
        without_line_ending_lint(&cr_rows),
        without_line_ending_lint(&lf_rows),
        "a lone-CR document must read identically to the `\\n` one"
    );
    // The terminators themselves are the one legitimate difference: the CR form
    // is not the expected `\n`, so it earns the W118 its twin cannot.
    assert!(
        cr_rows.iter().any(|(code, _)| code == "W118"),
        "the CR form's terminators must be reported: {cr_rows:?}"
    );
    assert!(
        !lf_rows.iter().any(|(code, _)| code == "W118"),
        "the `\\n` form's terminators are the expected ones: {lf_rows:?}"
    );
    // The two specific ways the raw form diverged, named so a regression is
    // legible rather than just "the vectors differ". W118 is exempt: it is a
    // whole-file verdict anchored at the top of the document, not a finding
    // about the line it sits on.
    assert!(
        without_line_ending_lint(&cr_rows)
            .iter()
            .all(|(_, line)| *line > 1),
        "no finding may collapse onto line 1: {cr_rows:?}"
    );
    assert!(
        !cr_rows.iter().any(|(code, _)| code == "E003"),
        "parsing the file as one command invents an arity error: {cr_rows:?}"
    );
}

/// Dialect *detection* must read the analysis form too.
///
/// `detect_dialect`'s directive, shebang and version-guard tiers scan by line,
/// and Rust's `lines()` splits on `\n` only, so on the raw form of an old-Mac
/// document the whole file is line 1 and those tiers see nothing. The document
/// below is routed to `f5-irules` by its content, and under the wrong dialect
/// `when` and `HTTP::uri` are unknown commands.
#[test]
fn diag_detects_the_dialect_of_a_cr_terminated_document() {
    let lf = "# ordinary header\nwhen HTTP_REQUEST {\n    set u [HTTP::uri]\n}\n";
    let cr = lf.replace('\n', "\r");

    let lf_rows = tcl_diag_rows("dialect-lf", lf);
    let cr_rows = tcl_diag_rows("dialect-cr", &cr);

    assert_eq!(
        without_line_ending_lint(&cr_rows),
        without_line_ending_lint(&lf_rows),
        "the dialect a lone-CR document resolves to must match its `\\n` twin"
    );
    assert!(
        !cr_rows.iter().any(|(code, _)| code == "W123"),
        "resolving as generic Tcl makes the iRules commands unknown: {cr_rows:?}"
    );
}

/// The cross-file evidence scans must read the analysis form too.
///
/// `tcl diag a.tcl b.tcl` is one compilation: the declared-procedure set
/// and the call-site scan decide what may be folded. On the raw form of a
/// lone-CR pair both scans parse each file as one command, so the caller in the
/// second file is invisible and the fold the pair should retract survives.
///
/// This is `diag_shares_call_sites_across_inputs` over the same fixtures with
/// every `\n` rewritten to `\r`.
#[test]
fn diag_shares_call_sites_across_cr_terminated_inputs() {
    let lib = std::fs::read_to_string(fixtures_dir().join("issue977Lib.tcl")).expect("lib fixture");
    let main =
        std::fs::read_to_string(fixtures_dir().join("issue977Main.tcl")).expect("main fixture");

    let alone = multi_file_diag_text("cr-alone", &[("issue977Lib.tcl", &lib.replace('\n', "\r"))]);
    assert!(
        alone.contains("I230"),
        "the library alone still has only agreeing callers: {alone}"
    );
    let together = multi_file_diag_text(
        "cr-together",
        &[
            ("issue977Lib.tcl", &lib.replace('\n', "\r")),
            ("issue977Main.tcl", &main.replace('\n', "\r")),
        ],
    );
    assert!(
        !together.contains("I230"),
        "the `dev` call in the second file must be visible on the CR form too: {together}"
    );
}

/// Write each `(name, text)` into one scratch directory, run `tcl diag` over
/// all of them in order, and return the combined report text.
fn multi_file_diag_text(tag: &str, files: &[(&str, &str)]) -> String {
    let scratch = Scratch::new(&format!("multi-{tag}"));
    let mut args: Vec<String> = vec!["diag".to_owned()];
    for (name, text) in files {
        let path = scratch.write(name, text);
        args.push(path.to_str().expect("utf-8 path").to_owned());
    }
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    String::from_utf8(run_tcl_allow_failure(&borrowed)).expect("utf-8 output")
}

/// Write `text` to a scratch `doc.tcl` and run `tcl` with `args` against it
/// (the path is appended last), returning stdout. Mirrors the scratch-file
/// shape [`tcl_diag_rows`] uses, for verbs other than `diag`.
fn run_tcl_on_scratch_doc(tag: &str, text: &str, args: &[&str]) -> Vec<u8> {
    run_on_scratch_doc(tag, text, args, run_tcl_allow_failure)
}

/// [`run_tcl_on_scratch_doc`] in a process of its own: for output compared
/// across invocations that carries process-history identities (the opaque
/// `bodyKey` numbering), which a reused worker continues rather than restarts.
fn run_fresh_tcl_on_scratch_doc(tag: &str, text: &str, args: &[&str]) -> Vec<u8> {
    run_on_scratch_doc(tag, text, args, |args| {
        tcl()
            .args(args)
            .output()
            .expect("failed to spawn tcl binary")
            .stdout
    })
}

fn run_on_scratch_doc(
    tag: &str,
    text: &str,
    args: &[&str],
    run: impl FnOnce(&[&str]) -> Vec<u8>,
) -> Vec<u8> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-cli-cr-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("doc.tcl");
    std::fs::write(&path, text).expect("write document");

    let mut full: Vec<&str> = args.to_vec();
    let path_str = path.to_str().expect("utf-8 path").to_owned();
    full.push(&path_str);
    let out = run(&full);
    std::fs::remove_dir_all(&dir).ok();
    out
}

/// `tcl find-legacy` (`rust/tcl-cli/src/commands/misc.rs`) must analyse the
/// normalised form of a lone-CR document (#1953): raw, the whole document
/// mis-parses as one command, so a legacy pattern past the first line is
/// either missed entirely or reported at the wrong line.
#[test]
fn find_legacy_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let lf = "set x 1\nset y [expr $x+1]\nputs $y\n";
    let cr = lf.replace('\n', "\r");

    let lf_out = run_tcl_on_scratch_doc("legacy-lf", lf, &["find-legacy", "--json"]);
    let cr_out = run_tcl_on_scratch_doc("legacy-cr", &cr, &["find-legacy", "--json"]);
    let lf_json: serde_json::Value = serde_json::from_slice(&lf_out).expect("find-legacy JSON");
    let cr_json: serde_json::Value = serde_json::from_slice(&cr_out).expect("find-legacy JSON");

    assert_eq!(
        lf_json["issues"][0]["code"], "W100",
        "the `\\n` form is the reference reading: {lf_json}"
    );
    assert_eq!(
        lf_json["issues"][0]["line"], 2,
        "the unbraced expr sits on line 2: {lf_json}"
    );
    assert_eq!(
        cr_json, lf_json,
        "a lone-CR document must report the same legacy pattern at the same line as its `\\n` twin"
    );
}

/// `tcl minimize` (`rust/tcl-cli/src/commands/minimize.rs`) must reduce the
/// normalised form of a lone-CR document (#1953): raw, the document mis-parses
/// as one command, so the target diagnostic never fires and reduction fails
/// outright rather than reproducing a wrong minimal snippet.
#[test]
fn minimize_reduces_a_cr_terminated_document_the_way_the_editor_does() {
    let lf = "set a 1\nset b 2\nputs $a\n";
    let cr = lf.replace('\n', "\r");

    // `tcl minimize FILE CODE [--json]`: CODE is the final positional
    // argument (`InputArgs::inputs.split_last()`), so the scratch path must
    // come *before* it — `run_tcl_on_scratch_doc` appends its path last and
    // would leave CODE looking like a second input file.
    let minimize = |tag: &str, text: &str| -> Vec<u8> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-minimize-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("doc.tcl");
        std::fs::write(&path, text).expect("write document");
        let out = run_tcl_allow_failure(&[
            "minimize",
            path.to_str().expect("utf-8 path"),
            "W211",
            "--json",
        ]);
        std::fs::remove_dir_all(&dir).ok();
        out
    };
    let lf_out = minimize("lf", lf);
    let cr_out = minimize("cr", &cr);
    let lf_json: serde_json::Value = serde_json::from_slice(&lf_out)
        .unwrap_or_else(|e| panic!("minimize JSON: {e}\n{lf_out:?}"));
    let cr_json: serde_json::Value = serde_json::from_slice(&cr_out)
        .unwrap_or_else(|e| panic!("minimize JSON: {e}\n{cr_out:?}"));

    assert_eq!(
        lf_json[0]["reproduces"], true,
        "the `\\n` form is the reference reading: {lf_json}"
    );
    assert_eq!(
        lf_json[0]["source"], "set a 2",
        "W211 minimises to the unused-set alone: {lf_json}"
    );
    assert_eq!(
        cr_json[0]["source"], lf_json[0]["source"],
        "a lone-CR document must minimise to the same reproducer as its `\\n` twin: {cr_json}"
    );
    assert_eq!(cr_json[0]["reproduces"], true);
}

/// `tcl callgraph` (`rust/tcl-cli/src/commands/graphs.rs`, shared by
/// `symbols` / `symbolgraph` / `dataflow` through the same `combine_sources`
/// call) must analyse the normalised form of a lone-CR document (#1953): raw,
/// both `proc` definitions collapse into one mis-parsed command and the whole
/// call graph comes back empty.
#[test]
fn callgraph_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let lf = "proc foo {} {\n    bar\n}\nproc bar {} {\n    puts hi\n}\nfoo\n";
    let cr = lf.replace('\n', "\r");

    let lf_out = run_fresh_tcl_on_scratch_doc("cg-lf", lf, &["callgraph", "--json"]);
    let cr_out = run_fresh_tcl_on_scratch_doc("cg-cr", &cr, &["callgraph", "--json"]);
    let lf_json: serde_json::Value = serde_json::from_slice(&lf_out).expect("callgraph JSON");
    let cr_json: serde_json::Value = serde_json::from_slice(&cr_out).expect("callgraph JSON");

    assert_eq!(
        lf_json["nodes"].as_array().expect("nodes").len(),
        2,
        "the `\\n` form is the reference reading: {lf_json}"
    );
    // Neither side names its input file, so the two payloads must agree
    // byte-for-byte once both are read as the analyser reads them.
    assert_eq!(
        cr_json, lf_json,
        "a lone-CR document's call graph must match its `\\n` twin: {cr_json}"
    );
}

/// `tcl diff` (`rust/tcl-cli/src/commands/diff.rs`) must combine and analyse
/// the normalised form of each side (#1953): raw, a lone-CR side mis-parses as
/// one command while its `\n` twin parses as three, so the AST/IR/CFG layers
/// report a structural difference between two documents that are the same
/// script under a different line ending.
#[test]
fn diff_treats_a_lone_cr_document_as_identical_to_its_lf_twin() {
    let lf = "set a 1\nset b 2\nputs $a\n";
    let cr = lf.replace('\n', "\r");

    let out = run_tcl_allow_failure(&[
        "diff",
        "--left-source",
        lf,
        "--right-source",
        &cr,
        "--show",
        "all",
        "--json",
    ]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diff JSON");

    assert_eq!(
        report["equal"], true,
        "a lone-CR document and its `\\n` twin carry the same script, so every \
         layer must report equal rather than a spurious structural diff: {report}"
    );
    for layer in ["ast", "ir", "cfg"] {
        assert_eq!(
            report["layers"][layer]["equal"], true,
            "layer `{layer}` must not diverge on line-ending alone: {report}"
        );
    }
}

/// `tcl pkg discover` (`rust/tcl-cli/src/commands/pkg_discover.rs`) must scan
/// the normalised form of a lone-CR document (#1953): raw, the whole document
/// mis-parses as one command, so every `package require` after the first line
/// is invisible to discovery.
#[test]
fn pkg_discover_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-cli-pkg-discover-cr-{nanos}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    std::fs::write(
        dir.join("tclpkg.tcl"),
        "package demo-app\nversion 0.1.0\ntcl >=8.6\n",
    )
    .expect("write manifest");

    let lf = "set x 1\nputs $x\npackage require json 1.0\nputs done\npackage require http 2.9\n";
    let cr = lf.replace('\n', "\r");
    std::fs::write(dir.join("lf.tcl"), lf).expect("write lf source");
    std::fs::write(dir.join("cr.tcl"), &cr).expect("write cr source");

    let discover = |source: &str| -> Vec<(String, u64)> {
        let manifest = dir.join("tclpkg.tcl");
        let out = run_tcl_allow_failure(&[
            "pkg",
            "discover",
            dir.join(source).to_str().expect("utf-8 path"),
            "--manifest",
            manifest.to_str().expect("utf-8 path"),
            "--json",
        ]);
        let report: serde_json::Value = serde_json::from_slice(&out)
            .unwrap_or_else(|e| panic!("pkg discover JSON: {e}\n{out:?}"));
        report["requirements"]
            .as_array()
            .expect("requirements array")
            .iter()
            .map(|r| {
                (
                    r["name"].as_str().expect("name").to_owned(),
                    r["line"].as_u64().expect("line"),
                )
            })
            .collect()
    };

    let lf_requirements = discover("lf.tcl");
    let cr_requirements = discover("cr.tcl");
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(
        lf_requirements,
        vec![("json".to_owned(), 3), ("http".to_owned(), 5)],
        "the `\\n` form is the reference reading"
    );
    assert_eq!(
        cr_requirements, lf_requirements,
        "a lone-CR document must discover the same requirements at the same lines \
         as its `\\n` twin: {cr_requirements:?}"
    );
}

/// `tcl compwasm` (`rust/tcl-cli/src/commands/compile.rs`) must build the
/// `CompilationUnit` from the normalised form of a lone-CR document (#1953):
/// raw, the document mis-parses as one command, so the compiled module never
/// defines the `proc` at all — a mis-parse is not just a wrong report here,
/// it is wrong emitted code.
#[test]
fn compwasm_compiles_a_cr_terminated_document_the_way_the_editor_does() {
    let lf = "proc foo {} {\n    return 1\n}\nputs [foo]\n";
    let cr = lf.replace('\n', "\r");

    let wat_for = |tag: &str, text: &str| -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-compwasm-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let src_path = dir.join("doc.tcl");
        std::fs::write(&src_path, text).expect("write document");
        let wasm_path = dir.join("out.wasm");
        let wat_path = dir.join("out.wat");
        let output = tcl()
            .args([
                "compwasm",
                src_path.to_str().expect("utf-8 path"),
                "-o",
                wasm_path.to_str().expect("utf-8 path"),
                "--wat-output",
                wat_path.to_str().expect("utf-8 path"),
            ])
            .reused_output()
            .expect("failed to spawn tcl binary");
        assert!(
            output.status.success(),
            "tcl compwasm exited {:?}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        let wat = std::fs::read_to_string(&wat_path).expect("read WAT output");
        std::fs::remove_dir_all(&dir).ok();
        wat
    };

    let lf_wat = wat_for("lf", lf);
    let cr_wat = wat_for("cr", &cr);

    assert!(
        lf_wat.contains("$::foo"),
        "the `\\n` form is the reference reading and must compile `foo` as a \
         function: {lf_wat}"
    );
    assert_eq!(
        cr_wat, lf_wat,
        "a lone-CR document must compile to the same module as its `\\n` twin"
    );
}
