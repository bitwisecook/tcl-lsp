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

//! Jim's original `Jim_GetIndex` worker. Concrete adapters retain the SAME
//! entered operation and header while supplying getters, safe `GetWideExpr` and
//! checked primary retirement/adoption. Geometry belongs to tcl-syntax.

use crate::CmdError;
use std::rc::Rc;
use tcl_syntax::{
    native_jim_index::{JimIndex, JimIndexEvaluationError, JimIndexExpression},
    value::ValueOps,
};

/// Physical Jim Index object stages, independent of public command execution.
pub trait NativeJimIndexObjects: ValueOps {
    /// Allocation safety and original entered operation; no native +1 lease.
    type Scope;
    /// Capture the same entered operation and original header allocation.
    ///
    /// # Errors
    /// Original operation/header or independently selected Jim issuer refusal.
    fn jim_index_scope(&mut self, original: &Self::Value) -> Result<Self::Scope, CmdError>;
    /// Validate that captured operation and original header, preserving first cause.
    ///
    /// # Errors
    /// The original operation or physical header is no longer current.
    fn jim_index_check(
        &mut self,
        scope: &Self::Scope,
        original: &Self::Value,
    ) -> Result<(), CmdError>;
    /// Inspect the exact original Int primary without string conversion.
    ///
    /// # Errors
    /// The original header or physical representation is unavailable.
    fn jim_index_integer(&mut self, original: &Self::Value) -> Result<Option<i64>, CmdError>;
    /// Inspect the exact original Jim Index primary without conversion.
    ///
    /// # Errors
    /// The original header or cached representation is unavailable.
    fn jim_index_cached(&mut self, original: &Self::Value) -> Result<Option<i32>, CmdError>;
    /// Reach the selected original String getter under captured currency.
    ///
    /// # Errors
    /// Original operation/header or String producer refusal.
    fn jim_index_string(
        &mut self,
        scope: &Self::Scope,
        original: &Self::Value,
    ) -> Result<Rc<[u8]>, CmdError>;
    /// Original expression or genuinely fresh end suffix, including its reached
    /// native +1/-1 and post-retirement check under the retained original scope.
    ///
    /// # Errors
    /// Safe expression guest failure or the full first Host refusal.
    fn jim_index_expression(
        &mut self,
        scope: &Self::Scope,
        original: &Self::Value,
        input: JimIndexExpression<'_>,
    ) -> Result<i64, CmdError>;
    /// Retire the actual old primary, check callbacks, then adopt this Index.
    ///
    /// # Errors
    /// Original currency/header or reached cleanup callback refusal.
    fn jim_index_install(
        &mut self,
        scope: &Self::Scope,
        original: &Self::Value,
        index: JimIndex,
    ) -> Result<(), CmdError>;
}

/// Convert the SAME original Jim header with every first Host cause intact.
///
/// # Errors
/// Original currency/header/backend refusal or the reached Jim bad-index error.
pub fn original_index<O: NativeJimIndexObjects>(
    ops: &mut O,
    original: &O::Value,
) -> Result<i32, CmdError> {
    let scope = ops.jim_index_scope(original)?;
    ops.jim_index_check(&scope, original)?;
    let integer = ops.jim_index_integer(original);
    ops.jim_index_check(&scope, original)?;
    if let Some(integer) = integer? {
        return Ok(JimIndex::from_integer(integer));
    }
    let cached = ops.jim_index_cached(original);
    ops.jim_index_check(&scope, original)?;
    if let Some(cached) = cached? {
        return Ok(cached);
    }
    let source = ops.jim_index_string(&scope, original);
    ops.jim_index_check(&scope, original)?;
    let source = source?;
    let reached = tcl_syntax::native_jim_index::evaluate_index(&source, |input| {
        ops.jim_index_check(&scope, original)?;
        let result = ops.jim_index_expression(&scope, original, input);
        ops.jim_index_check(&scope, original)?;
        result
    });
    ops.jim_index_check(&scope, original)?;
    let index = match reached {
        Ok(index) => index,
        Err(JimIndexEvaluationError::Expression(error))
            if error.native_execution_refusal().is_some() =>
        {
            return Err(error);
        }
        Err(JimIndexEvaluationError::Index(error)) if error.native_access_refusal().is_some() => {
            return Err(error.into());
        }
        Err(_) => {
            // Jim_SetResultFormatted %#s reaches the original getter again;
            // snprintf presents its CString extent. The errorCode is untouched.
            let source = ops.jim_index_string(&scope, original);
            ops.jim_index_check(&scope, original)?;
            let mut message = b"bad index \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(&source?));
            message.extend_from_slice(b"\": must be intexpr or end?[+-]intexpr?");
            return Err(CmdError::from_byte_details(crate::error::CmdErrorDetails {
                message,
                error_code: crate::error::CmdErrorCodeUpdate::Unchanged,
                error_info: None,
                error_line: None,
                primitive_getter: None,
                string_result: None,
            }));
        }
    };
    let installed = ops.jim_index_install(&scope, original, index);
    ops.jim_index_check(&scope, original)?;
    installed?;
    Ok(index.0)
}
