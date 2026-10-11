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
        std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
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
