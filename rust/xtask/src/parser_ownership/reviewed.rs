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

//! A signed scope records exact parser-relevant operations, their enclosing
//! production entry, occurrence, and source owner/differential evidence.

use super::{Facts, Source};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) const INDEX: &str =
    "docs/work/name-handling/validation-tools/parser-reviewed-scopes.tsv";

#[derive(Default)]
pub(super) struct Reviewed {
    scopes: BTreeSet<String>,
    files: BTreeMap<String, BTreeSet<String>>,
}

impl Reviewed {
    pub(super) fn parse(index: &str) -> Result<Self> {
        let mut reviewed = Self::default();
        for (n, row) in index.lines().enumerate().skip(1) {
            let columns: Vec<_> = row.split('\t').collect();
            ensure!(
                columns.len() == 4 && columns.iter().all(|s| !s.is_empty()),
                "invalid signed scope line {}",
                n + 1
            );
            let (kind, file, fact, evidence) = (columns[0], columns[1], columns[2], columns[3]);
            ensure!(
                !evidence.trim().is_empty()
                    && !Path::new(file).is_absolute()
                    && !file.split('/').any(|p| p == ".."),
                "invalid evidence or path on line {}",
                n + 1
            );
            match kind {
                "scope" => {
                    ensure!(
                        reviewed.scopes.insert(file.to_owned()),
                        "duplicate scope {file}"
                    );
                }
                "source" => {
                    ensure!(
                        reviewed
                            .files
                            .insert(file.to_owned(), BTreeSet::new())
                            .is_none(),
                        "duplicate source {file}"
                    );
                }
                "operation" => {
                    let Some(facts) = reviewed.files.get_mut(file) else {
                        anyhow::bail!("{file}: operation requires a preceding source row");
                    };
                    ensure!(
                        facts.insert(fact.to_owned()),
                        "duplicate parser operation {file} {fact}"
                    );
                }
                _ => anyhow::bail!("unknown signed-scope record {kind}"),
            }
        }
        ensure!(
            !reviewed.scopes.is_empty() && !reviewed.files.is_empty(),
            "signed scope inventory is empty"
        );
        for scope in &reviewed.scopes {
            ensure!(
                reviewed
                    .files
                    .keys()
                    .any(|file| scope == file || scope.ends_with('/') && file.starts_with(scope)),
                "signed scope {scope} has no reviewed source"
            );
        }
        for file in reviewed.files.keys() {
            ensure!(
                reviewed.covers(file),
                "signed source is outside a declared scope: {file}"
            );
        }
        Ok(reviewed)
    }

    pub(super) fn covers(&self, file: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope == file || scope.ends_with('/') && file.starts_with(scope))
    }

    pub(super) fn check_file(&self, file: &str, source: Option<&Source>, errors: &mut Vec<String>) {
        if !self.covers(file) {
            return;
        }
        let Some(reviewed) = self.files.get(file) else {
            errors.push(format!(
                "{file}: new unreviewed tracked source in signed parser scope"
            ));
            return;
        };
        let Some(source) = source else {
            errors.push(format!(
                "{file}: signed source has no checked Rust AST or foreign adapter differential"
            ));
            return;
        };
        let observed = operations(source);
        for fact in observed.difference(reviewed) {
            errors.push(format!("{file}: unreviewed parser operation {fact}"));
        }
        for fact in reviewed.difference(&observed) {
            errors.push(format!("{file}: obsolete parser disposition {fact}"));
        }
    }

    pub(super) fn missing(&self, tracked: &BTreeSet<&str>, errors: &mut Vec<String>) {
        for file in self
            .files
            .keys()
            .filter(|file| !tracked.contains(file.as_str()))
        {
            errors.push(format!("missing signed parser source {file}"));
        }
    }
}

fn relevant(facts: &Facts) -> bool {
    facts.structural()
        || !facts.leads.is_empty()
        || !facts.host_readers.is_empty()
        || facts.calls.keys().any(|call| owner_call(call))
}

fn owner_call(call: &str) -> bool {
    let leaf = call.rsplit("::").next().unwrap_or(call);
    super::ROUTES.iter().any(|route| call == route.owner)
        || matches!(
            leaf,
            "with_config"
                | "group_commands"
                | "decode_in"
                | "parse_expr"
                | "split_list"
                | "join_list"
                | "literal_commands"
        )
}

pub(super) fn operations(source: &Source) -> BTreeSet<String> {
    let mut observed = BTreeSet::new();
    for entries in source.functions.values() {
        for facts in entries.iter().filter(|f| relevant(f)) {
            let mut occurrences = BTreeMap::<String, usize>::new();
            for op in &facts.operations {
                *occurrences.entry(op.clone()).or_default() += 1;
            }
            for (callee, calls) in &facts.calls {
                let local = callee.strip_prefix("self::").unwrap_or(callee);
                let parser = owner_call(callee)
                    || source
                        .functions
                        .get(local)
                        .is_some_and(|bodies| bodies.iter().any(relevant));
                for args in calls {
                    let edge = if parser {
                        let inputs: Vec<_> = args
                            .iter()
                            .map(|part| part.as_deref().unwrap_or("<expression>"))
                            .collect();
                        format!("call {callee}({})", inputs.join(", "))
                    } else {
                        // An introduced helper in a different module can hide a
                        // new reader. Preserve its edge regardless of its name.
                        // Host message literals and layout do not affect it.
                        format!("edge {callee}")
                    };
                    *occurrences.entry(edge).or_default() += 1;
                }
            }
            for (operation, count) in occurrences {
                for n in 1..=count {
                    observed.insert(format!("{} | {operation} [{n}]", facts.scope));
                }
            }
        }
    }
    observed
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::visit::Visit;

    fn parse(text: &str) -> Source {
        let mut parsed = Source::default();
        parsed.visit_file(&syn::parse_file(text).unwrap());
        parsed
    }

    #[test]
    fn signed_scope_rejects_new_readers_and_files_and_accepts_host_edits() {
        let original = parse("fn words(data: S) { group_commands(t, data, config); }");
        let file = "runtime/rust/src/example.rs";
        let mut index = format!(
            "record\tpath\toperation\tevidence\nscope\truntime/rust/src/\t-\tchecked parser component\nsource\t{file}\t-\tchecked public reader\n"
        );
        for fact in operations(&original) {
            index.push_str(&format!(
                "operation\t{file}\t{fact}\tshared command grouper and public differential\n"
            ));
        }
        let signed = Reviewed::parse(&index).unwrap();
        let errors = |path, text| {
            let mut errors = vec![];
            signed.check_file(path, Some(&parse(text)), &mut errors);
            errors
        };
        assert!(errors(file, "// Docs\nfn words( data:S ) { group_commands(t,data,config); } fn host_name() -> &'static str { \"tool\" }").is_empty());
        assert!(
            !errors(
                file,
                "fn words(data: S) { group_commands(t, data, default); }"
            )
            .is_empty()
        );
        assert!(!errors(file, "fn words(data: S) { group_commands(t, data, config); } fn renamed(s: &[u8]) { for &x in s { if x == b']' { break; } } }").is_empty());
        assert!(!errors("runtime/rust/src/new.rs", "fn unrelated_name() {}").is_empty());
        let mut foreign = Vec::new();
        signed.check_file("runtime/rust/src/new_adapter.py", None, &mut foreign);
        assert!(!foreign.is_empty());
    }
}
