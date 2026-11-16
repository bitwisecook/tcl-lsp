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

//! One array located for an `array` ensemble operation: its default state, trace-aware element reads and invalidation.

use super::native_execution_error::NativeExecutionError;
use tcl_core_types::{FrameId, VarId};

/// Actual C Tcl 9 array-default command recipe, without issuer authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArrayDefaultProtocol {
    /// TIP 508 storage and operation order in the native Tcl 9 command.
    Tcl9,
}

/// State of the exact variable selected before an array-operation callback.
#[derive(Debug, Clone)]
pub enum ArrayDefaultState<V> {
    /// No selected variable, or its current contents are undefined.
    Undefined,
    /// The selected variable currently contains a scalar or an array element.
    NonArray,
    /// The selected array and its original default object, when defined.
    Array {
        /// Original object retained by the selected physical array cell.
        default: Option<V>,
    },
}

/// Guest failure after the array-default setter's fresh creating lookup.
#[derive(Debug, Clone)]
pub enum ArrayDefaultSetFailure {
    /// The creating lookup selected an array element rather than an array root.
    Element,
    /// The selected root is a defined scalar.
    Scalar,
    /// Native variable lookup failed before either storage classification.
    Lookup(tcl_syntax::naming::NativeVariableDiagnosticProjection),
}

/// One array name located for the duration of an `array` ensemble operation.
///
/// Tcl's `LocateArray` resolves the spelling once before firing an `array`
/// trace. Enumeration continues against that exact cell even when the callback
/// retargets an `upvar` alias; runtimes without stable variable identities use
/// the name-addressed form until they acquire that capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayTarget {
    pub(super) frame: FrameId,
    pub(super) name: tcl_core_types::NameBytes,
    pub(super) cell: Option<VarId>,
}

/// Result of one trace-aware element read performed for `array get`.
///
/// The command snapshots keys from [`ArrayTarget`] but reads each candidate
/// through the source spelling. A callback can therefore make the element
/// disappear without invalidating the walk, or destroy/retype the captured
/// array, which is a command error. Keeping that distinction in the runtime
/// contract prevents the shared command core from guessing storage identity.
#[derive(Debug, Clone)]
pub enum ArrayElementRead<V> {
    /// Actual first host cause; no guest trace/missing-element semantics apply.
    HostRefusal(NativeExecutionError),
    /// The selected element still has a value after its read traces. Pointer
    /// runtimes return this with one transient ownership hold already acquired;
    /// the shared command core releases it after list materialisation.
    Value(V),
    /// The element is absent, or its read trace errored. Tcl skips it while the
    /// captured base remains an array; the runtime retains any swallowed trace
    /// completion in its interpreter error state.
    Missing(ArrayReadMiss),
    /// A read trace failed and invalidated the captured base in the same
    /// callback. Unlike an ordinary trace miss, Tcl propagates the callback's
    /// contextual variable-read error rather than replacing it with a generic
    /// array invalidation diagnostic.
    TraceError(ArrayReadFailure),
    /// The array cell captured for the operation is no longer an array.
    ArrayInvalidated(ArrayInvalidation),
}

/// A variable-read trace failure which an `array get` candidate must
/// propagate because that same callback invalidated the captured array.
///
/// The message is already the variable subsystem's contextual `can't read
/// "a(k)": reason` result. The remaining fields preserve the callback's
/// accumulated error trace across the shared command/runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayReadFailure {
    pub(super) message: Vec<u8>,
    pub(super) code: Vec<u8>,
    pub(super) info: Option<Vec<u8>>,
    pub(super) line: Option<i64>,
}

impl ArrayReadFailure {
    /// Construct a hard traced-read failure.
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        code: Vec<u8>,
        info: Option<Vec<u8>>,
        line: Option<i64>,
    ) -> Self {
        Self::new_bytes(message.into().into_bytes(), code, info, line)
    }

    /// Construct a traced-read failure without projecting its native bytes.
    #[must_use]
    pub fn new_bytes(
        message: Vec<u8>,
        code: Vec<u8>,
        info: Option<Vec<u8>>,
        line: Option<i64>,
    ) -> Self {
        Self {
            message,
            code,
            info,
            line,
        }
    }

    /// Consume the failure into its command-error parts.
    #[must_use]
    pub fn into_parts(self) -> (Vec<u8>, Vec<u8>, Option<Vec<u8>>, Option<i64>) {
        (self.message, self.code, self.info, self.line)
    }
}

/// Completion metadata retained when `array get` swallows one candidate's
/// failed variable read.
///
/// Tcl exposes the variable subsystem's read classification on the surrounding
/// successful completion (`TCL READ VARNAME`, or `TCL LOOKUP VARNAME name`
/// when an operation trace retargeted the source alias away from an array). A
/// callback error also contributes its accumulated trace and source line; a
/// callback that merely removes the element contributes no error trace and
/// does not publish the error globals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayReadMiss {
    pub(super) code: Vec<u8>,
    pub(super) info: Option<Vec<u8>>,
    pub(super) line: Option<i64>,
}

impl Default for ArrayReadMiss {
    fn default() -> Self {
        Self::missing()
    }
}

impl ArrayReadMiss {
    /// A read that found no value after otherwise-successful callbacks.
    #[must_use]
    pub fn missing() -> Self {
        Self {
            code: b"TCL READ VARNAME".to_vec(),
            info: None,
            line: None,
        }
    }

    /// A source-spelling lookup that no longer resolves to an array.
    #[must_use]
    pub fn lookup(error_code: Vec<u8>) -> Self {
        Self {
            code: error_code,
            info: None,
            line: None,
        }
    }

    /// A read whose trace callback errored before `array get` swallowed it.
    #[must_use]
    pub fn trace_error(error_info: Option<Vec<u8>>, error_line: i64) -> Self {
        Self {
            code: b"TCL READ VARNAME".to_vec(),
            info: error_info,
            line: Some(error_line),
        }
    }

    /// Merge a later candidate miss into this operation's retained metadata.
    ///
    /// Tcl exposes the most recent read classification, but an ordinary later
    /// miss has no callback trace of its own and must not erase the earlier
    /// callback's `errorInfo` or source line.
    #[must_use]
    pub fn followed_by(self, later: Self) -> Self {
        Self {
            code: later.code,
            info: later.info.or(self.info),
            line: later.line.or(self.line),
        }
    }
}

/// Why a captured array can no longer supply a candidate element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayInvalidation {
    /// The captured variable is now undefined.
    Unset,
    /// The captured variable is now a defined non-array.
    Retyped,
}

impl ArrayTarget {
    /// Construct a name-addressed target.
    #[must_use]
    pub fn named(frame: FrameId, name: impl Into<String>) -> Self {
        Self {
            frame,
            name: tcl_core_types::NameBytes::from(name.into()),
            cell: None,
        }
    }

    /// Construct a target backed by one stable variable cell.
    #[must_use]
    pub fn cell(frame: FrameId, name: impl Into<String>, cell: VarId) -> Self {
        Self {
            frame,
            name: tcl_core_types::NameBytes::from(name.into()),
            cell: Some(cell),
        }
    }

    /// The frame in which the source spelling was resolved.
    #[must_use]
    pub const fn frame(&self) -> FrameId {
        self.frame
    }

    /// The source spelling used for paths that Tcl deliberately re-resolves.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
            .try_utf8()
            .expect("byte-valued array target requires name_bytes")
    }

    /// Construct an exact name-addressed target without decoding its operand.
    #[must_use]
    pub fn named_bytes(frame: FrameId, name: impl Into<tcl_core_types::NameBytes>) -> Self {
        Self {
            frame,
            name: name.into(),
            cell: None,
        }
    }

    /// Construct an exact stable-cell target retaining the original spelling.
    #[must_use]
    pub fn cell_bytes(
        frame: FrameId,
        name: impl Into<tcl_core_types::NameBytes>,
        cell: VarId,
    ) -> Self {
        Self {
            frame,
            name: name.into(),
            cell: Some(cell),
        }
    }

    /// Exact source spelling; it does not infer or authenticate the cell.
    #[must_use]
    pub fn name_bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }

    /// The stable reached cell, when the runtime provides one.
    #[must_use]
    pub const fn cell_id(&self) -> Option<VarId> {
        self.cell
    }
}
