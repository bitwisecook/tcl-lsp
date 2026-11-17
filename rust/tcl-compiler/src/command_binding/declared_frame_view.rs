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

use super::{SourceBindingPoint, SourceCommandBindings};
use crate::var_resolve::VariableExecutionFrame;

/// The frame a declared-body view keeps inside the body it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredFrameView {
    frame: VariableExecutionFrame,
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
}

impl SourceCommandBindings {
    /// This interpretation as one procedure body's own declared frame sees
    /// it: inside `body`, a command is bound only by the points the walk
    /// recorded in `frame`, never by the activation of a caller that entered
    /// the body; outside it, nothing changes. `None` when the walk reached no
    /// command of the body in that frame.
    pub(crate) fn declared_frame_view(
        &self,
        frame: &VariableExecutionFrame,
        body: tcl_lexer::Span,
    ) -> Option<Self> {
        let view = DeclaredFrameView {
            frame: frame.clone(),
            body,
        };
        let reached = self
            .dispatch_points
            .range(body.start()..body.end())
            .flat_map(|(_, indexes)| indexes)
            .any(|index| view.admits(&self.points[*index]));
        reached.then(|| Self {
            declared_frame_view: Some(std::sync::Arc::new(view)),
            derived_indexes: super::shared_inventory::DerivedIndexes::default(),
            module_projection_memo: super::retained_source::ModuleProjectionMemo::default(),
            declaration_flow_cache: super::declaration_flow::SourceDeclarationFlowCache::default(),
            ..self.clone()
        })
    }
}
