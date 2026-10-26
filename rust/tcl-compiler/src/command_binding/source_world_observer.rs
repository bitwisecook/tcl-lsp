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

//! Bounded passive ordinary-world transitions. All fields describe captured
//! source-analysis data; neither opacity nor a pointer grants execution proof.

use super::{
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceInvocationBinding,
    SourceOutcomes,
};
use std::sync::atomic::{AtomicUsize, Ordering};

static ROWS: AtomicUsize = AtomicUsize::new(0);
const MAX_ROWS: usize = 16;
const MAX_HEAD_BYTES: usize = 32;

/// Captured pre-command fields only; the address is never dereferenced later.
pub(super) fn before_command(state: &ModuleCommandBindings) -> (bool, Option<usize>) {
    (
        state.opaque_domain,
        state
            .current_source_origin
            .as_ref()
            .map(|origin| std::sync::Arc::as_ptr(origin) as usize),
    )
}

pub(super) fn observe_command(
    owner: &SourceCommandBindings,
    context: &SourceExecutionContext<'_>,
    image: &tcl_lexer::SourceImage,
    segment: &crate::segmenter::SegmentedCommand,
    before: (bool, Option<usize>),
    command: &SourceOutcomes,
    prefix_complete: bool,
) {
    trace_transition(owner, context, image, segment, before, command);
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_COMMAND_COMPLETION site={} prefix_complete={} command_complete={} normal={} abrupt={}",
            segment.span.start(),
            prefix_complete,
            command.normal_completion.is_some(),
            command.normal.is_some(),
            command.abrupt.len(),
        );
    }
}

fn trace_transition(
    owner: &SourceCommandBindings,
    context: &SourceExecutionContext<'_>,
    image: &tcl_lexer::SourceImage,
    segment: &crate::segmenter::SegmentedCommand,
    before: (bool, Option<usize>),
    command: &SourceOutcomes,
) {
    if before.0 || std::env::var("TCL_LSP_TRACE_N17_FOLD").as_deref() != Ok("first-gate-v1") {
        return;
    }
    let normal_opaque = command
        .normal
        .as_ref()
        .is_some_and(|state| state.opaque_domain);
    let first_opaque_abrupt_route = command
        .abrupt
        .iter()
        .find(|(_, state)| state.opaque_domain)
        .map(|(route, _)| route);
    let abrupt_opaque = first_opaque_abrupt_route.is_some();
    if (!normal_opaque && !abrupt_opaque)
        || ROWS
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |rows| {
                (rows < MAX_ROWS).then_some(rows + 1)
            })
            .is_err()
    {
        return;
    }
    let head = segment
        .texts
        .first()
        .map_or(&[][..], |head| head.as_bytes());
    let prefix = &head[..head.len().min(MAX_HEAD_BYTES)];
    let compilation = context.compilation();
    eprintln!(
        "N17G_WORLD_TRANSITION source_origin_before={:?} source_data={:p} source_len={} scope={:?} mode={:?} frame={:?} preview_depth={} depth={} site={} end={} head_len={} head_complete={} head_prefix={prefix:?} normal_opaque={normal_opaque} abrupt_opaque={abrupt_opaque} first_opaque_abrupt_route={first_opaque_abrupt_route:?} normal_exists={} abrupt_count={}",
        before.1,
        image.bytes().as_ptr(),
        image.len(),
        owner.compilation_scope,
        compilation.mode,
        compilation.frame,
        owner.declaration_preview_depth,
        context.depth,
        segment.span.start(),
        segment.span.end(),
        head.len(),
        head.len() <= MAX_HEAD_BYTES,
        command.normal.is_some(),
        command.abrupt.len(),
    );
}

/// Same retained live snapshot only; no lookup, owner minting or currentness query.
/// Tuple: origin address, source data address, bytes, opaque, namespace-known.
pub(crate) fn binding_world(
    binding: &SourceInvocationBinding,
) -> Option<(usize, usize, usize, bool, bool)> {
    let state = &binding.lookup_state.as_ref()?.state;
    let origin = state.current_source_origin.as_ref()?;
    let image = origin.source_image();
    Some((
        std::sync::Arc::as_ptr(origin) as usize,
        image.bytes().as_ptr() as usize,
        image.len(),
        state.opaque_domain,
        state.source_variables.namespace_known,
    ))
}
