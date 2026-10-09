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

//! The path-name routes of `file` — `join`, `dirname`, `tail`, `extension`,
//! `rootname` and `split` — over the shared cores both runtimes run
//! (`tcl_cmd_core::path`).
//!
//! A name is read as Unix reads it, and the route answers only where the
//! Windows reading agrees and every release does: a backslash (a separator
//! to Windows), a colon (a drive or volume), a leading `//` (a share root,
//! which 9.0 also keeps as a root of its own on Unix) or a `~` (a home
//! directory to 8.x) in any name declines with `ReleaseAmbiguous(Platform)`,
//! since no profile fixes the platform. `file normalize`, whose answer
//! is the host's working directory and its links, has no route.

use tcl_syntax::value::ValueOps;

use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{DependencyEvidence, EvalAnswer, TransferAnswer};
use super::builtins::{exact_operands, pure_outcome, result_type_transfer};
use super::const_ops::{ConstOps, ConstValue, Needs};
use super::context::Budget;
use super::decline::{Axis, DeclineReason};
use super::inputs::{AnalysisInputs, FactDomain};
use super::route::{EvalRoute, NativeEvalId};

/// The revision of the registry-owned path routes.
const REVISION: u64 = 1;

/// A path-name operation of `file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathOperation {
    /// `file join name ?name …?`.
    Join,
    /// `file dirname name`.
    Dirname,
    /// `file tail name`.
    Tail,
    /// `file extension name`.
    Extension,
    /// `file rootname name`.
    Rootname,
    /// `file split name`.
    Split,
}

/// The direct route of one path-name operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathSemantics {
    operation: PathOperation,
}

/// `file join`.
pub static FILE_JOIN: PathSemantics = PathSemantics {
    operation: PathOperation::Join,
};
/// `file dirname`.
pub static FILE_DIRNAME: PathSemantics = PathSemantics {
    operation: PathOperation::Dirname,
};
/// `file tail`.
pub static FILE_TAIL: PathSemantics = PathSemantics {
    operation: PathOperation::Tail,
};
/// `file extension`.
pub static FILE_EXTENSION: PathSemantics = PathSemantics {
    operation: PathOperation::Extension,
};
/// `file rootname`.
pub static FILE_ROOTNAME: PathSemantics = PathSemantics {
    operation: PathOperation::Rootname,
};
/// `file split`.
pub static FILE_SPLIT: PathSemantics = PathSemantics {
    operation: PathOperation::Split,
};

/// Whether every platform and release reads `name` as Unix does: no
/// backslash, no colon, no `~` and no leading `//` (measured with the test
/// shell's `testsetplatform windows` on 8.4.20 to 9.1.0).
#[must_use]
pub fn reads_alike_everywhere(name: &str) -> bool {
    !name.contains(['\\', ':', '~']) && !name.starts_with("//")
}

impl PathSemantics {
    /// The operation.
    #[must_use]
    pub const fn operation(self) -> PathOperation {
        self.operation
    }

    /// The catalogued evaluator.
    #[must_use]
    pub const fn evaluator(self) -> NativeEvalId {
        match self.operation {
            PathOperation::Join => NativeEvalId::PathJoin,
            PathOperation::Dirname => NativeEvalId::PathDirname,
            PathOperation::Tail => NativeEvalId::PathTail,
            PathOperation::Extension => NativeEvalId::PathExtension,
            PathOperation::Rootname => NativeEvalId::PathRootname,
            PathOperation::Split => NativeEvalId::PathSplit,
        }
    }

    /// The type of the result: a list of elements for `split`, a name
    /// otherwise.
    #[must_use]
    pub const fn result_type(self) -> TclType {
        match self.operation {
            PathOperation::Split => TclType::List,
            _ => TclType::String,
        }
    }

    /// The axes the cores read: a non-ASCII name's decoding, and how a list
    /// of elements renders.
    const fn needs(self) -> Needs {
        match self.operation {
            PathOperation::Split => Needs::SOURCE_ENCODING.union(Needs::LIST_RENDERING),
            _ => Needs::SOURCE_ENCODING,
        }
    }

    fn evaluate_path(self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        // Operand 0 is the subcommand word.
        let count = input.invocation().operands.len();
        let fits = match self.operation {
            PathOperation::Join => count >= 2,
            _ => count == 2,
        };
        if !fits {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let values = match exact_operands(input, 1..count) {
            Ok(values) => values,
            Err(answer) => return answer,
        };
        let mut ops = match ConstOps::admit(input.context(), budget, self.needs()) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target = *ops.target();
        let mut names = Vec::with_capacity(values.len());
        for value in &values {
            let name = match ops.admissible_text(value) {
                Ok(name) => name,
                Err(reason) => return EvalAnswer::Declined(reason),
            };
            if !reads_alike_everywhere(&name) {
                return EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(Axis::Platform));
            }
            names.push(name);
        }
        let bytes = names.iter().map(|name| name.len()).sum::<usize>();
        if let Err(reason) = ops.charge(u64::try_from(bytes).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let parts: Vec<&[u8]> = names.iter().map(|name| name.as_bytes()).collect();
        let name = || parts[0];
        let value = match self.operation {
            PathOperation::Join => text(&tcl_cmd_core::path::join(&parts)),
            PathOperation::Dirname => text(&tcl_cmd_core::path::dirname(name())),
            PathOperation::Tail => text(tcl_cmd_core::path::tail(name())),
            PathOperation::Extension => text(tcl_cmd_core::path::extension(name())),
            PathOperation::Rootname => text(tcl_cmd_core::path::rootname(name())),
            PathOperation::Split => {
                let elements = tcl_cmd_core::path::split(name())
                    .into_iter()
                    .map(text)
                    .collect();
                ops.new_list(elements)
            }
        };
        match ops.take(value) {
            Ok(value) => pure_outcome(
                self.evaluator(),
                REVISION,
                value,
                self.result_type(),
                DependencyEvidence {
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Err(reason) => EvalAnswer::Declined(reason),
        }
    }
}

/// A name the cores returned, as a value: a slice of ASCII or UTF-8 names
/// cut at `/` and `.`, so still text.
fn text(bytes: &[u8]) -> ConstValue {
    ConstValue::text(std::str::from_utf8(bytes).unwrap_or_default())
}

impl CommandSemantics for PathSemantics {
    fn identity(&self) -> &'static str {
        self.evaluator().as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: self.evaluator(),
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, self.result_type())
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        self.evaluate_path(input, budget)
    }
}
