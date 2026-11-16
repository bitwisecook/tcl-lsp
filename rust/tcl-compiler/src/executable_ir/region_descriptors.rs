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

//! The registry descriptor selected for one source statement: a lowered operation, an opaque region or a structured projection.

use super::StructuredRegionKind;
use crate::ir::Statement;
use tcl_core_types::Code as CompletionCode;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::completion::HandlerChain;

pub(super) fn lowered_operation_descriptor(statement: &Statement) -> Option<LoweringHookId> {
    match statement {
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. } => Some(LoweringHookId::Set),
        Statement::Incr { .. } => Some(LoweringHookId::Incr),
        Statement::ExprEval { .. } => Some(LoweringHookId::Expr),
        Statement::Return { .. } => Some(LoweringHookId::Return),
        Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OpaqueRegionDescriptor {
    Unidentified,
    Identified(LoweringHookId),
}

impl OpaqueRegionDescriptor {
    pub(super) const fn lowering_hook(self) -> Option<LoweringHookId> {
        match self {
            Self::Unidentified => None,
            Self::Identified(hook) => Some(hook),
        }
    }
}

pub(super) fn opaque_region_descriptor(statement: &Statement) -> Option<OpaqueRegionDescriptor> {
    match statement {
        Statement::Block { .. } => Some(OpaqueRegionDescriptor::Unidentified),
        Statement::UpFrame { .. } => {
            Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Uplevel))
        }
        Statement::If { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::If)),
        Statement::For { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::For)),
        Statement::While { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::While)),
        Statement::Foreach {
            is_lmap,
            is_dict_iteration,
            is_array_iteration,
            ..
        } => Some(OpaqueRegionDescriptor::Identified(if *is_array_iteration {
            LoweringHookId::ArrayFor
        } else if *is_dict_iteration {
            LoweringHookId::Dict
        } else if *is_lmap {
            LoweringHookId::Lmap
        } else {
            LoweringHookId::Foreach
        })),
        Statement::Catch { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Catch)),
        Statement::Try { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Try)),
        Statement::Switch { .. } => {
            Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Switch))
        }
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Call { .. }
        | Statement::Return { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. } => None,
    }
}

/// The structured projection selected for one source statement, or `None` when
/// the statement keeps a typed opaque region.
///
/// The identity is the registry-owned [`LoweringHookId`] the lowering already
/// chose; nothing here recognises a command spelling.
pub(super) fn structured_region_projection(
    statement: &Statement,
) -> Option<(LoweringHookId, StructuredRegionKind)> {
    match statement {
        Statement::If { .. } => Some((LoweringHookId::If, StructuredRegionKind::Conditional)),
        Statement::While { .. } => {
            Some((LoweringHookId::While, StructuredRegionKind::ConditionLoop))
        }
        Statement::For { .. } => Some((LoweringHookId::For, StructuredRegionKind::ConditionLoop)),
        Statement::Foreach {
            is_lmap,
            is_dict_iteration,
            is_array_iteration,
            ..
        } => {
            // `dict for`/`dict map` iterate a dictionary and Tcl 9's `array
            // for` iterates an array, neither of which is the Tcl-list cursor
            // this projection models, so both keep their opaque region.
            let hook = if *is_lmap {
                LoweringHookId::Lmap
            } else {
                LoweringHookId::Foreach
            };
            (!*is_dict_iteration && !*is_array_iteration)
                .then_some((hook, StructuredRegionKind::CursorLoop))
        }
        // An empty guarded body has no abrupt edge for a handler to join, so
        // there is nothing for the completion-class projection to describe.
        Statement::Catch { body, .. } => (!body.statements.is_empty())
            .then_some((LoweringHookId::Catch, StructuredRegionKind::Catch)),
        Statement::Try { body, .. } => (!body.statements.is_empty())
            .then_some((LoweringHookId::Try, StructuredRegionKind::Try)),
        Statement::Switch { .. } => Some((LoweringHookId::Switch, StructuredRegionKind::Switch)),
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Call { .. }
        | Statement::Return { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. } => None,
    }
}

/// The completion code a `try` handler clause selects, when its selector is
/// statically a completion code.
///
/// A handler selecting by `-errorcode` prefix (`trap`) always selects
/// `TCL_ERROR` and narrows it further by the prefix; one selecting by
/// completion code (`on`) names a code directly. The registry's handler chain
/// decodes the selector ([`HandlerChain::selected`]), never a local keyword
/// match.
pub(crate) fn try_handler_code_in(
    handler: &crate::ir::TryHandler,
    numbers: tcl_syntax::number::Numbers,
) -> Option<CompletionCode> {
    HandlerChain::selected(handler.kind, &handler.match_arg, numbers)
}
