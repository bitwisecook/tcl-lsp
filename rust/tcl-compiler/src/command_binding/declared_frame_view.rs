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
use crate::var_resolve::{ResolveContext, VariableExecutionFrame};
use std::sync::Arc;

/// The frame a declared-body view keeps inside the body it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredFrameView {
    frame: VariableExecutionFrame,
    /// The commands of the body the walk recorded in `frame`, by offset,
    /// with their ends; the commands of a script the body runs in another
    /// frame are not among them.
    own: Arc<std::collections::BTreeMap<u32, u32>>,
    /// Every command of the body any walk recorded, by offset, with ends.
    commands: Arc<std::collections::BTreeMap<u32, u32>>,
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
        !self.own.contains_key(&point.offset) || point.state.variable_frame == self.frame
    }

    /// Whether a read at `offset` belongs to a command of the body's own
    /// frame: the innermost command around it is one.
    fn owns_read_at(&self, offset: u32) -> bool {
        self.commands
            .range(..=offset)
            .rev()
            .find(|(_, end)| offset < **end)
            .is_some_and(|(start, _)| self.own.contains_key(start))
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
        let mut own = std::collections::BTreeMap::new();
        let mut commands = std::collections::BTreeMap::new();
        for (offset, indexes) in self.dispatch_points.range(body.start()..body.end()) {
            for index in indexes {
                let point = &self.points[*index];
                commands.insert(*offset, point.end);
                if point.state.variable_frame == *frame {
                    own.insert(*offset, point.end);
                }
            }
        }
        let view = DeclaredFrameView {
            own: Arc::new(own),
            commands: Arc::new(commands),
            frame: frame.clone(),
            activation: activation(frame)?.to_owned(),
            body,
        };
        let reached = !view.own.is_empty();
        if std::env::var_os("ZZ").is_some() {
            let mut seen = std::collections::BTreeMap::new();
            for (off, idx) in self.dispatch_points.range(body.start()..body.end()) {
                for i in idx { let p = &self.points[*i]; *seen.entry(format!("{} adm={} prev={} frame={:?}", off, view.admits(p), p.declaration_preview, p.state.variable_frame)).or_insert(0) += 1; }
            }
            eprintln!("ZZ view frame={frame:?} reached={reached} points={seen:#?}");
        }
        if !reached {
            return None;
        }
        let mut variable_accesses = self.variable_accesses.clone();
        for (_, accesses) in variable_accesses
            .range_mut(body.start()..body.end())
            .filter(|(offset, _)| view.owns_read_at(**offset))
        {
            *accesses = accesses
                .iter()
                .filter_map(|access| access.in_declared_frame(&view))
                .collect();
        }
        if std::env::var_os("ZZ").is_some() {
            for (off, acc) in self.variable_accesses.range(body.start()..body.end()) {
                for a in acc { eprintln!("ZZ acc {off} own={} kept={} acts={:?}", view.owns_read_at(*off), a.in_declared_frame(&view).is_some(), a.context_alternatives.iter().map(|c| c.activation.clone()).collect::<Vec<_>>()); }
            }
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

    /// Take into this interpretation what `walk`, the declared-frame walk
    /// of the called `bodies`, recorded inside them: each point it made
    /// from `first` on, and the reads, which carry its contexts beside this
    /// interpretation's own. A consumer of either frame then finds it.
    pub(super) fn adopt_declared_walk(
        &mut self,
        walk: &Self,
        first: usize,
        bodies: &[tcl_lexer::Span],
    ) {
        for body in bodies {
            for (offset, indexes) in walk.dispatch_points.range(body.start()..body.end()) {
                for index in indexes.iter().filter(|index| **index >= first) {
                    let own = self.points.len();
                    self.points.push(walk.points[*index].clone());
                    self.dispatch_points.entry(*offset).or_default().push(own);
                }
            }
            for (offset, accesses) in walk.variable_accesses.range(body.start()..body.end()) {
                self.variable_accesses.insert(*offset, accesses.clone());
            }
        }
        self.invalidate_unpositioned_projections();
    }
}
