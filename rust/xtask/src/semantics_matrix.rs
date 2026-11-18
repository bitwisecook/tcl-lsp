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

//! `cargo xtask semantics-matrix` — generate or verify the coverage matrix of
//! the four purity domains (`docs/generated/semantics-coverage.md`, issue
//! #1712).
//!
//! For every shipped command and subcommand of each group it records how
//! completely its side effects, world effects, completion and result
//! stability are known, the Tcl releases at which purity derives from them,
//! every row authored pure before purity was derived that no evidence
//! closes, the audit findings recorded as unknown, and the disposition of
//! each authored purity descriptor.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use tcl_dialect::model::{Family, SurfaceQuery};
use tcl_registry::commands;
use tcl_registry::knowledge::{Knowledge, KnowledgeLevel, complete_window_words};
use tcl_registry::purity::PurityDomains;
use tcl_registry::spec::CommandSpec;
use tcl_registry::world_effect::callback_knowledge;

use crate::util::repo_root;

const REPORT_PATH: &str = "docs/generated/semantics-coverage.md";
const EVIDENCE_DIR: &str = "rust/tcl-registry/tests/data/command_evidence";
const RELEASES: &[&str] = &["8.4", "8.5", "8.6", "9.0", "9.1"];

/// One shipped row: a command, or a subcommand of one.
struct Row {
    group: &'static str,
    name: String,
    domains: PurityDomains,
    /// The other domains that state their completeness: state transitions,
    /// and the callbacks the world effects carry.
    state_transitions: KnowledgeLevel,
    callbacks: KnowledgeLevel,
}

fn groups() -> Vec<(&'static str, Vec<CommandSpec>)> {
    vec![
        ("tcl", commands::tcl::tcl_command_specs()),
        ("stdlib", commands::stdlib::stdlib_command_specs()),
        ("tcllib", commands::tcllib::tcllib_command_specs()),
        ("tk", commands::tk::tk_command_specs()),
        ("expect", commands::expect::expect_command_specs()),
        ("irules", commands::irules::irules_command_specs()),
        ("iapps", commands::iapps::iapps_command_specs()),
    ]
}

fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (group, specs) in groups() {
        let mut seen = BTreeSet::new();
        for spec in &specs {
            if !seen.insert(spec.name) {
                continue;
            }
            rows.push(Row {
                group,
                name: spec.name.trim_start_matches("::").to_owned(),
                domains: PurityDomains::of_command(spec),
                state_transitions: spec.state_transitions.level(),
                callbacks: callback_knowledge(spec.world_effects).level(),
            });
            for sub in spec.subcommands {
                rows.push(Row {
                    group,
                    name: format!("{} {}", spec.name.trim_start_matches("::"), sub.name),
                    domains: PurityDomains::of_subcommand(spec, sub),
                    state_transitions: sub.state_transitions.or(spec.state_transitions).level(),
                    callbacks: callback_knowledge(sub.world_effects.or(spec.world_effects)).level(),
                });
            }
        }
    }
    rows
}

fn level<T>(knowledge: &Knowledge<T>) -> KnowledgeLevel {
    knowledge.level()
}

/// The release windows a domain is complete for, spelled as the DSL does.
fn windows<T>(knowledge: &Knowledge<T>) -> String {
    knowledge
        .complete_surfaces()
        .map_or_else(String::new, |surfaces| {
            surfaces
                .iter()
                .filter_map(complete_window_words)
                .map(|words| format!("{{{}}}", words.join(" ")))
                .collect::<Vec<_>>()
                .join(" ")
        })
}

/// `release → name → (verdict, classes)` from the C evidence.
/// `release → name → (verdict, classes)`.
type Evidence = BTreeMap<&'static str, BTreeMap<String, (String, String)>>;

fn c_evidence(root: &Path) -> Result<Evidence> {
    let mut out = BTreeMap::new();
    for release in RELEASES {
        let path = root.join(EVIDENCE_DIR).join(format!("tcl{release}.txt"));
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let lines = text
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let name = fields.next()?.to_owned();
                let verdict = fields.next()?.to_owned();
                let classes = fields.next().unwrap_or("-").to_owned();
                Some((name, (verdict, classes)))
            })
            .collect();
        out.insert(*release, lines);
    }
    Ok(out)
}

fn formerly_pure(root: &Path) -> Result<Vec<(String, String, String)>> {
    let path = root.join(EVIDENCE_DIR).join("formerly_pure.tsv");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.split('\t');
            let group = fields.next().unwrap_or_default().to_owned();
            let command = fields.next().unwrap_or_default().to_owned();
            let sub = fields.next().unwrap_or_default().to_owned();
            (group, command, sub)
        })
        .collect())
}

fn render(rows: &[Row], evidence: &Evidence, formerly: &[(String, String, String)]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Purity domains: coverage matrix\n\n\
         Generated by `cargo xtask semantics-matrix` (issue #1712); do not edit.\n\n\
         A call is pure at a release where its side effects, world effects, completion and \
         result stability are all `Complete` there and closed: no effect, no world access, \
         only `Ok` or `Error`, and a referentially transparent result \
         (`tcl_registry::purity::PurityDomains`). `Unknown` claims nothing and `Partial` is a \
         lower bound, so neither is ever pure. A domain is `Complete` only for the release \
         windows its implementation's source evidences \
         (`rust/tcl-registry/tests/data/command_evidence`, written by \
         `scripts/dev/tcl_command_evidence.py` and `scripts/dev/tcl_script_evidence.py`).\n"
    );

    render_groups(&mut out, rows);
    render_pure_rows(&mut out, rows);
    render_formerly_pure(&mut out, rows, evidence, formerly);
    render_f5(&mut out, formerly);
    render_findings(&mut out, rows);
    render_dispositions(&mut out);
    out
}

fn render_groups(out: &mut String, rows: &[Row]) {
    let _ = writeln!(out, "## Knowledge by group\n");
    let _ = writeln!(
        out,
        "Rows are commands and subcommands. Each domain is counted as unknown / partial / \
         complete; the last columns count the rows pure at each Tcl release. Callbacks are \
         the `callback` of the world-effect footprint, so their completeness is the world \
         effects' (`tcl_registry::world_effect::callback_knowledge`), only a lower bound where \
         the descriptor computes more per call.\n"
    );
    let _ = writeln!(
        out,
        "| group | rows | side effects | world effects | completion | result stability | state transitions | callbacks | pure at {} |",
        RELEASES.join(" / ")
    );
    let _ = writeln!(
        out,
        "| --- | ---: | --- | --- | --- | --- | --- | --- | --- |"
    );
    let mut by_group: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
    for row in rows {
        by_group.entry(row.group).or_default().push(row);
    }
    for (group, rows) in &by_group {
        let count = |pick: &dyn Fn(&Row) -> KnowledgeLevel| {
            let mut tally = [0usize; 3];
            for row in rows {
                tally[pick(row) as usize] += 1;
            }
            format!("{} / {} / {}", tally[0], tally[1], tally[2])
        };
        let world = |row: &Row| {
            let w = row.domains.world_effects;
            [w.form, w.subcommand, w.command]
                .into_iter()
                .map(|layer| layer.level())
                .find(|level| *level != KnowledgeLevel::Unknown)
                .unwrap_or(KnowledgeLevel::Unknown)
        };
        let pure: Vec<String> = RELEASES
            .iter()
            .map(|release| {
                rows.iter()
                    .filter(|row| {
                        row.domains
                            .pure_at(Some(&SurfaceQuery::core(Family::Tcl, release)))
                    })
                    .count()
                    .to_string()
            })
            .collect();
        let _ = writeln!(
            out,
            "| {group} | {} | {} | {} | {} | {} | {} | {} | {} |",
            rows.len(),
            count(&|row| level(&row.domains.side_effects)),
            count(&world),
            count(&|row| level(&row.domains.completion)),
            count(&|row| level(&row.domains.result_stability)),
            count(&|row| row.state_transitions),
            count(&|row| row.callbacks),
            pure.join(" / ")
        );
    }
}

fn render_pure_rows(out: &mut String, rows: &[Row]) {
    let _ = writeln!(out, "\n## Pure rows\n");
    let _ = writeln!(
        out,
        "Every row pure at some Tcl release, with the windows its four domains are complete for.\n"
    );
    let _ = writeln!(out, "| group | row | pure at | complete for |");
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for row in rows {
        let releases = row.domains.pure_releases(Family::Tcl);
        if releases.is_empty() {
            continue;
        }
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | `{}` |",
            row.group,
            row.name,
            releases.join(" "),
            windows(&row.domains.side_effects)
        );
    }
}

fn render_formerly_pure(
    out: &mut String,
    rows: &[Row],
    evidence: &Evidence,
    formerly: &[(String, String, String)],
) {
    let _ = writeln!(out, "\n## Formerly pure, not evidenced\n");
    let _ = writeln!(
        out,
        "Rows authored pure before purity was derived (`formerly_pure.tsv`) that no evidence \
         closes on any release. They stay impure. For a core Tcl row the reason is the first \
         world-touching class its C closure reaches on the newest release that has it; a row \
         with no C entry is a Tcl-library procedure, a spelling the C tables do not register, \
         or a Jim command. Other groups have no source evidence here.\n"
    );
    let _ = writeln!(out, "| group | row | why |");
    let _ = writeln!(out, "| --- | --- | --- |");
    let pure_names: BTreeSet<(&str, &str)> = rows
        .iter()
        .filter(|row| !row.domains.pure_releases(Family::Tcl).is_empty())
        .map(|row| (row.group, row.name.as_str()))
        .collect();
    let mut listed = BTreeSet::new();
    for (group, command, sub) in formerly {
        let name = if sub.is_empty() {
            command.trim_start_matches("::").to_owned()
        } else {
            format!("{} {sub}", command.trim_start_matches("::"))
        };
        if pure_names.contains(&(group.as_str(), name.as_str()))
            || !listed.insert((group.clone(), name.clone()))
        {
            continue;
        }
        let why = if group == "tcl" {
            RELEASES
                .iter()
                .rev()
                .find_map(|release| evidence.get(release).and_then(|lines| lines.get(&name)))
                .map_or_else(
                    || "no C entry".to_owned(),
                    |(verdict, classes)| {
                        if verdict == "closed" {
                            "closed, but authored facts are not closed".to_owned()
                        } else {
                            format!("open: `{}`", classes.split(',').next().unwrap_or(classes))
                        }
                    },
                )
        } else {
            format!("no source evidence for `{group}`")
        };
        let _ = writeln!(out, "| {group} | `{name}` | {why} |");
    }
}

/// Rows authored pure that a dialect's own release does not close: those
/// the core-Tcl evidence closes at 8.4, and the rest.
fn f5_lost(dialect: &str, formerly: &[(String, String, String)]) -> (Vec<String>, usize) {
    let authored: BTreeSet<(&str, &str)> = formerly
        .iter()
        .map(|(_, command, sub)| (command.as_str(), sub.as_str()))
        .collect();
    let context = tcl_registry::model::resolve_environment(dialect).default_context_registry();
    let registry = context.commands();
    let query = registry
        .profile()
        .map(tcl_dialect::DialectProfile::surface_query);
    let at_84 = SurfaceQuery::core(Family::Tcl, "8.4");
    let mut core = BTreeSet::new();
    let mut others = 0usize;
    let names: Vec<String> = registry.command_names().map(str::to_owned).collect();
    for name in &names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        let mut check = |sub: &str, domains: PurityDomains| {
            if !authored.contains(&(name.as_str(), sub)) || domains.pure_at(query.as_ref()) {
                return;
            }
            if domains.pure_at(Some(&at_84)) {
                let row = if sub.is_empty() {
                    name.clone()
                } else {
                    format!("{name} {sub}")
                };
                core.insert(row);
            } else {
                others += 1;
            }
        };
        check("", PurityDomains::of_command(spec));
        for sub in spec.subcommands {
            check(sub.name, PurityDomains::of_subcommand(spec, sub));
        }
    }
    (core.into_iter().collect(), others)
}

fn render_f5(out: &mut String, formerly: &[(String, String, String)]) {
    let _ = writeln!(out, "\n## Pure under Tcl 8.4, not evidenced for F5\n");
    let (core, others) = f5_lost("f5-irules", formerly);
    let _ = writeln!(
        out,
        "An iRules call resolves at the F5 iRules and F5 Tcl releases, which descend from \
         Tcl 8.4 by a fork whose source is not available, so the core-Tcl evidence does not \
         close them. These {} rows were authored pure and are pure at Tcl 8.4, and are not \
         pure under `f5-irules`; `docs/work/name-handling/bigip-name-checks.md` check 32 is the \
         device test that would evidence them. A further {others} rows authored pure under \
         `f5-irules` (iRules commands and core rows not closed at 8.4) have no evidence at all.\n",
        core.len()
    );
    let _ = writeln!(
        out,
        "{}",
        core.iter()
            .map(|row| format!("`{row}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn render_findings(out: &mut String, rows: &[Row]) {
    let _ = writeln!(out, "\n## Audit findings recorded as unknown\n");
    let tmsh: Vec<&str> = rows
        .iter()
        .filter(|row| row.group == "iapps" && row.name.starts_with("tmsh::"))
        .filter(|row| row.domains.side_effects.is_unknown())
        .map(|row| row.name.as_str())
        .collect();
    let _ = writeln!(
        out,
        "- `tmsh::*`: {} iApps rows state no side effects, so their side effects are `Unknown`, \
         never verified empty. `tmsh::create`, `modify` and `delete` change the BIG-IP \
         configuration; what each touches needs a device (`docs/work/name-handling/bigip-name-checks.md`).",
        tmsh.len()
    );
    let _ = writeln!(
        out,
        "- `SideEffectTarget::ChannelIo` is stated by no shipped row: the core channel commands \
         state `FileIo`, as a lower bound. Channel effects are `Partial` until a row is evidenced."
    );
    let _ = writeln!(
        out,
        "- iRules `event … disable` states no side effect of its own: whether it is event control \
         or connection control on a device is unknown, so its effects are `Unknown` for that form."
    );
}

fn render_dispositions(out: &mut String) {
    let _ = writeln!(out, "\n## Disposition of authored purity descriptors\n");
    let _ = writeln!(out, "| descriptor | disposition |");
    let _ = writeln!(out, "| --- | --- |");
    for (descriptor, disposition) in [
        (
            "the `PURE` trait",
            "retired; derived from the four domains (`CommandSpec::is_pure_at`, `InvocationFacts::is_pure`). `RETIRED_TRAITS` names the replacement and the retired-API gate refuses the spelling.",
        ),
        (
            "`SubCommand::pure`",
            "retired with the trait; a subcommand's purity derives from its own or its command's domains (`SubCommand::is_pure_at`). A pack's `pure` row is dropped with a notice.",
        ),
        (
            "stub `-pure` (`DeclaredCommand::pure`)",
            "retained, irreducible: the document is the authority for a command it declares itself, and states its purity rather than evidencing a shipped implementation.",
        ),
        (
            "`Traits::PURE_EVALUATION`",
            "retained, irreducible: a braced `expr` evaluates without effects of its own, while the calls inside it are each asked separately; it is not a footprint of the command.",
        ),
        (
            "`Traits::CSE_CANDIDATE`",
            "retained: chooses which pure calls the optimiser commons and the redundant-call check reports; purity is necessary, not sufficient (`InvocationFacts::is_cse_pure`).",
        ),
        (
            "`CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT`",
            "retained as an authoring base: it states the world effects and result stability as `Partial`, a lower bound, and closes nothing without evidence.",
        ),
    ] {
        let _ = writeln!(out, "| {descriptor} | {disposition} |");
    }
}

/// Write or verify the matrix.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let report = render(&rows(), &c_evidence(&root)?, &formerly_pure(&root)?);
    let path = root.join(REPORT_PATH);
    if check {
        let on_disk = fs::read_to_string(&path).unwrap_or_default();
        if on_disk != report {
            eprintln!(
                "semantics-matrix: {REPORT_PATH} is stale — run `cargo xtask semantics-matrix`"
            );
            return Ok(ExitCode::FAILURE);
        }
    } else {
        fs::write(&path, &report).with_context(|| format!("writing {REPORT_PATH}"))?;
    }
    println!("semantics-matrix: OK ({REPORT_PATH})");
    Ok(ExitCode::SUCCESS)
}
