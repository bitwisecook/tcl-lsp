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

//! Optional timings of reached original owners, outside retained semantics.

use std::cell::Cell;
use std::time::Instant;

#[derive(Clone, Copy)]
pub(super) enum Query {
    Branch,
    Frame,
    Snapshot,
    Fingerprint,
    Equality,
    Preflight,
    Layout,
    Body,
    ReceiverBody,
    DeferredEntry,
    SourceSegmentation,
    SourceWordTokens,
    CompilerSelection,
    SourceArguments,
    NativeInvocationPreparation,
    PointCapture,
    ContinuationCapture,
    SelectedCommand,
    PreflightPublication,
    SourcePublication,
    LexicalMemo,
    LexicalCapture,
    BodyMath,
    BodyExpression,
}
const NAMES: [&str; 24] = [
    "branch",
    "frame",
    "snapshot",
    "fingerprint",
    "equality",
    "preflight",
    "layout",
    "body",
    "receiver-body",
    "deferred-entry",
    "source-segmentation",
    "source-word-tokens",
    "compiler-selection",
    "source-arguments",
    "native-invocation-preparation",
    "point-capture",
    "continuation-capture",
    "selected-command",
    "preflight-publication",
    "source-publication",
    "lexical-memo",
    "lexical-capture",
    "body-math",
    "body-expression",
];
thread_local! {
    static TOTALS: Cell<[(u64, u128); 24]> = const { Cell::new([(0, 0); 24]) };
}
pub(super) struct Timing {
    query: Query,
    started: Instant,
}
impl Timing {
    pub(super) fn at(query: Query) -> Option<Self> {
        tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(|| Self {
                query,
                started: Instant::now(),
            })
    }
}
impl Drop for Timing {
    fn drop(&mut self) {
        TOTALS.with(|totals| {
            let mut current = totals.get();
            let value = &mut current[self.query as usize];
            value.0 = value.0.saturating_add(1);
            value.1 = value.1.saturating_add(self.started.elapsed().as_nanos());
            totals.set(current);
        });
    }
}
pub(super) fn report(stage: &str) {
    let totals = TOTALS.with(Cell::get);
    for (query, (calls, nanos)) in NAMES.iter().zip(totals) {
        eprintln!(
            "SOURCE_ROOT_OWNER_WORK stage={stage} query={query} scope=thread-cumulative-inclusive calls={calls} us={}",
            nanos / 1_000
        );
    }
}
