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

//! Namespace-operation conformance vectors.
//!
//! The third sibling of [`crate::naming::conformance`]: the operations that
//! *build* the namespace tree rather than resolve against it — colon
//! normalisation, import/export/forget, `namespace which` and `origin`,
//! `namespace path`, `namespace unknown`, and the deletion cascade.
//!
//! Each row of `tests/data/namespace_op_vectors.txt` names the namespace
//! the probe runs in, a list of setup mini-ops, a probe, and the expected
//! observable per release.  As in [`crate::var_conformance`], the
//! observable is the two-element Tcl list `[list <catch code> <result>]`,
//! and the whole row — setup included — runs inside that `catch`, so a row
//! that uses an 8.5+ subcommand states 8.4's real error text as its 8.4
//! column instead of being excluded.
//!
//! C references: `generic/tclNamesp.c` (Tcl 9.0.4, 8.6.16, and 8.4.20 for
//! the pre-TIP-181/230 subcommand set).

use crate::release_expectations::PerRelease;
use crate::vector_ops::{split_head, split_ops, split_row};
use std::fmt::Write as _;

/// The setup mini-op vocabulary of the namespace-operation domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// `ns(NS)` — create `NS`.
    Namespace,
    /// `delns(NS)` — delete `NS`.
    DeleteNamespace,
    /// `proc(NAME=RESULT)` — define `NAME` as a proc returning `RESULT`.
    Proc,
    /// `rename(OLD NEW)` — `rename OLD NEW`.
    Rename,
    /// `export(NS ARGS)` — `namespace export ARGS` inside `NS`.
    Export,
    /// `import(NS ARGS)` — `namespace import ARGS` inside `NS`.
    Import,
    /// `forget(NS ARGS)` — `namespace forget ARGS` inside `NS`.
    Forget,
    /// `path(NS ENTRIES)` — `namespace path {ENTRIES}` inside `NS` (8.5+).
    Path,
    /// `unknown(NS HANDLER)` — `namespace unknown HANDLER` inside `NS`
    /// (8.5+).
    Unknown,
    /// `var(NAME=VALUE)` — `set NAME VALUE`.
    Set,
    /// `eval(SCRIPT)` — SCRIPT verbatim, for shapes the vocabulary above
    /// cannot express (each use is commented in the vector file).
    Eval,
}

impl OpKind {
    fn parse(keyword: &str) -> Result<Self, String> {
        match keyword {
            "ns" => Ok(Self::Namespace),
            "delns" => Ok(Self::DeleteNamespace),
            "proc" => Ok(Self::Proc),
            "rename" => Ok(Self::Rename),
            "export" => Ok(Self::Export),
            "import" => Ok(Self::Import),
            "forget" => Ok(Self::Forget),
            "path" => Ok(Self::Path),
            "unknown" => Ok(Self::Unknown),
            "var" => Ok(Self::Set),
            "eval" => Ok(Self::Eval),
            other => Err(format!("unknown setup op {other:?}")),
        }
    }
}

/// One setup mini-op, in the order the row wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupOp {
    /// The op keyword.
    pub kind: OpKind,
    /// The text between the parentheses, verbatim.
    pub argument: String,
}

impl SetupOp {
    /// Render the op as the Tcl command it stands for.
    #[must_use]
    pub fn to_tcl(&self) -> String {
        let argument = &self.argument;
        match self.kind {
            OpKind::Namespace => format!("namespace eval {argument} {{}}"),
            OpKind::DeleteNamespace => format!("namespace delete {argument}"),
            OpKind::Proc => {
                let (name, result) = argument.split_once('=').unwrap_or((argument, ""));
                format!("proc {name} {{args}} {{return {{{result}}}}}")
            }
            OpKind::Rename => format!("rename {argument}"),
            OpKind::Export => in_namespace(argument, "namespace export"),
            OpKind::Import => in_namespace(argument, "namespace import"),
            OpKind::Forget => in_namespace(argument, "namespace forget"),
            OpKind::Path => {
                let (ns, entries) = split_head(argument);
                format!("namespace eval {ns} {{namespace path [list {entries}]}}")
            }
            OpKind::Unknown => in_namespace(argument, "namespace unknown"),
            OpKind::Set => {
                let (name, value) = argument.split_once('=').unwrap_or((argument, ""));
                format!("set {name} {{{value}}}")
            }
            OpKind::Eval => argument.clone(),
        }
    }
}

/// `NS ARGS` plus a `namespace` subcommand into `namespace eval NS {sub ARGS}`.
fn in_namespace(argument: &str, subcommand: &str) -> String {
    let (ns, rest) = split_head(argument);
    format!("namespace eval {ns} {{{subcommand} {rest}}}")
}

/// The probe vocabulary of the namespace-operation domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    /// `which NAME` — `namespace which NAME`.
    Which,
    /// `whichvar NAME` — `namespace which -variable NAME`.
    WhichVar,
    /// `origin NAME` — `namespace origin NAME`.
    Origin,
    /// `current` — `namespace current`.
    Current,
    /// `parent ?NS?` — `namespace parent ?NS?`.
    Parent,
    /// `nsexists NS` — `namespace exists NS`.
    NamespaceExists,
    /// `children ?NS? ?PATTERN?` — `lsort [namespace children …]`.
    Children,
    /// `commands PATTERN` — `lsort [info commands PATTERN]`.
    Commands,
    /// `vars PATTERN` — `lsort [info vars PATTERN]`.
    Vars,
    /// `globals PATTERN` — `info globals PATTERN`.
    Globals,
    /// `exports` — `namespace export` with no patterns.
    Exports,
    /// `path` — `namespace path` with no arguments.
    Path,
    /// `qualifiers TEXT` — `namespace qualifiers TEXT`.
    Qualifiers,
    /// `tail TEXT` — `namespace tail TEXT`.
    Tail,
    /// `autoqualify CMD NS` — the `auto_qualify` library proc.
    AutoQualify,
    /// `read NAME` — `set NAME`, the variable side of the colon matrix.
    Read,
    /// `call TEXT` — evaluate `TEXT` as written.
    Call,
}

/// What the row observes once its setup has run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The probe keyword.
    pub kind: ProbeKind,
    /// The rest of the probe column.
    pub argument: String,
}

impl Probe {
    /// Parse the probe column.
    ///
    /// # Errors
    /// Returns a message for an unknown probe keyword.
    pub fn parse(column: &str) -> Result<Self, String> {
        let (keyword, argument) = split_head(column);
        let kind = match keyword {
            "which" => ProbeKind::Which,
            "whichvar" => ProbeKind::WhichVar,
            "origin" => ProbeKind::Origin,
            "current" => ProbeKind::Current,
            "parent" => ProbeKind::Parent,
            "nsexists" => ProbeKind::NamespaceExists,
            "children" => ProbeKind::Children,
            "commands" => ProbeKind::Commands,
            "vars" => ProbeKind::Vars,
            "globals" => ProbeKind::Globals,
            "exports" => ProbeKind::Exports,
            "path" => ProbeKind::Path,
            "qualifiers" => ProbeKind::Qualifiers,
            "tail" => ProbeKind::Tail,
            "autoqualify" => ProbeKind::AutoQualify,
            "read" => ProbeKind::Read,
            "call" => ProbeKind::Call,
            other => return Err(format!("unknown probe {other:?}")),
        };
        Ok(Self {
            kind,
            argument: argument.to_owned(),
        })
    }

    /// Render the probe as the Tcl command whose result the row expects.
    #[must_use]
    pub fn to_tcl(&self) -> String {
        let argument = &self.argument;
        match self.kind {
            ProbeKind::Which => format!("namespace which {argument}"),
            ProbeKind::WhichVar => format!("namespace which -variable {argument}"),
            ProbeKind::Origin => format!("namespace origin {argument}"),
            ProbeKind::Current => "namespace current".to_owned(),
            ProbeKind::Parent => format!("namespace parent {argument}"),
            ProbeKind::NamespaceExists => format!("namespace exists {argument}"),
            ProbeKind::Children => format!("lsort [namespace children {argument}]"),
            ProbeKind::Commands => format!("lsort [info commands {argument}]"),
            ProbeKind::Vars => format!("lsort [info vars {argument}]"),
            ProbeKind::Globals => format!("info globals {argument}"),
            ProbeKind::Exports => "namespace export".to_owned(),
            ProbeKind::Path => "namespace path".to_owned(),
            ProbeKind::Qualifiers => format!("namespace qualifiers {argument}"),
            ProbeKind::Tail => format!("namespace tail {argument}"),
            ProbeKind::AutoQualify => format!("auto_qualify {argument}"),
            ProbeKind::Read => format!("set {argument}"),
            ProbeKind::Call => argument.clone(),
        }
    }
}

/// One namespace-operation scenario.
#[derive(Debug, Clone)]
pub struct NamespaceOpVector {
    /// `::`-rooted namespace the probe runs in.
    pub ns: String,
    /// Setup mini-ops, in order, all executed at global script level.
    pub ops: Vec<SetupOp>,
    /// The observation the row makes, inside `ns`.
    pub probe: Probe,
    /// `[list <catch code> <result>]` per modelled release.
    pub wants: PerRelease,
    /// 1-based line in the vector file (for failure messages).
    pub line: usize,
}

/// The raw vector table (see the file header for the format).
pub const RAW: &str = include_str!("../tests/data/namespace_op_vectors.txt");

/// Parse [`RAW`] into vectors.
///
/// # Panics
/// On a malformed row — the file is repo-controlled test data, so a parse
/// failure is a bug in the row, not an input condition.
#[must_use]
pub fn vectors() -> Vec<NamespaceOpVector> {
    let mut out = Vec::new();
    for (index, raw_line) in RAW.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields = split_row(line);
        assert!(
            fields.len() == 4,
            "vector line {number}: expected 4 |-separated fields, got {}: {raw_line:?}",
            fields.len(),
        );
        let fail = |why: String| -> ! { panic!("vector line {number}: {why}") };
        let ops = split_ops(fields[1])
            .unwrap_or_else(|why| fail(why))
            .into_iter()
            .map(|(keyword, argument)| SetupOp {
                kind: OpKind::parse(&keyword).unwrap_or_else(|why| fail(why)),
                argument,
            })
            .collect();
        out.push(NamespaceOpVector {
            ns: fields[0].to_owned(),
            ops,
            probe: Probe::parse(fields[2]).unwrap_or_else(|why| fail(why)),
            wants: PerRelease::parse(fields[3]).unwrap_or_else(|why| fail(why)),
            line: number,
        });
    }
    assert!(
        !out.is_empty(),
        "no namespace-op conformance vectors parsed"
    );
    out
}

/// The **setup** half of a vector's script: the mini-ops as Tcl, one per
/// line, in row order.
#[must_use]
pub fn vector_setup(vector: &NamespaceOpVector) -> String {
    let mut script = String::new();
    for op in &vector.ops {
        let _ = writeln!(script, "{}", op.to_tcl());
    }
    script
}

/// The **call** half of a vector's script: the probe, evaluated in the
/// vector's namespace.
#[must_use]
pub fn vector_call(vector: &NamespaceOpVector) -> String {
    let probe = vector.probe.to_tcl();
    if vector.ns == "::" {
        probe
    } else {
        format!("namespace eval {} {{{probe}}}", vector.ns)
    }
}

/// Render a vector as a runnable Tcl script whose **output** is the row's
/// observable: the two-element list `<catch code> <result-or-message>`.
#[must_use]
pub fn vector_script(vector: &NamespaceOpVector) -> String {
    let mut script = String::new();
    let _ = writeln!(
        script,
        "set __vecCode [catch {{\n{}{}\n}} __vecResult]",
        vector_setup(vector),
        vector_call(vector),
    );
    script.push_str("puts [list $__vecCode $__vecResult]\n");
    script
}

#[cfg(test)]
mod tests {
    use super::{ProbeKind, vector_script, vectors};

    #[test]
    fn every_row_parses_and_covers_the_ladder() {
        let rows = vectors();
        assert!(
            rows.len() >= 40,
            "expected at least 40 rows, got {}",
            rows.len()
        );
        for row in &rows {
            assert!(
                row.ns.starts_with("::"),
                "line {}: namespace {:?} is not ::-rooted",
                row.line,
                row.ns,
            );
        }
    }

    #[test]
    fn the_script_always_reports_a_catch_code_and_a_result() {
        for row in vectors() {
            let script = vector_script(&row);
            assert!(
                script.ends_with("puts [list $__vecCode $__vecResult]\n"),
                "line {}: script does not end with the observable",
                row.line,
            );
        }
    }

    #[test]
    fn the_release_differentiating_families_are_all_represented() {
        let rows = vectors();
        assert!(rows.iter().any(|row| row.probe.kind == ProbeKind::Path));
        assert!(rows.iter().any(|row| row.probe.kind == ProbeKind::WhichVar));
        assert!(rows.iter().any(|row| row.probe.kind == ProbeKind::Origin));
        assert!(
            rows.iter().any(|row| row.wants.is_release_tagged()),
            "a namespace-op table with no release-tagged row is not a multi-release oracle",
        );
    }
}
