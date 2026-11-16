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

//! Procedure introspection behind the `info` family: `Introspect`, `Procs` and the body, default and parameter answers.

use super::native_execution_error::NativeExecutionError;

/// Runtime introspection backing the `info` family: retained proc bodies, the
/// per-frame argv, and the command-frame stack (`errorInfo`/`info frame`).
/// (Contract surface; not yet implemented.)
pub trait Introspect {
    /// The runtime's value type.
    type Value;

    /// The current call-stack depth (`info level`).
    fn level(&self) -> usize;
    /// The argv of the call at `level` (`info level N`), if retained.
    /// An absent call is `Ok(None)`; original access/execution refusal stays
    /// outside Guest bad-level handling and preserves its exact first cause.
    fn level_argv(&self, level: usize) -> Result<Option<Self::Value>, NativeExecutionError>;
}

/// Procedure introspection backing `info body`/`args`/`default`: the retained
/// formal parameters (name + optional default) and source body of a user
/// procedure. Mirrors the `Proc`/`CompiledLocal` chain C's `InfoBodyCmd`/
/// `InfoArgsCmd`/`InfoDefaultCmd` walk.
///
/// Metadata queries return owned bytes. Default-value queries retain the
/// original runtime object, independently of its resident spelling or cache.
pub trait Procs: tcl_syntax::value::ValueOps {
    /// Full failure of a reached procedure query, preserving execution refusal.
    type OperationError: From<tcl_syntax::value::ValueError> + From<NativeExecutionError>;
    /// Query the selected retained declaration using the original byte name.
    /// A callback cannot replace that declaration with a same-named command.
    ///
    /// # Errors
    /// Retains full failure from reached getters or the original query owner.
    fn proc_info_bytes(&mut self, name: &[u8]) -> Result<Option<ProcInfo>, Self::OperationError>;

    /// Project a retained formal storage key or a query operand for native
    /// `info args`/`default`. This is introspection, not formal binding.
    fn formal_introspection_name_bytes(
        &self,
        name: &[u8],
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(name.to_vec())
    }

    /// Original formal-list result for an engine whose introspection exposes
    /// the whole retained declaration, including defaults and reference names.
    /// Returning None selects the stored-name projection below. A selected
    /// original-list engine must refuse if its genuine declaration is missing.
    fn proc_original_formal_list_value(
        &self,
        name: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError>;

    /// Stored formal names without materialising their default objects.
    fn proc_formal_names_bytes(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Vec<Vec<u8>>>, Self::OperationError> {
        Ok(self.proc_info_bytes(name)?.map(|info| {
            info.params
                .into_iter()
                .map(|parameter| parameter.name)
                .collect()
        }))
    }

    /// Select an actual original body-result header when this engine represents it.
    /// `None` explicitly selects byte compatibility. A represented refusal is
    /// terminal; consumers keep the returned owner through immediate publication.
    fn proc_body_original_value(
        &mut self,
        _original: &Self::Value,
    ) -> ProcBodyValueResult<Self::Value, Self::OperationError, Self> {
        Ok(None)
    }

    /// Query body bytes through the original command operand when represented.
    /// `None` explicitly selects the existing byte-name compatibility adapter.
    /// An original implementation returns a represented outcome or its terminal
    /// failure; refusal never licenses retry through a rendered command name.
    fn proc_body_original_bytes(
        &mut self,
        _original: &Self::Value,
    ) -> Result<Option<ProcBodyBytesQuery>, Self::OperationError> {
        Ok(None)
    }

    /// Retain body bytes without inspecting or materialising any default object.
    fn proc_body_bytes(&mut self, name: &[u8]) -> Result<Option<Vec<u8>>, Self::OperationError> {
        Ok(self.proc_info_bytes(name)?.map(|info| info.body))
    }

    /// Resolve a default under the selected formal introspection grammar.
    /// Concrete engines return the original retained default object.
    fn proc_default_value_bytes(
        &mut self,
        name: &[u8],
        arg: &[u8],
    ) -> Result<ProcDefaultValue<Self::Value>, Self::OperationError> {
        let selected = self.formal_introspection_name_bytes(arg)?;
        let Some(info) = self.proc_info_bytes(name)? else {
            return Ok(ProcDefaultValue::MissingProcedure);
        };
        for parameter in info.params {
            if self.formal_introspection_name_bytes(&parameter.name)? == selected {
                return Ok(ProcDefaultValue::Declared(
                    parameter.default.map(|bytes| self.new_bytes(&bytes)),
                ));
            }
        }
        Ok(ProcDefaultValue::MissingParameter)
    }
}

/// Checked borrowed body header without an extra native object or procedure role.
/// The issuer preserves allocation safety and original operation currency until
/// the consumer's actual result setter acquires its own result reference.
/// The actual consumer is supplied at each frontier so stateful adapters can
/// revalidate the same caller without retaining a pointer to a movable engine.
pub trait OriginalProcedureBodyValue<V, E, O: ?Sized> {
    /// Borrow the original header immediately before its actual result publication.
    ///
    /// # Errors
    /// Refuses stale allocation, declaration, caller or original operation state.
    fn checked_value(&self, actual: &mut O) -> Result<V, E>;

    /// Finish after actual publication, then release only allocation transports.
    /// A legal old-result callback can delete the queried declaration; this
    /// frontier checks original operation/caller currency, not continued binding.
    ///
    /// # Errors
    /// Retains the first Host cause or changed original operation/caller state.
    fn finish(self: Box<Self>, actual: &mut O) -> Result<(), E>;
}

/// A represented original body query, or `None` for byte compatibility.
pub(super) type ProcBodyValueResult<V, E, O> = Result<Option<ProcBodyValueQuery<V, E, O>>, E>;

/// Original body query outcome, distinct from a freshly constructed byte report.
pub enum ProcBodyValueQuery<V, E, O: ?Sized> {
    /// Actual original header and its allocation/currentness transport.
    Body(Box<dyn OriginalProcedureBodyValue<V, E, O>>),
    /// Original diagnostic spelling; diagnostic producer selection is independent.
    NotProcedure(Vec<u8>),
}

/// Body-byte query data, independent of a native body-result header or execution.
#[derive(Debug, Clone)]
pub enum ProcBodyBytesQuery {
    /// Reached bytes of the actual selected declaration's body.
    Body(Vec<u8>),
    /// Original diagnostic spelling after the selected command was not a proc.
    NotProcedure(Vec<u8>),
}

/// Result of native default lookup, preserving missing-definition distinctions.
#[derive(Debug, Clone)]
pub enum ProcDefaultValue<V> {
    /// The selected command is not a user procedure.
    MissingProcedure,
    /// No formal matches the original query under native introspection rules.
    MissingParameter,
    /// An actual formal, with its original default when one was declared.
    Declared(Option<V>),
}

/// A user procedure's introspectable definition — the [`Procs::proc_info_bytes`] answer.
#[derive(Debug, Clone)]
pub struct ProcInfo {
    /// The procedure body source (`info body`), byte-exact.
    pub body: Vec<u8>,
    /// The formal parameters, in declaration order (`info args`/`default`).
    pub params: Vec<ProcParam>,
}

/// One formal parameter of a [`ProcInfo`]: a name with an optional default value.
#[derive(Debug, Clone)]
pub struct ProcParam {
    /// The parameter name.
    pub name: Vec<u8>,
    /// The declared default value, if the parameter has one.
    pub default: Option<Vec<u8>>,
}
