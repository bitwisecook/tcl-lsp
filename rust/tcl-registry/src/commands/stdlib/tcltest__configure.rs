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

//! `tcltest::configure` command.
use crate::prelude::*;

/// Levels accepted by `-verbose`.  A combination is given as a list (or a
/// string of single-letter abbreviations), so the set is *not* closed.
/// `line` was added in tcltest 2.3; `msec` and `usec` in tcltest 2.5.
const VERBOSE_LEVELS: &[ArgValue] = &[
    ArgValue {
        value: "body",
        detail: "display the body of failed tests",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "pass",
        detail: "display all passed tests",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "skip",
        detail: "display all skipped tests",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "start",
        detail: "display each test as it starts",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "error",
        detail: "display errorInfo for failed tests",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "line",
        detail: "display source file line of failed tests (tcltest 2.3+)",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "msec",
        detail: "display each test's execution time in milliseconds (tcltest 2.5+)",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "usec",
        detail: "display each test's execution time in microseconds (tcltest 2.5+)",
        min_tcl: None,
        code: None,
    },
];

/// `-debug` levels (`AcceptInteger`, meaningful values 0–3 per the man page).
/// Not closed — any integer is syntactically accepted.
const DEBUG_LEVELS: &[ArgValue] = &[
    ArgValue {
        value: "0",
        detail: "no debug output (default)",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "1",
        detail: "report tests skipped by match/skip patterns",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "2",
        detail: "also dump tcltest variables and flags",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "3",
        detail: "also report test-harness operations",
        min_tcl: None,
        code: None,
    },
];

/// `-preservecore` levels (`AcceptInteger`, 0–2 per the man page).
const PRESERVECORE_LEVELS: &[ArgValue] = &[
    ArgValue {
        value: "0",
        detail: "do not check for core files (default)",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "1",
        detail: "notify when a core file is created",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "2",
        detail: "also save core files in -tmpdir",
        min_tcl: None,
        code: None,
    },
];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-verbose",
        value: OptionValue::enumerated(VERBOSE_LEVELS, false, "levelList"),
        detail: "verbosity of test-run output",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-match",
        value: OptionValue::value("patternList"),
        detail: "run only tests whose name matches one of these glob patterns",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-skip",
        value: OptionValue::value("patternList"),
        detail: "skip tests whose name matches one of these glob patterns",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-file",
        value: OptionValue::value("pattern"),
        detail: "run tests in files matching this glob pattern",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-notfile",
        value: OptionValue::value("pattern"),
        detail: "skip test files matching this glob pattern",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-relateddir",
        value: OptionValue::value("pattern"),
        detail: "run tests in directories matching this glob pattern",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-asidefromdir",
        value: OptionValue::value("pattern"),
        detail: "skip tests in directories matching this glob pattern",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-constraints",
        value: OptionValue::value("constraintList"),
        detail: "do not skip the listed constraints",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-limitconstraints",
        value: OptionValue::boolean(),
        detail: "run only tests with the listed constraints",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-singleproc",
        value: OptionValue::boolean(),
        detail: "run all tests in one process rather than one per file",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-debug",
        value: OptionValue::enumerated(DEBUG_LEVELS, false, "level"),
        detail: "internal debug level (integer)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-preservecore",
        value: OptionValue::enumerated(PRESERVECORE_LEVELS, false, "level"),
        detail: "how aggressively to preserve core files (integer)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-load",
        value: OptionValue::script(),
        detail: "script that loads the tested commands",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-loadfile",
        value: OptionValue::value("file"),
        detail: "file holding the script that loads the tested commands",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-tmpdir",
        value: OptionValue::value("dir"),
        detail: "directory for temporary files created during testing",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-testdir",
        value: OptionValue::value("dir"),
        detail: "directory searched for test files",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-outfile",
        value: OptionValue::value("file"),
        detail: "channel or file for test-run output",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-errfile",
        value: OptionValue::value("file"),
        detail: "channel or file for test-run errors",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-iterations",
        value: OptionValue::value("count"),
        detail: "number of times to run each test (tcltest 2.6+)",
        // Added in tcltest 2.6, which ships with Tcl 9.1 — gate by both the
        // package floor (an explicit `package require tcltest 2.6`) and the
        // core version (2.6 is bundled only from 9.1), matching how `-errorCode`
        // gates on 2.5 / 8.6.
        lifecycle: Lifecycle::introduced_in("2.6"),
        dialects: Some(DialectSet::TCL91),
        ..OptionSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::configure",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set tcltest configuration options.",
            synopsis: &["tcltest::configure ?option? ?value option value ...?"],
            snippet: "Options include ``-verbose``, ``-debug``, ``-outfile``, ``-errfile``, ``-tmpdir``, ``-testdir``, ``-file``, ``-notfile``, ``-relateddir``, ``-asidefromdir``, ``-match``, ``-skip``, ``-constraints``, ``-limitconstraints``, ``-singleproc``, ``-preservecore``, ``-load``, ``-loadfile``, and ``-iterations`` (tcltest 2.6+).",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
