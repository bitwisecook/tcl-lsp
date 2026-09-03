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

//! **The evaluation-loader gates** — design E (`SpecTcl` 2.0), now the one
//! loader.
//!
//! 1. **The fast-path gate**: every `.tclspec` the repository ships — the
//!    bundled packs under `specs/` and every file the corpus baseline
//!    covers — loads to byte-identical snapshots (`CommandSpec` debug form,
//!    the same exhaustive rendering `upgrade.rs`'s U9 round-trip compares)
//!    and identical notices with the static fast path on and off. The fast
//!    path is the loader's shortcut past the interpreter for a body of
//!    purely static vocabulary; this is what keeps it provably an
//!    *optimisation* rather than a second reading of the file.
//!    Cross-*build* stability — that a change to the loader cannot silently
//!    alter what a shipped pack means — is the golden-snapshot gate in
//!    `golden_packs.rs`, which replaced the two-loader equivalence gate when
//!    the CST loader was deleted.
//! 2. **The contract tests**: templating equivalence, the determinism
//!    denial, the budget axes, E-R1 target-dependence, and E-R2 provenance
//!    gating.

use std::path::{Path, PathBuf};

use tcl_dialect::model::{Placement, SpecProvider};
use tcl_spectcl::loader::{EvalOptions, Notice, Pack, evaluate_pack, evaluate_pack_with};
use tcl_spectcl::{LoadError, Tier};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Every `.tclspec` the repository ships — the one inventory
/// [`tcl_spectcl::golden::shipped_packs`] owns, shared with the golden gate
/// and its regeneration verb so no two of them can disagree about what
/// "every shipped pack" means.
fn inventory() -> Vec<tcl_spectcl::golden::ShippedPackFile> {
    tcl_spectcl::golden::shipped_pack_inventory(&repo_root())
}

/// One notice as the comparison sees it. The line is compared too: the
/// evaluation loader reconstructs absolute lines from the interpreter's
/// per-command line and the body base-line stack, and the gate is what
/// keeps that reconstruction honest.
fn notice_key(notice: &Notice) -> (String, u32, String, String) {
    (
        notice.context.clone(),
        notice.line,
        notice.class.name().to_owned(),
        notice.message.clone(),
    )
}

/// The exhaustive snapshot rendering of one pack: every command's complete
/// `CommandSpec` (the same debug form `upgrade.rs`'s U9 test compares,
/// covering strictly more than the `command_entry_json` projection), plus
/// the loader-level per-command facts.
fn snapshot(pack: &Pack) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "pack {} dsl {} display {:?} load_error {:?}",
        pack.name, pack.dsl_version, pack.display_name, pack.load_error
    );
    let _ = writeln!(out, "file_extensions {:?}", pack.file_extensions);
    let _ = writeln!(out, "ambient_packages {:?}", pack.ambient_packages);
    let _ = writeln!(out, "environments {:?}", pack.environments);
    let _ = writeln!(out, "dialects {:?}", pack.dialects);
    for command in &pack.commands {
        let _ = writeln!(
            out,
            "command {} line {} overrides {} degraded {}",
            command.spec.name, command.line, command.overrides_shipped, command.degraded
        );
        let _ = writeln!(out, "  hooks {:?}", command.hooks);
        let _ = writeln!(out, "  clause_grammar {:?}", command.clause_grammar);
        let _ = writeln!(out, "  spec {:?}", command.spec);
    }
    out
}

/// The first line at which two multi-line renderings differ, for a
/// readable assertion message.
fn first_diff(a: &str, b: &str) -> String {
    for (index, (left, right)) in a.lines().zip(b.lines()).enumerate() {
        if left != right {
            // Show the window around the first differing byte, not the
            // (often identical) head of a very long line.
            let at = left
                .bytes()
                .zip(right.bytes())
                .position(|(l, r)| l != r)
                .unwrap_or(left.len().min(right.len()));
            let window = |s: &str| {
                let from = (0..=at.saturating_sub(120).min(s.len()))
                    .rev()
                    .find(|&i| s.is_char_boundary(i))
                    .unwrap_or(0);
                let to = ((at + 280).min(s.len())..=s.len())
                    .find(|&i| s.is_char_boundary(i))
                    .unwrap_or(s.len());
                s.get(from..to).unwrap_or("").to_owned()
            };
            return format!(
                "line {} (byte {at}):\n  a: …{}…\n  b: …{}…",
                index + 1,
                window(left),
                window(right)
            );
        }
    }
    format!(
        "line counts differ: {} vs {}",
        a.lines().count(),
        b.lines().count()
    )
}

/// Load with the static fast path disabled: every body goes through the
/// interpreter, including a 20k-line declarative one.
///
/// The budget is raised well past the production default because the point
/// of the fast path is that the interpreter route on a large declarative
/// pack is *slow* — `eda_xilinx` compiles and runs 20k statements of pure
/// vocabulary, which blows the production wall clock by design. The gate
/// wants the comparison, not the budget behaviour (which
/// `a_budget_blowing_loop_fails_closed_naming_the_axis` covers), so it buys
/// the time.
fn evaluate_through_the_interpreter(source: &str) -> Pack {
    evaluate_pack_with(
        source,
        &EvalOptions {
            static_fast_path: false,
            config: tcl_spec_hooks::pack_eval::PackEvalConfig {
                budget: tcl_engine_api::Budget::of_commands(2_000_000_000)
                    .with_wall_clock(std::time::Duration::from_mins(30))
                    .with_max_value_bytes(512 * 1024 * 1024),
            },
            ..EvalOptions::default()
        },
    )
}

#[test]
fn every_shipped_pack_loads_identically_with_and_without_the_static_fast_path() {
    use tcl_registry::CommandRegistry;
    use tcl_registry::command_snapshot::command_entry_json;

    let files = inventory();
    assert!(
        files.len() >= 24,
        "the inventory must cover the shipped packs; found {files:?}"
    );

    // The `--verify` machinery's own view: each route's specs installed
    // into a registry, compared entry by entry as `command_entry_json`.
    let mut fast_registry = CommandRegistry::build_default();
    let mut slow_registry = CommandRegistry::build_default();

    let mut packs = 0_usize;
    let mut commands = 0_usize;
    let mut entries = 0_usize;
    let mut notices = 0_usize;
    for shipped in files {
        let path = &shipped.path;
        let source = std::fs::read_to_string(path).expect("readable pack");
        let fast = evaluate_pack(&source);
        let slow = evaluate_through_the_interpreter(&source);

        let fast_snapshot = snapshot(&fast);
        let slow_snapshot = snapshot(&slow);
        assert!(
            fast_snapshot == slow_snapshot,
            "{}: snapshots diverge at {}",
            path.display(),
            first_diff(&fast_snapshot, &slow_snapshot)
        );

        // Byte-identical `command_entry_json` per declared command, through
        // a real registry, under the pack's own dialect profile.
        let dialect = shipped.dialect;
        assert_eq!(
            fast.commands.len(),
            slow.commands.len(),
            "{}",
            path.display()
        );
        for (fast_command, slow_command) in fast.commands.iter().zip(&slow.commands) {
            fast_registry.insert_static(fast_command.spec);
            slow_registry.insert_static(slow_command.spec);
            let name = fast_command.spec.name;
            let left = command_entry_json(&fast_registry, dialect, name).map(|j| j.dumps_indent2());
            let right =
                command_entry_json(&slow_registry, dialect, name).map(|j| j.dumps_indent2());
            assert!(
                left == right,
                "{}: command_entry_json diverges for `{name}`\n  fast: {left:?}\n  slow: {right:?}",
                path.display()
            );
            if left.is_some() {
                entries += 1;
            }
        }

        let mut fast_notices: Vec<_> = fast.notices.iter().map(notice_key).collect();
        let mut slow_notices: Vec<_> = slow.notices.iter().map(notice_key).collect();
        fast_notices.sort();
        slow_notices.sort();
        assert!(
            fast_notices == slow_notices,
            "{}: notices diverge\n  fast: {fast_notices:#?}\n  slow: {slow_notices:#?}",
            path.display()
        );

        assert!(
            !fast.target_dependent,
            "{}: shipped packs are target-independent",
            path.display()
        );
        packs += 1;
        commands += fast.commands.len();
        notices += fast.notices.len();
    }

    // The corpus ships 24 packs today, 776 bundled EDA commands among them;
    // the floors keep the gate meaningful if the scan ever goes blind.
    println!(
        "fast-path gate: {packs} packs, {commands} commands \
         ({entries} registry entries), {notices} notices compared"
    );
    assert!(packs >= 24, "only {packs} packs compared");
    assert!(commands >= 800, "only {commands} commands compared");
    assert!(entries >= 800, "only {entries} registry entries compared");
    // Sanity: the baseline says the design drafts carry notices.
    assert!(notices >= 10, "only {notices} notices compared");
}

// Templating (design E's reason to exist)

#[test]
fn a_templated_pack_equals_its_hand_unrolled_twin() {
    let templated = r"speclib fleet 2.0 {
    proc fleet-command {name} {
        command math::fleet::$name {
            arity 2
            traits {PURE}
            option -verbose
            subcommand probe {
                arity 0
                detail {Probe one input.}
            }
        }
    }
    foreach name {alpha beta gamma delta} {
        fleet-command $name
    }
}
";
    let unrolled_body = |name: &str| {
        format!(
            "    command math::fleet::{name} {{\n        arity 2\n        traits {{PURE}}\n        \
             option -verbose\n        subcommand probe {{\n            arity 0\n            \
             detail {{Probe one input.}}\n        }}\n    }}\n"
        )
    };
    let unrolled = format!(
        "speclib fleet 2.0 {{\n{}{}{}{}}}\n",
        unrolled_body("alpha"),
        unrolled_body("beta"),
        unrolled_body("gamma"),
        unrolled_body("delta"),
    );

    let templated_pack = evaluate_pack(templated);
    let unrolled_pack = evaluate_pack(&unrolled);
    assert!(
        templated_pack.load_error.is_none(),
        "{:#?}",
        templated_pack.notices
    );
    assert_eq!(templated_pack.commands.len(), 4);

    let render = |pack: &Pack| {
        pack.commands
            .iter()
            .map(|command| format!("{:?}", command.spec))
            .collect::<Vec<_>>()
    };
    assert_eq!(render(&templated_pack), render(&unrolled_pack));

    // And the unrolled twin says the same thing through the interpreter
    // route, so the template's output is exactly what the declarative pack
    // would have said however it was read.
    assert_eq!(
        render(&evaluate_through_the_interpreter(&unrolled)),
        render(&templated_pack)
    );
}

#[test]
fn a_command_body_may_template_its_own_rows() {
    let source = r"speclib rows 2.0 {
    command lsortish {
        arity 1..
        foreach mode {-ascii -dictionary -integer -real} {
            option $mode
        }
    }
}
";
    let pack = evaluate_pack(source);
    assert!(pack.load_error.is_none(), "{:#?}", pack.notices);
    let command = pack.command("lsortish").expect("loads");
    let options: Vec<&str> = command.spec.options.iter().map(|o| o.name).collect();
    assert_eq!(options, vec!["-ascii", "-dictionary", "-integer", "-real"]);
}

/// A variable set once and substituted into rows reaches the model as its
/// **value**, at pack scope and inside a declaration body alike.
///
/// The capture layer prefers the file's own bytes to the evaluated
/// invocation whenever the source statement at that line has the same
/// shape, because evaluation loses per-word braced-ness and physical
/// lines. That preference has to stop at a statement that substitutes:
/// replaying `ambient Tk $tkver` verbatim hands the reader the dollar sign
/// instead of the version, and the pack silently means something it did not
/// say (issue #1643).
#[test]
fn a_variable_substitutes_into_rows_at_every_scope() {
    let source = "speclib versioned 2.0 {\n\nset tkver 8.6\n\nenvironment probe-shell {\n    core tcl 8.6\n    ambient Tk $tkver\n}\n\ncommand demo {\n    arity 1\n    available \"package Tk $tkver-\"\n}\n\n}\n";
    let pack = evaluate_pack(source);
    assert!(pack.load_error.is_none(), "{:#?}", pack.notices);

    // The environment's `ambient` row carries the version, not the spelling.
    let environment = pack
        .environments
        .iter()
        .find(|environment| environment.id == "probe-shell")
        .expect("the environment block loads");
    let placement = environment
        .placements
        .iter()
        .find(|row| row.package == "Tk")
        .expect("the ambient placement loads");
    assert!(placement.ambient);
    assert!(
        matches!(&placement.version, Placement::Pinned(version) if version.to_string() == "8.6"),
        "{:?}",
        placement.version
    );

    // And the `available` window names the same package by value.
    let surface = pack
        .command("demo")
        .expect("the command loads")
        .spec
        .surface
        .expect("the available row loads");
    assert!(
        surface
            .iter()
            .any(|row| matches!(&row.provider, SpecProvider::Package(name) if *name == "Tk")),
        "{surface:?}"
    );
    assert!(
        !pack
            .notices
            .iter()
            .any(|notice| notice.message.contains("$tkver")),
        "nothing reports the unsubstituted spelling: {:?}",
        pack.notices
    );
}

/// An `environment` body is an evaluated **script**, exactly as a `command`
/// body is: a `foreach` inside it registers one row per iteration, and an
/// `if` decides whether a row is registered at all.
///
/// This is what makes the block the answer to #1643 rather than a second
/// declarative dialect — the author writes ordinary Tcl, and the block
/// reader still owns what every row it produced means.
#[test]
fn an_environment_body_runs_as_a_program() {
    let source = "speclib looped 2.0 {\n\nset shipped 1\n\nenvironment looped-shell {\n    core tcl 8.6\n    foreach suffix {aaa bbb ccc} {\n        file_extension $suffix\n    }\n    if {$shipped} {\n        ambient looplib 1.5\n    }\n}\n\n}\n";
    let pack = evaluate_pack(source);
    assert!(pack.load_error.is_none(), "{:#?}", pack.notices);
    assert!(pack.notices.is_empty(), "{:#?}", pack.notices);

    let environment = pack
        .environments
        .iter()
        .find(|environment| environment.id == "looped-shell")
        .expect("the environment block loads");
    let extensions: Vec<&str> = environment
        .file_extensions
        .iter()
        .map(|claim| claim.extension.as_ref())
        .collect();
    assert_eq!(
        extensions,
        vec!["aaa", "bbb", "ccc"],
        "the loop registered one row per iteration"
    );
    assert!(
        environment
            .placements
            .iter()
            .any(|row| row.package == "looplib" && row.ambient),
        "the `if` registered its row: {:?}",
        environment.placements
    );
}

/// The block readers keep every notice they had, reached through the
/// evaluated path.
///
/// An `environment` body is a script now, but it is still *that* block's
/// body: an unknown row is semantic-class and rejects the whole block, a
/// reserved compiled name is refused, and a `dialect` whose axes reproduce
/// a compiled release is sent back to `environment`. None of that moved
/// into the evaluator.
#[test]
fn the_block_readers_still_report_from_the_evaluated_path() {
    let unknown = evaluate_pack(
        "speclib probe 2.0 {\n\nenvironment probe-shell {\n    core tcl 8.6\n    invented_row yes\n}\n\n}\n",
    );
    assert!(
        unknown.environments.is_empty(),
        "an unknown row rejects the block"
    );
    assert!(
        unknown
            .notices
            .iter()
            .any(|notice| notice.message.contains("invented_row")),
        "{:?}",
        unknown.notices
    );

    let reserved =
        evaluate_pack("speclib probe 2.0 {\n\nenvironment tcl8.6 {\n    core tcl 8.6\n}\n\n}\n");
    assert!(reserved.environments.is_empty());
    assert!(
        reserved
            .notices
            .iter()
            .any(|notice| notice.message.contains("compiled environment name")),
        "{:?}",
        reserved.notices
    );

    let missing_body = evaluate_pack("speclib probe 2.0 {\n\nenvironment probe-shell\n\n}\n");
    assert!(missing_body.environments.is_empty());
    assert!(
        missing_body
            .notices
            .iter()
            .any(|notice| notice.message.contains("has no `{ … }` block")),
        "{:?}",
        missing_body.notices
    );

    let classified =
        evaluate_pack("speclib probe 2.0 {\n\ndialect probe-lang {\n    release tcl9.0\n}\n\n}\n");
    assert!(classified.dialects.is_empty());
    assert!(
        classified
            .notices
            .iter()
            .any(|notice| notice.message.contains("not a new dialect")),
        "{:?}",
        classified.notices
    );
}

/// A block body is the declaration's body **word**, whatever its value
/// looks like.
///
/// Evaluation hands the handler values, not words, so a one-word body
/// (`{emit}`) is the same string a flag would be. Picking the body out by
/// whitespace therefore missed it, staged the declaration with no body at
/// all, and rejected `environment one-word {emit}` for its formatting
/// rather than its content (issue #1643).
#[test]
fn a_block_body_is_located_by_position_not_by_whitespace() {
    // `emit` is a pack-defined proc, so both bodies are a single bare
    // word. The second declaration is templated, so it matches no source
    // statement and its body comes from the argument layout alone.
    let one_word = "speclib shaped 2.0 {\n\nproc emit {} {\n    core tcl 8.6\n    \
                    file_extension one\n}\n\nenvironment spelled {emit}\n\n\
                    foreach id {templated} {\n    environment $id {emit}\n}\n\n}\n";
    let pack = evaluate_through_the_interpreter(one_word);
    assert!(pack.load_error.is_none(), "{:#?}", pack.notices);
    for id in ["spelled", "templated"] {
        let environment = pack
            .environments
            .iter()
            .find(|environment| environment.id == id)
            .unwrap_or_else(|| panic!("the one-word block `{id}` loads: {:#?}", pack.notices));
        assert!(environment.core.is_some(), "the body ran: {environment:#?}");
        assert_eq!(
            environment
                .file_extensions
                .iter()
                .map(|claim| claim.extension.as_ref())
                .collect::<Vec<&str>>(),
            vec!["one"],
            "the proc registered its row into the block"
        );
    }

    // The empty body and the ordinary multi-row body stage and replay the
    // same way whichever path reads them.
    let shapes = "speclib shaped 2.0 {\n\nenvironment empty-shell {}\n\n\
                  environment normal-shell {\n    core tcl 8.6\n    \
                  file_extension many\n}\n\ndialect empty-lang {}\n\n}\n";
    let fast = evaluate_pack(shapes);
    let slow = evaluate_through_the_interpreter(shapes);
    assert!(fast.load_error.is_none(), "{:#?}", fast.notices);
    let keys = |pack: &Pack| {
        let mut rows: Vec<_> = pack.notices.iter().map(notice_key).collect();
        rows.sort();
        rows
    };
    assert_eq!(keys(&fast), keys(&slow));
    assert_eq!(
        fast.environments
            .iter()
            .map(|environment| environment.id.as_str())
            .collect::<Vec<&str>>(),
        slow.environments
            .iter()
            .map(|environment| environment.id.as_str())
            .collect::<Vec<&str>>(),
    );
    let normal = fast
        .environments
        .iter()
        .find(|environment| environment.id == "normal-shell")
        .expect("the multi-row block loads");
    assert_eq!(
        normal
            .file_extensions
            .iter()
            .map(|claim| claim.extension.as_ref())
            .collect::<Vec<&str>>(),
        vec!["many"],
    );
}

/// A braced name is refused on the evaluated path too.
///
/// `environment {custom} { … }` is the issue-#1638 mistake the block
/// readers reject, but evaluation loses per-word braced-ness: rebuilding
/// the header from values alone made the name look bare, and the
/// declaration was accepted whenever the pack happened to need the
/// interpreter.
#[test]
fn a_braced_block_name_is_rejected_on_both_paths() {
    for word in ["environment", "dialect"] {
        // The declarative pack takes the static path; the `set` makes the
        // same declaration reach the interpreter instead.
        let declarative =
            format!("speclib braced 2.0 {{\n\n{word} {{custom}} {{\n    core tcl 8.6\n}}\n\n}}\n");
        let programmed = format!(
            "speclib braced 2.0 {{\n\nset forced 1\n\n{word} {{custom}} {{\n    \
             core tcl 8.6\n}}\n\n}}\n"
        );
        for pack in [
            evaluate_pack(&declarative),
            evaluate_pack(&programmed),
            evaluate_through_the_interpreter(&declarative),
        ] {
            assert!(
                pack.environments.is_empty() && pack.dialects.is_empty(),
                "`{word} {{custom}}` declares nothing: {:#?}",
                pack.notices
            );
            assert!(
                pack.notices.iter().any(|notice| notice.message
                    == format!("`{word}` needs a name and a `{{ … }}` block")),
                "{:?}",
                pack.notices
            );
        }
    }
}

// Determinism and budgets (§1.2)

#[test]
fn a_pack_calling_clock_fails_closed_with_the_determinism_notice() {
    let source = "speclib clocky 2.0 {\n    command fine { arity 1 }\n    \
                  set now [clock seconds]\n    command never { arity 1 }\n}\n";
    let pack = evaluate_pack(source);
    assert!(
        matches!(&pack.load_error, Some(LoadError::Determinism(message))
            if message.contains("`clock`") && message.contains("clock/time")),
        "{:?}",
        pack.load_error
    );
    assert!(pack.commands.is_empty(), "registration is transactional");
    let notice = pack
        .notices
        .iter()
        .find(|n| n.message.contains("deterministic"))
        .expect("the determinism notice");
    assert!(notice.message.contains("clock/time"), "{notice:?}");
}

#[test]
fn a_budget_blowing_loop_fails_closed_naming_the_axis() {
    let source = "speclib hungry 2.0 {\n    command fine { arity 1 }\n    \
                  set i 0\n    while {1} { set x [llength [list a b $i]]\n incr i }\n}\n";
    let options = EvalOptions {
        config: tcl_spec_hooks::pack_eval::PackEvalConfig {
            budget: tcl_engine_api::Budget::of_commands(2_000)
                .with_wall_clock(std::time::Duration::from_secs(5))
                .with_max_value_bytes(64 * 1024 * 1024),
        },
        ..EvalOptions::default()
    };
    let pack = evaluate_pack_with(source, &options);
    assert_eq!(
        pack.load_error,
        Some(LoadError::BudgetExhausted("command steps")),
        "{:#?}",
        pack.notices
    );
    assert!(pack.commands.is_empty(), "registration is transactional");
    let notice = pack.notices.first().expect("one explaining notice");
    assert!(
        notice.message.contains("command steps"),
        "the notice names the axis: {notice:?}"
    );
}

#[test]
fn a_wall_clock_blowing_loop_names_its_own_axis() {
    let source = "speclib spinny 2.0 {\n    set i 0\n    while {1} { incr i }\n}\n";
    let options = EvalOptions {
        config: tcl_spec_hooks::pack_eval::PackEvalConfig {
            budget: tcl_engine_api::Budget::of_commands(50_000_000)
                .with_wall_clock(std::time::Duration::from_millis(100))
                .with_max_value_bytes(64 * 1024 * 1024),
        },
        ..EvalOptions::default()
    };
    let pack = evaluate_pack_with(source, &options);
    assert_eq!(
        pack.load_error,
        Some(LoadError::BudgetExhausted("wall clock")),
        "{:#?}",
        pack.notices
    );
}

// E-R1: available? is a trap

#[test]
fn available_query_marks_the_pack_target_dependent_and_uncacheable() {
    let source = "speclib trap 2.0 {\n    default available {tcl 8.6-}\n    \
                  command base { arity 1 }\n    if {[available? {tcl 8.6-}]} {\n        \
                  command extra { arity 1 }\n    }\n}\n";
    let pack = evaluate_pack(source);
    assert!(pack.load_error.is_none(), "{:#?}", pack.notices);
    assert!(pack.target_dependent, "available? downgrades cacheability");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("target-dependent registration")),
        "{:#?}",
        pack.notices
    );
    // The union of the declared support ({tcl 8.6-}) intersects the query,
    // so the branch ran and both commands registered.
    assert!(pack.command("base").is_some());
    assert!(pack.command("extra").is_some());

    // And the cache refuses it.
    let tier = Tier::Bundled;
    let cached = tcl_spectcl::evaluate_pack_cached(source, tier);
    assert!(cached.target_dependent);
    assert!(
        !tcl_spectcl::snapshot_memoised(source, tier),
        "a target-dependent pack must not be memoised (E-R1)"
    );

    // A target-independent pack IS memoised, so the exclusion above is
    // meaningful.
    let independent = "speclib cacheable 2.0 {\n    command base { arity 1 }\n}\n";
    let _ = tcl_spectcl::evaluate_pack_cached(independent, tier);
    assert!(tcl_spectcl::snapshot_memoised(independent, tier));
}

#[test]
fn available_query_answers_against_the_declared_union() {
    // Declared support is Tk only; a query for f5-irules misses it, so the
    // guarded registration must not happen.
    let source = "speclib narrow 2.0 {\n    default available {package Tk}\n    \
                  if {[available? {f5-irules}]} {\n        command wrong { arity 1 }\n    }\n    \
                  if {[available? {package Tk}]} {\n        command right { arity 1 }\n    }\n}\n";
    let pack = evaluate_pack(source);
    assert!(
        pack.command("wrong").is_none(),
        "{:#?}",
        pack.commands
            .iter()
            .map(|c| c.spec.name)
            .collect::<Vec<_>>()
    );
    assert!(pack.command("right").is_some());
}

// E-R2: provenance gates the registration call

#[test]
fn an_untrusted_pack_touching_a_reserved_name_fails_with_the_provenance_error() {
    let source = "speclib sneaky 2.0 {\n    command lsort -override { arity 1.. }\n}\n";
    let pack = evaluate_pack_with(
        source,
        &EvalOptions {
            tier: Tier::StudioOverride,
            ..EvalOptions::default()
        },
    );
    assert!(
        matches!(&pack.load_error, Some(LoadError::Provenance(message))
            if message.contains("Spec Studio override") && message.contains("lsort")),
        "{:?}",
        pack.load_error
    );
    assert!(pack.commands.is_empty(), "the violation is transactional");

    // The same pack from the bundled tier is trusted and loads.
    let trusted = evaluate_pack(source);
    assert!(trusted.load_error.is_none(), "{:?}", trusted.load_error);
    assert!(trusted.command("lsort").is_some());
}

/// A **workspace** pack is `Provenance::WorkspaceTrusted` — §6.4 keys the
/// untrusted class on the editor's Workspace Trust state, not on where the
/// file was found — so it may still `-override` a shipped command, which is
/// the collision policy `install.rs` implements. The refusal an untrusted
/// tier *would* give is still available to an authoring tool, asked of the
/// snapshot.
#[test]
fn a_workspace_pack_may_still_override_a_shipped_command() {
    let source = "speclib bold 2.0 {\n    command lsort -override { arity 1.. }\n}\n";
    let pack = evaluate_pack_with(
        source,
        &EvalOptions {
            tier: Tier::Workspace,
            ..EvalOptions::default()
        },
    );
    assert!(pack.load_error.is_none(), "{:?}", pack.load_error);
    assert!(pack.command("lsort").is_some());
    let (_, why) = tcl_spectcl::provenance_violation(&pack, Tier::StudioOverride)
        .expect("the hypothetical refusal is still reportable");
    assert!(why.contains("lsort"), "{why}");
}

#[test]
fn an_untrusted_pack_declaring_dialect_axes_fails_with_the_provenance_error() {
    let source = "speclib axes 2.0 {\n    dialect my-tcl {\n        release tcl9.0\n    }\n}\n";
    let pack = evaluate_pack_with(
        source,
        &EvalOptions {
            tier: Tier::StudioOverride,
            ..EvalOptions::default()
        },
    );
    assert!(
        matches!(&pack.load_error, Some(LoadError::Provenance(message))
            if message.contains("dialect") && message.contains("Spec Studio override")),
        "{:?}",
        pack.load_error
    );
}

// Unknown words degrade by vocabulary class, not as Tcl errors

#[test]
fn an_unknown_registration_word_classifies_instead_of_erroring() {
    // Forward direction: the pack declares a vocabulary this build
    // postdates, so the unknown word gets its §6.1 class — here a
    // semantic-class word, which excludes the command rather than the pack.
    let forward = "speclib future 2.9 {\n    command risky {\n        arity 1\n        \
                   taint_gizmo colour\n    }\n    command safe { arity 1 }\n}\n";
    let fast = evaluate_pack(forward);
    let slow = evaluate_through_the_interpreter(forward);
    assert!(
        fast.load_error.is_none(),
        "not a Tcl error: {:?}",
        fast.load_error
    );
    assert!(fast.command("risky").is_none(), "semantic-class exclusion");
    assert!(fast.command("safe").is_some());
    let render = |pack: &Pack| {
        let mut rows = pack
            .notices
            .iter()
            .map(|n| (n.context.clone(), n.line, n.class, n.message.clone()))
            .collect::<Vec<_>>();
        rows.sort();
        rows
    };
    assert_eq!(render(&fast), render(&slow));

    // Backward direction: an unknown word under a known vocabulary is an
    // author's typo — presentation-class, warn and drop, command kept.
    let typo = "speclib typo 2.0 {\n    command fine {\n        arity 1\n        \
                hoverr {oops}\n    }\n}\n";
    let typo_pack = evaluate_pack(typo);
    assert!(typo_pack.load_error.is_none());
    assert!(typo_pack.command("fine").is_some());
    assert!(
        typo_pack
            .notices
            .iter()
            .any(|n| n.message.contains("unknown property `hoverr` dropped")),
        "{:#?}",
        typo_pack.notices
    );
}

/// The 2.0 word batch loads identically on both routes — the row-reader seam
/// means the capture layer's shortcut cannot drift on `provides`,
/// `co_provides`, `dynamic_surface`/`unknown_members`, or the
/// `environment -extend` block.
#[test]
fn the_new_two_point_oh_words_load_identically_on_both_routes() {
    let source = "speclib probe 2.0 {\n\
                  provides upf 1.0\n\
                  co_provides Tk -requires-exact tk\n\
                  environment synopsys-eda-tcl -extend {\n\
                  \x20   file_extension upfx -name {Probe UPF Extension}\n\
                  }\n\
                  command demo {\n\
                  \x20   arity 1\n\
                  \x20   dynamic_surface\n\
                  }\n\
                  }\n";
    let fast = evaluate_pack(source);
    let slow = evaluate_through_the_interpreter(source);
    assert!(fast.notices.is_empty(), "{:#?}", fast.notices);
    assert_eq!(
        snapshot(&fast),
        snapshot(&slow),
        "{}",
        first_diff(&snapshot(&fast), &snapshot(&slow))
    );
    assert_eq!(fast.provides, slow.provides);
    assert_eq!(fast.co_provides, slow.co_provides);
    assert!(slow.environments[0].extends);
}

/// A straight-line `include` pack splices the same fragment whichever route
/// the loader takes, under the same content-hash keyed determinism rules.
#[test]
fn an_included_fragment_loads_identically_on_both_routes() {
    let resolver = |name: &str| match name {
        "extra.frag" => Ok("command extra {\n arity 2\n}\n".to_owned()),
        other => Err(format!("no such fragment `{other}`")),
    };
    let source = "speclib probe 2.0 {\n include extra.frag\n command demo { arity 1 }\n}\n";
    let including = |source: &str, fast_path: bool| {
        tcl_spectcl::loader::evaluate_pack_in(
            source,
            &EvalOptions {
                static_fast_path: fast_path,
                ..EvalOptions::default()
            },
            Some(std::rc::Rc::new(tcl_spectcl::IncludeContext::new(resolver))),
        )
    };
    let fast = including(source, true);
    let slow = including(source, false);
    assert!(fast.notices.is_empty(), "{:#?}", fast.notices);
    assert!(slow.notices.is_empty(), "{:#?}", slow.notices);
    assert_eq!(
        snapshot(&fast),
        snapshot(&slow),
        "{}",
        first_diff(&snapshot(&fast), &snapshot(&slow))
    );
    assert!(fast.command("extra").is_some());

    // Both loaders refuse the same cycle, with the same notice.
    let cyclic_source = "speclib probe 2.0 {\n include self.frag\n}\n";
    let cycle = |name: &str| match name {
        "self.frag" => Ok("include self.frag\n".to_owned()),
        other => Err(format!("no such fragment `{other}`")),
    };
    let cycling = |fast_path: bool| {
        tcl_spectcl::loader::evaluate_pack_in(
            cyclic_source,
            &EvalOptions {
                static_fast_path: fast_path,
                ..EvalOptions::default()
            },
            Some(std::rc::Rc::new(tcl_spectcl::IncludeContext::new(cycle))),
        )
    };
    let fast_cycle = cycling(true);
    let slow_cycle = cycling(false);
    let keys = |pack: &Pack| {
        let mut keys: Vec<_> = pack
            .notices
            .iter()
            .map(|n| (n.context.clone(), n.class, n.message.clone()))
            .collect();
        keys.sort();
        keys
    };
    assert_eq!(keys(&fast_cycle), keys(&slow_cycle));
    assert!(
        fast_cycle
            .notices
            .iter()
            .any(|n| n.message.contains("include cycle")),
        "{:#?}",
        fast_cycle.notices
    );
}

/// The ratified words load identically on both routes: they are read at the
/// one row seam, so the capture layer's shortcut cannot drift on them.
#[test]
fn the_ratified_words_load_identically_on_both_routes() {
    let source = r"
speclib probe 2.0 {
    command probe::collect {
        arity 0
        result_stability Volatile
        data_collection -native HTTP_COLLECT
        side_switch_target Server
        event_handler_priority -default 500 -min 0 -max 1000 -warn-implicit
        event_requirement_form {append} -only-in {HTTP_REQUEST} {
            client_side yes
        }
        body_scope {
            name {probe body}
            command top {
                arity 1..2
                subcommand set { arity 1 }
            }
        }
        subcommand line {
            arity 0
            result_stability ReferentiallyTransparent
        }
    }
}
";
    let fast = evaluate_pack(source);
    let slow = evaluate_through_the_interpreter(source);
    assert!(fast.notices.is_empty(), "{:#?}", fast.notices);
    assert!(slow.notices.is_empty(), "{:#?}", slow.notices);
    assert_eq!(
        snapshot(&fast),
        snapshot(&slow),
        "{}",
        first_diff(&snapshot(&fast), &snapshot(&slow))
    );
}
