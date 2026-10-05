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

//! The value-transfer design's CLI witnesses: the `tcl explore` and
//! `tcl opt` evidence for its programs, driven through the built `tcl`
//! binary rather than the library directly. Separate from
//! `rust/tcl-cli/tests/cli.rs`, which the diagnostic policy's work edits.

use std::path::{Path, PathBuf};
use std::process::Command;

/// An `XDG_CONFIG_HOME` for one spawn that names no directory — unique to
/// the spawn and never created — so no `tcl-lsp/config.ini` can be found
/// under it: the global policy layer every verb resolves is empty, and a
/// developer's own `config.ini` cannot change what these witnesses see.
fn absent_config_home() -> PathBuf {
    static SPAWNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let spawn = SPAWNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "value-transfers-cli-tests-{}-{spawn}",
        std::process::id()
    ))
}

/// The built `tcl` binary, isolated from the machine's global configuration.
fn tcl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
    command.env("XDG_CONFIG_HOME", absent_config_home());
    command
}

/// Run the built `tcl` binary with `args`, returning captured stdout text.
fn run_tcl(args: &[&str]) -> String {
    let output = tcl()
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// The route tally, read from `tcl explore --show sccp`: two fused
/// `expr` statements are two expression entries, one nesting a
/// direct-routed `string length`, so `direct 1 · expression 2 ·
/// implementation 0`.
#[test]
fn explore_sccp_prints_the_route_tally() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set r [expr {\"x\"}]; set n [expr {[string length abcdef] * 2}]}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("r#1 = const('x')"), "{text}");
    assert!(text.contains("n#1 = const(12)"), "{text}");
    assert!(
        text.contains("routes entered: direct 1 · expression 2 · implementation 0"),
        "{text}"
    );
}

/// `tcl explore --show sccp` prints the
/// folded type a route's representation evidence states, beside the
/// value, and says nothing where a route states none. `binary format` is
/// the interface page's program (2) — `h#1 = const('ABCDEF')` typed
/// `bytearray (constructed)` — read beside `format`'s `string
/// (constructed)` and `list`'s `list (constructed)` so "types" is
/// not one route's alone; a plain literal constructs nothing and prints
/// no type line at all.
#[test]
fn explore_sccp_prints_folded_types() {
    let binary = run_tcl(&[
        "explore",
        "--source",
        "set h [binary format H* 414243444546]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(binary.contains("h#1 = const('ABCDEF')"), "{binary}");
    assert!(binary.contains("type: bytearray (constructed)"), "{binary}");

    let format = run_tcl(&[
        "explore",
        "--source",
        "set s [format %d 5]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(format.contains("s#1 = const(5)"), "{format}");
    assert!(format.contains("type: string (constructed)"), "{format}");

    let list = run_tcl(&[
        "explore",
        "--source",
        "set l [list a b c]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(list.contains("l#1 = const('a b c')"), "{list}");
    assert!(list.contains("type: list (constructed)"), "{list}");

    let literal = run_tcl(&[
        "explore",
        "--source",
        "set n 5",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(literal.contains("n#1 = const(5)"), "{literal}");
    assert!(!literal.contains("type:"), "{literal}");
}

/// The `tcl explore` witnesses — program (3)'s `incr` route
/// twice, a release-ambiguous decline, and `llength`'s direct route — each
/// hand-verified and pinned here. The decline was
/// `f5-irules`'s until ruling 8 gave iRules its declared 8.4 base: it now
/// folds `incr` over `010` to 9, as tclsh 8.4 does, and `tk`, a profile
/// that declares no release, is the one that declines.
#[test]
fn explore_sccp_prints_the_route_of_each_statement() {
    let program_three = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set n 1; incr n; incr n 2; return $n}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(program_three.contains("n#3 = const(4)"), "{program_three}");
    assert_eq!(
        program_three
            .matches("route incr: direct cell-increment (registry)")
            .count(),
        2,
        "{program_three}"
    );
    assert_eq!(
        program_three.matches("· answer: evaluated").count(),
        2,
        "{program_three}"
    );

    let leading_zero = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set z 010; incr z}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
        "--dialect",
        "tk",
    ]);
    assert!(
        leading_zero.contains("· answer: declined: release-ambiguous: numeral-grammar"),
        "{leading_zero}"
    );
    let irules = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set z 010; incr z}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
        "--dialect",
        "f5-irules",
    ]);
    assert!(irules.contains("z#2 = const(9)"), "{irules}");

    let llength = run_tcl(&[
        "explore",
        "--source",
        "set r [llength {a b}]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        llength.contains("route llength: direct list-length (registry)"),
        "{llength}"
    );
}

/// `tcl explore --show sccp --text` lists what a statement stores on each
/// completion path: `lassign` over an array `c` stores its first target and
/// stops, and over a place whose kind is not proven it may stop at any of its
/// targets, which its existence transfer lists beside the normal path.
#[test]
fn explore_sccp_prints_what_each_completion_path_stores() {
    let array = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {array set c {k keep}; set a old; lassign {new second} a c}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        array.contains("· answer: evaluated: error after 1 store"),
        "{array}"
    );
    assert!(
        array.contains("· path error after 1 store: write a = new"),
        "{array}"
    );
    assert!(
        array.contains("· path normal: write c(k) = keep"),
        "a statement that stores on its normal path alone lists that one:\n{array}"
    );

    let unproven = run_tcl(&[
        "explore",
        "--source",
        "proc p {c} {set a old; if {$c} {set b 1}; lassign {x y} a b}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        unproven.contains("· path normal: write a = x; write b = y"),
        "{unproven}"
    );
    assert!(
        unproven.contains("· path error: bind a as scalar; may-bind b as scalar"),
        "{unproven}"
    );
}

/// The existence branch decides inside the
/// fixed point — `unset x` leaves `x` provably unbound, so `[info exists
/// x]` folds `False` as an ordinary `Applied` branch — and the `if`'s
/// true block (`puts yes`) is outside `executable blocks`.
#[test]
fn explore_sccp_prints_the_existence_branch_decided() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; unset x; if {[info exists x]} {puts yes}}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        text.contains("branch entry_1: False") && text.contains("condition: [info exists x]"),
        "the existence branch decides inside the fixed point:\n{text}"
    );
    assert!(
        text.contains("executable blocks: entry_1, exit_5, if_end_2, if_next_4"),
        "the true block is outside executable blocks:\n{text}"
    );
    assert!(
        !text.contains("if_then_3"),
        "the if's true block never appears:\n{text}"
    );
}

/// `tcl opt --profile full --dialect tcl8.6` forwards program
/// (3)'s constant return into its call site (O100 / O103).
#[test]
fn opt_forwards_program_three() {
    let out = run_tcl(&[
        "opt",
        "--source",
        "proc p {} {set n 1; incr n; incr n 2; return $n}\nputs [p]",
        "--profile",
        "full",
        "--dialect",
        "tcl8.6",
    ]);
    assert!(out.contains("puts 4"), "{out}");
}

/// A value-position cell update whose read the host statement does
/// not hold keeps its store — `set n 1` stays ahead of `set result [incr
/// n]` rather than being read by a join the optimiser cannot see through.
#[test]
fn opt_keeps_the_store_behind_a_nested_increment() {
    let out = run_tcl(&[
        "opt",
        "--source",
        "proc p {} {set n 1; set result [incr n]; puts $result; puts $n}\np",
        "--profile",
        "full",
        "--dialect",
        "tcl8.6",
    ]);
    assert!(out.contains("set n 1"), "{out}");
}

/// #2214: a `::`-qualified global a nested `incr` writes is a
/// global write in its procedure's summary, so `set hits 0` and `puts
/// $hits` around `bump`'s `[incr ::hits]` are both kept — the program
/// prints `1` under `tclsh8.4` to `tclsh9.1`, as the original does.
#[test]
fn opt_keeps_a_global_a_nested_increment_writes() {
    let source = "set hits 0\nproc bump {} {set y [incr ::hits]; return $y}\nbump\nputs $hits\n";
    let out = run_tcl(&["opt", "--source", source, "--profile", "full"]);
    assert!(out.contains("set hits 0"), "{out}");
    assert!(out.contains("puts $hits"), "{out}");
}

/// The value-transfer design's executable example, whose one home is
/// the compiler's test fixtures.
const TENANT_PACK: &str =
    include_str!("../../tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec");

/// The example's three spellings: its own name, the rename, and the
/// subcommand form whose operand sits one word later.
const TENANT_SPELLINGS: [&str; 3] = ["tenant::label", "tenant::tag", "tenant label"];

/// A scratch workspace named `name` whose `.tcl-lsp/` directory holds
/// `pack`, the one pack the CLI then discovers from its working directory,
/// or nothing.
fn scratch_workspace(name: &str, pack: Option<&str>) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("value-transfers-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".tcl-lsp")).expect("a scratch workspace");
    if let Some(pack) = pack {
        std::fs::write(root.join(".tcl-lsp").join("tenant.tclspec"), pack).expect("the pack");
    }
    root
}

/// [`run_tcl`] with `workspace` as the working directory, so the pack in
/// its `.tcl-lsp/` is the one discovered.
fn run_tcl_in(workspace: &Path, args: &[&str]) -> String {
    let output = tcl()
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// The three spellings, one statement each: `set a [tenant::label acme]`,
/// `set b [tenant::tag acme]`, `set c [tenant label acme]`.
fn one_statement_per_spelling(statement: impl Fn(char, &str) -> String) -> String {
    ['a', 'b', 'c']
        .into_iter()
        .zip(TENANT_SPELLINGS)
        .map(|(variable, spelling)| statement(variable, spelling))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `tcl explore --show sccp --text` over one assignment per spelling in
/// `workspace`: each variable holds `tenant:acme`, from the implementation
/// route, and every answer is evaluated.
fn assert_the_explorer_folds(workspace: &Path) {
    let source = one_statement_per_spelling(|variable, spelling| {
        format!("set {variable} [{spelling} acme]")
    });
    let explored = run_tcl_in(
        workspace,
        &[
            "explore",
            "--source",
            &source,
            "--show",
            "sccp",
            "--text",
            "--no-colour",
        ],
    );
    for (variable, spelling) in ['a', 'b', 'c'].into_iter().zip(TENANT_SPELLINGS) {
        let head = spelling.split(' ').next().expect("a head");
        assert!(
            explored.contains(&format!("{variable}#1 = const('tenant:acme')")),
            "{spelling}:\n{explored}"
        );
        assert!(
            explored.contains(&format!("route {head}: implementation tenant.label.v1")),
            "{spelling}:\n{explored}"
        );
    }
    assert_eq!(
        explored.matches("· answer: evaluated").count(),
        3,
        "{explored}"
    );
    assert!(explored.contains("implementation 3"), "{explored}");
}

/// The completion test on the CLI and the pack surfaces
/// (`docs/design/compiler/value-transfers.md` § *The completion test*),
/// beside `the_completion_test_needs_no_consumer_edit` in the compiler
/// witnesses: the executable example's three spellings, from a workspace
/// pack alone, fold `acme` to `tenant:acme` in `tcl explore`, make the
/// condition over it constant in `tcl diag` (I230) and forward the constant
/// in `tcl opt` (O100). `tcl spec export`'s canonical pack and a studio
/// form edit of each declaration keep all three evaluators — the renderer
/// writes a `-native` placeholder for the body it cannot draw, and the
/// studio carries the author's own bytes forward over it — and each folds
/// again from its own workspace. A workspace without the pack folds
/// nothing.
#[test]
fn the_completion_test_reaches_every_surface() {
    let workspace = scratch_workspace("fixture", Some(TENANT_PACK));
    assert_the_explorer_folds(&workspace);
    let conditions = one_statement_per_spelling(|_, spelling| {
        format!("if {{[{spelling} acme] eq \"tenant:acme\"}} {{puts yes}} else {{puts no}}")
    });
    let diagnosed = run_tcl_in(&workspace, &["diag", "--source", &conditions]);
    assert_eq!(
        diagnosed.matches("I230").count(),
        3,
        "every condition is constant:\n{diagnosed}"
    );
    let forwarded = one_statement_per_spelling(|variable, spelling| {
        format!("set {variable} [{spelling} acme]; puts ${variable}")
    });
    let optimised = run_tcl_in(
        &workspace,
        &["opt", "--source", &forwarded, "--profile", "full"],
    );
    assert_eq!(
        optimised.matches("puts tenant:acme").count(),
        3,
        "{optimised}"
    );
    assert!(optimised.contains("O100"), "{optimised}");

    let bare = scratch_workspace("bare", None);
    let explored = run_tcl_in(
        &bare,
        &[
            "explore",
            "--source",
            "set r [tenant::label acme]",
            "--show",
            "sccp",
            "--text",
            "--no-colour",
        ],
    );
    assert!(explored.contains("r#1 = overdefined"), "{explored}");
    assert!(
        explored.contains("implementation 0") && !explored.contains("tenant:acme"),
        "{explored}"
    );

    let exported = run_tcl_in(&workspace, &["spec", "export", ".tcl-lsp/tenant.tclspec"]);
    assert_eq!(
        exported
            .matches("evaluate -implementation tenant.label.v1")
            .count(),
        3,
        "{exported}"
    );
    let from_export = scratch_workspace("exported", Some(&exported));
    assert_the_explorer_folds(&from_export);

    let mut store = tcl_spec_studio::store::PackStore::from_source(TENANT_PACK);
    // A declared implementation's body is not on the spec, so the draft names
    // `semantics` as the part it cannot recover and the renderer writes the
    // `-native` placeholder in its place: the loss is stated, never silent,
    // and the studio's carry-forward below is what keeps the body.
    let drafted = store.draft("tenant::label").expect("tenant::label").clone();
    assert!(
        drafted[tcl_spec_studio::draft::UNRENDERABLE_KEY]
            .as_array()
            .is_some_and(|lost| lost.iter().any(|key| key == "semantics")),
        "{drafted:?}"
    );
    let rendered = tcl_spec_studio::render_spectcl::render_pack(&[drafted], "tenant");
    assert!(
        rendered.contains("semantics -native tenant::label::semantics"),
        "{rendered}"
    );
    for name in ["tenant::label", "tenant::tag", "tenant"] {
        let mut edited = store.draft(name).expect(name).clone();
        edited.insert("return_type".to_owned(), serde_json::json!("String"));
        let write = store.set_command(name, &edited, false);
        assert!(
            write.dropped.is_empty(),
            "{name}: {:?}\n{}",
            write.dropped,
            store.source()
        );
    }
    let studio = store.source().to_owned();
    assert_eq!(
        studio
            .matches("evaluate -implementation tenant.label.v1")
            .count(),
        3,
        "{studio}"
    );
    assert!(!studio.contains("-native"), "{studio}");
    let from_studio = scratch_workspace("studio", Some(&studio));
    assert_the_explorer_folds(&from_studio);

    for root in [workspace, bare, from_export, from_studio] {
        let _ = std::fs::remove_dir_all(root);
    }
}

/// The releases the oracle runs, oldest first.
const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// Run `script` from a file under `tclsh`: whether it exited cleanly and
/// what it printed. A script run from a file exits non-zero on an error,
/// where one read from stdin does not.
fn run_tclsh(tclsh: &str, script: &str) -> Option<(bool, String)> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "value-transfers-cli-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, script).ok()?;
    let output = Command::new(tclsh).arg(&path).output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// Every release from `first` on with its reference interpreter, oldest
/// first, as the shared oracle lookup finds it
/// ([`tcl_test_support::witness_tclsh`]): a release with none fails the test
/// where `TCL_LSP_REQUIRE_TCLSH` requires it, and is reported as skipped
/// otherwise.
fn tclshs_from(first: &str) -> Vec<(&'static str, String)> {
    RELEASES
        .iter()
        .filter(|&&series| series >= first)
        .filter_map(|&series| {
            let version = tcl_dialect::TclVersion::from_version_string(series)?;
            let tclsh = tcl_test_support::witness_tclsh(version)?;
            Some((series, tclsh.path.to_string_lossy().into_owned()))
        })
        .collect()
}

/// Program (4): a `switch` over the string two appends build, whose `baz`
/// arm no run reaches.
const PROGRAM_FOUR_BUILD: &str = "set acc \"\"; append acc foo; append acc bar\n";

/// Program (4)'s arms: `default` is the one that runs.
const PROGRAM_FOUR_ARMS: &str = "{\n    baz     { puts never }\n    default { puts always }\n}\n";

/// The shared-body arms: `baz` passes its body on with a bare `-`, the one
/// spelling every release reads alike, to `qux`'s, which never runs either.
const PROGRAM_FOUR_SHARED: &str =
    "{\n    baz     -\n    qux     { puts never }\n    default { puts always }\n}\n";

/// `(line, column, code)` of every diagnostic `tcl diag` reports for
/// `source` under `dialect`, with `enable` turned on.
fn diagnostics_at(source: &str, dialect: &str, enable: &[&str]) -> Vec<(u64, u64, String)> {
    let mut args = vec!["diag", "--json", "--source", source, "--dialect", dialect];
    for code in enable {
        args.extend(["--enable", code]);
    }
    let output = tcl()
        .args(&args)
        .output()
        .expect("failed to spawn tcl binary");
    // `tcl diag` exits 1 when it reports a warning or an error, 0 otherwise.
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("diag JSON");
    report
        .as_array()
        .expect("a report per input")
        .iter()
        .flat_map(|file| file["diagnostics"].as_array().expect("diagnostics").iter())
        .map(|diagnostic| {
            (
                diagnostic["line"].as_u64().expect("a line"),
                diagnostic["column"].as_u64().expect("a column"),
                diagnostic["code"].as_str().expect("a code").to_owned(),
            )
        })
        .collect()
}

/// Program (4) yields I231 on the dead arm for every form of `switch`,
/// through the shipped binary: `tcl diag` reports the arm — `baz`'s pattern
/// at line 3, and `qux`'s at line 4 in the shared-body form — and `tcl opt
/// --profile full` folds the `switch` to `puts always`, which prints
/// `always` under every tclsh release that has the form, as the original
/// does. `-nocase` is 8.5's: a profile that may be 8.4 reports no arm and
/// keeps the statement, and tclsh 8.4 rejects the option.
#[test]
fn program_four_reaches_diag_and_opt_in_every_form() {
    let forms = [
        (
            "exact",
            format!("switch -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.4",
        ),
        (
            "-glob",
            format!("switch -glob -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.4",
        ),
        (
            "-regexp",
            format!("switch -regexp -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.4",
        ),
        (
            "-nocase",
            format!("switch -nocase -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.5",
        ),
        (
            "shared body",
            format!("switch -- $acc {PROGRAM_FOUR_SHARED}"),
            "8.4",
        ),
    ];
    for (form, switch, first) in forms {
        let source = format!("{PROGRAM_FOUR_BUILD}{switch}");
        let dead: &[u64] = if form == "shared body" { &[3, 4] } else { &[3] };
        let found = diagnostics_at(&source, "tcl8.6", &[]);
        for &line in dead {
            assert!(
                found.contains(&(line, 5, "I231".to_owned())),
                "{form}: I231 on line {line}: {found:?}\n{source}"
            );
        }
        let optimised = run_tcl(&[
            "opt",
            "--source",
            &source,
            "--profile",
            "full",
            "--dialect",
            "tcl8.6",
        ]);
        let statements: String = optimised
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            statements.contains("puts always")
                && !statements.contains("puts never")
                && !statements.contains("switch")
                && optimised.contains("O112"),
            "{form}:\n{optimised}"
        );
        for (series, tclsh) in tclshs_from(first) {
            for program in [source.as_str(), optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, "always\n".to_owned())),
                    "{form} under tclsh{series}:\n{program}"
                );
            }
        }
    }

    let nocase = format!("{PROGRAM_FOUR_BUILD}switch -nocase -- $acc {PROGRAM_FOUR_ARMS}");
    for dialect in ["tcl8.4", "tk"] {
        assert!(
            !diagnostics_at(&nocase, dialect, &[])
                .iter()
                .any(|(_, _, code)| code == "I231"),
            "{dialect}: no selection is made"
        );
        let kept = run_tcl(&[
            "opt",
            "--source",
            &nocase,
            "--profile",
            "full",
            "--dialect",
            dialect,
        ]);
        assert!(kept.contains("switch -nocase"), "{dialect}:\n{kept}");
    }
    if let Some((_, tclsh)) = tclshs_from("8.4")
        .into_iter()
        .find(|(series, _)| *series == "8.4")
    {
        assert_eq!(
            run_tclsh(&tclsh, &nocase).map(|(ran, _)| ran),
            Some(false),
            "tclsh8.4 rejects -nocase"
        );
    }
}

/// `tcl explore --show sccp --text` prints the selection the solver makes
/// at an opaque `switch`: over program (4)'s `-glob` form the selected arm
/// and the arm whose body runs are both `default`, with a line naming
/// `baz`'s arm as never selected; a subject that matches names the arm; a
/// subject of two members names one per member; an arm that passes its body
/// on with `-` selects one arm and runs another's; and nothing prints where
/// no selection is made — a parameter subject, and a quoted or braced `-`
/// body under 9.1, which reads two ways there.
#[test]
fn explore_sccp_prints_the_selection() {
    let explore = |source: &str, dialect: &str| {
        run_tcl(&[
            "explore",
            "--source",
            source,
            "--show",
            "sccp",
            "--text",
            "--no-colour",
            "--dialect",
            dialect,
        ])
    };
    let glob = explore(
        &format!("{PROGRAM_FOUR_BUILD}switch -glob -- $acc {PROGRAM_FOUR_ARMS}"),
        "tcl8.6",
    );
    assert!(glob.contains("selection: default"), "{glob}");
    assert!(glob.contains("bodies: default"), "{glob}");
    assert!(glob.contains("arm never selected: baz"), "{glob}");

    let matched = explore(
        "set s foobar\nswitch -glob -- $s {\n    foo*    { puts a }\n    default { puts d }\n}\n",
        "tcl8.6",
    );
    assert!(matched.contains("selection: arm 0"), "{matched}");
    assert!(matched.contains("bodies: arm 0"), "{matched}");

    let finite = explore(
        "proc p {c} {\n    if {$c} { set s foo } else { set s bar }\n    \
         switch -glob -- $s {\n        foo     { puts a }\n        bar     { puts b }\n        \
         default { puts d }\n    }\n}\n",
        "tcl8.6",
    );
    assert!(finite.contains("selection: arm 1, arm 0"), "{finite}");

    let passed_on = explore(
        "set s abc\nswitch -glob -- $s {\n    a*      -\n    b*      { puts ab }\n    \
         default { puts d }\n}\n",
        "tcl9.1",
    );
    assert!(passed_on.contains("selection: arm 0"), "{passed_on}");
    assert!(passed_on.contains("bodies: arm 1"), "{passed_on}");

    let parameter = explore(
        "proc p {x} {\n    switch -glob -- $x {\n        a*      { puts a }\n        \
         default { puts d }\n    }\n}\n",
        "tcl8.6",
    );
    assert!(!parameter.contains("selection:"), "{parameter}");

    let delimited = "set s abc\nswitch -glob -- $s a* {-} b* { puts ab } default { puts d }\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let read = explore(delimited, dialect);
        assert!(read.contains("selection: arm 0"), "{dialect}: {read}");
    }
    for dialect in ["tcl9.1", "tk"] {
        let declined = explore(delimited, dialect);
        assert!(!declined.contains("selection:"), "{dialect}: {declined}");
    }
}

/// The two applied-reachability consumers through `tcl diag`: a loop whose
/// header the solver decides is W240 or W241 where W242 hinted at its
/// counter, and an iRule's `HTTP::header` after a `HTTP::respond` in an arm
/// that never runs draws no IRULE1201, where one after a live respond does.
#[test]
fn diag_reads_the_decided_loop_header_and_the_dead_respond() {
    let loops = "set n 0\nwhile {$n} {puts \"never runs\"}\nset go 1\nwhile {$go} {puts x}\n\
                 proc p {n} {\n    set i 0\n    while {$i < $n} {puts $i}\n}\n";
    let found = diagnostics_at(loops, "tcl8.6", &["W242"]);
    for expected in [(2, 7, "W240"), (4, 7, "W241"), (7, 11, "W242")] {
        assert!(
            found.contains(&(expected.0, expected.1, expected.2.to_owned())),
            "{expected:?} in {found:?}"
        );
    }
    assert_eq!(
        found.iter().filter(|(_, _, code)| code == "W242").count(),
        1,
        "the decided headers draw no W242: {found:?}"
    );

    let dead = "when HTTP_REQUEST {\n    if {0} { HTTP::respond 200 }\n    \
                HTTP::header insert X-Custom val\n}\n";
    let live = "when HTTP_REQUEST {\n    if {[HTTP::method] eq \"GET\"} { HTTP::respond 200 }\n    \
                HTTP::header insert X-Custom val\n}\n";
    let irules = |source: &str| {
        diagnostics_at(source, "f5-irules", &[])
            .into_iter()
            .filter(|(_, _, code)| code == "IRULE1201")
            .count()
    };
    assert_eq!(irules(dead), 0);
    assert_eq!(irules(live), 1);
}

/// `tcl opt --profile full` over `source` under `series`' dialect: the
/// rewritten program, with the report the verb appends as comments.
fn opt_under(source: &str, series: &str) -> String {
    run_tcl(&[
        "opt",
        "--source",
        source,
        "--profile",
        "full",
        "--dialect",
        &format!("tcl{series}"),
    ])
}

/// `opt`'s program without its trailing report.
fn statements_of(optimised: &str) -> String {
    optimised
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The ordered evaluation state's exit evidence: `tcl explore --show sccp`
/// over the expression that reads `x`, increments it and reads it again
/// prints the expression's value, 5, as `r#1` and the last write the
/// expression made, 2, as `x#2`.
#[test]
fn explore_sccp_prints_the_ordered_state() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; set r [expr {$x + [incr x] + $x}]; return $x}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("r#1 = const(5)"), "{text}");
    assert!(text.contains("x#2 = const(2)"), "{text}");
}

/// The completion paths' exit evidence: `tcl explore --show sccp` over a
/// procedure that keeps the code of a `catch` in a substitution prints the
/// code, 1, as `c#1` and the message the script raised, `boom`, as `m#1`, and
/// the route the command was run on, `catch-protected`, answering `evaluated`.
#[test]
fn explore_sccp_prints_what_a_catch_completed_with() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set c [catch {error boom} m]; return $c}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("c#1 = const(1)"), "{text}");
    assert!(text.contains("m#1 = const('boom')"), "{text}");
    assert!(
        text.contains("route catch: direct catch-protected (registry)"),
        "{text}"
    );
    assert!(text.contains("· answer: evaluated"), "{text}");
}

/// `tcl opt` over programs whose `catch` stands in a substitution forwards
/// what the script completed with and keeps what it wrote: the code and the
/// message, the value a body's `incr` left, and the condition #2231 reads
/// after one, each printing under tclsh 8.4 to 9.1 what the original prints.
#[test]
fn opt_forwards_what_a_nested_catch_completed_with() {
    let programs = [
        (
            "proc p {} {\n    set c [catch {error boom} m]\n    puts \"$c|$m\"\n}\np\n",
            "1|boom\n",
            "puts \"1|boom\"",
        ),
        (
            "set x 1\nset c [catch {incr x} m]\nputs \"$c|$m\"\nputs $x\n",
            "0|2\n2\n",
            "puts \"0|2\"\nputs 2",
        ),
        (
            "set x 1\nset c [catch {incr x}]\nif {$x == 2} {puts two} else {puts other}\n",
            "two\n",
            "puts two",
        ),
    ];
    for (source, printed, forwarded) in programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            let statements = statements_of(&optimised);
            assert!(statements.contains(forwarded), "tcl{series}:\n{optimised}");
            for program in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A `catch` the flow graph lowers into a procedure's blocks — straight-line
/// statements — ends at a statement with no words of its own, and the solver
/// evaluates the `catch` as it does the one the graph keeps whole:
/// `tcl explore --show sccp` over `catch {set v 1} r` prints `r#1` as the 1 its
/// script returned, and over `set x 1; catch {incr x} m` prints `m#1` as 2 and
/// `x#2` as 2, where the state after the script would give 3.
#[test]
fn explore_sccp_prints_what_a_flattened_catch_returned() {
    let result = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {catch {set v 1} r; return $r}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(result.contains("r#1 = const(1)"), "{result}");
    assert!(
        result.contains("route catch: direct catch-protected (registry)"),
        "{result}"
    );
    assert!(result.contains("· answer: evaluated"), "{result}");

    let before = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; catch {incr x} m; return $m}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(before.contains("m#1 = const(2)"), "{before}");
    assert!(before.contains("x#2 = const(2)"), "{before}");
}

/// The value a flattened `catch` script ends with is the result variable's, so
/// the store it ends with is not one nothing reads: `tcl opt` keeps `set v $c`
/// where the result is a parameter's, and a global, and prints under tclsh 8.4
/// to 9.1 what the original prints.
#[test]
fn opt_keeps_the_store_a_flattened_catch_returns() {
    let programs = [
        (
            "proc p {c} {\n    catch {set v $c} m\n    puts \"<$m>\"\n}\np 7\n",
            "<7>\n",
            "set v ",
        ),
        (
            "set g 0\nproc p {c} {\n    global g\n    catch {set v $c} g\n}\np 7\nputs $g\n",
            "7\n",
            "set v ",
        ),
    ];
    for (source, printed, kept) in programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            assert!(
                statements_of(&optimised).contains(kept),
                "tcl{series}:\n{optimised}"
            );
            for program in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// The interface page's seven ordered-state programs through `tcl opt`,
/// each printing under tclsh 8.4 to 9.1 what the original prints: the value
/// of the expression and what `x` holds after it, 5 and 2, 0 and 1, 21 and
/// 10, 5 and 3, 2 and 2, 3 and 2, and — for an error between a write and the
/// end of the expression inside a `catch` — the message and 2. Both reads
/// after the expression are the proved values, and the error path's is not.
#[test]
fn opt_prints_what_tclsh_prints_for_the_seven_ordered_state_programs() {
    let expressions = [
        ("{$x + [incr x] + $x}", "5", "2"),
        ("{0 && [incr x]}", "0", "1"),
        ("{$x + [set x 10] + $x}", "21", "10"),
        ("{[incr x] + [incr x]}", "5", "3"),
        ("{$x ? [incr x] : [incr x 10]}", "2", "2"),
        ("\"$x + [incr x]\"", "3", "2"),
    ];
    let mut programs: Vec<(String, String, bool)> = expressions
        .iter()
        .map(|(expression, value, after)| {
            (
                format!("set x 1\nset r [expr {expression}]\nputs $r\nputs $x\n"),
                format!("{value}\n{after}\n"),
                true,
            )
        })
        .collect();
    programs.push((
        "set x 1\ncatch {expr {[incr x] + [error mid]}} msg\nputs $msg\nputs $x\n".to_owned(),
        "mid\n2\n".to_owned(),
        false,
    ));
    for (source, printed, forwarded) in &programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            let statements = statements_of(&optimised);
            let (value, after) = printed.split_once('\n').expect("two lines");
            if *forwarded {
                assert!(
                    statements.contains(&format!("puts {value}\nputs {after}")),
                    "tcl{series}:\n{optimised}"
                );
            } else {
                assert!(
                    statements.contains("set x 1") && statements.contains("puts $x"),
                    "tcl{series}: no earlier x is forwarded\n{optimised}"
                );
            }
            for program in [source.as_str(), optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.clone())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// The absent-cell table's `unset` line through `tcl explore --show sccp`:
/// `unset p nosuch q` unbinds `p`, raises on the absent name and leaves `q`,
/// so the route answers an error after one store, and that store is the
/// unbinding of `p`.
#[test]
fn explore_sccp_prints_what_an_unset_stores_before_its_error() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set p 1; set q 2; unset p nosuch q}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        text.contains("route unset: direct variable-unset (registry)"),
        "{text}"
    );
    assert!(
        text.contains("· answer: evaluated: error after 1 store"),
        "{text}"
    );
    assert!(
        text.contains("· path error after 1 store: unbind p"),
        "{text}"
    );
}

/// The ordered-state witness whose expression raises after a write, in a
/// procedure, where the `catch` is lowered into blocks: `tcl explore --show
/// sccp` proves `x` 2 on the error path — the expression's route answers an
/// error after one store, the write of 2 — and so `x` after the `catch`.
#[test]
fn explore_sccp_prints_x_exact_on_the_error_path() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; catch {expr {[incr x] + [error mid]}} msg; return $x}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("route expr: expression tcl.expr"), "{text}");
    assert!(
        text.contains("· answer: evaluated: error after 1 store"),
        "{text}"
    );
    assert!(
        text.contains("· path error after 1 store: write x = 2"),
        "{text}"
    );
    assert!(text.contains("x#3 = const(2)"), "{text}");
    assert!(text.contains("msg#1 = const('mid')"), "{text}");
}

/// The programs of the design page's prefix rule through the shipped binary,
/// each in a procedure: a command that raises part-way leaves the stores
/// before its failure and nothing after — save `scan`, which goes on past
/// the array — and `tcl opt` keeps what each program prints under every
/// release from the first that has its command.
#[test]
fn opt_prints_what_tclsh_prints_for_the_prefix_programs() {
    let programs: [(&str, &str, &str, &str, &str); 10] = [
        (
            "set a old\n    array set b {k keep}",
            "lassign {new second} a b",
            "puts \"$a $b(k)\"",
            "new keep\n",
            "8.5",
        ),
        (
            "array set b {k keep}\n    set a old\n    set c old",
            "lassign {x y z} a b c",
            "puts \"$a $c\"",
            "x old\n",
            "8.5",
        ),
        (
            "array set b {k keep}\n    set a old",
            "foreach {a b} {new second} {set inside 1}",
            "puts \"$a [info exists inside]\"",
            "new 0\n",
            "8.4",
        ),
        (
            "array set b {k keep}\n    set a old",
            "scan {1 2} {%d %d} a b",
            "puts $a",
            "1\n",
            "8.4",
        ),
        (
            "array set a {k keep}\n    set b old",
            "scan {1 2} {%d %d} a b",
            "puts \"$a(k) $b\"",
            "keep 2\n",
            "8.4",
        ),
        (
            "array set b {k keep}\n    set a old",
            "regexp {(x)(y)} xy a b",
            "puts $a",
            "xy\n",
            "8.4",
        ),
        (
            "set p 1\n    set q 2",
            "unset p nosuch q",
            "puts \"[info exists p] $q\"",
            "0 2\n",
            "8.4",
        ),
        (
            "set x 1",
            "append x 2 [error boom]",
            "puts $x",
            "1\n",
            "8.4",
        ),
        (
            "set w 0",
            "set r [expr {1 + [error mid]}]",
            "puts [info exists r]",
            "0\n",
            "8.4",
        ),
        (
            "set x 1",
            "expr {[incr x] + [error mid]}",
            "puts $x",
            "2\n",
            "8.4",
        ),
    ];
    for (before, body, after, printed, first) in programs {
        let source =
            format!("proc p {{}} {{\n    {before}\n    catch {{{body}}} m\n    {after}\n}}\np\n");
        for (series, tclsh) in tclshs_from(first) {
            let optimised = opt_under(&source, series);
            for program in [source.as_str(), optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// #2141's programs through the shipped binary: a write nested in a braced
/// `expr` word is a write of the frame, so `tcl opt` forwards nothing past it
/// to the `puts $x` that follows a `puts` argument — `5` then `2` where it
/// printed `5` then `1` — and keeps the stores the expression reads, so the
/// optimised program neither raises `can't read "n"` nor answers 3 for
/// `[incr n] + [incr n]`; `tcl diag` calls neither store unused.
#[test]
fn opt_keeps_what_the_issue_2141_programs_read() {
    let programs = [
        (
            "set x 1\nputs [expr {$x + [incr x] + $x}]\nputs $x\n",
            "5\n2\n",
            "puts $x",
        ),
        (
            "set x 1\nputs [expr {$x + [set x 10] + $x}]\nputs $x\n",
            "21\n10\n",
            "puts $x",
        ),
        (
            "proc p {} {\n    set n 1\n    set r [expr {$n + [incr n]}]\n    return $r\n}\nputs [p]\n",
            "3\n",
            "set n 1",
        ),
        (
            "proc q {} {\n    set n 1\n    set r [expr {[incr n] + [incr n]}]\n    return $r\n}\nputs [q]\n",
            "5\n",
            "set n 1",
        ),
    ];
    for (source, printed, kept) in programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            assert!(
                statements_of(&optimised).contains(kept),
                "tcl{series}:\n{optimised}"
            );
            for program in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
        let found = diagnostics_at(source, "tcl8.6", &[]);
        assert!(
            !found.iter().any(|(_, _, code)| code == "W211"),
            "no store is unused: {found:?}\n{source}"
        );
    }
}

/// The nested equality program: inside `if {$x eq "a"}`'s arm `x` is `a`, so
/// `if {$x eq "b"}` there never holds. The procedure's argument is a global
/// no call site pins, so the outer test is the only evidence.
const NESTED_EQUALITY: &str = "proc p {x} {\n    if {$x eq \"a\"} {\n        if {$x eq \"b\"} {puts never} else {puts inner}\n    }\n}\nset ::v a\np $::v\nset ::v b\np $::v\n";

/// The nested equality decides through the shipped binary under every
/// release: `tcl diag` reports I230 on the inner condition (line 3), `tcl
/// opt --profile full` drops `puts never`, and both programs print `inner`
/// under every tclsh release on the oracle's path.
#[test]
fn the_nested_equality_decides_through_diag_and_opt() {
    for series in RELEASES {
        let dialect = format!("tcl{series}");
        let found = diagnostics_at(NESTED_EQUALITY, &dialect, &[]);
        assert!(
            found
                .iter()
                .any(|(line, _, code)| *line == 3 && code == "I230"),
            "{dialect}: {found:?}"
        );
        let optimised = statements_of(&opt_under(NESTED_EQUALITY, series));
        assert!(!optimised.contains("puts never"), "{dialect}:\n{optimised}");
    }
    for (series, tclsh) in tclshs_from("8.4") {
        let optimised = statements_of(&opt_under(NESTED_EQUALITY, series));
        for program in [NESTED_EQUALITY, optimised.as_str()] {
            assert_eq!(
                run_tclsh(&tclsh, program),
                Some((true, "inner\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// The eleven loop programs of the interface page (§ *Bounded-loop
/// enumeration*) through the shipped binary: each loop runs to its exit over
/// exact state, so `tcl diag` reports I230 on the branch after it (line 2) and
/// `tcl opt --profile full` keeps only the arm that runs, under every release
/// the analysis names; the original and the optimised program print the same
/// under every tclsh release on the oracle's path.
#[test]
fn the_eleven_loop_witnesses() {
    let witnesses = [
        ("for {set i 0} {$i < 5} {incr i} {}", "$i == 5", true),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {incr t $i}",
            "$t == 6",
            true,
        ),
        (
            "for {set i 0} {$i < 10} {incr i} {if {$i == 3} break}",
            "$i == 3",
            true,
        ),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {if {$i == 1} continue; incr t}",
            "$t == 3",
            true,
        ),
        ("for {set i 0} {$i < 2.5} {incr i} {}", "$i == 3", true),
        ("set i 0; for {} {$i < 3} {} {incr i 2}", "$i == 4", true),
        (
            "set n 0; for {set i 0} {$i < 3} {incr i} {set i [expr {$i + 1}]; incr n}",
            "$i == 4 && $n == 2",
            true,
        ),
        (
            "catch {for {set i 0} {$i < 5} {incr i} {if {$i == 2} {error x}}}",
            "$i == 2",
            true,
        ),
        ("foreach x {} {}", "[info exists x]", false),
        ("foreach {a b} {1 2 3} {}", "$a == 3 && $b eq \"\"", true),
        (
            "set n 0; foreach x {1 2 3} {if {$x == 2} break; incr n}",
            "$n == 1 && $x == 2",
            true,
        ),
    ];
    let tclshs = tclshs_from("8.4");
    for (program, condition, holds) in witnesses {
        let source = format!("{program}\nif {{{condition}}} {{puts yes}} else {{puts no}}\n");
        let (printed, dropped) = if holds { ("yes", "no") } else { ("no", "yes") };
        for series in RELEASES {
            let dialect = format!("tcl{series}");
            let found = diagnostics_at(&source, &dialect, &[]);
            assert!(
                found
                    .iter()
                    .any(|(line, _, code)| *line == 2 && code == "I230"),
                "{dialect}: {source}{found:?}"
            );
            let optimised = statements_of(&opt_under(&source, series));
            assert!(
                !optimised.contains(&format!("puts {dropped}")),
                "{dialect}: {source}{optimised}"
            );
            for (_, tclsh) in tclshs.iter().filter(|(found, _)| *found == series) {
                for text in [source.as_str(), optimised.as_str()] {
                    assert_eq!(
                        run_tclsh(tclsh, text),
                        Some((true, format!("{printed}\n"))),
                        "tclsh{series}:\n{text}"
                    );
                }
            }
        }
    }
}
