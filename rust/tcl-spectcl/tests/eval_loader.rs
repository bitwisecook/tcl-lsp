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

//! **The evaluation-loader gates** — design E (`SpecTcl` 2.0) against the
//! CST loader it must not diverge from.
//!
//! 1. **The equivalence gate**: every `.tclspec` the repository ships — the
//!    bundled packs under `specs/` and every file the corpus baseline
//!    covers — loads through BOTH loaders to byte-identical snapshots
//!    (`CommandSpec` debug form, the same exhaustive rendering
//!    `upgrade.rs`'s U9 round-trip compares) and identical notices modulo
//!    the allowed wording map below.
//! 2. **The contract tests**: templating equivalence, the determinism
//!    denial, the budget axes, E-R1 target-dependence, and E-R2 provenance
//!    gating.

use std::path::{Path, PathBuf};

use tcl_spectcl::loader::{
    EvalOptions, Notice, Pack, evaluate_pack, evaluate_pack_with, load_pack,
};
use tcl_spectcl::{LoadError, Tier};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Every `.tclspec` the repository ships: the same three directories the
/// corpus harness (`spec_corpus.rs`) scans, so the two inventories cannot
/// drift.
fn inventory() -> Vec<PathBuf> {
    let root = repo_root();
    let mut files = Vec::new();
    for dir in [
        root.join("specs"),
        root.join("docs/design/spec-dsl-examples"),
        root.join("docs/design/spec-dsl-examples/external"),
    ] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut here: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == tcl_spectcl::PACK_EXTENSION)
            })
            .collect();
        here.sort();
        files.extend(here);
    }
    files
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

/// Notices the evaluation loader adds or words differently, allowed by the
/// gate. Kept deliberately tiny: an entry here must name a structural
/// difference between "read from the CST" and "evaluated", never paper over
/// a divergent row.
fn allowed_eval_only(notice: &Notice) -> bool {
    // E-R1's cacheability notice exists only under evaluation.
    notice.message.contains("target-dependent registration")
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
                "line {} (byte {at}):\n  cst : …{}…\n  eval: …{}…",
                index + 1,
                window(left),
                window(right)
            );
        }
    }
    format!(
        "line counts differ: cst {} vs eval {}",
        a.lines().count(),
        b.lines().count()
    )
}

/// The dialect a pack's commands are exercised under — the same mapping the
/// corpus harness uses, so `command_entry_json` resolves the vendor packs
/// through the profile that actually admits them.
fn dialect_for(stem: &str) -> &'static str {
    match stem {
        "eda_xilinx" | "sdc_base" => "xilinx-eda-tcl",
        "upf" | "eda_synopsys" => "synopsys-eda-tcl",
        "eda_microchip" => "microchip-libero-eda-tcl",
        "eda_cadence" => "cadence-eda-tcl",
        "eda_quartus" => "intel-quartus-eda-tcl",
        "eda_mentor" => "mentor-eda-tcl",
        "irules-http-header" => "f5-irules",
        _ => "tcl9.1",
    }
}

#[test]
fn equivalence_gate_every_shipped_pack_loads_identically_through_both_loaders() {
    use tcl_registry::CommandRegistry;
    use tcl_registry::command_snapshot::command_entry_json;

    let files = inventory();
    assert!(
        files.len() >= 24,
        "the inventory must cover the shipped packs; found {files:?}"
    );

    // The `--verify` machinery's own view: each loader's specs installed
    // into a registry, compared entry by entry as `command_entry_json`.
    let mut cst_registry = CommandRegistry::build_default();
    let mut eval_registry = CommandRegistry::build_default();

    let mut packs = 0_usize;
    let mut commands = 0_usize;
    let mut entries = 0_usize;
    let mut notices = 0_usize;
    for path in files {
        let source = std::fs::read_to_string(&path).expect("readable pack");
        let cst = load_pack(&source);
        let eval = evaluate_pack(&source);

        let cst_snapshot = snapshot(&cst);
        let eval_snapshot = snapshot(&eval);
        assert!(
            cst_snapshot == eval_snapshot,
            "{}: snapshots diverge at {}",
            path.display(),
            first_diff(&cst_snapshot, &eval_snapshot)
        );

        // Byte-identical `command_entry_json` per declared command, through
        // a real registry, under the pack's own dialect profile.
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let dialect = dialect_for(stem);
        assert_eq!(
            cst.commands.len(),
            eval.commands.len(),
            "{}",
            path.display()
        );
        for (cst_command, eval_command) in cst.commands.iter().zip(&eval.commands) {
            cst_registry.insert_static(cst_command.spec);
            eval_registry.insert_static(eval_command.spec);
            let name = cst_command.spec.name;
            let left = command_entry_json(&cst_registry, dialect, name).map(|j| j.dumps_indent2());
            let right =
                command_entry_json(&eval_registry, dialect, name).map(|j| j.dumps_indent2());
            assert!(
                left == right,
                "{}: command_entry_json diverges for `{name}`\n  cst : {left:?}\n  eval: {right:?}",
                path.display()
            );
            if left.is_some() {
                entries += 1;
            }
        }

        let mut cst_notices: Vec<_> = cst.notices.iter().map(notice_key).collect();
        let mut eval_notices: Vec<_> = eval
            .notices
            .iter()
            .filter(|notice| !allowed_eval_only(notice))
            .map(notice_key)
            .collect();
        cst_notices.sort();
        eval_notices.sort();
        assert!(
            cst_notices == eval_notices,
            "{}: notices diverge\n  cst : {cst_notices:#?}\n  eval: {eval_notices:#?}",
            path.display()
        );

        assert!(
            !eval.target_dependent,
            "{}: shipped packs are target-independent",
            path.display()
        );
        packs += 1;
        commands += cst.commands.len();
        notices += cst.notices.len();
    }

    // The corpus ships 24 packs today, 776 bundled EDA commands among them;
    // the floors keep the gate meaningful if the scan ever goes blind.
    println!(
        "equivalence gate: {packs} packs, {commands} commands \
         ({entries} registry entries), {notices} notices compared"
    );
    assert!(packs >= 24, "only {packs} packs compared");
    assert!(commands >= 800, "only {commands} commands compared");
    assert!(entries >= 800, "only {entries} registry entries compared");
    // Sanity: the baseline says the design drafts carry notices.
    assert!(notices >= 10, "only {notices} notices compared");
}

// ---------------------------------------------------------------------------
// Templating (design E's reason to exist)
// ---------------------------------------------------------------------------

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

    // And the unrolled twin agrees with the CST loader, so the template's
    // output is exactly what the declarative pack would have said.
    let cst = load_pack(&unrolled);
    assert_eq!(render(&cst), render(&templated_pack));
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

// ---------------------------------------------------------------------------
// Determinism and budgets (§1.2)
// ---------------------------------------------------------------------------

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
        tier: Tier::Bundled,
        config: tcl_spec_hooks::pack_eval::PackEvalConfig {
            budget: tcl_engine_api::Budget::of_commands(2_000)
                .with_wall_clock(std::time::Duration::from_secs(5))
                .with_max_value_bytes(64 * 1024 * 1024),
        },
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
        tier: Tier::Bundled,
        config: tcl_spec_hooks::pack_eval::PackEvalConfig {
            budget: tcl_engine_api::Budget::of_commands(50_000_000)
                .with_wall_clock(std::time::Duration::from_millis(100))
                .with_max_value_bytes(64 * 1024 * 1024),
        },
    };
    let pack = evaluate_pack_with(source, &options);
    assert_eq!(
        pack.load_error,
        Some(LoadError::BudgetExhausted("wall clock")),
        "{:#?}",
        pack.notices
    );
}

// ---------------------------------------------------------------------------
// E-R1: available? is a trap
// ---------------------------------------------------------------------------

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

    // And the snapshot memo refuses it.
    let tier = Tier::Bundled;
    let cached = tcl_spectcl::evaluate_pack_cached(source, tier);
    assert!(cached.target_dependent);
    assert!(
        !tcl_spectcl::loader::eval_snapshot_memoised(source, &EvalOptions::default()),
        "a target-dependent pack must not be memoised (E-R1)"
    );

    // A target-independent pack IS memoised, so the exclusion above is
    // meaningful.
    let independent = "speclib cacheable 2.0 {\n    command base { arity 1 }\n}\n";
    let _ = tcl_spectcl::evaluate_pack_cached(independent, tier);
    assert!(tcl_spectcl::loader::eval_snapshot_memoised(
        independent,
        &EvalOptions::default()
    ));
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

// ---------------------------------------------------------------------------
// E-R2: provenance gates the registration call
// ---------------------------------------------------------------------------

#[test]
fn an_untrusted_pack_touching_a_reserved_name_fails_with_the_provenance_error() {
    let source = "speclib sneaky 2.0 {\n    command lsort -override { arity 1.. }\n}\n";
    let pack = evaluate_pack_with(
        source,
        &EvalOptions {
            tier: Tier::Workspace,
            ..EvalOptions::default()
        },
    );
    assert!(
        matches!(&pack.load_error, Some(LoadError::Provenance(message))
            if message.contains("workspace") && message.contains("lsort")),
        "{:?}",
        pack.load_error
    );
    assert!(pack.commands.is_empty(), "the violation is transactional");

    // The same pack from the bundled tier is trusted and loads.
    let trusted = evaluate_pack(source);
    assert!(trusted.load_error.is_none(), "{:?}", trusted.load_error);
    assert!(trusted.command("lsort").is_some());
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

// ---------------------------------------------------------------------------
// Unknown words degrade by vocabulary class, not as Tcl errors
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_registration_word_classifies_instead_of_erroring() {
    // Forward direction: the pack declares a vocabulary this build
    // postdates, so the unknown word gets its §6.1 class — here a
    // semantic-class word, which excludes the command rather than the pack.
    let forward = "speclib future 2.9 {\n    command risky {\n        arity 1\n        \
                   taint_gizmo colour\n    }\n    command safe { arity 1 }\n}\n";
    let cst = load_pack(forward);
    let eval = evaluate_pack(forward);
    assert!(
        eval.load_error.is_none(),
        "not a Tcl error: {:?}",
        eval.load_error
    );
    assert!(eval.command("risky").is_none(), "semantic-class exclusion");
    assert!(eval.command("safe").is_some());
    let render = |pack: &Pack| {
        pack.notices
            .iter()
            .map(|n| (n.context.clone(), n.line, n.class, n.message.clone()))
            .collect::<Vec<_>>()
    };
    let mut cst_notices = render(&cst);
    let mut eval_notices = render(&eval);
    cst_notices.sort();
    eval_notices.sort();
    assert_eq!(cst_notices, eval_notices);

    // Backward direction: an unknown word under a known vocabulary is an
    // author's typo — presentation-class, warn and drop, command kept.
    let typo = "speclib typo 2.0 {\n    command fine {\n        arity 1\n        \
                hoverr {oops}\n    }\n}\n";
    let eval = evaluate_pack(typo);
    assert!(eval.load_error.is_none());
    assert!(eval.command("fine").is_some());
    assert!(
        eval.notices
            .iter()
            .any(|n| n.message.contains("unknown property `hoverr` dropped")),
        "{:#?}",
        eval.notices
    );
}
