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

//! Variable storage, the call-frame stack and variable traces: `VarStore`, `Frames`, `Traces` and the results a store reports.

use super::array_access::{
    ArrayDefaultSetFailure, ArrayDefaultState, ArrayElementRead, ArrayInvalidation, ArrayReadMiss,
    ArrayTarget,
};
use super::native_execution_error::NativeExecutionError;
use tcl_core_types::{FrameId, NsId};

/// A stored variable value and return options retained from its reached read.
/// Consumers apply both fields, including when the update itself succeeds.
#[derive(Debug, Clone)]
pub struct VariableUpdateResult<V> {
    /// Value read back from the retained receiver after the store callbacks.
    pub value: V,
    /// Authentic read metadata carried on the resulting successful completion.
    pub options: V,
}

/// A command-level variable removal rejected by the store.
///
/// Storage-only consumers use [`VarStore::unset`] when they have already
/// performed the Tcl command's policy checks. Command implementations use
/// [`VarStore::unset_command`] so runtimes preserve structured failures such
/// as Tcl 9's immutable-variable rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarUnsetError {
    /// The resolved variable cell is a Tcl 9 `const`.
    IsConstant,
}

/// Why a call-frame link exists.
///
/// Tcl's `info consts` excludes ordinary `global`/`upvar`/`variable` aliases,
/// but includes the automatic instance-variable projections installed for a
/// `TclOO` method when their target is constant. Keeping this on the binding
/// makes that distinction available to every runtime without teaching the
/// shared `info` command core about `TclOO` command names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameLinkOrigin {
    /// An ordinary Tcl variable alias.
    #[default]
    Ordinary,
    /// An automatic `TclOO` method-frame instance-variable projection, in a
    /// method whose compiled body has **no** local slot for the name.
    TclOoInstance,
    /// The same projection, in a method whose body *did* compile a local slot
    /// for the name — because it references it (`$pub`, `info exists pub`).
    ///
    /// The link behaves identically; the distinction exists only for
    /// enumeration. C lists a projection the body never mentions and stops
    /// listing one it reads, so `info consts` reports [`Self::TclOoInstance`]
    /// and not this (#2173). A *dynamic* read (`set $n`) compiles no slot and
    /// therefore stays the plain variant.
    TclOoInstanceCompiled,
}

// -- Family-B role traits --
//
// Small, composable traits over an associated `Value`, each mirroring a
// `runtime/rust` storage module. A consumer depends only on the subset it
// needs; do not collapse them into one umbrella `Interp` trait. Impls grow
// over time; the trait surface is the contract.

/// Variable storage: scalars, arrays, and `upvar`/`global`/`variable` links,
/// addressed by call frame. Corresponds to `frame.rs`'s `Var`/`VarTable`.
pub trait VarStore {
    /// The runtime's value type.
    type Value;

    /// Read a scalar variable in `frame`, following links.
    fn get(&self, frame: FrameId, name: &str) -> Option<Self::Value>;
    /// Write a scalar variable in `frame` (firing write traces).
    fn set(&mut self, frame: FrameId, name: &str, value: Self::Value);
    /// Remove a variable in `frame`; returns whether it existed.
    fn unset(&mut self, frame: FrameId, name: &str) -> bool;
    /// Remove a variable as a Tcl command operation, preserving policy errors.
    ///
    /// An absent variable is not an error and returns `Ok(false)`. The default
    /// keeps storage implementations source-compatible; runtimes with
    /// immutable bindings override it to reject them before mutation.
    fn unset_command(&mut self, frame: FrameId, name: &str) -> Result<bool, VarUnsetError> {
        Ok(self.unset(frame, name))
    }
    /// Whether removing `name`, resolved from `frame`, is refused because
    /// the engine confines stores to the running procedure's own frame and
    /// the variable lives outside it — asked by a command before it removes
    /// anything. An engine that confines nothing refuses nothing.
    fn unset_confined(&self, frame: FrameId, name: &str) -> bool {
        let _ = (frame, name);
        false
    }
    /// Counted-byte counterpart; Unicode-only stores refuse before lookup.
    fn unset_confined_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.unset_confined(frame, &name))
    }
    /// Whether a variable exists in `frame`.
    fn exists(&self, frame: FrameId, name: &str) -> bool;

    /// Present a reached physical variable failure through the adapter's
    /// independently authenticated naming issuer. Input parts and failure site
    /// remain separate from the selected binding/cell identity.
    fn variable_diagnostic_at(
        &self,
        operation: tcl_syntax::naming::NativeVariableDiagnosticOperation,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
    ) -> Result<tcl_syntax::naming::NativeVariableDiagnosticProjection, tcl_syntax::value::ValueError>
    {
        let _ = (operation, reason, site, input);
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "variable diagnostic",
        ))
    }

    /// Read an exact combined native name. Unicode-only adapters refuse an
    /// unrepresentable input before performing any lookup.
    fn get_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.get(frame, &name))
    }
    /// Write an exact combined native name, without repairing its spelling.
    fn set_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
        value: Self::Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        self.set(frame, &name, value);
        Ok(())
    }
    /// Remove an exact combined native name.
    fn unset_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.unset(frame, &name))
    }
    /// Command removal with operational access refusal outside the guest
    /// immutable-binding outcome.
    fn unset_command_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Result<bool, VarUnsetError>, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.unset_command(frame, &name))
    }
    /// Test an exact combined native name.
    fn exists_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.exists(frame, &name))
    }
    /// Read native root and element operands without reconstructing a combined
    /// name. The adapter selects each operand's native name protocol.
    fn get_elem_bytes(
        &self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let root = tcl_syntax::raw_string::RawString::from_bytes(root).unicode()?;
        let element = tcl_syntax::raw_string::RawString::from_bytes(element).unicode()?;
        Ok(self.get_elem(frame, &root, &element))
    }
    /// Write native root and element operands separately.
    fn set_elem_bytes(
        &mut self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
        value: Self::Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let root = tcl_syntax::raw_string::RawString::from_bytes(root).unicode()?;
        let element = tcl_syntax::raw_string::RawString::from_bytes(element).unicode()?;
        self.set_elem(frame, &root, &element, value);
        Ok(())
    }

    // Array-element access. The `name` is the array *base* and `key` the element,
    // already split from `base(key)` — runtimes differ on whether the by-name
    // accessors parse `a(k)`, so element ops are explicit. Implementations must
    // preserve this pair through storage resolution: recomposing `base(key)` and
    // parsing it again is ambiguous when the base itself contains `(`. Mirror
    // the scalar ops.

    /// Read array element `name(key)` in `frame`, following links.
    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<Self::Value>;
    /// Write array element `name(key)` in `frame` (firing write traces).
    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: Self::Value);
    /// Remove array element `name(key)`; returns whether it existed.
    fn unset_elem(&mut self, frame: FrameId, name: &str, key: &str) -> bool;
    /// Whether array element `name(key)` exists.
    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool;

    /// The element keys of array `name` in `frame`, in the runtime's storage
    /// order, or `None` if `name` is not an array (a scalar, or unset). An array
    /// with no elements yields `Some(vec![])` — the existence signal `array
    /// exists`/`info exists` need. This is the **enumeration** surface the
    /// otherwise-deliberately-listing-free state traits expose for the `array`
    /// family (`names`/`get`/`size`/`exists`/`unset`).
    fn array_keys(&self, frame: FrameId, name: &str) -> Option<Vec<String>>;

    /// Root storage protocol of this variable adapter. The legacy trait contract
    /// uses distinct C Tcl scalar/array cells; dictionary-valued adapters must
    /// select their protocol explicitly.
    fn variable_container_model(&self) -> tcl_dialect::VariableContainerModel {
        tcl_dialect::VariableContainerModel::DistinctArray
    }

    /// Enumerate an array value, preserving dictionary decoding failures.
    /// Distinct scalar/array stores return ordinary non-array absence; a
    /// dictionary-valued store distinguishes malformed contents from absence.
    fn array_keys_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<String>>, tcl_syntax::value::ValueError> {
        Ok(self.array_keys_at(target))
    }

    /// Enumerate exact key bytes from the captured array. Unicode-only stores
    /// inherit their checked key projection; byte-capable stores override it.
    fn array_key_bytes_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        self.array_keys_checked_at(target)
            .map(|keys| keys.map(|keys| keys.into_iter().map(String::into_bytes).collect()))
    }

    /// `Tcl_HashStats` over the captured array's physical element table, or
    /// `None` when the target is not an array. Stores without a native table
    /// layout refuse rather than invent one.
    fn array_hash_statistics_at(
        &self,
        _target: &ArrayTarget,
        _unguarded_empty_average: bool,
    ) -> Result<Option<String>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native array hash table",
        ))
    }

    /// Read an exact key in the captured array, preserving native host refusal.
    /// A store whose key API requires Unicode checks that projection first.
    fn array_read_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<ArrayElementRead<Self::Value>, tcl_syntax::value::ValueError> {
        let key = tcl_syntax::raw_string::RawString::from_bytes(key).unicode()?;
        Ok(self.array_read_elem_at(target, &key))
    }

    /// Locate an array spelling before its operation trace fires. The default
    /// remains name-addressed; a stable-cell runtime overrides this and the
    /// `*_at` methods below so callbacks cannot steer enumeration elsewhere.
    fn array_target(&self, frame: FrameId, name: &str) -> ArrayTarget {
        ArrayTarget::named(frame, name)
    }

    /// Capture an exact native array root before operation callbacks.
    fn array_target_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<ArrayTarget, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.array_target(frame, &name))
    }

    /// Inspect the same physical root after its array-operation callbacks.
    /// Missing native storage support is distinct from an undefined variable.
    fn array_default_state_at(
        &self,
        _target: &ArrayTarget,
    ) -> Result<ArrayDefaultState<Self::Value>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "array default storage",
        ))
    }

    /// Clear the default on the exact selected root without a new name lookup.
    fn unset_array_default_at(
        &mut self,
        _target: &ArrayTarget,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "array default storage",
        ))
    }

    /// Perform the setter's fresh creating lookup after array callbacks.
    /// A successful native C setter owns two references to `value`, returns no
    /// value, and does not fire variable-write callbacks.
    fn set_array_default_bytes(
        &mut self,
        _frame: FrameId,
        _name: &[u8],
        _value: Self::Value,
    ) -> Result<Result<(), ArrayDefaultSetFailure>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "array default storage",
        ))
    }

    /// Enumerate the cell captured by [`array_target`](Self::array_target).
    fn array_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        self.array_keys(target.frame(), target.name())
    }

    /// Physical element-table keys captured for an active array search.
    /// Unlike [`array_keys_at`](Self::array_keys_at), this includes attached
    /// undefined cells created by traces or links: Tcl's hash iterator sees
    /// those entries and skips them only when each candidate is reached.
    fn array_search_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        self.array_keys_at(target)
    }

    /// Exact physical search keys, with operational refusal outside absence.
    fn array_search_key_bytes_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        Ok(self
            .array_search_keys_at(target)
            .map(|keys| keys.into_iter().map(String::into_bytes).collect()))
    }

    /// Test an exact physical search key without firing read callbacks.
    fn array_elem_exists_bytes_at(
        &self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let key = tcl_syntax::raw_string::RawString::from_bytes(key).unicode()?;
        Ok(self.array_elem_exists_at(target, &key))
    }

    /// Whether one candidate in the captured array currently has a value,
    /// without firing its read trace.
    fn array_elem_exists_at(&self, target: &ArrayTarget, key: &str) -> bool {
        self.exists_elem(target.frame(), target.name(), key)
    }

    /// Structural revision of the captured array cell, when the runtime can
    /// distinguish invalidation of an active Tcl array search. The value
    /// changes on Tcl-level key insertion/removal or an explicit element
    /// unset; defining or replacing an attached element does not invalidate
    /// the search, nor does garbage-collecting an undefined trace shell.
    fn array_revision_at(&self, _target: &ArrayTarget) -> Option<u64> {
        None
    }

    /// Remove an element from the cell captured by
    /// [`array_target`](Self::array_target).
    fn unset_elem_at(&mut self, target: &ArrayTarget, key: &str) -> bool {
        self.unset_elem(target.frame(), target.name(), key)
    }

    /// Remove an exact byte key for the selected `ArrayUnset` operation.
    /// Native adapters select their release's retained-array or original-name
    /// lookup rule. Unicode-only stores refuse an unrepresentable key first.
    fn unset_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let key = tcl_syntax::raw_string::RawString::from_bytes(key).unicode()?;
        Ok(self.unset_elem_at(target, &key))
    }

    /// Read one `array get` candidate through variable-read traces.
    ///
    /// The default preserves storage-only implementations. Tcl runtimes
    /// override this to pin the live element before callbacks, retain a
    /// swallowed callback error in interpreter metadata, and distinguish a
    /// missing element from destruction of `target`.
    fn array_read_elem_at(
        &mut self,
        target: &ArrayTarget,
        key: &str,
    ) -> ArrayElementRead<Self::Value> {
        let value = self.get_elem(target.frame(), target.name(), key);
        if self.array_keys_at(target).is_none() {
            let invalidation = if self.exists(target.frame(), target.name()) {
                ArrayInvalidation::Retyped
            } else {
                ArrayInvalidation::Unset
            };
            ArrayElementRead::ArrayInvalidated(invalidation)
        } else {
            value.map_or_else(
                || ArrayElementRead::Missing(ArrayReadMiss::missing()),
                ArrayElementRead::Value,
            )
        }
    }
}

/// The call-frame stack: proc-call frames and the `uplevel` active-level dance.
/// Mirrors `runtime/rust`'s `frame.rs` `FrameStack` (`framePtr`/`varFramePtr`).
pub trait Frames {
    /// Push a new call frame whose namespace context is `ns`; returns its id.
    fn push(&mut self, ns: NsId) -> FrameId;
    /// Pop the current call frame.
    fn pop(&mut self);
    /// The current (top) frame.
    fn current(&self) -> FrameId;
    /// Install a link (`upvar`/`global`/`variable`) in `here` to `target`'s
    /// variable `target_name`.
    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str);

    // -- active-frame variable enumeration (backs the *frame-local* half of `info
    // vars`/`info locals`, the namespace-free counterpart to `Namespaces::vars_in`).

    /// Whether the active frame is a **procedure** activation (vs the global or a
    /// `namespace eval` frame) — `info vars` lists the frame's own variables in a
    /// proc, the current namespace's variables otherwise (C's `InfoVarsCmd`).
    fn in_proc(&self) -> bool;
    /// The variable names of the **active** frame. Genuine locals (scalars,
    /// arrays) are always included; `upvar`/`global`/`variable` **links** are
    /// included iff `include_links` — `info vars` lists links (by their local
    /// alias), `info locals` does not.
    fn var_names(&self, include_links: bool) -> Vec<String>;

    /// Exact active-frame bindings, preserving every native name.
    fn var_names_bytes(&self, include_links: bool) -> Vec<Vec<u8>> {
        self.var_names(include_links)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// Ordered owned frame inventory. Concrete native adapters validate their
    /// selected table protocol before returning names, including duplicate
    /// compiled declarations. The compatibility default preserves the supplied
    /// inventory without claiming an ABI or deriving a native hash order.
    fn var_names_bytes_checked(
        &self,
        include_links: bool,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        Ok(self.var_names_bytes(include_links))
    }

    /// Retain the actual compiled-name scan versus dynamic hash-key search
    /// selected by this entered frame. Bytes alone cannot identify that purpose.
    /// This supplies no native object header, lookup existence or value receipt.
    fn var_name_pattern_inputs_bytes_checked(
        &self,
        _include_links: bool,
    ) -> Result<
        Vec<(Vec<u8>, tcl_syntax::native_glob::NativeNameGlobPurpose)>,
        tcl_syntax::value::ValueError,
    > {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original frame variable pattern inputs",
        ))
    }

    /// The active frame's `info consts` bindings: direct constants plus typed
    /// `TclOO` instance projections whose target is constant. Ordinary link
    /// aliases are excluded even though `info constant alias` follows them.
    fn const_names(&self) -> Vec<String>;

    /// [`const_names`](Self::const_names) without a lossy UTF-8 round trip.
    fn const_names_bytes(&self) -> Vec<Vec<u8>> {
        self.const_names()
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }
}

/// Variable traces, preserving operational refusal outside guest completion.
pub trait Traces {
    /// The runtime's value type.
    type Value;

    /// Fire traces for a Unicode name through the same checked byte owner.
    /// The outer error retains the original Host refusal; the inner error is
    /// the actual callback's Guest error value.
    fn fire(
        &mut self,
        var: &str,
        op: &str,
    ) -> Result<Result<(), Self::Value>, NativeExecutionError> {
        self.fire_bytes(var.as_bytes(), op)
    }

    /// Fire traces for an exact native name. Implementations preserve the
    /// complete typed first Host cause outside the callback's Guest result.
    /// Callers handle the outer result before interpreting or formatting a
    /// Guest error; no protocol label or message substitutes for that cause.
    fn fire_bytes(
        &mut self,
        var: &[u8],
        op: &str,
    ) -> Result<Result<(), Self::Value>, NativeExecutionError>;
}
