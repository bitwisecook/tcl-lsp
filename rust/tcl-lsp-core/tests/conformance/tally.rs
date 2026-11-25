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

//! Engine answers against C Tcl of the same release, counting an explicit
//! refusal (an abstention) apart from a different answer (a wrong one).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::LazyLock;

use regex::Regex;

use crate::rows::Row;

static REFUSAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\brefused: |\bnot supported\b|\bnot implemented\b").expect("refusal pattern")
});

#[derive(Default, Clone, Copy)]
pub struct Counts {
    agree: usize,
    abstain: usize,
    wrong: usize,
}

/// Per engine column (`vm9.0`, `rt8.6`, …), the agreement counts.
#[derive(Default)]
pub struct Tally(BTreeMap<String, Counts>);

impl Tally {
    pub fn add(&mut self, row: &Row) {
        for (column, outcome) in &row.engines {
            let Some(release) = column
                .strip_prefix("vm")
                .or_else(|| column.strip_prefix("rt"))
            else {
                continue;
            };
            let reference = format!("tclsh{release}");
            let Some((_, tclsh)) = row.engines.iter().find(|(name, _)| *name == reference) else {
                continue;
            };
            let counts = self.0.entry(column.clone()).or_default();
            if outcome == tclsh {
                counts.agree += 1;
            } else if REFUSAL.is_match(&outcome.output) && !REFUSAL.is_match(&tclsh.output) {
                counts.abstain += 1;
            } else {
                counts.wrong += 1;
            }
        }
    }

    pub fn report(&self) -> String {
        let mut text = String::from("engine\tagree\tabstain\twrong\n");
        for (column, counts) in &self.0 {
            let _ = writeln!(
                text,
                "{column}\t{}\t{}\t{}",
                counts.agree, counts.abstain, counts.wrong
            );
        }
        text
    }
}
