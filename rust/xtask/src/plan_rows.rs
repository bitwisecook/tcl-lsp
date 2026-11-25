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

//! Conformance before migration: every migration item of the name-handling
//! plan names the conformance rows it turns green, and every named row
//! exists — a probe fixture or area under `oo-dispatch-probe/`, a committed
//! vector file, or an explicitly named Rust test.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::LazyLock;

use regex::Regex;

const PLAN: &str = "docs/work/name-handling/plan.md";
const PROBES: &str = "docs/work/name-handling/handover/oo-dispatch-probe";
/// The conformance umbrella of item 205; they always pass the check.
const UMBRELLA: &[u32] = &[25, 28, 37, 65, 75, 131, 198];
/// Working-order stages whose items migrate consumers onto owners.
const MIGRATION_STAGES: std::ops::RangeInclusive<u32> = 6..=13;
const ROW_EXTENSIONS: &[&str] = &["tsv", "txt", "jsonl", "tcl", "out"];

static ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)\. \*\*\[([a-z ]+)\]\*\*").unwrap());
static STAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- \*\*Stage (\d+)[^:]*:\*\*(.*)$").unwrap());
static NUMBERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:phase \d+b?|Orders? \d+(?:–\d+)?|queue item \d+)|(\d+)(?:–(\d+))?").unwrap()
});
static BACKTICKED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());
static FIXTURE_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z]{1,3}\d{1,3}[a-z]?(?:-[a-z0-9-]+)?$").unwrap());

pub struct Item {
    pub number: u32,
    pub status: String,
    pub text: String,
}

pub fn items(plan: &str) -> Vec<Item> {
    let mut found: Vec<Item> = Vec::new();
    let mut open = false;
    for line in plan.lines() {
        if let Some(captures) = ITEM.captures(line) {
            found.push(Item {
                number: captures[1].parse().expect("item number"),
                status: captures[2].to_owned(),
                text: line.to_owned(),
            });
            open = true;
        } else if open && (line.starts_with("   ") || line.trim().is_empty()) {
            let item = found.last_mut().expect("open item");
            item.text.push('\n');
            item.text.push_str(line);
        } else {
            open = false;
        }
    }
    found
}

/// Items the working order lists in the migration stages.
pub fn migration_items(plan: &str) -> BTreeSet<u32> {
    let mut numbers = BTreeSet::new();
    for line in plan.lines() {
        let Some(captures) = STAGE.captures(line) else {
            continue;
        };
        let stage: u32 = captures[1].parse().expect("stage number");
        if !MIGRATION_STAGES.contains(&stage) {
            continue;
        }
        for range in NUMBERS.captures_iter(&captures[2]) {
            let Some(first) = range.get(1) else { continue };
            let first: u32 = first.as_str().parse().expect("number");
            let last: u32 = range
                .get(2)
                .map_or(first, |m| m.as_str().parse().expect("number"));
            numbers.extend(first..=last);
        }
    }
    numbers
}

/// The rows that can be named: probe areas with their fixture stems, and
/// committed vector files.
pub struct Rows {
    areas: BTreeMap<String, Vec<String>>,
    files: Vec<String>,
    root: PathBuf,
}

impl Rows {
    #[cfg(test)]
    pub fn new(areas: BTreeMap<String, Vec<String>>, files: Vec<String>) -> Self {
        Self {
            areas,
            files,
            root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        }
    }

    fn load(root: &Path) -> anyhow::Result<Self> {
        let output = std::process::Command::new("git")
            .current_dir(root)
            .args(["ls-files"])
            .output()?;
        anyhow::ensure!(output.status.success(), "git ls-files failed");
        let files: Vec<String> = String::from_utf8(output.stdout)?
            .lines()
            .map(str::to_owned)
            .collect();
        let mut areas: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for file in &files {
            let Some(relative) = file.strip_prefix(PROBES).and_then(|r| r.strip_prefix('/')) else {
                continue;
            };
            let Some((area, name)) = relative.rsplit_once('/') else {
                continue;
            };
            if let Some(stem) = name.strip_suffix(".tcl") {
                areas
                    .entry(area.to_owned())
                    .or_default()
                    .push(stem.to_owned());
            }
        }
        Ok(Self {
            areas,
            files,
            root: root.to_path_buf(),
        })
    }

    fn fixture(&self, area: &str, id: &str) -> bool {
        let id = id.strip_suffix(".tcl").unwrap_or(id);
        self.areas.iter().any(|(name, stems)| {
            (name == area || name.starts_with(&format!("{area}/")))
                && (id.is_empty() || id == "*" || stems.iter().any(|stem| stem.starts_with(id)))
        })
    }

    fn area_with(&self, id: &str) -> Option<&str> {
        self.areas
            .iter()
            .find(|(_, stems)| stems.iter().any(|stem| stem.starts_with(id)))
            .map(|(name, _)| name.as_str())
    }

    fn file(&self, pattern: &str) -> bool {
        let pattern = regex::escape(pattern).replace(r"\*", "[^/]*");
        let Ok(matcher) = Regex::new(&format!("(^|/){pattern}$")) else {
            return false;
        };
        self.files.iter().any(|file| matcher.is_match(file))
    }

    fn rust_test(&self, path: &str, name: &str) -> bool {
        if !self.files.iter().any(|file| file == path) {
            return false;
        }
        std::fs::read_to_string(self.root.join(path))
            .ok()
            .and_then(|text| syn::parse_file(&text).ok())
            .is_some_and(|file| has_named_test(&file.items, name))
    }
}

fn has_named_test(items: &[syn::Item], name: &str) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Fn(function) if function.sig.ident == name => {
            function.attrs.iter().any(|attribute| {
                attribute
                    .path()
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "test")
            })
        }
        syn::Item::Mod(module) => module
            .content
            .as_ref()
            .is_some_and(|(_, items)| has_named_test(items, name)),
        _ => false,
    })
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub rows: Vec<String>,
    pub missing: Vec<String>,
}

impl Verdict {
    pub fn passes(&self) -> bool {
        !self.rows.is_empty() && self.missing.is_empty()
    }
}

pub fn check(text: &str, rows: &Rows) -> Verdict {
    let mut verdict = Verdict::default();
    let mut area: Option<String> = None;
    for captures in BACKTICKED.captures_iter(text) {
        let token = captures[1].trim();
        let token = token.strip_prefix("oo-dispatch-probe/").unwrap_or(token);
        let (head, rest) = token.split_once('/').unwrap_or((token, ""));
        let probe_area = rows
            .areas
            .keys()
            .find(|name| *name == head || name.split('/').next() == Some(head));
        if token == "*/" || token == "*" {
            verdict.rows.push(format!("oo-dispatch-probe/{token}"));
            continue;
        }
        if token.contains('/') && probe_area.is_some() {
            let (area_name, id) = rows
                .areas
                .keys()
                .filter(|name| token.starts_with(&format!("{name}/")))
                .max_by_key(|name| name.len())
                .map_or((head.to_owned(), rest), |name| {
                    (name.clone(), &token[name.len() + 1..])
                });
            area = Some(area_name.clone());
            let directory = token.trim_end_matches('/');
            if rows.fixture(&area_name, id)
                || rows
                    .areas
                    .keys()
                    .any(|name| name.starts_with(&format!("{directory}/")))
                || rows.file(&format!("{PROBES}/{token}"))
            {
                verdict.rows.push(token.to_owned());
            } else {
                verdict.missing.push(token.to_owned());
            }
        } else if FIXTURE_ID.is_match(token) {
            let Some(area) = &area else { continue };
            if rows.fixture(area, token) {
                verdict.rows.push(format!("{area}/{token}"));
            } else if let Some(other) = rows.area_with(token) {
                verdict.rows.push(format!("{other}/{token}"));
            } else {
                verdict.missing.push(format!("{area}/{token}"));
            }
        } else if let Some((path, name)) = token.split_once('#').filter(|(path, _)| {
            Path::new(path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        }) {
            if rows.rust_test(path, name) {
                verdict.rows.push(token.to_owned());
            } else {
                verdict.missing.push(token.to_owned());
            }
        } else if token.contains('/')
            && !token.contains(' ')
            && Path::new(token)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ROW_EXTENSIONS.contains(&ext))
        {
            if rows.file(token) {
                verdict.rows.push(token.to_owned());
            } else {
                verdict.missing.push(token.to_owned());
            }
        }
    }
    verdict
}

/// Check the named items, or the umbrella and every migration item already
/// in progress. With `all`, report every open migration item.
pub fn run(requested: &[u32], all: bool) -> anyhow::Result<ExitCode> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plan = std::fs::read_to_string(root.join(PLAN))?;
    let rows = Rows::load(&root)?;
    let items = items(&plan);
    let migration = migration_items(&plan);
    let selected: Vec<&Item> = items
        .iter()
        .filter(|item| {
            if !requested.is_empty() {
                return requested.contains(&item.number);
            }
            UMBRELLA.contains(&item.number)
                || (migration.contains(&item.number)
                    && item.status != "completed"
                    && (all || item.status == "in progress"))
        })
        .collect();
    let mut failed = 0;
    for item in &selected {
        let verdict = check(&item.text, &rows);
        if verdict.passes() {
            println!(
                "ok    {:>3}  {} row reference(s)",
                item.number,
                verdict.rows.len()
            );
        } else {
            failed += 1;
            let reason = if verdict.rows.is_empty() && verdict.missing.is_empty() {
                "names no conformance rows".to_owned()
            } else if verdict.rows.is_empty() {
                format!("names only missing rows: {}", verdict.missing.join(", "))
            } else {
                format!("names missing rows: {}", verdict.missing.join(", "))
            };
            println!("FAIL  {:>3}  {reason}", item.number);
        }
    }
    println!(
        "plan-rows: {} of {} item(s) name existing conformance rows",
        selected.len() - failed,
        selected.len()
    );
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Rows {
        Rows::new(
            BTreeMap::from([
                (
                    "dynamic".to_owned(),
                    vec!["e02-eval-literal".to_owned(), "e04-upvar".to_owned()],
                ),
                ("namespaces/nav".to_owned(), vec!["k01-import".to_owned()]),
            ]),
            vec!["rust/tcl-syntax/tests/data/namespace_op_vectors.txt".to_owned()],
        )
    }

    #[test]
    fn fixtures_continue_the_last_named_area() {
        let verdict = check("`dynamic/e02` (L4), `e04` and `e13`", &rows());
        assert_eq!(verdict.rows, ["dynamic/e02", "dynamic/e04"]);
        assert_eq!(verdict.missing, ["dynamic/e13"]);
        assert!(!verdict.passes());
    }

    #[test]
    fn a_bare_fixture_resolves_in_another_area_when_the_last_lacks_it() {
        let verdict = check("`namespaces/nav/k01` then `e04`", &rows());
        assert_eq!(verdict.rows, ["namespaces/nav/k01", "dynamic/e04"]);
        assert!(verdict.passes());
    }

    #[test]
    fn a_directory_of_fixture_areas_is_a_row() {
        let mut rows = rows();
        rows.areas.insert(
            "rename-alias/editor-rename/before".to_owned(),
            vec!["e01-calls".to_owned()],
        );
        rows.areas.insert(
            "rename-alias".to_owned(),
            vec!["r01-rename-basic".to_owned()],
        );
        assert!(check("every `rename-alias/editor-rename/` fixture", &rows).passes());
    }

    #[test]
    fn vector_files_and_areas_are_rows() {
        let verdict = check(
            "`tcl-syntax/tests/data/namespace_op_vectors.txt` and `namespaces/nav/` and `src/lib.rs`",
            &rows(),
        );
        assert_eq!(
            verdict.rows,
            [
                "tcl-syntax/tests/data/namespace_op_vectors.txt",
                "namespaces/nav/"
            ]
        );
        assert!(verdict.passes());
    }

    #[test]
    fn an_item_without_rows_fails() {
        let verdict = check(
            "Move the parser into `tcl-syntax`; `cargo xtask check` passes.",
            &rows(),
        );
        assert!(verdict.rows.is_empty());
        assert!(!verdict.passes());
    }

    #[test]
    fn a_missing_vector_file_fails() {
        let verdict = check("`tcl-syntax/tests/data/missing_vectors.tsv`", &rows());
        assert_eq!(
            verdict.missing,
            ["tcl-syntax/tests/data/missing_vectors.tsv"]
        );
    }

    #[test]
    fn a_named_rust_test_must_exist_and_have_a_test_attribute() {
        let path = "rust/xtask/src/plan_rows.rs";
        let mut rows = rows();
        rows.files.push(path.to_owned());
        let verdict = check(
            &format!("`{path}#vector_files_and_areas_are_rows` and `{path}#has_named_test`"),
            &rows,
        );
        assert_eq!(
            verdict.rows,
            [format!("{path}#vector_files_and_areas_are_rows")]
        );
        assert_eq!(verdict.missing, [format!("{path}#has_named_test")]);
    }

    #[test]
    fn stage_lists_select_migration_items() {
        let plan = "- **Stage 5, foundations (phase 0):** 77, 78.\n- **Stage 7, existing owners (phase 1):** 22, 59–61; 86 Orders 1–2.\n";
        assert_eq!(migration_items(plan), BTreeSet::from([22, 59, 60, 61, 86]));
    }
}
