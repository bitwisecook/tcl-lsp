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

//! Three-way characterisation for the two native Tcl implementations.
//!
//! A `tcl-vm` ↔ `runtime/rust` pair is useful for detecting drift, but it has
//! no oracle: the two backends can agree on the same bug. This module classifies
//! every three-way run against C Tcl 9 so a campaign distinguishes one backend
//! diverging, both backends sharing a divergence, and a genuinely three-way
//! disagreement. The subprocess details stay in `harness`; this module only
//! owns the interpretation of the three outcomes.

use crate::harness::{Outcome, Verdict, compare_outcomes};

/// How the C Tcl 9 oracle, `tcl-vm`, and `runtime/rust` relate for one script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Classification {
    /// All three completed and C Tcl 9 agrees with both implementations.
    AllAgree,
    /// Only `tcl-vm` differs from the C Tcl 9 result.
    TclVmDiverges,
    /// Only `runtime/rust` differs from the C Tcl 9 result.
    RuntimeRustDiverges,
    /// The two implementations agree with each other but both differ from C
    /// Tcl 9. This is the important shared-bug case a two-way pair misses.
    SharedDivergence,
    /// Each pair differs, so the result requires manual reduction.
    ThreeWayDivergence,
    /// At least one backend did not produce an ordinary completed outcome.
    /// Timeouts and unavailable tools are reported separately rather than
    /// being mistaken for semantic agreement.
    Incomplete,
}

impl Classification {
    /// Whether this classification represents a semantic divergence from C
    /// Tcl 9 that should be retained as a finding.
    #[must_use]
    pub const fn is_finding(self) -> bool {
        matches!(
            self,
            Self::TclVmDiverges
                | Self::RuntimeRustDiverges
                | Self::SharedDivergence
                | Self::ThreeWayDivergence
        )
    }
}

/// Classify three backend outcomes. Error-text comparison is opt-in for the
/// same reason it is in a normal pair campaign: independent Tcl engines can
/// legitimately phrase an equivalent error differently.
#[must_use]
pub fn classify(
    oracle: &Outcome,
    tclvm: &Outcome,
    runtime_rust: &Outcome,
    compare_error_text: bool,
) -> Classification {
    if !matches!(oracle, Outcome::Ran { .. })
        || !matches!(tclvm, Outcome::Ran { .. })
        || !matches!(runtime_rust, Outcome::Ran { .. })
    {
        return Classification::Incomplete;
    }

    let same_vm = compare_outcomes(oracle, tclvm, compare_error_text) == Verdict::Match;
    let same_runtime = compare_outcomes(oracle, runtime_rust, compare_error_text) == Verdict::Match;
    let implementations_match =
        compare_outcomes(tclvm, runtime_rust, compare_error_text) == Verdict::Match;

    match (same_vm, same_runtime, implementations_match) {
        (true, true, _) => Classification::AllAgree,
        (false, true, _) => Classification::TclVmDiverges,
        (true, false, _) => Classification::RuntimeRustDiverges,
        (false, false, true) => Classification::SharedDivergence,
        (false, false, false) => Classification::ThreeWayDivergence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ran(text: &str) -> Outcome {
        Outcome::Ran {
            stdout: text.to_owned(),
            stderr: String::new(),
            errored: false,
        }
    }

    #[test]
    fn classifies_each_oracle_relationship() {
        assert_eq!(
            classify(&ran("ok"), &ran("ok"), &ran("ok"), false),
            Classification::AllAgree
        );
        assert_eq!(
            classify(&ran("ok"), &ran("vm"), &ran("ok"), false),
            Classification::TclVmDiverges
        );
        assert_eq!(
            classify(&ran("ok"), &ran("ok"), &ran("runtime"), false),
            Classification::RuntimeRustDiverges
        );
        assert_eq!(
            classify(&ran("oracle"), &ran("shared"), &ran("shared"), false),
            Classification::SharedDivergence
        );
        assert_eq!(
            classify(&ran("oracle"), &ran("vm"), &ran("runtime"), false),
            Classification::ThreeWayDivergence
        );
    }

    #[test]
    fn unavailable_or_timed_out_backend_is_incomplete_not_agreement() {
        assert_eq!(
            classify(&ran("ok"), &Outcome::Timeout, &ran("ok"), false),
            Classification::Incomplete
        );
        assert_eq!(
            classify(
                &ran("ok"),
                &ran("ok"),
                &Outcome::Unavailable("missing".to_owned()),
                false,
            ),
            Classification::Incomplete
        );
    }
}
