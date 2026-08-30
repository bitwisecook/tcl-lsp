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

//! Variable lookup and creation conformance vectors.
//!
//! The sibling of [`crate::naming::conformance`] for the *variable* name
//! space: `TclLookupVar`, `Tcl_FindNamespaceVar`, `MakeUpvar`, and the
//! TIP 278 removal of the global secondary lookup at 9.0 — the one place
//! where variables and commands stopped agreeing.
//!
//! Each row of `tests/data/variable_resolution_vectors.txt` names a
//! current namespace, a frame kind, a list of setup mini-ops, a probe, and
//! the expected observable per release.  The observable is always the
//! two-element Tcl list `[list <catch code> <result-or-message>]`, so a row
//! that errors on one release and answers on another is one row with two
//! columns rather than two incomparable rows.
//!
//! C references: `TclLookupVar` / `TclObjLookupVarEx` (`generic/tclVar.c`),
//! `Tcl_FindNamespaceVar` (`generic/tclNamesp.c`), Tcl 9.0.4 and 8.6.16.

use crate::release_expectations::PerRelease;
use crate::vector_ops::{split_ops, split_row};
use std::fmt::Write as _;

/// Where the probe runs relative to the current namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// At script level in the global namespace — no procedure call frame,
    /// so `TclLookupVar` resolves against `::` directly.
    Global,
    /// Inside `namespace eval <ns> { … }` — a call frame whose namespace is
    /// `ns` but which has no local variable table of its own.
    NsEval,
    /// Inside a procedure defined in `ns` and called once — a real local
    /// frame, which is what `global` / `variable` link into.
    Proc,
}

impl FrameKind {
    /// Parse the frame column.
    ///
    /// # Errors
    /// Returns a message for an unknown frame name.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name.trim() {
            "global" => Ok(Self::Global),
            "nseval" => Ok(Self::NsEval),
            "proc" => Ok(Self::Proc),
            other => Err(format!(
                "unknown frame kind {other:?} (want global, nseval, or proc)"
            )),
        }
    }
}

/// One setup mini-op, in the order the row wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupOp {
    /// The op keyword (`ns`, `var`, `decl`, …).
    pub kind: OpKind,
    /// The text between the parentheses, verbatim.
    pub argument: String,
}

/// The setup mini-op vocabulary of the variable domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// `ns(NS)` — create `NS` (relative names resolve in the running frame).
    Namespace,
    /// `delns(NS)` — delete `NS`.
    DeleteNamespace,
    /// `var(NAME=VALUE)` — `set NAME VALUE`.
    Set,
    /// `decl(TEXT)` — a declaration command verbatim: `global x`,
    /// `variable x`, `variable x 5`.
    Declare,
    /// `unset(NAME)` — `unset NAME`.
    Unset,
    /// `upvar(ARGS)` — `upvar ARGS`.
    Upvar,
    /// `nsupvar(ARGS)` — `namespace upvar ARGS` (8.5+).
    NamespaceUpvar,
    /// `eval(SCRIPT)` — SCRIPT verbatim, for the few shapes the vocabulary
    /// above cannot express (each use is commented in the vector file).
    Eval,
}

impl OpKind {
    fn parse(keyword: &str) -> Result<Self, String> {
        match keyword {
            "ns" => Ok(Self::Namespace),
            "delns" => Ok(Self::DeleteNamespace),
            "var" => Ok(Self::Set),
            "decl" => Ok(Self::Declare),
            "unset" => Ok(Self::Unset),
            "upvar" => Ok(Self::Upvar),
            "nsupvar" => Ok(Self::NamespaceUpvar),
            "eval" => Ok(Self::Eval),
            other => Err(format!("unknown setup op {other:?}")),
        }
    }
}

impl SetupOp {
    /// Render the op as the Tcl command it stands for.
    #[must_use]
    pub fn to_tcl(&self) -> String {
        let argument = &self.argument;
        match self.kind {
            OpKind::Namespace => format!("namespace eval {argument} {{}}"),
            OpKind::DeleteNamespace => format!("namespace delete {argument}"),
            OpKind::Set => {
                let (name, value) = argument.split_once('=').unwrap_or((argument, ""));
                format!("set {name} {{{value}}}")
            }
            OpKind::Declare | OpKind::Eval => argument.clone(),
            OpKind::Unset => format!("unset {argument}"),
            OpKind::Upvar => format!("upvar {argument}"),
            OpKind::NamespaceUpvar => format!("namespace upvar {argument}"),
        }
    }
}

/// What the row observes once its setup has run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The probe keyword.
    pub kind: ProbeKind,
    /// The rest of the probe column.
    pub argument: String,
}

/// The probe vocabulary of the variable domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    /// `read NAME` — `set NAME`, the pure lookup path.
    Read,
    /// `set NAME VALUE` — the creation path, which does not always agree
    /// with the lookup path.
    Write,
    /// `which NAME` — `namespace which -variable NAME`.
    Which,
    /// `exists NAME` — `info exists NAME`.
    Exists,
    /// `vars PATTERN` — `lsort [info vars PATTERN]`.
    Vars,
}

impl Probe {
    /// Parse the probe column.
    ///
    /// # Errors
    /// Returns a message for an unknown probe keyword.
    pub fn parse(column: &str) -> Result<Self, String> {
        let (keyword, argument) = crate::vector_ops::split_head(column);
        let kind = match keyword {
            "read" => ProbeKind::Read,
            "set" => ProbeKind::Write,
            "which" => ProbeKind::Which,
            "exists" => ProbeKind::Exists,
            "vars" => ProbeKind::Vars,
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
            ProbeKind::Read | ProbeKind::Write => format!("set {argument}"),
            ProbeKind::Which => format!("namespace which -variable {argument}"),
            ProbeKind::Exists => format!("info exists {argument}"),
            ProbeKind::Vars => format!("lsort [info vars {argument}]"),
        }
    }
}

/// One variable-resolution scenario.
#[derive(Debug, Clone)]
pub struct VariableVector {
    /// `::`-rooted namespace the frame belongs to.
    pub ns: String,
    /// Which kind of frame the setup and probe run in.
    pub frame: FrameKind,
    /// Setup mini-ops, in order, all executed inside the frame.
    pub ops: Vec<SetupOp>,
    /// The observation the row makes.
    pub probe: Probe,
    /// `[list <catch code> <result>]` per modelled release.
    pub wants: PerRelease,
    /// 1-based line in the vector file (for failure messages).
    pub line: usize,
}

/// The raw vector table (see the file header for the format).
pub const RAW: &str = include_str!("../tests/data/variable_resolution_vectors.txt");

/// Parse [`RAW`] into vectors.
///
/// # Panics
/// On a malformed row — the file is repo-controlled test data, so a parse
/// failure is a bug in the row, not an input condition.
#[must_use]
pub fn vectors() -> Vec<VariableVector> {
    let mut out = Vec::new();
    for (index, raw_line) in RAW.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields = split_row(line);
        assert!(
            fields.len() == 5,
            "vector line {number}: expected 5 |-separated fields, got {}: {raw_line:?}",
            fields.len(),
        );
        let fail = |why: String| -> ! { panic!("vector line {number}: {why}") };
        let ops = split_ops(fields[2])
            .unwrap_or_else(|why| fail(why))
            .into_iter()
            .map(|(keyword, argument)| SetupOp {
                kind: OpKind::parse(&keyword).unwrap_or_else(|why| fail(why)),
                argument,
            })
            .collect();
        out.push(VariableVector {
            ns: fields[0].to_owned(),
            frame: FrameKind::parse(fields[1]).unwrap_or_else(|why| fail(why)),
            ops,
            probe: Probe::parse(fields[3]).unwrap_or_else(|why| fail(why)),
            wants: PerRelease::parse(fields[4]).unwrap_or_else(|why| fail(why)),
            line: number,
        });
    }
    assert!(!out.is_empty(), "no variable conformance vectors parsed");
    out
}

/// The **setup** half of a vector's script: the mini-ops as Tcl, one per
/// line, in row order.
///
/// Split out from [`vector_script`] the way the command-resolution
/// renderer splits it, so a backend can pair the setup with its own result
/// capture.
#[must_use]
pub fn vector_setup(vector: &VariableVector) -> String {
    let mut script = String::new();
    for op in &vector.ops {
        let _ = writeln!(script, "{}", op.to_tcl());
    }
    script
}

/// The **call** half of a vector's script: the probe as Tcl.
#[must_use]
pub fn vector_call(vector: &VariableVector) -> String {
    vector.probe.to_tcl()
}

/// The frame body — setup then probe — as one script.
fn frame_body(vector: &VariableVector) -> String {
    format!("{}{}\n", vector_setup(vector), vector_call(vector))
}

/// Render a vector as a runnable Tcl script whose **output** is the row's
/// observable: the two-element list `<catch code> <result-or-message>`.
///
/// The whole frame — setup included — sits inside the `catch`, so a row
/// whose *setup* is what a release rejects (`namespace upvar` before 8.5)
/// reports that release's error text rather than killing the interpreter.
#[must_use]
pub fn vector_script(vector: &VariableVector) -> String {
    let mut script = String::new();
    if vector.ns != "::" {
        let _ = writeln!(script, "namespace eval {} {{}}", vector.ns);
    }
    let body = frame_body(vector);
    let invocation = match vector.frame {
        FrameKind::Global => body,
        FrameKind::NsEval => format!("namespace eval {} {{\n{body}}}\n", vector.ns),
        FrameKind::Proc => {
            let _ = writeln!(
                script,
                "namespace eval {} {{\nproc __vecProbe {{}} {{\n{body}}}\n}}",
                vector.ns,
            );
            let holder = if vector.ns == "::" { "" } else { &vector.ns };
            format!("{holder}::__vecProbe\n")
        }
    };
    let _ = writeln!(
        script,
        "set __vecCode [catch {{\n{invocation}}} __vecResult]"
    );
    script.push_str("puts [list $__vecCode $__vecResult]\n");
    script
}

#[cfg(test)]
mod tests {
    use super::{FrameKind, ProbeKind, vector_script, vectors};

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
            assert!(
                row.frame != FrameKind::Global || row.ns == "::",
                "line {}: a global frame is only ever the global namespace",
                row.line,
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
            assert!(
                script.contains(&row.probe.to_tcl()),
                "line {}: script does not contain its probe",
                row.line,
            );
        }
    }

    #[test]
    fn the_write_probe_is_distinct_from_the_read_probe() {
        // `set x` and `set x v` take different paths through TclLookupVar
        // (lookup vs create), and the vector file must exercise both.
        let rows = vectors();
        assert!(rows.iter().any(|row| row.probe.kind == ProbeKind::Read));
        assert!(rows.iter().any(|row| row.probe.kind == ProbeKind::Write));
        assert!(rows.iter().any(|row| row.probe.kind == ProbeKind::Which));
    }
}
