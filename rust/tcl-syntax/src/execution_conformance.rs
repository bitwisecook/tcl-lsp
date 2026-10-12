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

//! Executable binding, package and autoload observations shared by consumers.
//! The scripts use the interpreter's own operations; these rows do not replace
//! the production resolver or bless a compiler result as its own oracle.

use crate::release_expectations::PerRelease;
use crate::vector_ops::split_row;

pub const UNSUPPORTED: &str = "!UNSUPPORTED";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionDomain {
    CommandBinding,
    PackageLookup,
    Autoload,
}

impl ExecutionDomain {
    pub const ALL: [Self; 3] = [Self::CommandBinding, Self::PackageLookup, Self::Autoload];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CommandBinding => "command-binding",
            Self::PackageLookup => "package-lookup",
            Self::Autoload => "autoload",
        }
    }

    const fn data(self) -> &'static str {
        match self {
            Self::CommandBinding => include_str!("../tests/data/command_binding_vectors.txt"),
            Self::PackageLookup => include_str!("../tests/data/package_lookup_vectors.txt"),
            Self::Autoload => include_str!("../tests/data/autoload_vectors.txt"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionVector {
    pub domain: ExecutionDomain,
    pub line: usize,
    pub id: String,
    pub capabilities: Vec<String>,
    pub source: String,
    pub wants: PerRelease,
    pub jim_want: String,
}

impl ExecutionVector {
    /// Capture completion and result together without depending on a trailing
    /// assignment command after the tested operations have changed dispatch.
    #[must_use]
    pub fn script(&self) -> String {
        format!("puts [list [catch {{{}}} value] $value]\n", self.source)
    }
}

#[must_use]
/// Read the shared cases for one domain without resolving their command text.
///
/// ```
/// use tcl_syntax::execution_conformance::{ExecutionDomain, vectors};
/// let cases = vectors(ExecutionDomain::CommandBinding);
/// let import = cases.iter().find(|case| case.id == "import_rename_identity").unwrap();
/// assert_ne!(import.wants.newest(), import.jim_want);
/// ```
pub fn vectors(domain: ExecutionDomain) -> Vec<ExecutionVector> {
    domain
        .data()
        .lines()
        .enumerate()
        .filter_map(|(index, text)| {
            let text = text.trim();
            if text.is_empty() || text.starts_with('#') {
                return None;
            }
            let fields = split_row(text);
            assert_eq!(
                fields.len(),
                5,
                "{} vector line {}",
                domain.name(),
                index + 1
            );
            Some(ExecutionVector {
                domain,
                line: index + 1,
                id: fields[0].to_owned(),
                capabilities: if fields[1] == "-" {
                    Vec::new()
                } else {
                    fields[1].split(',').map(str::to_owned).collect()
                },
                source: fields[2].to_owned(),
                wants: PerRelease::parse(fields[3]).expect("complete release expectation"),
                jim_want: fields[4].to_owned(),
            })
        })
        .collect()
}

/// A standalone script whose rewrites must preserve actual file execution.
/// Package/trace cases intentionally retain feature errors on unsupported Jim
/// builds; absence never substitutes a C Tcl implementation contract.
#[derive(Clone, Copy, Debug)]
pub struct RewriteCase {
    pub name: &'static str,
    pub source: &'static str,
    pub c_stdout: &'static str,
    pub jim_stdout: &'static str,
    pub c_error: Option<&'static str>,
    pub jim_error: Option<&'static str>,
}

/// Deterministic cross-consumer inputs, including positive and error outcomes.
#[must_use]
pub fn rewrite_cases() -> &'static [RewriteCase] {
    REWRITE_CASES
}

const REWRITE_CASES: &[RewriteCase] = &[
    RewriteCase {
        name: "child-alias-target-frame",
        source: include_str!("../tests/data/resolution/child-alias-target-frame.tcl"),
        c_stdout: "LOCAL ::N\n",
        jim_stdout: "LOCAL ::N\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "child-interpreter-isolation",
        source: include_str!("../tests/data/resolution/child-interpreter-isolation.tcl"),
        c_stdout: "PARENT CHILD 1\n",
        jim_stdout: "PARENT CHILD 1\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "child-alias-parent-rebinding",
        source: include_str!("../tests/data/resolution/child-alias-parent-rebinding.tcl"),
        c_stdout: "PREFIX X\nNEW PREFIX X\n1\n",
        jim_stdout: "PREFIX X\nNEW PREFIX X\n1\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "shadowed-expr",
        source: include_str!("../tests/data/resolution/shadowed-expr.tcl"),
        c_stdout: "SHADOW\nSHADOW\n",
        jim_stdout: "SHADOW\nSHADOW\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "alias-prefix-expr",
        source: include_str!("../tests/data/resolution/alias-prefix-expr.tcl"),
        c_stdout: "123\n101\n",
        jim_stdout: "",
        c_error: None,
        jim_error: Some("wrong # args: should be \"interp\""),
    },
    RewriteCase {
        name: "renamed-expr",
        source: include_str!("../tests/data/resolution/renamed-expr.tcl"),
        c_stdout: "43\n",
        jim_stdout: "43\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "conditional-set-shadow",
        source: include_str!("../tests/data/resolution/conditional-set-shadow.tcl"),
        c_stdout: "",
        jim_stdout: "",
        c_error: Some("can't read \"x\": no such variable"),
        jim_error: Some("can't read \"x\": no such variable"),
    },
    RewriteCase {
        name: "dead-procedure-command-transition",
        source: include_str!("../tests/data/resolution/dead-procedure-command-transition.tcl"),
        c_stdout: "",
        jim_stdout: "",
        c_error: Some("can't read \"x\": no such variable"),
        jim_error: Some("can't read \"x\": no such variable"),
    },
    RewriteCase {
        name: "relative-cell-write",
        source: include_str!("../tests/data/resolution/relative-cell-write.tcl"),
        c_stdout: "10\n",
        jim_stdout: "",
        c_error: None,
        jim_error: Some("can't read \"::N::R::x\": no such variable"),
    },
    RewriteCase {
        name: "substitution-rebinding",
        source: include_str!("../tests/data/resolution/substitution-rebinding.tcl"),
        c_stdout: "NEW\n1\n",
        jim_stdout: "NEW\n1\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "import-command-lifetime",
        source: include_str!("../tests/data/resolution/import-command-lifetime.tcl"),
        c_stdout: "OLD\n::a::saved\n",
        jim_stdout: "NEW\n::a::f\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "upvar-retarget",
        source: include_str!("../tests/data/resolution/upvar-retarget.tcl"),
        c_stdout: "X\nNEW\n",
        jim_stdout: "X\nNEW\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "alias-retarget-expression",
        source: include_str!("../tests/data/resolution/alias-retarget-expression.tcl"),
        c_stdout: "2\n10\n",
        jim_stdout: "2\n10\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "upvar-unset-recreate",
        source: include_str!("../tests/data/resolution/upvar-unset-recreate.tcl"),
        c_stdout: "NEW\nNEW\n",
        jim_stdout: "NEW\nNEW\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "execution-trace-rebinding",
        source: include_str!("../tests/data/resolution/execution-trace-rebinding.tcl"),
        c_stdout: "NEW\nNEW\n",
        jim_stdout: "",
        c_error: None,
        jim_error: Some("invalid command name \"trace\""),
    },
    RewriteCase {
        name: "package-loader-mutation",
        source: include_str!("../tests/data/resolution/package-loader-mutation.tcl"),
        c_stdout: "OLD\n1.0\nNEW\n",
        jim_stdout: "",
        c_error: None,
        jim_error: Some(
            "package, unknown command \"ifneeded\": should be forget, names, provide, require",
        ),
    },
    RewriteCase {
        name: "body-nested-indentation",
        source: include_str!("../tests/data/resolution/body-nested-indentation.tcl"),
        c_stdout: "9\n",
        jim_stdout: "9\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "body-quoted-escapes",
        source: include_str!("../tests/data/resolution/body-quoted-escapes.tcl"),
        c_stdout: "4 {A\\nB}\n",
        jim_stdout: "4 {A\\nB}\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "body-trailing-comment",
        source: include_str!("../tests/data/resolution/body-trailing-comment.tcl"),
        c_stdout: "OK\n",
        jim_stdout: "OK\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "final-empty-if-result",
        source: include_str!("../tests/data/resolution/final-empty-if-result.tcl"),
        c_stdout: "{}\n",
        jim_stdout: "{}\n",
        c_error: None,
        jim_error: None,
    },
    RewriteCase {
        name: "final-false-for-result",
        source: include_str!("../tests/data/resolution/final-false-for-result.tcl"),
        c_stdout: "{}\n",
        jim_stdout: "{}\n",
        c_error: None,
        jim_error: None,
    },
];

/// Source files and observations shared by package-loading consumer tests.
#[derive(Clone, Copy, Debug)]
pub struct FilesystemExecutionCase {
    /// Stable scenario name.
    pub name: &'static str,
    /// Source files installed under the isolated oracle fixture root.
    pub files: &'static [(&'static str, &'static str)],
    /// Tcl source returning the observable package/load result.
    pub source: &'static str,
    /// Captured completion/result expected from each C Tcl release.
    pub c_want: &'static str,
    /// Independently measured current Jim completion/result.
    pub jim_want: &'static str,
}

impl FilesystemExecutionCase {
    /// Render a caught observation; the harness supplies `::oracle_fixture_root`.
    #[must_use]
    pub fn script(&self) -> String {
        format!("puts [list [catch {{{}}} value] $value]\n", self.source)
    }
}

/// Package loader fixtures executed unchanged by references and rewrite tests.
/// C receives a `pkgIndex.tcl`; Jim finds the same plain package source directly.
#[must_use]
pub const fn filesystem_cases() -> &'static [FilesystemExecutionCase] {
    &FILESYSTEM_CASES
}

const FILESYSTEM_CASES: [FilesystemExecutionCase; 3] = [
    FilesystemExecutionCase {
        name: "implicit_loader_provision_and_global_activation",
        files: &[
            (
                "plain.tcl",
                "set ::plainContext [list [namespace current] [info level]]; set ::plainDone 1",
            ),
            (
                "pkgIndex.tcl",
                "package ifneeded plain 1.0 [list source [file join $dir plain.tcl]]",
            ),
        ],
        source: "set auto_path [linsert $auto_path 0 $::oracle_fixture_root]; namespace eval caller {proc p {} {package require plain}; set ::loadStatus [catch p ::loadResult]}; list $loadStatus $plainContext $plainDone",
        c_want: "0 {1 {:: 0} 1}",
        jim_want: "0 {0 {:: 0} 1}",
    },
    FilesystemExecutionCase {
        name: "recursive_loader_provisional_package",
        files: &[
            (
                "recursive.tcl",
                "set ::recursiveSeen [list [catch {package require recursive} e] $e]; package provide recursive 1.0",
            ),
            (
                "pkgIndex.tcl",
                "package ifneeded recursive 1.0 [list source [file join $dir recursive.tcl]]",
            ),
        ],
        source: "set auto_path [linsert $auto_path 0 $::oracle_fixture_root]; list [package require recursive] [lindex $recursiveSeen 0] [expr {[lindex $recursiveSeen 1] eq {}}]",
        c_want: "0 {1.0 1 0}",
        jim_want: "0 {1.0 0 1}",
    },
    FilesystemExecutionCase {
        name: "failed_loader_provision_and_partial_mutation",
        files: &[
            (
                "bad.tcl",
                "proc ::side {} {return yes}; package provide bad 2.0; error stop",
            ),
            (
                "pkgIndex.tcl",
                "package ifneeded bad 2.0 [list source [file join $dir bad.tcl]]",
            ),
        ],
        source: r#"set auto_path [linsert $auto_path 0 $::oracle_fixture_root]; list [catch {package require bad} e] [lindex [split $e "\n"] 0] [expr {[lsearch -exact [package names] bad] >= 0}] [list [catch {package provide bad} provided] $provided] [side]"#,
        c_want: "0 {1 stop 1 {0 {}} yes}",
        jim_want: "0 {1 stop 0 {0 {}} yes}",
    },
];
