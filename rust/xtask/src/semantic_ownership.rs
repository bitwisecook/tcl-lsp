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

mod evidence;
mod layering;
mod source;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use crate::util::{repo_root, test_sources, tsv, waivers::Waivers};

const DIRECTORY: &str = "docs/work/name-handling/ownership";
const GRAMMARS: &str = "grammars.tsv";
const WALKERS: &str = "walkers.tsv";
const PRIMITIVES: &str = "grammar-bypasses.tsv";
const MEMOS: &str = "memos.tsv";
const RELEASES: &str = "release-tables.tsv";
const BYPASS_TAG: &str = "grammar-bypass-ok:";

const GATES: &[&str] = &[
    "walker-ownership",
    "crate-layer",
    "memo-type",
    "release-table",
    "grammar-bypass",
    "contract-evidence",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Gate {
    Walker,
    Memo,
    Release,
    Grammar,
}

impl Gate {
    fn files(self) -> (&'static str, &'static str) {
        match self {
            Self::Walker => ("walker-ownership", WALKERS),
            Self::Memo => ("memo-type", MEMOS),
            Self::Release => ("release-table", RELEASES),
            Self::Grammar => ("grammar-bypass", PRIMITIVES),
        }
    }
}

#[derive(Debug)]
struct Grammar<'a> {
    reader: &'a str,
    writer: &'a str,
}

fn grammars<'a>(root: &Path, text: &'a str) -> Result<BTreeMap<&'a str, Grammar<'a>>> {
    let mut owners = BTreeMap::new();
    for row in tsv::rows(text) {
        let fields: Vec<_> = tsv::fields(row).collect();
        let [name, reader, writer] = fields.as_slice() else {
            bail!("invalid grammar owner row: {row}");
        };
        for path in [reader, writer] {
            check_source_reference(root, path)?;
        }
        if owners.insert(*name, Grammar { reader, writer }).is_some() {
            bail!("duplicate grammar owner: {name}");
        }
    }
    for name in source::GRAMMARS {
        if !owners.contains_key(name) {
            bail!("grammar has no owner: {name}");
        }
    }
    Ok(owners)
}

fn walker_facts<'a>(root: &Path, text: &'a str) -> Result<BTreeMap<&'a str, &'a str>> {
    let mut facts = BTreeMap::new();
    for row in tsv::rows(text) {
        let fields: Vec<_> = tsv::fields(row).collect();
        let [fact, owner, disposition] = fields.as_slice() else {
            bail!("invalid walker ownership row: {row}");
        };
        check_source_reference(root, owner)?;
        if disposition.is_empty() || facts.insert(*fact, *owner).is_some() {
            bail!("walker fact repeats an owner or lacks a disposition: {row}");
        }
    }
    Ok(facts)
}

fn check_source_reference(root: &Path, reference: &str) -> Result<()> {
    let (path, symbol) = reference
        .split_once('#')
        .context("an owner needs a source path and symbol")?;
    check_path(path)?;
    let source = fs::read_to_string(root.join(path)).with_context(|| path.to_owned())?;
    if !source::declares(&source, symbol) {
        bail!("owner symbol {symbol} is missing from {path}");
    }
    Ok(())
}

fn check_path(path: &str) -> Result<()> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        bail!("expected a repository-relative path, got {path}");
    }
    Ok(())
}

fn run(gate: Gate, report: bool) -> Result<ExitCode> {
    let root = repo_root();
    let (name, ledger) = gate.files();
    let manifest = root.join(DIRECTORY);
    let (grammar_rows, walker_rows);
    let owners = if gate == Gate::Grammar {
        grammar_rows = fs::read_to_string(manifest.join(GRAMMARS))?;
        grammars(&root, &grammar_rows)?
    } else {
        BTreeMap::new()
    };
    let facts = if gate == Gate::Walker {
        walker_rows = fs::read_to_string(manifest.join("walker-facts.tsv"))?;
        walker_facts(&root, &walker_rows)?
    } else {
        BTreeMap::new()
    };
    let mut paths = Vec::new();
    for crate_name in source::crates(gate) {
        if gate == Gate::Release && *crate_name == "tcl-registry" {
            continue;
        }
        collect(&root.join("rust").join(crate_name).join("src"), &mut paths)?;
    }
    if matches!(gate, Gate::Grammar | Gate::Release) {
        collect(&root.join("runtime/rust/src"), &mut paths)?;
    }
    if gate == Gate::Memo {
        for crate_name in ["tcl-expr", "tcl-registry", "tcl-syntax"] {
            collect(&root.join("rust").join(crate_name).join("src"), &mut paths)?;
        }
    }
    paths.sort_unstable();

    let mut actual: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for path in paths {
        let relative = path
            .strip_prefix(&root)?
            .to_string_lossy()
            .replace('\\', "/");
        if test_sources::is_test_module_file(&relative) {
            continue;
        }
        let text = fs::read_to_string(&path).with_context(|| relative.clone())?;
        lint(gate, &relative, &text, &owners, &mut actual)?;
    }
    if gate == Gate::Walker {
        for key in actual.keys() {
            let fact = tcl_test_support::fixture::tsv_fields(key)
                .nth(1)
                .context("missing walker fact")?;
            if !facts.contains_key(fact) {
                bail!("walker fact {fact} has no owner");
            }
        }
    }
    if report {
        for (key, lines) in &actual {
            println!("{}", tsv::record([key.as_str(), &lines.len().to_string()]));
        }
        return Ok(ExitCode::SUCCESS);
    }
    let ledger = fs::read_to_string(root.join(DIRECTORY).join(ledger))?;
    let problems = compare(&actual, &ledger)?;
    if problems.is_empty() {
        println!(
            "{name}: OK ({} inventoried sites)",
            actual.values().map(Vec::len).sum::<usize>()
        );
        return Ok(ExitCode::SUCCESS);
    }
    for problem in problems {
        eprintln!("{name}: {problem}");
    }
    Ok(ExitCode::FAILURE)
}

fn lint(
    gate: Gate,
    path: &str,
    text: &str,
    owners: &BTreeMap<&str, Grammar<'_>>,
    actual: &mut BTreeMap<String, Vec<usize>>,
) -> Result<()> {
    let waivers = (gate == Gate::Grammar).then(|| Waivers::new(text));
    for hit in source::scan(gate, path, text) {
        let mut reason = "-";
        if let Some(waivers) = &waivers {
            let owner = &owners[hit.rule];
            if [owner.reader, owner.writer]
                .iter()
                .any(|reference| owns_path(reference, path))
            {
                continue;
            }
            if let Some((_, payload)) = waivers.site(hit.line, BYPASS_TAG) {
                let explanation = payload
                    .split_once(" — ")
                    .filter(|(name, explanation)| {
                        *name == hit.rule && !explanation.trim().is_empty()
                    })
                    .map(|(_, explanation)| explanation)
                    .with_context(|| {
                        format!(
                            "{path}:{}: expected {BYPASS_TAG} {} — <reason>, owner at {}",
                            hit.line, hit.rule, owner.reader
                        )
                    })?;
                reason = explanation;
            }
        }
        let key = format!("{path}\t{}\t{}\t{reason}", hit.rule, hit.site);
        actual.entry(key).or_default().push(hit.line);
    }
    Ok(())
}

fn owns_path(reference: &str, path: &str) -> bool {
    let Some((file, _)) = reference.split_once('#') else {
        return false;
    };
    file == path
        || file.strip_suffix(".rs").is_some_and(|stem| {
            path.strip_prefix(stem)
                .is_some_and(|rest| rest.starts_with('/'))
        })
}

fn compare(actual: &BTreeMap<String, Vec<usize>>, ledger: &str) -> Result<Vec<String>> {
    let mut expected = BTreeMap::new();
    for row in tsv::rows(ledger) {
        let (key, count) =
            tcl_test_support::fixture::tsv_split_last(row).context("ratchet row needs a count")?;
        let count: usize = count.parse()?;
        if count == 0
            || tcl_test_support::fixture::tsv_fields(key).count() != 4
            || expected.insert(key, count).is_some()
        {
            bail!("invalid or duplicate ratchet entry: {row}");
        }
    }
    let mut problems = Vec::new();
    for (key, lines) in actual {
        let count = expected.remove(key.as_str()).unwrap_or_default();
        let (path, site) =
            tcl_test_support::fixture::tsv_split_once(key).context("missing source path")?;
        if lines.len() > count {
            problems.push(format!(
                "{path}: {:?}: {site} has {} site(s), allowed {count}",
                lines,
                lines.len()
            ));
        } else if lines.len() < count {
            problems.push(format!(
                "lower stale entry {key} from {count} to {}",
                lines.len()
            ));
        }
    }
    problems.extend(
        expected
            .keys()
            .map(|key| format!("remove stale entry {key}")),
    );
    Ok(problems)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if entry.file_name() != "tests" {
                collect(&path, out)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") && entry.file_name() != "tests.rs"
        {
            out.push(path);
        }
    }
    Ok(())
}

#[derive(clap::Subcommand)]
pub(crate) enum Command {
    /// Run the semantic architecture checks.
    Check,
    /// Check secondary source walkers and resolvers.
    WalkerOwnership {
        /// Print current sites for review.
        #[arg(long)]
        report: bool,
    },
    /// Enforce the workspace layering in deny.toml.
    CrateLayer,
    /// Check analysis memoisation outside the database.
    MemoType {
        /// Print current sites for review.
        #[arg(long)]
        report: bool,
    },
    /// Check release tables outside their owners.
    ReleaseTable {
        /// Print current sites for review.
        #[arg(long)]
        report: bool,
    },
    /// Check host primitives used in Tcl grammar consumers.
    GrammarBypass {
        /// Print current sites for review.
        #[arg(long)]
        report: bool,
    },
    /// Validate the tests and vector rows linked by contract claims.
    ContractEvidence,
}

impl Command {
    pub(crate) fn run(self) -> Result<ExitCode> {
        match self {
            Self::Check => Ok(crate::batch(
                &GATES.iter().map(ToString::to_string).collect::<Vec<_>>(),
            )),
            Self::WalkerOwnership { report } => run(Gate::Walker, report),
            Self::CrateLayer => layering::run(&repo_root()),
            Self::MemoType { report } => run(Gate::Memo, report),
            Self::ReleaseTable { report } => run(Gate::Release, report),
            Self::GrammarBypass { report } => run(Gate::Grammar, report),
            Self::ContractEvidence => evidence::run(&repo_root()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratchet_rejects_a_second_site_and_requires_a_removed_site_to_be_deleted() {
        let key = "rust/source.rs\twords\tf:trim\t-";
        let ledger = format!("{key}\t1\n");
        assert_eq!(
            compare(&BTreeMap::from([(key.to_owned(), vec![12])]), &ledger).unwrap(),
            [] as [String; 0]
        );
        assert_eq!(
            compare(&BTreeMap::from([(key.to_owned(), vec![12, 14])]), &ledger)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(compare(&BTreeMap::new(), &ledger).unwrap().len(), 1);
    }

    #[test]
    fn owner_waiver_cannot_create_a_new_baseline_entry() {
        let owners = BTreeMap::from([(
            "lists",
            Grammar {
                reader: "list.rs#split",
                writer: "list.rs#join",
            },
        )]);
        let mut actual = BTreeMap::new();
        let snippet = "fn f(tcl: &str) { tcl.split_whitespace(); }";
        lint(
            Gate::Grammar,
            "consumer.rs",
            &format!("// {BYPASS_TAG} words — wrong\n{snippet}"),
            &owners,
            &mut actual,
        )
        .unwrap_err();
        lint(
            Gate::Grammar,
            "consumer.rs",
            &format!("// {BYPASS_TAG} lists — old consumer\n{snippet}"),
            &owners,
            &mut actual,
        )
        .unwrap();
        assert_eq!(compare(&actual, "").unwrap().len(), 1);
    }
}
