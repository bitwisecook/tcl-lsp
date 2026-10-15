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

//! Running bytecode as a release that keeps the commands it compiled does
//! (`TclVersion::revalidates_running_bytecode`): 8.4 checks its compile epoch
//! only when bytecode is entered, so a command-table change reaches a running
//! unit only through the commands it looks up as they run.
//!
//! The compiler says which unit the release compiled each instruction in
//! ([`tcl_bytecode::Instruction::release_unit`]); an activation records when
//! it entered each one ([`ReleaseUnits`]), and the VM keeps the builtins a
//! change replaced ([`ReplacedBuiltins`]) so an invocation the release
//! compiled still runs the command the unit was entered with.

use tcl_bytecode::Instruction;
#[cfg(test)]
use tcl_bytecode::ReleaseBodyEntry;
#[cfg(test)]
use tcl_runtime_api::ReleaseBinding;

use crate::command::BuiltinFn;

/// When an activation entered a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitEntry {
    /// The compilation-deopt epoch at entry.
    pub(crate) epoch: u64,
    /// Whether the activation's command bindings no longer held at entry, so
    /// that the release compiled the unit against the changed table.
    pub(crate) stale: bool,
}

/// A unit an activation runs in that another activation entered: the body of
/// a command the release compiled into that activation's unit, which this VM
/// runs as an activation of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InheritedUnit {
    /// When the unit was entered.
    pub(crate) entry: UnitEntry,
    /// Whether the release compiled the unit as a procedure body.
    pub(crate) in_procedure: bool,
}

/// The units an activation's instructions run in and when it entered each.
///
/// The activation's own unit is unit `0`. Each script body the compiler
/// inlined ([`Instruction::release_body`]) runs in the unit enclosing it where
/// the release compiles the body's owner into that unit, and otherwise in a
/// unit of its own, numbered as the body and entered afresh at each of the
/// body's entries. Only the activation's own unit can be a procedure body.
#[derive(Debug, Default)]
pub(crate) struct ReleaseUnits {
    /// Body → the unit it runs in, as resolved at its latest entry.
    bodies: Vec<Option<u32>>,
    /// Unit → when the activation last entered it.
    entries: Vec<Option<UnitEntry>>,
    /// Whether the activation's own unit is a procedure body.
    in_procedure: bool,
    /// The unit the activation inherited from the invocation that runs it.
    inherited: Option<InheritedUnit>,
}

impl ReleaseUnits {
    /// The units of an activation whose own unit is a procedure body when
    /// `in_procedure` says so.
    pub(crate) fn new(in_procedure: bool) -> Self {
        Self {
            in_procedure,
            ..Self::default()
        }
    }

    /// The unit `instruction` runs in.
    pub(crate) fn unit_of(&self, instruction: &Instruction) -> u32 {
        self.body_unit(instruction.release_body)
    }

    fn body_unit(&self, body: u32) -> u32 {
        Self::slot(&self.bodies, body).unwrap_or(0)
    }

    fn slot<T: Copy>(slots: &[Option<T>], index: u32) -> Option<T> {
        slots.get(usize::try_from(index).ok()?).copied().flatten()
    }

    fn set<T: Copy>(slots: &mut Vec<Option<T>>, index: u32, value: T) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        if slots.len() <= index {
            slots.resize(index + 1, None);
        }
        slots[index] = Some(value);
    }

    /// Whether the release compiled `unit` as a procedure body.
    pub(crate) fn in_procedure(&self, unit: u32) -> bool {
        unit == 0 && self.in_procedure
    }

    /// When the activation last entered `unit`.
    pub(crate) fn entry(&self, unit: u32) -> Option<UnitEntry> {
        Self::slot(&self.entries, unit)
    }

    /// The unit `instruction` runs in, as an activation running the body of
    /// an invocation in it inherits it.
    pub(crate) fn current(&self, instruction: &Instruction) -> Option<InheritedUnit> {
        let unit = self.unit_of(instruction);
        Some(InheritedUnit {
            entry: self.entry(unit)?,
            in_procedure: self.in_procedure(unit),
        })
    }

    /// Whether running `instruction` enters a unit: it is the activation's
    /// first, or starts a body.
    pub(crate) fn enters(&self, instruction: &Instruction) -> bool {
        instruction.release_body_entry.is_some() || self.entry(0).is_none()
    }

    /// Run `instruction`, which [enters](Self::enters) a unit at `entry`.
    pub(crate) fn enter(&mut self, instruction: &Instruction, entry: UnitEntry) {
        if self.entry(0).is_none() {
            Self::set(&mut self.entries, 0, entry);
        }
        let Some(start) = instruction.release_body_entry else {
            return;
        };
        let enclosing = self.body_unit(start.parent);
        let body = instruction.release_body;
        if start.owner.binds(self.in_procedure(enclosing)) {
            Self::set(&mut self.bodies, body, enclosing);
        } else {
            Self::set(&mut self.bodies, body, body);
            Self::set(&mut self.entries, body, entry);
        }
    }

    /// Run in `unit` as the activation's own unit.
    pub(crate) fn inherit(&mut self, unit: InheritedUnit) {
        Self::set(&mut self.entries, 0, unit.entry);
        self.in_procedure = unit.in_procedure;
        self.inherited = Some(unit);
    }

    /// The unit the activation inherited, which the bodies a driver
    /// activation runs inherit in turn.
    pub(crate) fn inherited(&self) -> Option<InheritedUnit> {
        self.inherited
    }
}

/// One builtin a command-table change removed from its key.
#[derive(Debug, Clone)]
struct Replaced {
    epoch: u64,
    key: String,
    builtin: BuiltinFn,
}

/// The builtins command-table changes removed, each with the epoch the change
/// was made at.
#[derive(Debug, Default)]
pub(crate) struct ReplacedBuiltins {
    log: Vec<Replaced>,
}

impl ReplacedBuiltins {
    /// Record that a change at `epoch` removes `builtin` from `key`.
    pub(crate) fn record(&mut self, epoch: u64, key: &str, builtin: BuiltinFn) {
        self.log.push(Replaced {
            epoch,
            key: key.to_owned(),
            builtin,
        });
    }

    /// The builtin `key` held when a unit was entered at `entered`, where a
    /// change since has removed it.
    pub(crate) fn at_entry(&self, key: &str, entered: u64) -> Option<BuiltinFn> {
        self.log
            .iter()
            .find(|replaced| replaced.epoch >= entered && replaced.key == key)
            .map(|replaced| replaced.builtin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::{Vm, ok};
    use crate::value::Value;
    use tcl_runtime_api::Completion;

    fn first(_: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::string("first"))
    }

    fn second(_: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::string("second"))
    }

    fn body(body: u32, parent: u32, owner: ReleaseBinding) -> Instruction {
        let mut instruction = Instruction::new(tcl_bytecode::Op::NOP, Vec::new());
        instruction.release_body = body;
        instruction.release_body_entry = Some(ReleaseBodyEntry { parent, owner });
        instruction
    }

    fn at(epoch: u64) -> UnitEntry {
        UnitEntry {
            epoch,
            stale: false,
        }
    }

    #[test]
    fn a_body_runs_in_its_own_unit_unless_its_owner_is_compiled_into_the_enclosing_one() {
        let mut units = ReleaseUnits::new(true);
        let first = Instruction::new(tcl_bytecode::Op::NOP, Vec::new());
        assert!(units.enters(&first));
        units.enter(&first, at(1));
        assert!(!units.enters(&first));

        // A `foreach` in a procedure body is compiled into it, and so is a
        // `while` in that body.
        let loop_body = body(1, 0, ReleaseBinding::CompiledInProcedure);
        units.enter(&loop_body, at(2));
        assert_eq!(units.unit_of(&loop_body), 0);
        assert_eq!(units.entry(0), Some(at(1)));

        // A `switch` arm is a unit of its own, where a `foreach` is not
        // compiled, so its body is a unit of its own too.
        let arm = body(2, 1, ReleaseBinding::Invoked);
        units.enter(&arm, at(3));
        assert_eq!(units.unit_of(&arm), 2);
        assert!(!units.in_procedure(2));
        let nested = body(3, 2, ReleaseBinding::CompiledInProcedure);
        units.enter(&nested, at(4));
        assert_eq!(units.unit_of(&nested), 3);
        assert_eq!(units.entry(3), Some(at(4)));
        assert_eq!(units.entry(2), Some(at(3)));
    }

    #[test]
    fn an_inherited_unit_decides_its_bodies() {
        let mut units = ReleaseUnits::new(false);
        units.inherit(InheritedUnit {
            entry: at(5),
            in_procedure: true,
        });
        let loop_body = body(1, 0, ReleaseBinding::CompiledInProcedure);
        units.enter(&loop_body, at(6));
        assert_eq!(units.unit_of(&loop_body), 0);
        assert_eq!(units.entry(0), Some(at(5)));
        assert!(units.inherited().is_some());
    }

    #[test]
    fn the_builtin_at_entry_is_the_first_one_removed_since() {
        let mut replaced = ReplacedBuiltins::default();
        replaced.record(3, "expr", first as BuiltinFn);
        replaced.record(7, "expr", second as BuiltinFn);
        let at = |entered| {
            replaced
                .at_entry("expr", entered)
                .map(|builtin| builtin as usize)
        };
        assert_eq!(at(1), Some(first as BuiltinFn as usize));
        assert_eq!(at(4), Some(second as BuiltinFn as usize));
        assert_eq!(at(8), None);
        assert!(replaced.at_entry("set", 0).is_none());
    }
}
