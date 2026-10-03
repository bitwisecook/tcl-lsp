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

//! The limits an embedder sets on an evaluation when it hosts this interpreter
//! as an engine: how many commands the evaluation may dispatch, how long it may
//! run, and how many bytes one value it builds may take.
//!
//! The interpreter enforces each itself, at the points every evaluation passes
//! through — command dispatch, the loop commands' poll, the allocation `string
//! repeat` asks for — so the limits hold however the interpreter is hosted:
//! natively behind the extension interface (`crate::engine`), or compiled to
//! `wasm32` under a host that adds fuel for what no command count sees.
//!
//! A limit once exceeded stays exceeded until the next evaluation begins: every
//! command dispatched after it fails with the same error, so a body that
//! catches the error cannot run on past its budget, and the embedder reads
//! which limit it was from [`Interp::limit_exceeded`] whatever the body did
//! with the error.

use std::time::Duration;

use crate::interp::{Code, Interp};

/// Which limit an evaluation outran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// The command count.
    Commands,
    /// The wall clock.
    WallClock,
    /// The size of a single value.
    ValueSize,
}

impl LimitKind {
    /// The error an evaluation that outran this limit fails with: the message
    /// C Tcl's `interp limit` gives for the first two, and the bytecode VM's
    /// for all three, so an embedder reads one vocabulary from either engine.
    #[must_use]
    pub const fn message(self) -> &'static [u8] {
        match self {
            Self::Commands => b"command count limit exceeded",
            Self::WallClock => b"time limit exceeded",
            Self::ValueSize => b"value size limit exceeded",
        }
    }
}

/// How many dispatches pass between two reads of the clock: a read costs more
/// than the dispatch it would guard, and a deadline is honoured to within the
/// time these take.
const DISPATCHES_PER_CLOCK_READ: u32 = 64;

/// How many iterations of a loop that dispatches nothing pass between two reads
/// of the clock — the throttle `interp limit time` uses. The loop commands need
/// the numeric tower, so without it there is no loop to charge.
#[cfg(have_tommath)]
const TICKS_PER_CLOCK_READ: u32 = 4096;

/// The limits, and where the running evaluation stands against them.
#[derive(Debug, Default)]
pub(crate) struct Budget {
    commands: Option<u64>,
    wall_clock: Option<Duration>,
    value_bytes: Option<u64>,
    /// The interpreter's dispatch count when the running evaluation began.
    start: u64,
    /// The host clock's millisecond reading the evaluation must finish by.
    deadline: Option<i128>,
    /// The limit the running evaluation outran, kept until the next begins.
    exceeded: Option<LimitKind>,
    /// Charges since the clock was last read.
    since_clock: u32,
}

impl Budget {
    /// Whether anything is armed, so dispatch pays one test when nothing is.
    fn armed(&self) -> bool {
        self.commands.is_some() || self.deadline.is_some() || self.exceeded.is_some()
    }
}

impl Interp {
    /// Set the limits every evaluation begun with [`Self::begin_evaluation`]
    /// runs under: at most `commands` dispatched commands, `wall_clock` of
    /// time, and `value_bytes` bytes in any one value `string repeat` builds.
    /// `None` leaves that limit off.
    pub fn set_limits(
        &self,
        commands: Option<u64>,
        wall_clock: Option<Duration>,
        value_bytes: Option<u64>,
    ) {
        let mut budget = self.budget.borrow_mut();
        budget.commands = commands;
        budget.wall_clock = wall_clock;
        budget.value_bytes = value_bytes;
    }

    /// Begin an evaluation under the limits: its command count starts here, its
    /// deadline is set from now, and a limit an earlier evaluation outran is
    /// forgotten.
    pub fn begin_evaluation(&self) {
        let now = self
            .budget
            .borrow()
            .wall_clock
            .map(|_| self.host().clock().now_millis());
        let mut budget = self.budget.borrow_mut();
        budget.start = self.cmd_count();
        budget.deadline = budget
            .wall_clock
            .zip(now)
            .map(|(wall_clock, now)| now.saturating_add(wall_clock.as_millis() as i128));
        budget.exceeded = None;
        budget.since_clock = 0;
    }

    /// The limit the running (or last) evaluation outran, if it outran one.
    #[must_use]
    pub fn limit_exceeded(&self) -> Option<LimitKind> {
        self.budget.borrow().exceeded
    }

    /// The commands dispatched since the evaluation began.
    #[must_use]
    pub fn commands_since_begin(&self) -> u64 {
        self.cmd_count().saturating_sub(self.budget.borrow().start)
    }

    /// Record that the evaluation outran `kind`, and raise its error: what a
    /// host does when work it ran for the evaluation outran a limit the
    /// interpreter could not see.
    pub fn exceed_limit(&mut self, kind: LimitKind) -> Code {
        self.budget.borrow_mut().exceeded = Some(kind);
        self.error(kind.message())
    }

    /// Charge one command dispatch: the command count, and the clock every
    /// [`DISPATCHES_PER_CLOCK_READ`] dispatches. Called by the dispatch
    /// boundary after it counts the command; `Some` is the error the command
    /// fails with instead of running.
    pub(crate) fn charge_dispatch(&mut self) -> Option<Code> {
        let kind = {
            let mut budget = self.budget.borrow_mut();
            if !budget.armed() {
                return None;
            }
            match budget.exceeded {
                Some(kind) => Some(kind),
                None => {
                    let spent = self.cmd_count().saturating_sub(budget.start);
                    if budget.commands.is_some_and(|limit| spent > limit) {
                        Some(LimitKind::Commands)
                    } else {
                        budget.since_clock += 1;
                        (budget.deadline.is_some()
                            && budget.since_clock >= DISPATCHES_PER_CLOCK_READ)
                            .then(|| {
                                budget.since_clock = 0;
                                budget.deadline
                            })
                            .flatten()
                            .filter(|&deadline| self.host().clock().now_millis() >= deadline)
                            .map(|_| LimitKind::WallClock)
                    }
                }
            }
        };
        kind.map(|kind| self.exceed_limit(kind))
    }

    /// Charge one iteration of a loop that may dispatch nothing (`while 1 {}`):
    /// the clock, every [`TICKS_PER_CLOCK_READ`] iterations, and a limit
    /// already exceeded.
    #[cfg(have_tommath)]
    pub(crate) fn charge_tick(&mut self) -> Option<Code> {
        let kind = {
            let mut budget = self.budget.borrow_mut();
            if !budget.armed() {
                return None;
            }
            match budget.exceeded {
                Some(kind) => Some(kind),
                None => {
                    budget.since_clock += 1;
                    (budget.deadline.is_some() && budget.since_clock >= TICKS_PER_CLOCK_READ)
                        .then(|| {
                            budget.since_clock = 0;
                            budget.deadline
                        })
                        .flatten()
                        .filter(|&deadline| self.host().clock().now_millis() >= deadline)
                        .map(|_| LimitKind::WallClock)
                }
            }
        };
        kind.map(|kind| self.exceed_limit(kind))
    }

    /// Whether an embedder has set a value-size limit, so a command can skip
    /// measuring what it would build when none is.
    pub(crate) fn has_value_limit(&self) -> bool {
        self.budget.borrow().value_bytes.is_some()
    }

    /// Refuse to build a value of `bytes` bytes over the value-size limit,
    /// before anything is allocated: one command can ask for more memory than
    /// the process has while the command count and the clock are still nearly
    /// untouched.
    pub(crate) fn charge_allocation(&mut self, bytes: u64) -> Option<Code> {
        let over = self
            .budget
            .borrow()
            .value_bytes
            .is_some_and(|limit| bytes > limit);
        over.then(|| self.exceed_limit(LimitKind::ValueSize))
    }
}
