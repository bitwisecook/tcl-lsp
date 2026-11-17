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

//! Leaving a spliced procedure body for its call ([`tcl_bytecode::SpliceRange`]).

use super::{Code, Completion, Frame, Value, Vm};

/// A splice range's instruction indexes: `[start, end)`, `end` the fallback.
#[derive(Clone, Copy)]
struct SpliceBounds {
    start: usize,
    end: usize,
    entry_depth: usize,
}

fn instruction_index(f: &Frame, label: &str) -> Option<usize> {
    let offset = *f.asm.labels.get(label)?;
    let offset = i32::try_from(offset).ok()?;
    f.off2idx.get(&offset).copied()
}

/// The innermost splice range holding instruction `index`.
fn innermost_splice(f: &Frame, index: usize) -> Option<SpliceBounds> {
    f.asm
        .splice_ranges
        .iter()
        .filter_map(|range| {
            let start = instruction_index(f, &range.start)?;
            let end = instruction_index(f, &range.end)?;
            (start <= index && index < end).then_some(SpliceBounds {
                start,
                end,
                entry_depth: usize::try_from(range.entry_depth).ok()?,
            })
        })
        .min_by_key(|bounds| bounds.end - bounds.start)
}

impl Vm {
    /// A completion other than OK that instruction `pc - 1` of `f` raised inside a spliced body leaves it for the fallback.
    pub(super) fn leaves_splice_range(&mut self, f: &mut Frame, c: &Completion<Value>) -> bool {
        c.code != Code::Ok
            && !self.exit_pending()
            && self.execution_refusal.is_none()
            && self.exit_splice_range(f)
    }

    /// Leave the spliced body that instruction `pc - 1` failed in: drop what the
    /// body built, forget its error, and resume at the fallback invocation,
    /// which runs the call as the uninlined procedure would. A catch range the
    /// body itself opened around the failure keeps it.
    pub(super) fn exit_splice_range(&mut self, f: &mut Frame) -> bool {
        if f.asm.splice_ranges.is_empty() {
            return false;
        }
        let Some(fault) = f.pc.checked_sub(1) else {
            return false;
        };
        let Some(bounds) = innermost_splice(f, fault) else {
            return false;
        };
        let inside = |index: usize| bounds.start <= index && index < bounds.end;
        if f
            .catch_ranges
            .iter()
            .any(|range| !range.in_handler && range.start_idx <= fault && fault < range.end_idx && inside(range.begin_idx))
        {
            return false;
        }
        self.leave_splice_range(f, bounds);
        true
    }

    /// Leave the spliced body holding `f.pc` before it runs that instruction,
    /// as a guard the body was compiled under no longer holds.
    pub(super) fn leave_stale_splice_range(&mut self, f: &mut Frame) -> bool {
        if f.asm.splice_ranges.is_empty() {
            return false;
        }
        let Some(bounds) = innermost_splice(f, f.pc) else {
            return false;
        };
        self.leave_splice_range(f, bounds);
        true
    }

    fn leave_splice_range(&mut self, f: &mut Frame, bounds: SpliceBounds) {
        let inside = |index: usize| bounds.start <= index && index < bounds.end;
        f.catch_ranges.retain(|range| !inside(range.begin_idx));
        f.entered_commands.retain(|entered| !inside(entered.resume));
        f.entered_native_compiler_selections
            .retain(|&(start, _)| !inside(start));
        f.entered_native_operations
            .retain(|entered| !inside(entered.start));
        f.foreach_stack.retain(|state| !inside(state.body_idx));
        f.expand_markers.retain(|&depth| depth <= bounds.entry_depth);
        f.stack.truncate(bounds.entry_depth);
        f.failed_operands = None;
        self.abandon_error_episode();
        f.pc = bounds.end;
    }
}
