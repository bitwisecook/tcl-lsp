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

//! The measured split of iRules' 31 "disabled" stock-Tcl builtins into
//! its **two distinct mechanisms** — environment/realm policy data for
//! the `f5-irules` environment
//! (`docs/design/bigip-irule-parser-measurements.md` §4b, re-probed
//! through `eval` at runtime on BIG-IP 21.1.0.1).
//!
//! A literal reference to any of the 31 is refused when the rule is
//! **loaded** (`command is disabled: "X"`, §5), which is why none of them
//! carries the `IRULES` bit in its command spec. But the refusals are not
//! one fact:
//!
//! - **16 are absent from TMM's interpreter** — `invalid command name`
//!   even when reached through `eval` at runtime. A smaller interpreter
//!   build: a *language fact* about what exists in TMM.
//! - **15 are present in the interpreter and refused only by the rule
//!   compiler** — reachable via `eval`, and `rename` demonstrably works.
//!   Pure load-time policy about rule *source*: the commands are right
//!   there, and only the compiler's opinion stops you.
//!
//! The distinction pins diagnostic severity: an interpreter-absent
//! command is unconditionally unavailable (error-grade, like any unknown
//! command under the closed world), while a compiler-refused one is a
//! **policy warning about rule source** — the same name reached through
//! dynamic evaluation is real, so an analyser that follows §4c's dynamic
//! code must not claim the command does not exist.
//!
//! // measurements §4b: this module is the data layer only. The consumer
//! // wiring lives in the analyser: an interpreter-absent literal head
//! // keeps the language-fact unavailable-command diagnostic (W002), a
//! // compiler-refused literal head draws the distinct IRULE2004 policy
//! // warning instead (and is excluded from "Unknown command" claims),
//! // and the §4c lexical-scan mirroring recurses the load-time checks
//! // through braced `eval`/`uplevel` literals while a variable-held
//! // script widens the realm state (`tcl-compiler`'s
//! // `analyser::diagnostics::validity` / `analyser::commands`).

/// Which of the two measured mechanisms keeps one stock builtin out of
/// iRules source (§4b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrulesDisabledClass {
    /// Absent from TMM's interpreter: `invalid command name` even via
    /// `eval` at runtime — a **language fact** (the smaller interpreter
    /// build), diagnosable as an error.
    InterpreterAbsent,
    /// Present in TMM's interpreter but refused by the rule compiler at
    /// load: reachable via `eval` at runtime — a **rule-source policy**,
    /// diagnosable as a policy warning, never as "this command does not
    /// exist".
    CompilerRefused,
}

impl IrulesDisabledClass {
    /// The diagnostic severity class the mechanism pins: `true` when the
    /// finding is a statement about the language (error-grade), `false`
    /// when it is a policy statement about rule source (warning-grade).
    #[must_use]
    pub const fn is_language_fact(self) -> bool {
        matches!(self, Self::InterpreterAbsent)
    }
}

/// The 16 commands **absent from TMM's interpreter** (measurements §4b:
/// `invalid command name` even through `eval`).
pub const IRULES_INTERPRETER_ABSENT: &[&str] = &[
    "auto_execok",
    "auto_import",
    "auto_load",
    "auto_qualify",
    "cd",
    "exec",
    "exit",
    "fconfigure",
    "file",
    "glob",
    "load",
    "open",
    "pwd",
    "socket",
    "source",
    "unknown",
];

/// The 15 commands **present in TMM's interpreter but refused by the rule
/// compiler** at load (measurements §4b: reachable via `eval`; `rename`
/// demonstrably works).
pub const IRULES_COMPILER_REFUSED: &[&str] = &[
    "eof",
    "fblocked",
    "fcopy",
    "flush",
    "gets",
    "interp",
    "namespace",
    "package",
    "pid",
    "rename",
    "seek",
    "tell",
    "time",
    "update",
    "vwait",
];

/// The mechanism that keeps `command` out of iRules source, or `None`
/// when the command is not one of the 31 measured "disabled" builtins.
#[must_use]
pub fn irules_disabled_class(command: &str) -> Option<IrulesDisabledClass> {
    if IRULES_INTERPRETER_ABSENT.contains(&command) {
        Some(IrulesDisabledClass::InterpreterAbsent)
    } else if IRULES_COMPILER_REFUSED.contains(&command) {
        Some(IrulesDisabledClass::CompilerRefused)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::DialectSet;

    /// The §4b split is exact: 16 + 15 disjoint commands, together the
    /// §5 31-command disabled list.
    #[test]
    fn the_split_is_sixteen_plus_fifteen_and_disjoint() {
        assert_eq!(IRULES_INTERPRETER_ABSENT.len(), 16);
        assert_eq!(IRULES_COMPILER_REFUSED.len(), 15);
        for absent in IRULES_INTERPRETER_ABSENT {
            assert!(
                !IRULES_COMPILER_REFUSED.contains(absent),
                "{absent} cannot be in both classes"
            );
        }
        // The §5 disabled list, verbatim.
        let disabled = [
            "auto_execok",
            "auto_import",
            "auto_load",
            "auto_qualify",
            "cd",
            "eof",
            "exec",
            "exit",
            "fblocked",
            "fconfigure",
            "fcopy",
            "file",
            "flush",
            "gets",
            "glob",
            "interp",
            "load",
            "namespace",
            "open",
            "package",
            "pid",
            "pwd",
            "rename",
            "seek",
            "socket",
            "source",
            "tell",
            "time",
            "unknown",
            "update",
            "vwait",
        ];
        assert_eq!(disabled.len(), 31);
        for command in disabled {
            assert!(
                irules_disabled_class(command).is_some(),
                "{command} must classify"
            );
        }
        assert_eq!(irules_disabled_class("set"), None);
        assert_eq!(irules_disabled_class("HTTP::uri"), None);
        assert_eq!(
            irules_disabled_class("exec"),
            Some(IrulesDisabledClass::InterpreterAbsent)
        );
        assert_eq!(
            irules_disabled_class("rename"),
            Some(IrulesDisabledClass::CompilerRefused)
        );
        assert!(IrulesDisabledClass::InterpreterAbsent.is_language_fact());
        assert!(!IrulesDisabledClass::CompilerRefused.is_language_fact());
    }

    /// Every classified command exists in the compiled universe as a Tcl
    /// spec that does NOT carry the `IRULES` bit — the two lists refine
    /// the spec-level exclusion, they never contradict it.
    #[test]
    fn the_classes_refine_the_spec_level_exclusion() {
        let registry = crate::CommandRegistry::build_default();
        for command in IRULES_INTERPRETER_ABSENT
            .iter()
            .chain(IRULES_COMPILER_REFUSED)
        {
            let specs = registry.specs(command);
            assert!(!specs.is_empty(), "{command} must have a spec");
            for spec in specs {
                let gate = spec.dialects.expect("stock builtins carry a gate");
                assert!(
                    !gate.intersects(DialectSet::IRULES),
                    "{command} must not carry the IRULES bit"
                );
            }
        }
    }
}
