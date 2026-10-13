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

//! The registry's extension host (`tcl_registry::extension_host`) on the WASM
//! runtime: each evaluation a fresh instance with the extension loaded into it,
//! the load under the host's budget and the evaluation under the host's
//! budget narrowed by its own.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tcl_engine_api::{Budget, BudgetKind, EngineError};
use tcl_registry::extension_host::{ExtensionHost, LoadedExtension};
use tcl_registry::value_transfer::{BudgetLimit, DeclineReason, ImplementationBudget};

use crate::session::{Failure, Session};
use crate::{Extension, WasmRuntime, limits_of};

/// The registry's extension host on the WASM runtime.
pub struct WasmExtensionHost {
    runtime: WasmRuntime,
    budget: Budget,
    extensions: RefCell<HashMap<u64, Extension>>,
}

impl WasmExtensionHost {
    /// A host over `runtime` whose evaluations run under its default budget: a
    /// hundred thousand commands, two seconds, and 16 MiB in one value.
    #[must_use]
    pub fn new(runtime: &WasmRuntime) -> Self {
        Self::with_budget(
            runtime,
            Budget::of_commands(100_000)
                .with_wall_clock(Duration::from_secs(2))
                .with_max_value_bytes(16 * 1024 * 1024),
        )
    }

    /// A host whose loads run under `budget`, and its evaluations under
    /// `budget` narrowed by each one's own.
    #[must_use]
    pub fn with_budget(runtime: &WasmRuntime, budget: Budget) -> Self {
        Self {
            runtime: runtime.clone(),
            budget,
            extensions: RefCell::new(HashMap::new()),
        }
    }

    /// A fresh instance armed with the host's own budget, the first-use fuel
    /// included.
    fn session(&self) -> Result<Session, DeclineReason> {
        let mut session = Session::new(&self.runtime).map_err(|_| DeclineReason::Transient)?;
        session
            .arm(limits_of(self.budget).with_first_use())
            .map_err(decline)?;
        Ok(session)
    }
}

impl ExtensionHost for WasmExtensionHost {
    fn load(&self, artefact: &[u8], prefix: &str) -> Result<LoadedExtension, DeclineReason> {
        let extension = self
            .runtime
            .extension(artefact, prefix)
            .map_err(|_| DeclineReason::Unsupported)?;
        let mut session = self.session()?;
        let (before_interpreter, before) = command_inventory(&mut session)?;
        session.load(&extension).map_err(decline)?;
        let (after_interpreter, after) = command_inventory(&mut session)?;
        if before_interpreter != after_interpreter {
            return Err(DeclineReason::Unsupported);
        }
        let retained: HashSet<_> = before
            .into_iter()
            .map(|(row, _)| (row.namespace, row.generation, row.slot))
            .collect();
        let mut commands = Vec::new();
        for (row, label) in after {
            if !retained.contains(&(row.namespace, row.generation, row.slot)) {
                commands.push(label);
            }
        }
        commands.sort();
        let hash = extension.hash();
        self.extensions.borrow_mut().insert(hash, extension);
        Ok(LoadedExtension {
            hash,
            prefix: prefix.to_owned(),
            commands,
        })
    }

    fn evaluate(
        &self,
        hash: u64,
        words: &[String],
        budget: &ImplementationBudget,
    ) -> Result<String, DeclineReason> {
        let extension = self
            .extensions
            .borrow()
            .get(&hash)
            .cloned()
            .ok_or(DeclineReason::Transient)?;
        let mut session = self.session()?;
        session.load(&extension).map_err(decline)?;
        let budget = narrowed(self.budget, *budget);
        // Each evaluation is a fresh instance's first, so it has the first-use
        // fuel. It has no whitelist and no confinement: the extension reaches the
        // interpreter only through the C API, which declares no eval or variable
        // door (and the loader refuses the runtime's other exports), and the
        // instance is this evaluation's alone.
        session
            .arm(limits_of(budget).with_first_use())
            .map_err(decline)?;
        let words: Vec<&[u8]> = words.iter().map(String::as_bytes).collect();
        let completion = session.evaluate(&words).map_err(decline)?;
        if let Some(kind) = session.exceeded().map_err(decline)? {
            return Err(DeclineReason::Budget(limit_of(kind)));
        }
        if completion.code != 0 {
            // An error, or any other completion, is never a value.
            return Err(DeclineReason::Unsupported);
        }
        let over = budget.max_value_bytes.is_some_and(|bytes| {
            u64::try_from(completion.result.len()).is_ok_and(|length| length > bytes)
        });
        if over {
            return Err(DeclineReason::Budget(BudgetLimit::ResultBytes));
        }
        String::from_utf8(completion.result).map_err(|_| DeclineReason::NotText)
    }
}

/// Actual visible rows with independently checked root-context command operands.
fn command_inventory(
    session: &mut Session,
) -> Result<
    (
        tcl_engine_api::engine_interpreter_identity::EngineInterpreterIdentity,
        Vec<(
            tcl_runtime_api::command_inventory::EngineCommandInventoryRow,
            String,
        )>,
    ),
    DeclineReason,
> {
    let (interpreter, inventory) = session.command_inventory().map_err(decline_engine)?;
    let protocol = tcl_syntax::naming::NativeNameProtocol::for_tcl_version(inventory.version);
    let mut rows = Vec::with_capacity(inventory.rows.len());
    for row in inventory.rows {
        tcl_syntax::naming::checked_command_slot_utf8(&row.slot)
            .map_err(|_| DeclineReason::NotText)?;
        let operand = row
            .root_operand
            .as_deref()
            .ok_or(DeclineReason::Unsupported)?;
        let label = tcl_syntax::naming::native_command_root_source_spelling(protocol, &row.slot)
            .filter(|label| label.as_bytes() == operand)
            .ok_or(DeclineReason::Unsupported)?;
        rows.push((row, label));
    }
    Ok((interpreter, rows))
}

/// The host's budget with an evaluation's own narrowing it: each field the
/// evaluation names is capped at the host's.
fn narrowed(host: Budget, declared: ImplementationBudget) -> Budget {
    fn cap<T: Ord + Copy>(host: Option<T>, declared: Option<T>) -> Option<T> {
        match (host, declared) {
            (Some(host), Some(declared)) => Some(host.min(declared)),
            (host, None) => host,
            (None, declared) => declared,
        }
    }
    Budget {
        commands: cap(host.commands, declared.commands),
        wall_clock: cap(
            host.wall_clock,
            declared.wall_clock_ms.map(Duration::from_millis),
        ),
        max_value_bytes: cap(host.max_value_bytes, declared.value_bytes),
    }
}

/// The decline a session failure is: a budget it outran, or no value — an
/// extension that cannot be linked, an entry point or a command that failed,
/// a fault — which the same words meet again.
fn decline(failure: Failure) -> DeclineReason {
    decline_engine(failure.into_engine_error())
}

fn decline_engine(error: EngineError) -> DeclineReason {
    match error {
        EngineError::BudgetExceeded(kind) => DeclineReason::Budget(limit_of(kind)),
        _ => DeclineReason::Unsupported,
    }
}

/// The registry's name for a budget an evaluation outran: the command count
/// and the fuel that stands in for it are the evaluation's fuel, the wall
/// clock the request's time, the value size the allocation bound.
fn limit_of(kind: BudgetKind) -> BudgetLimit {
    match kind {
        BudgetKind::Commands => BudgetLimit::Fuel,
        BudgetKind::WallClock => BudgetLimit::Request,
        BudgetKind::ValueSize => BudgetLimit::AllocationBytes,
    }
}

#[cfg(test)]
#[path = "extension_inventory_tests.rs"]
mod inventory_tests;
