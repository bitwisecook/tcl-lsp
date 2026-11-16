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

//! The map from original to compacted names and its report format.

use super::*;

/// Map of original names to compacted names, grouped by scope
/// (only the fields the active tiers populate).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolMap {
    /// Per-scope `{original_var: short}` reports, keyed by display label.
    /// Colliding labels are disambiguated after edit selection.
    pub variables: BTreeMap<String, BTreeMap<String, String>>,
    /// `{original_proc: short}`.
    pub procs: BTreeMap<String, String>,
    /// `{original_command: alias_var}` (aggressive tier).
    pub command_aliases: BTreeMap<String, String>,
    /// `{original_argument: alias_var}` (aggressive tier).
    pub argument_aliases: BTreeMap<String, String>,
    /// `{original_literal: alias_var}` (aggressive tier).
    pub string_aliases: BTreeMap<String, String>,
    /// `{original_dynamic_string: folded_static_value}` (aggressive
    /// tier, SCCP static-substring folding).
    pub static_folds: BTreeMap<String, String>,
}

/// Separate report labels, without a source scope, frame or variable identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopedSymbolMapLabel {
    /// Original transformation's display scope label.
    pub scope_label: String,
    /// Compacted spelling recorded by that transformation.
    pub compacted: String,
}

/// Failure of the lossless structured report, independently of legacy display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolMapReportError {
    /// No structured record is present; legacy display parsing remains possible.
    MissingRecord,
    /// A present structured record is malformed or has trailing non-report data.
    InvalidRecord,
    /// The record uses a schema this reader does not understand.
    UnsupportedVersion(u32),
}

pub(super) const SYMBOL_MAP_RECORD: &str = "# tcl-lsp-symbol-map ";

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SymbolMapReport {
    pub(super) version: u32,
    symbols: SymbolMap,
}

impl SymbolMap {
    /// Human-readable labels followed by the authoritative lossless report.
    #[must_use]
    pub fn format(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if !self.procs.is_empty() {
            lines.push("# Procs".to_owned());
            for (original, short) in &self.procs {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        for (scope_name, var_map) in &self.variables {
            lines.push(format!("# Variables in {scope_name}"));
            let mut entries: Vec<(&String, &String)> = var_map.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1));
            for (original, short) in entries {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        if !self.command_aliases.is_empty() {
            lines.push("# Command aliases".to_owned());
            for (original, alias) in &self.command_aliases {
                lines.push(format!("  ${alias} <- {original}"));
            }
        }
        if !self.argument_aliases.is_empty() {
            lines.push("# Argument aliases".to_owned());
            for (original, alias) in &self.argument_aliases {
                lines.push(format!("  ${alias} <- {original}"));
            }
        }
        if !self.string_aliases.is_empty() {
            lines.push("# String literal aliases".to_owned());
            let mut entries: Vec<(&String, &String)> = self.string_aliases.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1));
            for (original, alias) in entries {
                lines.push(format!("  ${alias} <- {original:?}"));
            }
        }
        if !self.static_folds.is_empty() {
            lines.push("# Static substring folds (SCCP)".to_owned());
            for (original, folded) in &self.static_folds {
                lines.push(format!("  {folded:?} <- {original:?}"));
            }
        }
        let record = serde_json::json!({"version": 1, "symbols": self});
        lines.push(format!("{SYMBOL_MAP_RECORD}{record}"));
        lines.join("\n")
    }

    /// Exact separate scope/compacted report labels, retaining every original
    /// spelling when a compacted label is duplicated. This supplies no binding.
    #[must_use]
    pub fn scoped_reverse(&self) -> BTreeMap<ScopedSymbolMapLabel, Vec<String>> {
        let mut reversed: BTreeMap<ScopedSymbolMapLabel, Vec<String>> = BTreeMap::new();
        for (scope, variables) in &self.variables {
            for (original, compacted) in variables {
                reversed
                    .entry(ScopedSymbolMapLabel {
                        scope_label: scope.clone(),
                        compacted: compacted.clone(),
                    })
                    .or_default()
                    .push(original.clone());
            }
        }
        reversed
    }

    /// Approximate display labels. Bare ties keep the first row; ambiguous
    /// concatenated scope labels are omitted. This grants no name identity.
    #[must_use]
    pub fn reverse(&self) -> BTreeMap<String, String> {
        let mut reversed = self.bare_reverse();
        let mut scoped: BTreeMap<String, Option<String>> = BTreeMap::new();
        for (label, originals) in self.scoped_reverse() {
            let key = format!("{}:{}", label.scope_label, label.compacted);
            let value = match originals.as_slice() {
                [original] => Some(format!("{}:{original}", label.scope_label)),
                _ => None,
            };
            scoped
                .entry(key)
                .and_modify(|value| *value = None)
                .or_insert(value);
        }
        for (label, original) in scoped {
            if let Some(original) = original {
                reversed.entry(label).or_insert(original);
            }
        }
        reversed
    }

    pub(super) fn bare_reverse(&self) -> BTreeMap<String, String> {
        let mut rev: BTreeMap<String, String> = BTreeMap::new();
        for (original, short) in &self.procs {
            rev.entry(short.clone()).or_insert_with(|| original.clone());
        }
        for var_map in self.variables.values() {
            for (original, short) in var_map {
                rev.entry(short.clone()).or_insert_with(|| original.clone());
            }
        }
        for aliases in [
            &self.command_aliases,
            &self.argument_aliases,
            &self.string_aliases,
        ] {
            for (original, alias) in aliases {
                rev.entry(alias.clone()).or_insert_with(|| original.clone());
            }
        }
        rev
    }

    /// Read the terminal structured record exactly. A malformed or unsupported
    /// record cannot fall back to the lossy display sections.
    ///
    /// # Errors
    /// Returns the missing, malformed or unsupported structured-record category.
    pub fn parse_structured(text: &str) -> Result<Self, SymbolMapReportError> {
        let record = text
            .lines()
            .rfind(|line| !line.trim().is_empty())
            .and_then(|line| line.strip_prefix(SYMBOL_MAP_RECORD))
            .ok_or_else(|| {
                if text.lines().any(|line| line.starts_with(SYMBOL_MAP_RECORD)) {
                    SymbolMapReportError::InvalidRecord
                } else {
                    SymbolMapReportError::MissingRecord
                }
            })?;
        let report: SymbolMapReport =
            serde_json::from_str(record).map_err(|_| SymbolMapReportError::InvalidRecord)?;
        if report.version != 1 {
            return Err(SymbolMapReportError::UnsupportedVersion(report.version));
        }
        Ok(report.symbols)
    }

    /// Parse the lossless [`Self::format`] report, or legacy variable/procedure
    /// display sections. Invalid structured records produce an empty report;
    /// [`Self::parse_structured`] exposes their exact refusal category.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        match Self::parse_structured(text) {
            Ok(report) => return report,
            Err(SymbolMapReportError::MissingRecord) => {}
            Err(_) => return Self::default(),
        }
        let mut sm = SymbolMap::default();
        let mut section = "";
        let mut section_name = String::new();
        for line in text.lines() {
            let stripped = line.trim();
            if stripped.is_empty() {
                continue;
            }
            if let Some(rest) = stripped.strip_prefix("# Variables in ") {
                section = "variables";
                section_name.clear();
                section_name.push_str(rest);
                continue;
            }
            if stripped.starts_with("# Procs") {
                section = "procs";
                continue;
            }
            if stripped.starts_with('#') {
                section = "";
                continue;
            }
            // Entry line: `short <- original`.
            let Some((short, original)) = stripped.split_once(" <- ") else {
                continue;
            };
            let (short, original) = (short.trim().to_owned(), original.trim().to_owned());
            // registry-axis-ok: irreducible — `section` is this parser's own
            // section tag, set a few lines up from *this format's own*
            // `# Procs` / `# Variables in …` headers (the `Self::format`
            // writer's own vocabulary for its symbol-map file), not a
            // dispatch on `info procs` / `info variables`; the spelling
            // coincides with `info`'s subcommand names only; until never
            match section {
                "procs" => {
                    sm.procs.insert(original, short);
                }
                "variables" => {
                    sm.variables
                        .entry(section_name.clone())
                        .or_default()
                        .insert(original, short);
                }
                _ => {}
            }
        }
        sm
    }
}
