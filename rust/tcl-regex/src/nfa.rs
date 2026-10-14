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

//! The leftmost-start scan: a flat instruction program compiled from the
//! [`Node`] tree of a regular pattern (no backreference, no lookahead), run
//! Pike-style over the subject once to find where the leftmost match starts —
//! or that none does — in time proportional to the subject's length times
//! the program's. [`crate::exec::Matcher::search`] then computes extent and
//! captures from that start alone, instead of re-running its reachable-set
//! simulation from every earlier start, which is cubic on `(a+)+b` over a run
//! of `a`s with no `b`.

use crate::ast::{Anchor, CharSet, Node};
use crate::defs::DUPINF;

/// The most instructions a program may hold: a pattern whose bounded repeats
/// unroll past this is searched without the scan.
const MAX_PROGRAM: usize = 20_000;

/// One instruction.
#[derive(Clone, Copy)]
pub(crate) enum Inst<'n> {
    /// Consume one character in the set.
    Set(&'n CharSet),
    /// Continue only where the zero-width assertion holds.
    Assert(Anchor),
    /// Continue at both targets.
    Split(usize, usize),
    /// Continue at the target.
    Jump(usize),
    /// A match ends here.
    Match,
}

/// A compiled program; execution starts at instruction 0.
pub(crate) struct Program<'n> {
    pub(crate) insts: Vec<Inst<'n>>,
}

impl<'n> Program<'n> {
    /// The program for `root`, or `None` for a pattern the scan cannot run:
    /// one with a backreference or a lookahead, or too large once unrolled.
    pub(crate) fn compile(root: &'n Node) -> Option<Self> {
        let mut program = Self { insts: Vec::new() };
        program.emit(root)?;
        program.push(Inst::Match)?;
        Some(program)
    }

    fn push(&mut self, inst: Inst<'n>) -> Option<usize> {
        if self.insts.len() >= MAX_PROGRAM {
            return None;
        }
        self.insts.push(inst);
        Some(self.insts.len() - 1)
    }

    fn emit(&mut self, node: &'n Node) -> Option<()> {
        match node {
            Node::Empty => {}
            Node::Set(set) => {
                self.push(Inst::Set(set))?;
            }
            Node::Anchor(anchor) => {
                self.push(Inst::Assert(*anchor))?;
            }
            Node::Look { .. } | Node::Backref { .. } => return None,
            Node::Capture { sub, .. } => self.emit(sub)?,
            Node::Concat(items) => {
                for item in items {
                    self.emit(item)?;
                }
            }
            Node::Alt(branches) => {
                let mut exits = Vec::new();
                for (i, branch) in branches.iter().enumerate() {
                    if i + 1 == branches.len() {
                        self.emit(branch)?;
                    } else {
                        let split = self.push(Inst::Split(0, 0))?;
                        self.emit(branch)?;
                        exits.push(self.push(Inst::Jump(0))?);
                        let next = self.insts.len();
                        self.insts[split] = Inst::Split(split + 1, next);
                    }
                }
                let end = self.insts.len();
                for exit in exits {
                    self.insts[exit] = Inst::Jump(end);
                }
            }
            Node::Repeat { sub, min, max, .. } => {
                for _ in 0..*min {
                    self.emit(sub)?;
                }
                if *max >= DUPINF {
                    let split = self.push(Inst::Split(0, 0))?;
                    self.emit(sub)?;
                    self.push(Inst::Jump(split))?;
                    let end = self.insts.len();
                    self.insts[split] = Inst::Split(split + 1, end);
                } else {
                    let mut splits = Vec::new();
                    for _ in (*min).max(0)..*max {
                        splits.push(self.push(Inst::Split(0, 0))?);
                        self.emit(sub)?;
                    }
                    let end = self.insts.len();
                    for split in splits {
                        self.insts[split] = Inst::Split(split + 1, end);
                    }
                }
            }
        }
        Some(())
    }
}
