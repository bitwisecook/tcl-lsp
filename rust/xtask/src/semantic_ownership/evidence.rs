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

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, ensure};

const CONTRACT: &str = "docs/work/name-handling/ownership/README.md";

fn validate(source: &str, linked: impl Fn(&str, &str) -> Result<()>) -> Result<usize> {
    let mut claims = BTreeSet::new();
    for claim in source
        .lines()
        .filter_map(|line| line.strip_prefix("- **Claim** "))
    {
        let (statement, evidence) = claim
            .split_once(" [Evidence](")
            .with_context(|| format!("contract claim has no evidence: {claim}"))?;
        ensure!(
            claims.insert(statement),
            "duplicate contract claim: {statement}"
        );
        let (path, anchor) = evidence
            .strip_suffix(')')
            .and_then(|evidence| evidence.split_once('#'))
            .with_context(|| format!("evidence needs a path and a test or vector row: {claim}"))?;
        linked(path, anchor).with_context(|| format!("invalid evidence for {statement}"))?;
    }
    ensure!(!claims.is_empty(), "the contract has no claims");
    Ok(claims.len())
}

fn check_link(root: &Path, path: &str, anchor: &str) -> Result<()> {
    let path = path
        .strip_prefix("../../../..")
        .and_then(|path| path.strip_prefix('/'))
        .context("evidence must link to a file in the repository")?;
    super::check_path(path)?;
    let target = fs::read_to_string(root.join(path)).with_context(|| path.to_owned())?;
    let found = match Path::new(path).extension().and_then(|part| part.to_str()) {
        Some("rs") => super::source::test_declares(&target, anchor),
        Some("tsv") => crate::util::tsv::rows(&target)
            .any(|row| crate::util::tsv::fields(row).next() == Some(anchor)),
        _ => false,
    };
    ensure!(
        found,
        "test or vector row {anchor} does not exist in {path}"
    );
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<ExitCode> {
    let source = fs::read_to_string(root.join(CONTRACT))?;
    let count = validate(&source, |path, anchor| check_link(root, path, anchor))?;
    println!("contract-evidence: OK ({count} claims with executable evidence)");
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unlinked_claim_fails_even_if_the_other_claims_have_evidence() {
        let valid = "- **Claim** Lists preserve empty words. [Evidence](lists.rs#empty_words)\n";
        let linked = |path: &str, anchor: &str| {
            ensure!(
                path == "lists.rs" && anchor == "empty_words",
                "missing test"
            );
            Ok(())
        };
        assert_eq!(validate(valid, linked).unwrap(), 1);
        assert!(validate(&format!("{valid}- **Claim** All words are safe.\n"), linked).is_err());
    }

    #[test]
    fn a_production_function_is_not_test_evidence() {
        assert!(!super::super::source::test_declares(
            "fn empty_words() {}",
            "empty_words"
        ));
        assert!(super::super::source::test_declares(
            "#[test]\nfn empty_words() {}",
            "empty_words"
        ));
    }
}
