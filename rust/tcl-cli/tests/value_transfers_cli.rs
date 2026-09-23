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

//! The value-transfer lane's CLI witnesses
//! (`docs/design/lanes/value-transfers.md` § *Plan for slices 2–13*, D9):
//! the `tcl explore` and `tcl opt` evidence the lane's exit criteria name,
//! driven through the built `tcl` binary rather than the library directly.
//! Separate from `rust/tcl-cli/tests/cli.rs`, which the diagnostic-policy
//! lane edits; every later slice extends this file instead.

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

/// VT3.9's route tally, read from `tcl explore --show sccp`: two fused
/// `expr` statements are two expression entries, one nesting a
/// direct-routed `string length`, so `direct 1 · expression 2 ·
/// implementation 0` — the exit evidence's own line
/// (`docs/design/lanes/value-transfers.md` § *Slice 3* › *Goal and exit*).
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

/// VT2.10's deferred CLI witness binary (D9, D35): the `tcl explore` exit
/// lines slice 2's § *Goal and exit* names — program (3)'s `incr` route
/// twice, a release-ambiguous decline, and `llength`'s direct route — each
/// hand-verified at the time and now pinned here. The decline was
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

/// VT2.10: `tcl opt --profile full --dialect tcl8.6` forwards program
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

/// VT2.10: a value-position cell update whose read the host statement does
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

/// VT2.9 (#2214): a `::`-qualified global a nested `incr` writes is a
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

/// The value-transfer lane's executable example (VT4.13), whose one home is
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

/// The step-1 completion test on the CLI and the pack surfaces (VT4.13;
/// `docs/design/compiler/value-transfers.md` § *The completion test*),
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
