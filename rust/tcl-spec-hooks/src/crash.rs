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

//! Crash containment: what is recorded when a hook takes the boundary down,
//! and what the user is offered.
//!
//! `docs/design/spec-packs.md`: "a spec must never be able to take the LSP
//! down". A panic inside a hook is converted to abstention, the hook is
//! quarantined for the session, and a structured record is written — pack name
//! and content hash, command and hook family, the SpecTcl vocabulary and
//! server versions, the input word **shapes**, and the panic payload.
//!
//! **Shapes, not words.** The record deliberately carries `literal`/`dynamic`/
//! `expanded`/`opaque` per word and nothing else: raw user-code words are
//! included only in what a user reviews and sends, never in what the process
//! accumulates. That is what makes a record safe to log and safe to pre-fill
//! an issue from.

use tcl_registry::invocation_words::InvocationWordKind;
use tcl_registry::pack_hooks::{HookCall, HookFamily};

/// Why a hook stopped being trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashKind {
    /// The body (or the engine running it) panicked.
    Panic,
    /// The body outran its command budget.
    CommandBudget,
    /// The body outran its wall-clock budget.
    WallClockBudget,
}

impl CrashKind {
    /// A stable, user-facing phrase for the toast and the issue title.
    #[must_use]
    pub fn summary(self) -> &'static str {
        match self {
            Self::Panic => "a spec pack hook crashed; tcl-lsp is unaffected",
            Self::CommandBudget => "a spec pack hook exceeded its command budget",
            Self::WallClockBudget => "a spec pack hook exceeded its time budget",
        }
    }
}

/// One crash, with everything a bug report needs and nothing a user would not
/// want transmitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashRecord {
    /// The pack that declared the hook.
    pub pack: String,
    /// The pack source's content hash.
    pub content_hash: String,
    /// `command subcommand field` — enough to find the declaration.
    pub hook: String,
    /// The family, which fixes what the hook was being asked for.
    pub family: HookFamily,
    /// The SpecTcl vocabulary version the pack declared.
    pub dsl_version: String,
    /// The server build that ran it.
    pub server_version: String,
    /// One entry per input word: its **kind**, never its text.
    pub word_shapes: Vec<InvocationWordKind>,
    /// Why it stopped being trusted, and what the boundary recovered.
    pub kind: CrashKind,
    /// The panic payload or budget message.
    pub detail: String,
}

impl CrashRecord {
    /// The word shapes of `call` — the only thing about the invocation a
    /// record keeps.
    #[must_use]
    pub fn shapes_of(call: &HookCall<'_>) -> Vec<InvocationWordKind> {
        call.words.iter().map(|word| word.kind).collect()
    }

    /// A one-line rendering for a log or an issue title.
    #[must_use]
    pub fn headline(&self) -> String {
        format!(
            "{}: {} in pack {} ({})",
            self.kind.summary(),
            self.hook,
            self.pack,
            self.content_hash
        )
    }
}
