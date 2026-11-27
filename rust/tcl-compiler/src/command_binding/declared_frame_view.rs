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

//! One procedure body's own declared frame, seen through an interpretation
//! that also entered the body from its callers.

use super::{SourceBindingPoint, SourceCommandBindings, SourceVariableAccess};
use crate::var_resolve::ResolveContext;
use std::sync::Arc;
use tcl_names::variable::VariableExecutionFrame;

/// The frame a declared-body view keeps inside the body it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredFrameView {
    frame: VariableExecutionFrame,
    activation: String,
    body: tcl_lexer::Span,
}

impl DeclaredFrameView {
    /// Whether a command at `offset` lies in the covered body.
    pub(super) fn covers(&self, offset: u32) -> bool {
        (self.body.start()..self.body.end()).contains(&offset)
    }

    /// Whether `point` was recorded in the covered body's declared frame.
    pub(super) fn admits(&self, point: &SourceBindingPoint) -> bool {
        point.state.variable_frame == self.frame
    }

    /// Whether a variable read resolved under `context` ran in the covered
    /// body's declared frame.
    fn admits_context(&self, context: &ResolveContext) -> bool {
        context.activation.as_deref() == Some(self.activation.as_str())
    }
}

/// The activation identity of a procedure frame.
fn activation(frame: &VariableExecutionFrame) -> Option<&str> {
    match frame {
        VariableExecutionFrame::NamespaceIdentity { frame, .. } => activation(frame),
        VariableExecutionFrame::Procedure { identity, .. } => Some(identity),
        _ => None,
    }
}

impl SourceVariableAccess {
    /// This read as the declared frame of `view` alone reached it; `None`
    /// where only a caller's activation did.
    fn in_declared_frame(&self, view: &DeclaredFrameView) -> Option<Self> {
        let contexts = self
            .context_alternatives
            .iter()
            .filter(|context| view.admits_context(context))
            .cloned()
            .collect::<Vec<_>>();
        let mut joined = Arc::clone(contexts.first()?);
        for context in &contexts[1..] {
            if *joined != **context {
                Arc::make_mut(&mut joined).join(context);
            }
        }
        Some(Self {
            variable_context: Arc::clone(&joined),
            context_fingerprint: Self::fingerprint(&joined),
            selected_places: self
                .selected_places
                .iter()
                .filter(|selected| view.admits_context(&selected.context))
                .cloned()
                .collect(),
            context_alternatives: contexts,
            ..self.clone()
        })
    }
}

impl SourceCommandBindings {
    /// This interpretation as one procedure body's own declared frame sees
    /// it: inside `body`, a command is bound only by the points the walk
    /// recorded in `frame`, never by the activation of a caller that entered
    /// the body; outside it, nothing changes. A called procedure's body is
    /// read from the walk that also entered it in its declared frame
    /// ([`Self::declared_frame_walk`]). `None` when no walk reached a command
    /// of the body in that frame.
    pub(crate) fn declared_frame_view(
        &self,
        frame: &VariableExecutionFrame,
        body: tcl_lexer::Span,
    ) -> Option<Self> {
        if let Some(walk) = self.declared_frame_walk.as_deref()
            && let Some(view) = walk.declared_frame_view(frame, body)
        {
            return Some(view);
        }
        let view = DeclaredFrameView {
            frame: frame.clone(),
            activation: activation(frame)?.to_owned(),
            body,
        };
        let reached = self
            .dispatch_points
            .range(body.start()..body.end())
            .flat_map(|(_, indexes)| indexes)
            .any(|index| view.admits(&self.points[*index]));
        if !reached {
            return None;
        }
        let mut variable_accesses = self.variable_accesses.clone();
        for (_, accesses) in variable_accesses.range_mut(body.start()..body.end()) {
            *accesses = accesses
                .iter()
                .filter_map(|access| access.in_declared_frame(&view))
                .collect();
        }
        Some(Self {
            declared_frame_view: Some(Arc::new(view)),
            variable_accesses,
            derived_indexes: super::shared_inventory::DerivedIndexes::default(),
            module_projection_memo: super::retained_source::ModuleProjectionMemo::default(),
            declaration_flow_cache: super::declaration_flow::SourceDeclarationFlowCache::default(),
            ..self.clone()
        })
    }
}
