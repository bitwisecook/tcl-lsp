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

//! Scheduled execution outcomes. Executable activation capabilities are minted
//! and validated by each concrete runtime, never constructed from frame indices.

use crate::{Completion, NativeExecutionError};

/// Why a retained activation cannot execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetainedActivationRefusal {
    /// The capability belongs to another runtime instance.
    ForeignRuntime,
    /// Its interpreter incarnation was deleted.
    RetiredInterpreter,
    /// Its original physical call frame no longer exists.
    RetiredFrame,
    /// The original rule source was unloaded or replaced.
    RetiredRule,
    /// Its coroutine is currently owned by another active operation.
    BusyActivation,
    /// Its retained source or invocation policies no longer match.
    ChangedPolicy,
    /// No explicit simulator activation provider supplies this context.
    UnsupportedContext,
}

/// A callback outcome preserves guest completion and host refusal separately.
#[derive(Debug)]
pub enum ScheduledCallbackOutcome<V> {
    /// The callback executed and returned its complete guest options.
    Guest(Completion<V>),
    /// The execution engine refused before a guest completion could be made.
    Host(NativeExecutionError),
    /// The original activation cannot be entered; no callback code ran.
    Activation(RetainedActivationRefusal),
}

/// One deterministic scheduler dispatch.
#[derive(Debug)]
pub struct ScheduledCallbackReport<V> {
    /// Never-reused timer identity within its owning runtime.
    pub id: u64,
    /// Original event metadata, without a physical frame grant.
    pub event: V,
    /// Original logical rule identity.
    pub rule: V,
    /// Complete execution outcome.
    pub outcome: ScheduledCallbackOutcome<V>,
}
