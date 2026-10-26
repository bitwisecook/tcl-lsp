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

//! Shared Tcl index parsing (`Tcl_GetIntForIndex` / `GetEndOffsetFromObj`).
//!
//! One parser for every indexed command — `string index/range`, `lindex`,
//! `lrange`, `linsert`, `lreplace`, … — so the accepted forms and the error
//! message live once. The grammar is the release's: an integer, `end` with an
//! offset (`end-2`, `end--1` = `end - (-1)`), and from 8.5 `end+N` and the
//! sums `1+1` and `0-1`, each spelt and each integer read as that release
//! reads it (`read`).

use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

use crate::error::CmdError;

/// Numeric search/sort primitive, separate from Index spelling and String bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListNumericKind {
    Integer,
    Real,
}

/// Exact primitive output returned by the selected original scalar owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListNumber {
    Integer(i64),
    Real(f64),
}

/// Actual local sorting refuses independently of comparator completion.
pub enum NativeSortError<E> {
    Operation(CmdError),
    Comparison(E),
}

/// A selected original `ByteArray` receipt and its bytes, or `None` when the
/// value is not binary-eligible.
type IndexBinarySelection<R> = Result<Option<(R, std::rc::Rc<[u8]>)>, CmdError>;

/// Original index conversion at the command boundary, with full Host causes.
/// Portable adapters explicitly use the neutral default; physical Jim adapters
/// select their original-object worker independently of the numeral grammar.
pub trait NativeIndexOps: ValueOps {
    /// Cloneable allocation-only transport for a selected key's actual role.
    /// Cloning this guard must not acquire another native object reference.
    type IndexKeyOwner: Clone;

    /// Borrow a selected original/member key without an invented native +1.
    fn index_borrowed_key(
        &mut self,
        original: &Self::Value,
    ) -> Result<Self::IndexKeyOwner, CmdError>;

    /// Own cleanup of the exact fresh-zero selected constructor.
    fn index_created_key(&mut self, created: Self::Value) -> Result<Self::IndexKeyOwner, CmdError>;

    /// Project the original header/value without a String getter or clone.
    fn index_key_value(owner: &Self::IndexKeyOwner) -> &Self::Value;

    /// Allocation-only transport into a reached native container constructor.
    /// The constructor supplies its own actual member references.
    fn index_key_argument(&mut self, owner: &Self::IndexKeyOwner) -> Result<Self::Value, CmdError>;

    /// Transfer the selected key into an actual command-result role.
    /// `created` and `created_ancestor` are constructor facts, never count or
    /// pointer guesses. A parent may retire only after its selected child holds.
    fn index_key_result(
        &mut self,
        owner: &Self::IndexKeyOwner,
        created: bool,
        created_ancestor: bool,
    ) -> Result<IndexResult<Self::Value>, CmdError>;

    /// Hold the selected child at its genuine result boundary before parent retirement.
    fn index_hold_key_result(
        &mut self,
        owner: &Self::IndexKeyOwner,
    ) -> Result<Self::IndexKeyOwner, CmdError>;

    /// Transfer the same held child after mandatory Created backing cleanup.
    fn index_finish_held_key_result(
        &mut self,
        held: Self::IndexKeyOwner,
    ) -> Result<IndexResult<Self::Value>, CmdError>;

    /// The actual entered operation. Neutral adapters explicitly use `()`.
    type IndexOperation;

    /// Allocation-only selected C84 original primary/backing receipt. Neutral
    /// adapters use (); later C branches do not borrow this captured-pointer role.
    type IndexBinaryReceipt;

    /// Inspect actual `ByteArray` eligibility without invoking a String getter.
    /// The release is the actual selected C producer, not neutral grammar.
    fn index_c_select_binary(
        &mut self,
        _original: &Self::Value,
        _version: TclVersion,
    ) -> IndexBinarySelection<Self::IndexBinaryReceipt> {
        Ok(None)
    }

    /// Original C84/85 instruction pointer captured before its index getter.
    /// Absence of this provider is a refusal, not a non-binary observation.
    ///
    /// # Errors
    /// The actual captured-pointer producer is unavailable or its input refuses.
    fn index_c_instruction_binary(
        &mut self,
        _original: &Self::Value,
        _version: TclVersion,
    ) -> IndexBinarySelection<Self::IndexBinaryReceipt> {
        Err(ValueError::CommandProtocolUnavailable(
            "original compiled index binary selection provider",
        )
        .into())
    }

    /// Check C84's original captured storage immediately before pointer use.
    fn index_c_check_binary_storage(
        &mut self,
        _original: &Self::Value,
        _receipt: &Self::IndexBinaryReceipt,
    ) -> Result<(), CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C binary storage receipt").into())
    }

    /// Reread the same original binary header at C85's reached getter stage.
    fn index_c_binary_bytes(
        &mut self,
        _original: &Self::Value,
    ) -> Result<std::rc::Rc<[u8]>, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C binary getter").into())
    }

    /// Reach the original String representation/count without length shortcuts.
    fn index_c_string_storage(
        &mut self,
        _original: &Self::Value,
    ) -> Result<crate::string::NativeCStringStorage, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C String storage").into())
    }

    /// Actual local backend promotion after selected original String admission.
    fn index_c_plain_char(
        &mut self,
    ) -> Result<tcl_syntax::native_tcl_utf::NativeCPlainChar, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C String ABI").into())
    }

    /// Fresh counted String constructor for `GetRange`'s raw byte-slice branch.
    fn index_c_counted_string_result(
        &mut self,
        _bytes: std::rc::Rc<[u8]>,
        _count: usize,
    ) -> Result<Self::Value, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C counted String constructor").into())
    }

    /// Capture the original operation before any operand getter or length.
    /// This entry grants no String, numeric, name or cache purpose.
    ///
    /// # Errors
    /// The complete first refusal of the actual selected interpreter.
    fn enter_index_operation(&mut self) -> Result<Self::IndexOperation, CmdError>;

    /// Check the SAME original operation after reached effects and cleanup.
    ///
    /// # Errors
    /// The complete first refusal, including change-and-restore.
    fn check_index_operation(&mut self, original: &Self::IndexOperation) -> Result<(), CmdError>;

    /// Observe the reached Jim range constructor before its result transfers.
    fn completed_jim_string_range(
        &mut self,
        _original: &Self::IndexOperation,
        _construction: &crate::string::NativeJimStringRangeConstruction<'_, Self::Value>,
    ) -> Result<(), CmdError> {
        Ok(())
    }

    /// Construct an integer result under the command's original operation.
    /// Actual C84 adapters select `Tcl_NewIntObj`'s Long descriptor and queried ABI;
    /// the explicit neutral default retains its ordinary constructor.
    ///
    /// # Errors
    /// Unavailable selected construction or the original full execution refusal.
    fn index_integer_result(
        &mut self,
        _original: &Self::IndexOperation,
        integer: i64,
    ) -> Result<Self::Value, CmdError> {
        Ok(self.new_int(integer))
    }

    /// Retain an intrinsic first Host refusal before mandatory cleanup.
    /// This latch grants no operation, getter, header or result role. Neutral
    /// adapters explicitly choose a no-op; physical adapters use their actual
    /// interpreter's unconditional first-cause owner.
    fn retain_index_failure(&mut self, error: &CmdError);

    /// Discard a reached Created, Detached or `ReturnedOwnedTransport` result.
    /// Borrowed input transport is released by the shared role owner instead.
    /// Mandatory retirement continues after refusal and grants no new role.
    fn discard_owned_index_result(&mut self, created: Self::Value);

    /// Character count at the same selected String stage, preserving native
    /// shortcuts and checking old-primary retirement before cache adoption.
    ///
    /// # Errors
    /// Selected String/count protocol refusal or full original Host cause.
    fn index_character_count(&mut self, original: &Self::Value) -> Result<usize, CmdError> {
        self.native_char_len(original).map_err(Into::into)
    }

    /// Unicode projection only after the checked original counted String door.
    ///
    /// # Errors
    /// Full original getter/Host failure or actual invalid Unicode extent.
    fn index_unicode(&mut self, original: &Self::Value) -> Result<std::rc::Rc<str>, CmdError> {
        tcl_syntax::raw_string::RawString::from_bytes(self.index_operand_bytes(original)?)
            .unicode()
            .map_err(Into::into)
    }

    /// The reached original List conversion under the complete Index operation.
    /// Physical adapters check both the original currency and header inside the
    /// List worker, including old-primary retirement before backing adoption.
    ///
    /// # Errors
    /// Original List conversion or the complete first Host refusal.
    fn index_list_elements(
        &mut self,
        original: &Self::Value,
    ) -> Result<Vec<Self::Value>, CmdError> {
        self.list_elements(original).map_err(Into::into)
    }

    /// The selected length purpose retains its native cached/constant/abstract
    /// shortcuts. Only a reached conversion uses the checked List worker.
    ///
    /// # Errors
    /// Selected length conversion or the complete original Host cause.
    fn index_list_length(&mut self, original: &Self::Value) -> Result<usize, CmdError> {
        self.list_len(original).map_err(Into::into)
    }

    /// The independently selected member accessor preserves native abstract
    /// list shortcuts; its result is checked before the consumer proceeds.
    ///
    /// # Errors
    /// Selected member conversion or the complete original Host cause.
    fn index_list_member(
        &mut self,
        original: &Self::Value,
        index: usize,
    ) -> Result<Option<IndexResult<Self::Value>>, CmdError> {
        self.list_index(original, index)
            .map(|member| member.map(IndexResult::Borrowed))
            .map_err(Into::into)
    }

    /// Borrow Jim's actual List member headers for an original index path.
    /// None is only a neutral adapter; no original is recreated from text.
    ///
    /// # Errors
    /// Original path List conversion or complete first Host refusal.
    fn original_jim_index_path(
        &mut self,
        _original: &Self::Value,
    ) -> Result<Option<Vec<Self::Value>>, CmdError> {
        Ok(None)
    }

    /// Resolve the SAME original Jim index header. `None` is a neutral adapter;
    /// an unavailable selected native worker must return its actual refusal.
    ///
    /// # Errors
    /// Original header, operation or selected getter/engine refusal, or bad index.
    fn original_jim_index(&mut self, _original: &Self::Value) -> Result<Option<i32>, CmdError> {
        Ok(None)
    }

    /// Prepare original C search/sort option members before the input List.
    /// None is an explicit neutral/Jim adapter, never a recreated original.
    /// `report` selects the public validation versus source-selected quiet
    /// second preparation of the last lsort path.
    ///
    /// # Errors
    /// Original member/getter/width failure or complete first Host refusal.
    fn original_c_option_path(
        &mut self,
        _original: &Self::Value,
        _command: crate::native_option_index::OptionCommand,
        _report: bool,
    ) -> Result<Option<Vec<tcl_syntax::native_compiled_index::NativeCompiledListIndex>>, CmdError>
    {
        Ok(None)
    }

    /// Read the original search/sort stride with the selected scalar command
    /// owner. This is primitive Int/Wide/Long conversion, never Index grammar.
    /// A neutral adapter explicitly returns None; actual unavailable purposes
    /// refuse. Actual owners include the source-selected minimum diagnostic.
    ///
    /// # Errors
    /// Complete first Host cause, original primitive Guest, or command minimum.
    fn original_list_stride(
        &mut self,
        _original: &Self::Value,
        _command: crate::native_option_index::OptionCommand,
    ) -> Result<Option<i64>, CmdError> {
        Ok(None)
    }

    /// Read the same original numeric pattern/key through the independently
    /// selected search/sort Int/Long/Wide/Double owner. None denotes only an
    /// explicit neutral adapter; physical unavailable purposes refuse.
    fn original_list_number(
        &mut self,
        _original: &Self::Value,
        _command: crate::native_option_index::OptionCommand,
        _kind: ListNumericKind,
    ) -> Result<Option<ListNumber>, CmdError> {
        Ok(None)
    }

    /// C84's `SortElement` record holds every original input member before the
    /// first comparator. This temporary role is distinct from a result hold.
    fn index_c84_sort_member_hold(
        &mut self,
        _original: &Self::Value,
    ) -> Result<Self::IndexKeyOwner, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C84 sort member hold").into())
    }

    /// Actual C85+ command List copy, retaining its genuine temporary hold.
    fn index_sort_copy(
        &mut self,
        _original: &Self::Value,
    ) -> Result<Self::IndexKeyOwner, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original sort List copy").into())
    }

    /// Duplicate the genuine prefix or selected Jim input without String rebuilding.
    /// `only_shared` samples the original receiver before a working hold.
    fn index_sort_duplicate(
        &mut self,
        _original: &Self::Value,
        _only_shared: bool,
    ) -> Result<Self::IndexKeyOwner, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original sort duplication").into())
    }

    /// Construct the actual group/parent List; `held` is its source-selected +1.
    fn index_sort_list(
        &mut self,
        _members: Vec<Self::Value>,
        _held: bool,
    ) -> Result<Self::IndexKeyOwner, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original sort List constructor").into())
    }

    /// Append through the existing checked List worker on the selected header.
    fn index_sort_append(
        &mut self,
        _list: &Self::IndexKeyOwner,
        _members: &[Self::Value],
    ) -> Result<(), CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original sort List append").into())
    }

    /// Replace through the existing checked range worker without another COW test.
    fn index_sort_replace(
        &mut self,
        _list: &Self::IndexKeyOwner,
        _first: usize,
        _delete: usize,
        _members: &[Self::Value],
    ) -> Result<(), CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original sort List replacement").into())
    }

    /// True only for the actual selected Jim command producer, independent of
    /// neutral Index grammar. This read-only fact grants no host or getter.
    fn index_jim_sort_selected(&self) -> bool {
        false
    }

    /// Reach the actual local C sorting producer with original operation checks.
    /// Comparison errors remain complete caller-owned completions. This default
    /// cannot claim a native producer; neutral consumers use their pure route.
    fn index_native_sort<E>(
        &mut self,
        _indices: &mut [usize],
        _compare: &mut impl FnMut(&mut Self, usize, usize) -> Result<core::cmp::Ordering, E>,
    ) -> Result<(), NativeSortError<E>>
    where
        Self: Sized,
    {
        Err(NativeSortError::Operation(
            ValueError::CommandProtocolUnavailable("original local C list sorting producer").into(),
        ))
    }

    /// Original C `ListIndex` worker, including its source-selected owner/copy order.
    /// Neutral and Jim adapters return None; actual C adapters cannot reparse
    /// their original headers through the neutral Unicode fallback.
    fn original_c_list_index(
        &mut self,
        _list: &Self::Value,
        _indices: &[Self::Value],
        _path: bool,
    ) -> Result<Option<IndexResult<Self::Value>>, CmdError> {
        Ok(None)
    }

    /// Original C `ListRange` worker and actual returned ownership transport.
    fn original_c_list_range(
        &mut self,
        _list: &Self::Value,
        _first: &Self::Value,
        _last: &Self::Value,
    ) -> Result<Option<IndexResult<Self::Value>>, CmdError> {
        Ok(None)
    }

    /// Actual selected C release, independent of neutral lexical grammar.
    fn index_c_version(&self) -> Option<TclVersion> {
        None
    }

    /// Resolve the SAME original C index header, including its native caches.
    /// None is the explicit neutral adapter, never a native parser fallback.
    ///
    /// # Errors
    /// Selected numeric/offset worker failure or full original Host refusal.
    fn original_c_index(
        &mut self,
        _original: &Self::Value,
        _length: usize,
    ) -> Result<Option<i64>, CmdError> {
        Ok(None)
    }

    /// Reach original C Unicode storage under the retained actual operation.
    ///
    /// # Errors
    /// Selected native unit producer/cache or full original Host refusal.
    fn index_c_unicode_units(
        &mut self,
        original: &Self::Value,
    ) -> Result<std::rc::Rc<[u32]>, CmdError> {
        self.native_unicode_units(original).map_err(Into::into)
    }

    /// Actual original pure `ByteArray` eligibility at the reached C stage.
    /// C9 proper binary backing remains eligible with a resident String.
    /// Snapshots grant no producer purpose.
    fn index_c_pure_binary(
        &mut self,
        _original: &Self::Value,
    ) -> Result<Option<std::rc::Rc<[u8]>>, CmdError> {
        Ok(None)
    }

    /// Sharing of the exact original, before any working-reference acquisition.
    fn index_c_original_is_shared(&mut self, _original: &Self::Value) -> Result<bool, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C sharing").into())
    }

    /// Original duplicate constructor, with source context captured before its
    /// reached callbacks and full refusal retained before unused fresh cleanup.
    fn index_c_duplicate_original(
        &mut self,
        _original: &Self::Value,
    ) -> Result<Self::Value, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C duplicate constructor").into())
    }

    /// Genuine fresh binary constructor selected for this original C command.
    fn index_c_binary_result(
        &mut self,
        _bytes: std::rc::Rc<[u8]>,
    ) -> Result<Self::Value, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C binary result constructor").into())
    }

    /// Checked same-original binary splice; allocation leases add no native +1.
    fn index_c_binary_splice(
        &mut self,
        _original: &Self::Value,
        _bytes: std::rc::Rc<[u8]>,
    ) -> Result<(), CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original C binary splice").into())
    }

    /// Genuine fresh C Unicode constructor, independent of host Unicode text.
    ///
    /// # Errors
    /// The selected native constructor or original operation is unavailable.
    fn index_c_unicode_result(
        &mut self,
        units: std::rc::Rc<[u32]>,
        version: TclVersion,
    ) -> Result<Self::Value, CmdError> {
        self.native_unicode_string_result(units, version)
            .map_err(Into::into)
    }

    /// Append an original operand to the privately created native result.
    /// No copy-on-write decision or native-reference grant is supplied here.
    ///
    /// # Errors
    /// Reached native append/getter/cache or full original Host refusal.
    fn index_c_append_object(
        &mut self,
        _result: &Self::Value,
        _original: &Self::Value,
    ) -> Result<(), CmdError> {
        Err(ValueError::CommandProtocolUnavailable("private original C String append").into())
    }

    /// Append reached native Unicode units to the privately created result.
    ///
    /// # Errors
    /// Selected native Unicode append or full original Host refusal.
    fn index_c_append_unicode(
        &mut self,
        _result: &Self::Value,
        _units: &[u32],
    ) -> Result<(), CmdError> {
        Err(ValueError::CommandProtocolUnavailable("private original C Unicode append").into())
    }

    /// Checked original spelling for a reached index diagnostic.
    ///
    /// # Errors
    /// Original getter, header or operation refusal.
    fn index_operand_bytes(
        &mut self,
        original: &Self::Value,
    ) -> Result<std::rc::Rc<[u8]>, CmdError> {
        self.native_string_bytes(original).map_err(Into::into)
    }

    /// Apply the native getter to an explicitly fresh textual argument. This
    /// separate door grants no relationship to another original header/cache.
    ///
    /// # Errors
    /// The same selected physical worker failures as `original_jim_index`.
    fn textual_jim_index(&mut self, _source: &[u8]) -> Result<Option<i32>, CmdError> {
        Ok(None)
    }
}

/// The actual return constructor records ownership instead of inferring it
/// from spelling, native counts, cache state or pointer equality.
pub enum IndexResult<V> {
    /// An original argument or selected member: no fresh native object role.
    Borrowed(V),
    /// A reached constructor: retire its fresh result on operation refusal.
    Created(V),
    /// The exact selected child detached from backing created by this worker.
    /// Its live-zero result ownership is explicit, independent of construction.
    Detached(V),
    /// Existing C ListIndex/Range owner transferred by its actual returned-owner API.
    /// This role supplies no reference to the original input during getters.
    ReturnedOwnedTransport(V),
}

/// Finish the original operation before classifying a command outcome. Any
/// reached fresh result is retired before the final post-cleanup check; the
/// first refusal remains the reported cause even if retirement also refuses.
pub(crate) fn finish_index_result<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    reached: Result<IndexResult<O::Value>, CmdError>,
) -> Result<O::Value, CmdError> {
    if let Err(first) = ops.check_index_operation(original) {
        match reached {
            Ok(
                IndexResult::Created(created)
                | IndexResult::Detached(created)
                | IndexResult::ReturnedOwnedTransport(created),
            ) => ops.discard_owned_index_result(created),
            Ok(IndexResult::Borrowed(borrowed)) => drop(borrowed),
            Err(_) => {}
        }
        // Mandatory cleanup cannot replace the first original cause.
        let _ = ops.check_index_operation(original);
        return Err(first);
    }
    reached.map(|result| match result {
        IndexResult::Created(value)
        | IndexResult::Detached(value)
        | IndexResult::ReturnedOwnedTransport(value)
        | IndexResult::Borrowed(value) => value,
    })
}

pub(crate) fn run_index_result<O: NativeIndexOps>(
    ops: &mut O,
    worker: impl FnOnce(&mut O) -> Result<IndexResult<O::Value>, CmdError>,
) -> Result<O::Value, CmdError> {
    run_index_result_using(ops, |ops, _| worker(ops))
}

pub(crate) fn run_index_result_using<O: NativeIndexOps>(
    ops: &mut O,
    worker: impl FnOnce(&mut O, &O::IndexOperation) -> Result<IndexResult<O::Value>, CmdError>,
) -> Result<O::Value, CmdError> {
    let original = ops.enter_index_operation()?;
    let reached = worker(ops, &original);
    finish_index_result(ops, &original, reached)
}

pub(crate) fn run_created_index_result<O: NativeIndexOps>(
    ops: &mut O,
    worker: impl FnOnce(&mut O) -> Result<O::Value, CmdError>,
) -> Result<O::Value, CmdError> {
    run_index_result(ops, |ops| worker(ops).map(IndexResult::Created))
}

/// Result-shape owner supplies the exact cleanup for its reached output. This
/// works for compound command receipts without guessing native ownership.
pub(crate) fn run_index_output<O: NativeIndexOps, T, E>(
    ops: &mut O,
    worker: impl FnOnce(&mut O) -> Result<T, E>,
    discard: impl FnOnce(&mut O, T),
    mut host_error: impl FnMut(CmdError) -> E,
) -> Result<T, E> {
    let original = ops.enter_index_operation().map_err(&mut host_error)?;
    let reached = worker(ops);
    if let Err(first) = ops.check_index_operation(&original) {
        if let Ok(output) = reached {
            discard(ops, output);
        }
        let _ = ops.check_index_operation(&original);
        return Err(host_error(first));
    }
    reached
}

/// Construct a selected integer using the already entered command operation.
/// A reached constructor is retired on refusal before the final original check.
///
/// # Errors
/// Selected construction or the retained first original execution refusal.
pub fn integer_result<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::IndexOperation,
    integer: i64,
) -> Result<O::Value, CmdError> {
    ops.check_index_operation(original)?;
    let reached = ops.index_integer_result(original, integer);
    if let Err(first) = &reached {
        ops.retain_index_failure(first);
    }
    if let Err(first) = ops.check_index_operation(original) {
        ops.retain_index_failure(&first);
        if let Ok(created) = reached {
            ops.discard_owned_index_result(created);
        }
        let _ = ops.check_index_operation(original);
        return Err(first);
    }
    reached
}

/// Checked metadata has no newly created native result to retire.
pub(crate) fn run_index_metadata<O: NativeIndexOps, T>(
    ops: &mut O,
    worker: impl FnOnce(&mut O) -> Result<T, CmdError>,
) -> Result<T, CmdError> {
    let original = ops.enter_index_operation()?;
    let reached = worker(ops);
    if let Err(error) = &reached {
        ops.retain_index_failure(error);
    }
    ops.check_index_operation(&original)?;
    reached
}

/// Read selected original bytes under the existing operation and retain an
/// intrinsic first cause before the caller releases its key, view or temporary.
/// Neutral adapters use their explicitly neutral byte implementation.
///
/// # Errors
/// Original selected String getter or complete original operation refusal.
pub fn checked_operand_bytes<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
) -> Result<std::rc::Rc<[u8]>, CmdError> {
    run_index_metadata(ops, |ops| ops.index_operand_bytes(original))
}

/// Read a reached scalar stride without reparsing original bytes as an Index.
/// None permits only the explicit neutral lexical adapter.
///
/// # Errors
/// Original scalar primitive, source-selected command minimum, or first Host cause.
pub fn original_list_stride<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    command: crate::native_option_index::OptionCommand,
) -> Result<Option<i64>, CmdError> {
    run_index_metadata(ops, |ops| ops.original_list_stride(original, command))
}

/// Read the original search/sort primitive without a preliminary String getter.
/// Neutral callers alone receive None and can choose their independent parser.
///
/// # Errors
/// Exact original primitive Guest or complete original Host cause.
pub fn original_list_number<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    command: crate::native_option_index::OptionCommand,
    kind: ListNumericKind,
) -> Result<Option<ListNumber>, CmdError> {
    run_index_metadata(ops, |ops| ops.original_list_number(original, command, kind))
}

/// Shared entered-operation bridge to the independently supplied actual host.
/// The host sorts metadata only; every reached comparator checks the same
/// original currency before and after, and no subsequent getter runs on refusal.
/// Callers retain intrinsic full causes before returning comparator completion.
pub fn native_sort_with_host<O: NativeIndexOps, E>(
    ops: &mut O,
    environment: &dyn tcl_platform::NativeListSortingEnvironment,
    indices: &mut [usize],
    compare: &mut impl FnMut(&mut O, usize, usize) -> Result<core::cmp::Ordering, E>,
) -> Result<(), NativeSortError<E>> {
    let original = ops
        .enter_index_operation()
        .map_err(NativeSortError::Operation)?;
    let mut first = None;
    let sorted = environment.sort_indices(indices, &mut |left, right| {
        if let Err(error) = ops.check_index_operation(&original) {
            ops.retain_index_failure(&error);
            first = Some(NativeSortError::Operation(error));
            return Err(());
        }
        let reached = compare(ops, left, right);
        if let Err(error) = ops.check_index_operation(&original) {
            ops.retain_index_failure(&error);
            first = Some(NativeSortError::Operation(error));
            return Err(());
        }
        match reached {
            Ok(order) => Ok(order),
            Err(error) => {
                first = Some(NativeSortError::Comparison(error));
                Err(())
            }
        }
    });
    ops.check_index_operation(&original)
        .map_err(NativeSortError::Operation)?;
    if let Some(error) = first {
        return Err(error);
    }
    sorted.map_err(|_| {
        NativeSortError::Operation(
            ValueError::CommandProtocolUnavailable(
                "original local C sorting producer or error frontier",
            )
            .into(),
        )
    })
}

/// Convert Jim's encoded absolute/end-relative index against this container.
#[must_use]
pub fn resolve_jim_encoding(encoded: i32, len: usize) -> i64 {
    if encoded < 0 && encoded != -i32::MAX {
        i64::try_from(len)
            .unwrap_or(i64::MAX)
            .wrapping_add(i64::from(encoded))
    } else {
        i64::from(encoded)
    }
}
use tcl_syntax::number::{Number, NumberSyntax, ParseFlags, Radix};

/// One index word as a release reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexReading {
    /// The index the word names, negative or past the end as it falls.
    At(i64),
    /// No index: the release raises `bad index`.
    Bad,
    /// The index where the host's C `long` is 64 bits and `bad index` where
    /// it is 32 bits — 64-bit Windows and every 32-bit build.
    HostLong(i64),
    /// A word 8.5 reads as [`Self::HostLong`] and 8.6 raises on, read under
    /// the numeral grammar the two releases share with neither named.
    Unsure,
}

impl IndexReading {
    /// The index a runtime on this host reads: C Tcl's on the same host.
    #[must_use]
    pub fn on_this_host(self) -> Option<i64> {
        match self {
            Self::At(index) => Some(index),
            Self::HostLong(index) if tcl_syntax::value::host_long_is_wide() => Some(index),
            Self::HostLong(_) | Self::Bad | Self::Unsure => None,
        }
    }

    fn map(self, f: impl FnOnce(i64) -> i64) -> Self {
        match self {
            Self::At(value) => Self::At(f(value)),
            Self::HostLong(value) => Self::HostLong(f(value)),
            other => other,
        }
    }

    fn zip(self, other: Self, f: impl FnOnce(i64, i64) -> i64) -> Self {
        match (self, other) {
            (Self::Bad, _) | (_, Self::Bad) => Self::Bad,
            (Self::Unsure, _) | (_, Self::Unsure) => Self::Unsure,
            (Self::At(a), Self::At(b)) => Self::At(f(a, b)),
            (Self::At(a) | Self::HostLong(a), Self::At(b) | Self::HostLong(b)) => {
                Self::HostLong(f(a, b))
            }
        }
    }
}

/// Resolve an index `spec` against a container length `len` (so `end` is
/// `len - 1`) under the numeral grammar `numbers`, as a runtime on this host
/// reads it. The result may be negative or `>= len`; callers clamp per their
/// command's rules. Errors with the canonical message on an unparseable spec.
/// The numeral grammar 8.5 and 8.6 share reads as 8.6 here.
///
/// # Errors
/// `bad index` for a spec `numbers` does not read as an index on this host.
pub fn resolve_with(spec: &str, len: usize, numbers: NumberSyntax) -> Result<i64, CmdError> {
    read_with(spec, len, numbers)
        .on_this_host()
        .ok_or_else(|| bad_index_with(spec.trim(), numbers))
}

/// [`resolve_with`] under the grammar this thread's interpreter installed
/// (`tcl_syntax::number::runtime_syntax`). Runtime-only: a caller under a
/// stated profile — a registry folder, the compiler, a test running two
/// releases in one process — names its grammar through [`resolve_with`] or
/// [`resolve_opt_with`] instead, since the ambient is whatever the thread
/// last installed.
///
/// # Errors
/// As [`resolve_with`].
pub fn resolve(spec: &str, len: usize) -> Result<i64, CmdError> {
    resolve_with(spec, len, tcl_syntax::number::runtime_syntax())
}

/// Legacy C modern-index compatibility API; use [`resolve_opt_in`] for
/// selected native semantics.
///
/// Resolve an index `spec` against `len`, returning `None` (rather than an
/// error) on an unparseable spec — the `lsearch`/`lsort -index` driving needs the
/// raw option to classify "bad index" vs "out of range" itself. Runtime-only,
/// as [`resolve`] is.
#[must_use]
pub fn resolve_opt(spec: &str, len: usize) -> Option<i64> {
    read_with(spec, len, tcl_syntax::number::runtime_syntax()).on_this_host()
}

/// [`resolve_opt`] reading the index under an explicitly named numeral
/// grammar, for a compile-time caller whose target need not be the release
/// this process was built for: `None` also for a spec whose reading turns on
/// the host or on which release the grammar stands for, which
/// [`read_with`] tells apart.
#[must_use]
pub fn resolve_opt_with(spec: &str, len: usize, numbers: NumberSyntax) -> Option<i64> {
    match read_with(spec, len, numbers) {
        IndexReading::At(index) => Some(index),
        IndexReading::Bad | IndexReading::HostLong(_) | IndexReading::Unsure => None,
    }
}

/// One index `spec` against `len` as the numeral grammar `numbers` reads it;
/// the grammar 8.5 and 8.6 share names neither release.
#[must_use]
pub fn read_with(spec: &str, len: usize, numbers: NumberSyntax) -> IndexReading {
    read(spec, len, numbers, Range::of_numbers(numbers))
}

/// One index `spec` against `len` as `release` reads it.
#[must_use]
pub fn read_under(spec: &str, len: usize, release: TclVersion) -> IndexReading {
    read(
        spec,
        len,
        release.number_syntax(),
        Range::of_release(release),
    )
}

/// Whether `release` reads `spec` apart when it is a literal its compiler
/// encodes: 8.6 compiles a literal index of `string index`, `string range`,
/// `lindex` and `lrange` through `TclIndexEncode`, which places an `end`
/// offset whose sum with `end` passes the 32-bit `int` after the end, where
/// the same index read at run time wraps before the first element (measured,
/// tclsh 8.6.18: `string range abcdefghijkl 0 end+2147483647` is the whole
/// string, and the empty one with the index in a variable).
#[must_use]
pub fn compiles_apart(spec: &str, len: usize, release: TclVersion) -> bool {
    release == TclVersion::V8_6
        && spec.starts_with('e')
        && spec
            .strip_prefix("end")
            .and_then(|rest| signed_operand(rest, release.number_syntax()))
            .is_some_and(|(negative, number)| {
                let offset = match int_32(&number, Range::Tcl86) {
                    IndexReading::At(value) if negative => wrap_32(-value),
                    IndexReading::At(value) => value,
                    _ => return false,
                };
                offset > 0 && end_32(len) + offset > i64::from(i32::MAX)
            })
}

/// A literal index word fixed ahead of the length it indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoded {
    /// This index, zero or more.
    Index(i64),
    /// `end` plus this offset.
    FromEnd(i64),
    /// Before the first element, whatever the length.
    Before,
    /// After the last element, whatever the length.
    After,
}

/// The literal index word `spec` as `release` reads it whatever the length,
/// `None` for a word it reads apart by length or by host, or not at all.
///
/// `compiled` says whether the release compiles the word with its command, as
/// it does a literal in any body it compiles, rather than reading it as the
/// command runs: 8.6 compiles an index of `string range`, `lindex` and `lrange`
/// through `TclIndexEncode`, which places an `end` offset that is positive
/// after the 32-bit wrap after the end, and one below `INT_MIN + 2` before the
/// first element, whatever the length. Read as the command runs, 8.4 to 8.6
/// sum `end` and its offset in an `int`, so a positive offset passes
/// `INT_MAX` and wraps for a long enough value, and that word has no fixed
/// reading. Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: see
/// `a_literal_index_is_fixed_as_each_release_compiles_or_reads_it`.
#[must_use]
pub fn encode(spec: &str, release: TclVersion, compiled: bool) -> Option<Encoded> {
    let at = |len| match read_under(spec, len, release) {
        IndexReading::At(index) => Some(index),
        IndexReading::Bad | IndexReading::HostLong(_) | IndexReading::Unsure => None,
    };
    let (one, two) = (at(1)?, at(2)?);
    if one == two {
        return Some(if one < 0 {
            Encoded::Before
        } else {
            Encoded::Index(one)
        });
    }
    // An `end` word: against one element `end` is 0, so the reading is the
    // offset, as the release reads it.
    let offset = one;
    let int_min = i64::from(i32::MIN);
    if compiled && release == TclVersion::V8_6 {
        return Some(if offset > 0 {
            Encoded::After
        } else if offset < int_min + 2 {
            Encoded::Before
        } else {
            Encoded::FromEnd(offset)
        });
    }
    let sums_in_int = matches!(
        release,
        TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6
    );
    // An `int` sum holds for every length only from `INT_MIN + 1` to zero: a
    // positive offset wraps for a long enough value, and `INT_MIN` for an
    // empty one.
    (!sums_in_int || (int_min < offset && offset <= 0)).then_some(Encoded::FromEnd(offset))
}

/// Whether `release` compiles the literal index word `spec` to a reading apart
/// from the one its command makes as it runs ([`encode`]), for some length: 8.6
/// places an `end` offset that passes `INT_MAX` after the end, where its
/// command wraps it before the first element.
#[must_use]
pub fn encodes_apart(spec: &str, release: TclVersion) -> bool {
    encode(spec, release, true) != encode(spec, release, false)
}

/// Whether `release`'s compiler reads some literal index word apart from its
/// command ([`encodes_apart`]), so that a runtime evaluating a command it
/// compiles needs its literal words: 8.6's `TclIndexEncode` alone.
#[must_use]
pub const fn compiles_literals_apart(release: TclVersion) -> bool {
    matches!(release, TclVersion::V8_6)
}

/// A command whose literal index words 8.6's compiler encodes
/// (`TclGetIndexFromToken`), each by the rule of its compile procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexCompiler {
    /// `lindex list index`, three words only (`TclCompileLindexCmd`).
    Lindex,
    /// `lrange list first last`, both literal (`TclCompileLrangeCmd`).
    Lrange,
    /// `linsert list index ?element ...?` (`TclCompileLinsertCmd`).
    Linsert,
    /// `lreplace list first last ?element ...?`, both literal and in an
    /// order the compiler can settle (`TclCompileLreplaceCmd`).
    Lreplace,
    /// `string range string first last` (`TclCompileStringRangeCmd`): a
    /// `first` after the end settles the command alone, else both literal.
    StringRange,
    /// `string replace string first last ?newString?`, both literal and as
    /// [`string_replace_compiles_encoded`] says (`TclCompileStringReplaceCmd`).
    StringReplace {
        /// Whether the command has a replacement string.
        replacing: bool,
    },
}

impl IndexCompiler {
    /// How many leading words select the compile procedure: the command word,
    /// and an ensemble's subcommand word (`TclCompileEnsemble`). The release
    /// compiles the command only where each is one literal word; a
    /// substituted one leaves it to the generic invocation, which reads every
    /// index word as the command runs.
    #[must_use]
    pub const fn selecting_words(self) -> usize {
        match self {
            Self::StringRange | Self::StringReplace { .. } => 2,
            Self::Lindex | Self::Lrange | Self::Linsert | Self::Lreplace => 1,
        }
    }
}

/// The index words of a command `compiler` compiles, each respelt where
/// `release` compiles it to a reading apart from the one its command makes as
/// it runs ([`encodes_apart`]): `2147483647` after the end and `-1` before
/// the first element, which every release's command reads so whatever the
/// length. `words` holds each index word's text where it is a literal and
/// `None` where it is substituted; the answer is aligned with it, `None`
/// where the word is passed as written.
#[must_use]
pub fn compiled_index_words(
    compiler: IndexCompiler,
    words: &[Option<&str>],
    release: TclVersion,
) -> Vec<Option<&'static str>> {
    let encoded: Vec<Option<Encoded>> = words
        .iter()
        .map(|word| word.and_then(|word| encode(word, release, true)))
        .collect();
    let all = encoded.iter().all(Option::is_some);
    let settled: Vec<bool> = match (compiler, encoded.as_slice()) {
        (IndexCompiler::StringRange, [Some(Encoded::After), _]) => vec![true, false],
        (IndexCompiler::Lreplace, [Some(first), Some(last)]) => {
            vec![lreplace_compiles_encoded(*first, *last); 2]
        }
        (IndexCompiler::StringReplace { replacing }, [Some(first), Some(last)]) => {
            vec![string_replace_compiles_encoded(*first, *last, replacing); 2]
        }
        (IndexCompiler::Lreplace | IndexCompiler::StringReplace { .. }, _) => {
            vec![false; words.len()]
        }
        _ => vec![all; words.len()],
    };
    words
        .iter()
        .zip(encoded)
        .zip(settled)
        .map(|((word, encoded), settled)| {
            if !settled || !word.is_some_and(|word| encodes_apart(word, release)) {
                return None;
            }
            match encoded? {
                Encoded::After => Some("2147483647"),
                Encoded::Before => Some("-1"),
                Encoded::Index(_) | Encoded::FromEnd(_) => None,
            }
        })
        .collect()
}

/// `TclIndexEncode`'s spaces: `TCL_INDEX_END` is -2 with `end-N` below it,
/// `TCL_INDEX_BEFORE` -1, `TCL_INDEX_START` 0 and `TCL_INDEX_AFTER` `INT_MAX`.
const ENCODED_BEFORE: i64 = -1;
const ENCODED_END: i64 = -2;
const ENCODED_START: i64 = 0;
const ENCODED_AFTER: i64 = i32::MAX as i64;

/// `encoded` in `TclIndexEncode`'s space, with what `before` and `after` the
/// first element and the end stand for.
fn encoded_value(encoded: Encoded, before: i64, after: i64) -> i64 {
    match encoded {
        Encoded::Index(index) if index >= ENCODED_AFTER => after,
        Encoded::Index(index) => index,
        Encoded::FromEnd(offset) => ENCODED_END + offset,
        Encoded::Before => before,
        Encoded::After => after,
    }
}

/// Whether 8.6's `TclCompileLreplaceCmd` compiles `lreplace` on its encoded
/// indices: where it can tell which of `first` and `last + 1` the kept suffix
/// starts at.
#[must_use]
pub fn lreplace_compiles_encoded(first: Encoded, last: Encoded) -> bool {
    let first = encoded_value(first, ENCODED_START, ENCODED_AFTER);
    let last = encoded_value(last, ENCODED_BEFORE, ENCODED_END);
    first == ENCODED_AFTER
        || last == ENCODED_BEFORE
        || last == ENCODED_END
        || (last < ENCODED_END && first <= ENCODED_END)
        || (last >= ENCODED_START && first >= ENCODED_START)
}

/// Whether 8.6's `TclCompileStringReplaceCmd` compiles `string replace` on
/// its encoded indices, rather than falling through to `INST_STR_REPLACE`
/// over the words as written: always without a replacement string, and with
/// one only for a no-op it can prove, `first` before the first element, or
/// `last` after the end with `first` relative to `end`.
#[must_use]
pub fn string_replace_compiles_encoded(first: Encoded, last: Encoded, replacing: bool) -> bool {
    let first = encoded_value(first, ENCODED_BEFORE, ENCODED_AFTER);
    let last = encoded_value(last, ENCODED_BEFORE, ENCODED_AFTER);
    let no_op = last == ENCODED_BEFORE
        || first == ENCODED_AFTER
        || (first <= ENCODED_END && last <= ENCODED_END && last < first)
        || (first >= ENCODED_START && last >= ENCODED_START && last < first);
    no_op
        || !replacing
        || (first == ENCODED_BEFORE && last >= ENCODED_START)
        || (last == ENCODED_AFTER && first <= ENCODED_END)
}

/// How a release's `TclCompileStringReplaceCmd` settles a `string replace`
/// on its literal indices, so what the compiled command reads and returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringReplaceSettlement {
    /// A replacement it proves changes nothing: the string itself, unread.
    Unchanged,
    /// The whole string replaced: the replacement itself, unread.
    Replacement,
    /// The whole string removed: the empty string.
    Empty,
    /// The kept part read by character (`INST_STR_RANGE_IMM`) and joined.
    Ranges,
    /// Left to `INST_STR_REPLACE` over the words as written.
    Runtime,
}

/// [`StringReplaceSettlement`] for the indices `first` and `last` as the
/// release encodes them (`None` for a word it does not settle), with a
/// replacement string where `replacing` says; it settles exactly where
/// [`string_replace_compiles_encoded`] says it compiles.
#[must_use]
pub fn settle_string_replace(
    first: Option<Encoded>,
    last: Option<Encoded>,
    replacing: bool,
) -> StringReplaceSettlement {
    let (Some(first), Some(last)) = (first, last) else {
        return StringReplaceSettlement::Runtime;
    };
    if !string_replace_compiles_encoded(first, last, replacing) {
        return StringReplaceSettlement::Runtime;
    }
    let first = encoded_value(first, ENCODED_BEFORE, ENCODED_AFTER);
    let last = encoded_value(last, ENCODED_BEFORE, ENCODED_AFTER);
    let no_op = last == ENCODED_BEFORE
        || first == ENCODED_AFTER
        || (first <= ENCODED_END && last <= ENCODED_END && last < first)
        || (first >= ENCODED_START && last >= ENCODED_START && last < first);
    if no_op {
        StringReplaceSettlement::Unchanged
    } else if replacing && first == ENCODED_BEFORE && last == ENCODED_AFTER {
        StringReplaceSettlement::Replacement
    } else if !replacing
        && (first == ENCODED_BEFORE || first == ENCODED_START)
        && (last == ENCODED_END || last == ENCODED_AFTER)
    {
        StringReplaceSettlement::Empty
    } else {
        StringReplaceSettlement::Ranges
    }
}

/// A failed `lsearch`/`lsort -index` path step: C's message and, where C sets
/// one, its `-errorcode`.
pub struct IndexPathError {
    pub message: Vec<u8>,
    pub code: Option<&'static [u8]>,
}

/// Why `TclIndexEncode` refuses an `-index` path element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// Not index syntax at all.
    Bad,
    /// An index it cannot encode.
    OutOfRange,
}

/// An `-index` path element as `TclIndexEncode` encodes it: an absolute index,
/// or one `end` plus a non-positive offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathEncoding {
    At(i64),
    FromEnd(i64),
}

impl PathEncoding {
    /// Encode `spec`, or refuse it: out of range is an absolute index outside
    /// `0..=INT_MAX`, `end+N`, or `end-N` with `N` at or past `INT_MAX`
    /// (measured on 64-bit tclsh 9.0.4). End-relativity is told apart by
    /// resolving against two different lengths.
    fn of(spec: &str) -> Result<Self, Refusal> {
        const BIG: usize = 1 << 20;
        let r_big = resolve_opt(spec, BIG + 1).ok_or(Refusal::Bad)?;
        let r_small = resolve_opt(spec, 1).ok_or(Refusal::Bad)?;
        let int_max = i64::from(i32::MAX);
        if r_big == r_small {
            return if (0..=int_max).contains(&r_big) {
                Ok(Self::At(r_big))
            } else {
                Err(Refusal::OutOfRange)
            };
        }
        let offset = r_big - i64::try_from(BIG).unwrap_or(i64::MAX);
        if (1 - int_max..=0).contains(&offset) {
            Ok(Self::FromEnd(offset))
        } else {
            Err(Refusal::OutOfRange)
        }
    }

    /// The index into a level of `len` elements.
    fn decode(self, len: usize) -> i64 {
        match self {
            Self::At(index) => index,
            Self::FromEnd(offset) => i64::try_from(len).unwrap_or(i64::MAX) - 1 + offset,
        }
    }
}

/// Check every `-index` path element encodes (`TclIndexEncode` at option
/// parse): a syntactically bad spec is `bad index`, an unencodable one `index
/// "…" out of range`.
///
/// # Errors
/// The first element that does not encode.
pub fn check_index_path(path: &[Vec<u8>]) -> Result<(), IndexPathError> {
    for spec in path {
        PathEncoding::of(&String::from_utf8_lossy(spec))
            .map_err(|refusal| path_error(spec, refusal))?;
    }
    Ok(())
}

fn path_error(spec: &[u8], refusal: Refusal) -> IndexPathError {
    match refusal {
        Refusal::Bad => {
            let (message, code) = bad_index_parts(spec);
            IndexPathError { message, code }
        }
        Refusal::OutOfRange => {
            let mut message = b"index \"".to_vec();
            message.extend_from_slice(spec);
            message.extend_from_slice(b"\" out of range");
            IndexPathError {
                message,
                code: Some(b"TCL VALUE INDEX OUTOFRANGE"),
            }
        }
    }
}

/// Drill into a (nested) list `value` by an index `path` (`lsearch`/`lsort
/// -index`) over plain [`ValueOps`]: each spec steps one level. A step past
/// its level is `element <n> missing from sublist "<list>"`, naming the index
/// as `release`'s `SelectObjFromSublists` does ([`failed_step_name`]); a
/// non-list step stops, returning the current value (C's `TclLindexFlat`
/// fallthrough). Upstream's release-aware `drill`; work's native
/// [`drill`]/[`drill_prepared_c`] keep the selected original getters.
///
/// # Errors
/// `bad index` / `out of range`, the list-parse error, or `element … missing
/// from sublist …`.
pub fn drill_in_release<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
    release: TclVersion,
) -> Result<O::Value, IndexPathError> {
    let mut cur = value.clone();
    for spec in path {
        let len = ops.list_len(&cur).map_err(|e| {
            let error = CmdError::from(e).into_byte_details();
            IndexPathError {
                message: error.message,
                code: None,
            }
        })?;
        let encoded = PathEncoding::of(&String::from_utf8_lossy(spec))
            .map_err(|refusal| path_error(spec, refusal))?;
        let idx = encoded.decode(len);
        if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) >= len {
            let named = failed_step_name(encoded, idx, release);
            let mut message = format!("element {named} missing from sublist \"").into_bytes();
            message.extend_from_slice(&ops.as_bytes(&cur));
            message.push(b'"');
            return Err(IndexPathError {
                message,
                code: (release >= TclVersion::V8_6).then_some(b"TCL OPERATION LSORT INDEXFAILED"),
            });
        }
        match ops.list_index(&cur, usize::try_from(idx).unwrap_or(0)) {
            Ok(Some(e)) => cur = e,
            _ => return Ok(cur),
        }
    }
    Ok(cur)
}

/// Resolve an index with the complete selected native policy.
///
/// # Errors
/// A dialect-specific bad-index error when the index is invalid.
pub fn resolve_in(
    spec: &str,
    len: usize,
    syntax: tcl_dialect::IndexSyntax,
) -> Result<i64, CmdError> {
    resolve_opt_in(spec, len, syntax).ok_or_else(|| bad_index_in(spec, syntax))
}

/// Parse an index without discarding its native index grammar.
#[must_use]
pub fn resolve_opt_in(spec: &str, len: usize, syntax: tcl_dialect::IndexSyntax) -> Option<i64> {
    use tcl_dialect::IndexGrammar;
    let len = i64::try_from(len).ok()?;
    if syntax.grammar == IndexGrammar::Jim {
        let (expression, end_relative) = jim_expression(spec)?;
        let grammar = tcl_dialect::LexerGrammar {
            numbers: syntax.numbers,
            expr_comments: tcl_dialect::ExprCommentStyle::None,
            ..tcl_dialect::LexerGrammar::default()
        };
        let tree = tcl_syntax::expr::parser::parse_expr_with_grammar(expression, &grammar);
        let value = tcl_syntax::expr::wide::literal_expression(
            &tree,
            syntax.numbers,
            tcl_dialect::NativeArithmetic::JimWide,
        )
        .ok()?;
        return finish_jim_index(value, end_relative, len);
    }
    if let Some(value) = parse_int_whole(spec, syntax.numbers) {
        return narrow_tcl_integer(value, syntax);
    }
    let (base, rest) = if spec == "end"
        || (syntax.end_abbreviations
            && !spec.is_empty()
            && spec.len() < 3
            && "end".starts_with(spec))
    {
        return Some(len - 1);
    } else if let Some(rest) = spec.strip_prefix("end") {
        (len - 1, rest)
    } else {
        if syntax.grammar == IndexGrammar::Tcl84 {
            return None;
        }
        let (base, rest) = parse_int_prefix(spec.trim_start(), syntax.numbers)?;
        (narrow_tcl_integer(base, syntax)?, rest)
    };
    let connector = *rest.as_bytes().first()?;
    if connector != b'-' && (connector != b'+' || syntax.grammar == IndexGrammar::Tcl84) {
        return None;
    }
    let operand_text = rest.get(1..)?;
    if syntax.grammar != IndexGrammar::Tcl84 && operand_text.chars().next()?.is_whitespace() {
        return None;
    }
    let operand = narrow_tcl_integer(parse_int_whole(operand_text, syntax.numbers)?, syntax)?;
    let value = match (syntax.width, connector) {
        (tcl_dialect::IndexIntegerWidth::Tcl64, b'-') => base.saturating_sub(operand),
        (tcl_dialect::IndexIntegerWidth::Tcl64, _) => base.saturating_add(operand),
        (_, b'-') => base.wrapping_sub(operand),
        _ => base.wrapping_add(operand),
    };
    Some(if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64 {
        value
    } else {
        i64::from(tcl_syntax::number::native_int32_low_bits(value))
    })
}

fn narrow_tcl_integer(value: i64, syntax: tcl_dialect::IndexSyntax) -> Option<i64> {
    if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64 {
        Some(value)
    } else if (-i64::from(u32::MAX)..=i64::from(u32::MAX)).contains(&value) {
        Some(i64::from(tcl_syntax::number::native_int32_low_bits(value)))
    } else {
        None
    }
}

/// Whether a literal C Tcl index fits the native compiler's immediate encoding.
/// The wide releases can depend on the target's container-size width; those
/// ranges retain an unknown result rather than assuming the host compiling us.
#[must_use]
pub fn compiler_encodable_in(spec: &str, syntax: tcl_dialect::IndexSyntax) -> Option<bool> {
    use tcl_dialect::{IndexGrammar, IndexIntegerWidth};
    if syntax.grammar == IndexGrammar::Jim {
        return None;
    }
    let value = resolve_opt_in(spec, 1, syntax);
    if syntax.width == IndexIntegerWidth::Tcl32 {
        return Some(value.is_some());
    }
    let value = value?;
    let end_relative = spec.starts_with("end");
    if end_relative {
        return (value > -i64::from(i32::MAX)).then_some(true);
    }
    if parse_int_whole(spec, syntax.numbers).is_none()
        && parse_int_prefix(spec.trim_start(), syntax.numbers)?.0 < 0
    {
        // The native end-offset internal representation also distinguishes
        // arithmetic whose original base is negative.
        return None;
    }
    (value <= i64::from(i32::MAX) || value >= i64::MAX - 1).then_some(true)
}

/// Compile-known list-index encoding after native token-shape validation.
/// `Ok(None)` means the native compiler evaluates the original index object;
/// `Err` retains target-width uncertainty rather than choosing that path.
///
/// # Errors
/// Reports unavailable native container-size width for an immediate operand.
pub fn compiled_list_index_in(
    spec: &str,
    version: tcl_dialect::TclVersion,
) -> Result<Option<tcl_syntax::native_compiled_index::NativeCompiledListIndex>, ValueError> {
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex as Index;
    let syntax = tcl_dialect::IndexSyntax::for_version(version);
    if version == tcl_dialect::TclVersion::V8_4 {
        return Ok(None);
    }
    if version == tcl_dialect::TclVersion::V8_5 {
        return Ok(parse_int_whole(spec, syntax.numbers)
            .and_then(|value| narrow_tcl_integer(value, syntax))
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value >= 0)
            .map(Index::from_encoded));
    }
    // C9's positive end-offset conversion does not retain an encodable
    // negative-base primary. The member compiler evaluates that index object.
    if version >= tcl_dialect::TclVersion::V9_0
        && spec.starts_with("end")
        && resolve_opt_in(spec, 1, syntax).is_some_and(|value| value > 0)
    {
        return Ok(None);
    }
    compiled_list_bound_in(spec, version, -1, -1)
}

/// Encode a compile-known list coordinate with its native boundary policy.
/// The native range and insertion compilers clamp before/after coordinates
/// differently from a member lookup. This does not construct an index object.
///
/// # Errors
/// Reports native container-width uncertainty rather than choosing an opcode.
pub fn compiled_list_bound_in(
    spec: &str,
    version: tcl_dialect::TclVersion,
    before: i32,
    after: i32,
) -> Result<Option<tcl_syntax::native_compiled_index::NativeCompiledListIndex>, ValueError> {
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex as Index;
    let syntax = tcl_dialect::IndexSyntax::for_version(version);
    if version < tcl_dialect::TclVersion::V8_6 {
        return Ok(None);
    }
    let Some(value) = resolve_opt_in(spec, 1, syntax) else {
        return Ok(None);
    };
    if compiler_encodable_in(spec, syntax).is_none() {
        return Err(ValueError::CommandProtocolUnavailable(
            "compiled index target width",
        ));
    }
    let encoded = if spec.starts_with("end") {
        if value > 0 {
            after
        } else if value < i64::from(i32::MIN) + 2 {
            before
        } else {
            i32::try_from(value - 2).expect("native encoded end offset")
        }
    } else if value < 0 {
        before
    } else if (version < tcl_dialect::TclVersion::V9_0 && value == i64::from(i32::MAX))
        || value > i64::from(i32::MAX)
    {
        after
    } else {
        i32::try_from(value).expect("native encoded absolute index")
    };
    Ok(Some(Index::from_encoded(encoded)))
}

/// Extract Jim's safe integer expression, retaining `end` relativity.
#[must_use]
pub fn jim_expression(spec: &str) -> Option<(&str, bool)> {
    if let Some(rest) = spec.strip_prefix("end") {
        if rest.is_empty() {
            return Some(("0", true));
        }
        matches!(rest.as_bytes().first(), Some(b'+' | b'-')).then_some((rest, true))
    } else {
        Some((spec, false))
    }
}

/// Convert Jim's evaluated wide integer to its encoded native index.
#[must_use]
pub fn finish_jim_index(value: i64, end_relative: bool, length: i64) -> Option<i64> {
    if end_relative {
        if value > 0 {
            return Some(i64::from(i32::MAX));
        }
        let encoded = tcl_syntax::number::native_int32_low_bits(value.wrapping_sub(1));
        Some(length.wrapping_add(i64::from(encoded)))
    } else if value < 0 {
        Some(-i64::from(i32::MAX))
    } else {
        (value <= i64::from(i32::MAX)).then_some(value)
    }
}

/// Resolve through a runtime value adapter, including Jim's safe expressions.
///
/// # Errors
/// A bad-index error, or an unselected native policy.
pub fn resolve_for_ops<O: NativeIndexOps>(
    ops: &mut O,
    spec: &str,
    len: usize,
) -> Result<i64, CmdError> {
    resolve_for_ops_with(ops, spec, len, None)
}

/// [`resolve_for_ops`] reading a Tcl (non-Jim) index's integers under
/// `numbers` where the caller names the grammar (a runtime its pinned
/// release's, a folder the target's) rather than the ops' own.
///
/// # Errors
/// As [`resolve_for_ops`].
pub fn resolve_for_ops_with<O: NativeIndexOps>(
    ops: &mut O,
    spec: &str,
    len: usize,
    numbers: Option<NumberSyntax>,
) -> Result<i64, CmdError> {
    run_index_metadata(ops, |ops| {
        let mut syntax = ops
            .index_syntax()
            .ok_or_else(|| CmdError::new("container index dialect is not selected"))?;
        if syntax.grammar != tcl_dialect::IndexGrammar::Jim
            && let Some(numbers) = numbers
        {
            syntax.numbers = numbers;
            // A caller naming 8.4's numerals reads 8.4's index grammar too
            // (`integer or end?-integer?`, its sole bad-index wording).
            if numbers == NumberSyntax::Tcl84 {
                syntax.grammar = tcl_dialect::IndexGrammar::Tcl84;
            }
        }
        if syntax.grammar != tcl_dialect::IndexGrammar::Jim {
            if let Some(value) = resolve_opt_in(spec, len, syntax) {
                return Ok(value);
            }
            if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64
                && let Some((expression, connector)) = wide_expression(spec, syntax)
            {
                let value = ops.eval_index_expression(expression).map_err(|error| {
                    if error.native_access_refusal().is_some() {
                        CmdError::from(error)
                    } else {
                        bad_index_for_ops(ops, spec, syntax)
                    }
                })?;
                let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
                return Ok(match connector {
                    Some(b'-') => end.saturating_sub(value),
                    Some(_) => end.saturating_add(value),
                    None => value,
                });
            }
            return Err(bad_index_for_ops(ops, spec, syntax));
        }
        if let Some(encoded) = ops.textual_jim_index(spec.as_bytes())? {
            return Ok(resolve_jim_encoding(encoded, len));
        }
        let (expression, relative) =
            jim_expression(spec).ok_or_else(|| bad_index_for_ops(ops, spec, syntax))?;
        let value = ops.eval_index_expression(expression).map_err(|error| {
            if error.native_access_refusal().is_some() {
                CmdError::from(error)
            } else {
                bad_index_for_ops(ops, spec, syntax)
            }
        })?;
        finish_jim_index(value, relative, i64::try_from(len).unwrap_or(i64::MAX))
            .ok_or_else(|| bad_index_for_ops(ops, spec, syntax))
    })
}

pub(crate) fn wide_expression(
    spec: &str,
    syntax: tcl_dialect::IndexSyntax,
) -> Option<(&str, Option<u8>)> {
    let flags = index_int_flags(syntax.numbers);
    let whole_integer = |text: &str| tcl_syntax::number::parse_whole_with(text, flags).is_some();
    if let Some(rest) = spec.strip_prefix("end") {
        let connector = *rest.as_bytes().first()?;
        let operand = rest.get(1..)?;
        if !matches!(connector, b'+' | b'-')
            || operand.chars().next()?.is_whitespace()
            || !whole_integer(operand)
        {
            return None;
        }
        return Some((operand, Some(connector)));
    }
    if whole_integer(spec) {
        return Some((spec, None));
    }
    let trimmed = spec.trim_start();
    let first = tcl_syntax::number::parse(trimmed, flags)?;
    let rest = trimmed.get(first.end..)?;
    let connector = *rest.as_bytes().first()?;
    let operand = rest.get(1..)?;
    (matches!(connector, b'+' | b'-')
        && !operand.chars().next()?.is_whitespace()
        && whole_integer(operand))
    .then_some((spec, None))
}

/// Resolve one runtime value under the selected index policy.
///
/// # Errors
/// The same errors as [`resolve_for_ops`].
pub fn resolve_value_with<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    len: usize,
    numbers: NumberSyntax,
) -> Result<i64, CmdError> {
    run_index_metadata(ops, |ops| {
        if ops
            .index_syntax()
            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
            && let Some(encoded) = ops.original_jim_index(value)?
        {
            return Ok(resolve_jim_encoding(encoded, len));
        }
        if ops.index_c_version().is_some()
            && let Some(resolved) = ops.original_c_index(value, len)?
        {
            return Ok(resolved);
        }
        let spelling = ops.index_unicode(value)?;
        resolve_for_ops_with(ops, &spelling, len, Some(numbers))
    })
}

/// [`resolve_value_with`] under the numeral grammar the selected ops carry.
///
/// # Errors
/// As [`resolve_for_ops`].
pub fn resolve_value<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    len: usize,
) -> Result<i64, CmdError> {
    run_index_metadata(ops, |ops| {
        if ops
            .index_syntax()
            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
            && let Some(encoded) = ops.original_jim_index(value)?
        {
            return Ok(resolve_jim_encoding(encoded, len));
        }
        if ops.index_c_version().is_some()
            && let Some(resolved) = ops.original_c_index(value, len)?
        {
            return Ok(resolved);
        }
        let spelling = ops.index_unicode(value)?;
        resolve_for_ops(ops, &spelling, len)
    })
}

/// Prepare a complete Jim original index vector before container conversion.
/// `None` describes only neutral adapters. A mixed physical/neutral vector is
/// unavailable rather than evaluating a prefix twice under a new receipt.
///
/// # Errors
/// The first reached original-index failure or an inconsistent adapter.
pub fn prepare_jim_indices<O: NativeIndexOps>(
    ops: &mut O,
    originals: &[&O::Value],
) -> Result<Option<Vec<i32>>, CmdError> {
    run_index_metadata(ops, |ops| {
        if !ops
            .index_syntax()
            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
        {
            return Ok(None);
        }
        let mut indices = Vec::with_capacity(originals.len());
        let mut neutral = false;
        for original in originals {
            match ops.original_jim_index(original)? {
                Some(index) if !neutral => indices.push(index),
                None if indices.is_empty() => neutral = true,
                _ => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "mixed original Jim Index adapter",
                    )
                    .into());
                }
            }
        }
        Ok((!neutral).then_some(indices))
    })
}

/// Validate whether a selected native index can be encoded for a search path.
///
/// # Errors
/// Bad index or the actual first native operation/getter failure.
pub fn encodable_for_ops<O: NativeIndexOps>(ops: &mut O, spec: &str) -> Result<bool, CmdError> {
    run_index_metadata(ops, |ops| {
        let syntax = ops.index_syntax().ok_or_else(|| {
            CmdError::from(ValueError::CommandProtocolUnavailable(
                "container index dialect",
            ))
        })?;
        let value = resolve_for_ops(ops, spec, 1)?;
        let end_relative = match syntax.grammar {
            tcl_dialect::IndexGrammar::Jim => spec.starts_with("end"),
            _ => spec.starts_with("end") || (!spec.is_empty() && "end".starts_with(spec)),
        };
        Ok(if end_relative { value <= 0 } else { value >= 0 })
    })
}

/// Native bad-index message, preserving the authored spelling.
#[must_use]
pub fn bad_index_in(spec: impl AsRef<[u8]>, syntax: tcl_dialect::IndexSyntax) -> CmdError {
    let grammar = match syntax.grammar {
        tcl_dialect::IndexGrammar::Tcl84 => "integer or end?-integer?",
        tcl_dialect::IndexGrammar::TclModern => "integer?[+-]integer? or end?[+-]integer?",
        tcl_dialect::IndexGrammar::Jim => "intexpr or end?[+-]intexpr?",
    };
    let mut message = b"bad index \"".to_vec();
    message.extend_from_slice(spec.as_ref());
    message.extend_from_slice(b"\": must be ");
    message.extend_from_slice(grammar.as_bytes());
    CmdError::new_bytes(message)
}

/// Publish the reached index diagnostic with its independently selected
/// physical String producer. Unavailable physical issuers remain host refusals.
#[must_use]
pub fn bad_index_for_ops<O: ValueOps>(
    ops: &O,
    spec: impl AsRef<[u8]>,
    syntax: tcl_dialect::IndexSyntax,
) -> CmdError {
    let diagnostic = if syntax.grammar == tcl_dialect::IndexGrammar::Jim {
        spec.as_ref()
    } else {
        // C Tcl's native bad-index diagnostic consumes Tcl_GetString through
        // its original %s field; counted parser extent remains independent.

        tcl_core_types::c_string_extent(spec.as_ref())
    };
    let error = bad_index_in(diagnostic, syntax);
    match ops.index_error_string_protocol() {
        Ok(Some(protocol)) => {
            let mut details = error.into_byte_details();
            let mut string_result = true;
            if let Some(version) = protocol.tcl_version() {
                if tcl_syntax::number::native_index_octal_hint(spec.as_ref(), version) {
                    details
                        .message
                        .extend_from_slice(b" (looks like invalid octal number)");
                    // C84 Tcl_AppendResult reaches the interpreter CString
                    // result, later materialised as a fresh NULL-primary object.
                    string_result = version != TclVersion::V8_4;
                }
                if version >= TclVersion::V8_6 {
                    details.error_code =
                        crate::error::CmdErrorCodeUpdate::Set(b"TCL VALUE INDEX".to_vec());
                }
            }
            let error = CmdError::from_byte_details(details);
            if string_result {
                error.with_native_string_result(protocol)
            } else {
                error
            }
        }
        Ok(None) => error,
        Err(refusal) => refusal.into(),
    }
}

/// A selected key and backing constructors needed to keep its actual header live.
/// Guards clone allocation transport only. No key is reconstructed from text.
#[derive(Clone)]
pub struct IndexedKey<K> {
    selected: K,
    created: bool,
    ancestors: Vec<(K, bool)>,
}

/// Borrow the selected value without an extra native reference.
pub fn key_value<O: NativeIndexOps>(key: &IndexedKey<O::IndexKeyOwner>) -> &O::Value {
    O::index_key_value(&key.selected)
}

/// Transport the same selected original into a reached container constructor.
///
/// # Errors
/// The original header or entered operation refuses the selected argument role.
pub fn key_argument<O: NativeIndexOps>(
    ops: &mut O,
    key: &IndexedKey<O::IndexKeyOwner>,
) -> Result<O::Value, CmdError> {
    ops.index_key_argument(&key.selected)
}

/// Transfer the exact selected result before mandatory backing cleanup.
pub fn key_result<O: NativeIndexOps>(
    ops: &mut O,
    key: IndexedKey<O::IndexKeyOwner>,
) -> Result<IndexResult<O::Value>, CmdError> {
    let created_ancestor = key.ancestors.iter().any(|(_, created)| *created);
    if created_ancestor && !key.created {
        let held = ops.index_hold_key_result(&key.selected)?;
        drop(key); // mandatory backing cleanup while the real selected child holds
        return ops.index_finish_held_key_result(held);
    }
    ops.index_key_result(&key.selected, key.created, false)
}

fn step_key<O: NativeIndexOps>(
    ops: &mut O,
    mut current: IndexedKey<O::IndexKeyOwner>,
    selected: IndexResult<O::Value>,
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    let (next, created) = match selected {
        IndexResult::Borrowed(member) => {
            let selected = ops.index_borrowed_key(&member);
            if let Err(error) = &selected {
                ops.retain_index_failure(error);
            }
            (selected?, false)
        }
        IndexResult::Created(created)
        | IndexResult::Detached(created)
        | IndexResult::ReturnedOwnedTransport(created) => (ops.index_created_key(created)?, true),
    };
    current.ancestors.push((current.selected, current.created));
    Ok(IndexedKey {
        selected: next,
        created,
        ancestors: current.ancestors,
    })
}

/// Drill a search/sort path while retaining explicit selected constructor roles.
/// Parent backing remains live through the last key use; the key guard supplies
/// no additional native reference while original getters execute.
///
/// # Errors
/// Original conversion/index failures or the complete first Host refusal.
pub fn drill<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    run_index_output(
        ops,
        |ops| {
            let mut current = IndexedKey {
                selected: ops.index_borrowed_key(value)?,
                created: false,
                ancestors: Vec::new(),
            };
            for spec in path {
                let value = O::index_key_value(&current.selected);
                let len = ops.index_list_length(value)?;
                let spec_text = std::str::from_utf8(spec).map_err(|error| {
                    tcl_syntax::raw_string::UnicodeAccessError {
                        valid_up_to: error.valid_up_to(),
                        error_len: error.error_len(),
                    }
                })?;
                let idx = resolve_for_ops(ops, spec_text, len)?;
                if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) >= len {
                    let mut message = b"element ".to_vec();
                    message.extend_from_slice(spec);
                    message.extend_from_slice(b" missing from sublist \"");
                    message.extend_from_slice(&ops.as_bytes(value));
                    message.push(b'"');
                    return Err(CmdError::new_bytes(message));
                }
                match ops.index_list_member(value, usize::try_from(idx).unwrap_or(0)) {
                    Ok(Some(member)) => current = step_key(ops, current, member)?,
                    Err(error) if error.native_execution_refusal().is_some() => return Err(error),
                    _ => return Ok(current),
                }
            }
            Ok(current)
        },
        |_, key| drop(key),
        |error| error,
    )
}

/// Select prepared original C coordinates, without reparsing an index object.
/// Original Length precedes the independently selected member getter.
///
/// # Errors
/// Reached original List/member/diagnostic or complete first Host refusal.
pub fn drill_prepared_c<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    path: &[tcl_syntax::native_compiled_index::NativeCompiledListIndex],
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    run_index_output(
        ops,
        |ops| {
            let version = ops
                .index_c_version()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "prepared original C option path issuer",
                ))?;
            let mut current = IndexedKey {
                selected: ops.index_borrowed_key(original)?,
                created: false,
                ancestors: Vec::new(),
            };
            for coordinate in path {
                let value = O::index_key_value(&current.selected);
                let reached_length = ops.index_list_length(value);
                if let Err(error) = &reached_length {
                    ops.retain_index_failure(error);
                }
                let length = reached_length?;
                let end = i128::try_from(length).map_err(|_| {
                    let error = CmdError::from(ValueError::CommandProtocolUnavailable(
                        "original option List length",
                    ));
                    ops.retain_index_failure(&error);
                    error
                })? - 1;
                let decoded = coordinate.decode(end);
                let reached_member = match usize::try_from(decoded) {
                    Ok(index) => ops.index_list_member(value, index),
                    Err(_) => Ok(None),
                };
                if let Err(error) = &reached_member {
                    ops.retain_index_failure(error);
                }
                let member = reached_member?;
                if let Some(member) = member {
                    current = step_key(ops, current, member)?;
                    continue;
                }
                let mut message = b"element ".to_vec();
                if version >= TclVersion::V9_0 && decoded < 0 {
                    message.extend_from_slice(
                        format!("end-{}", -2_i64 - i64::from(coordinate.encoded())).as_bytes(),
                    );
                } else if version == TclVersion::V8_4 && decoded < 0 {
                    // 8.4.20 names a step before the first element by its
                    // encoding, `end` being -2 (`lsort -index end-5 {{a b}
                    // {c d}}` names `-7`); 8.5 and 8.6 the decoded index.
                    message.extend_from_slice(coordinate.encoded().to_string().as_bytes());
                } else {
                    message.extend_from_slice(decoded.to_string().as_bytes());
                }
                message.extend_from_slice(b" missing from sublist \"");
                let reached_bytes = ops.index_operand_bytes(value);
                if let Err(error) = &reached_bytes {
                    ops.retain_index_failure(error);
                }
                message.extend_from_slice(tcl_core_types::c_string_extent(&reached_bytes?));
                message.push(b'"');
                return Err(if version >= TclVersion::V8_6 {
                    CmdError::with_error_code_bytes(
                        message,
                        b"TCL OPERATION LSORT INDEXFAILED".to_vec(),
                    )
                } else {
                    CmdError::new_bytes(message)
                });
            }
            Ok(current)
        },
        |_, key| drop(key),
        std::convert::identity,
    )
}

/// Drill Jim's original member path with its independently selected native
/// Index objects. Every original coordinate precedes every container getter.
/// The reached missing-member message is Jim's `CString` diagnostic, without a
/// C errorCode or a native String-result claim.
///
/// # Errors
/// Original conversion, Jim missing-member failure or complete Host refusal.
pub fn drill_original_jim<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    indices: &[O::Value],
) -> Result<IndexedKey<O::IndexKeyOwner>, CmdError> {
    run_index_output(
        ops,
        |ops| {
            let encoded = prepare_jim_indices(ops, &indices.iter().collect::<Vec<_>>())?;
            let mut current = IndexedKey {
                selected: ops.index_borrowed_key(value)?,
                created: false,
                ancestors: Vec::new(),
            };
            for (ordinal, original) in indices.iter().enumerate() {
                let members = ops.index_list_elements(key_value::<O>(&current))?;
                let at = match &encoded {
                    Some(encoded) => resolve_jim_encoding(encoded[ordinal], members.len()),
                    None => resolve_value(ops, original, members.len())?,
                };
                if let Some(member) = usize::try_from(at).ok().and_then(|at| members.get(at)) {
                    current = step_key(ops, current, IndexResult::Borrowed(member.clone()))?;
                    continue;
                }
                let encoded = encoded
                    .as_ref()
                    .map_or(at, |indices| i64::from(indices[ordinal]));
                let out_of_range = encoded < 0
                    || usize::try_from(encoded).is_ok_and(|index| index > members.len());
                let spelling = ops.index_operand_bytes(original)?;
                let mut message = if out_of_range {
                    b"index \"".to_vec()
                } else {
                    b"element ".to_vec()
                };
                message.extend_from_slice(tcl_core_types::c_string_extent(&spelling));
                if out_of_range {
                    message.extend_from_slice(b"\" out of range");
                } else {
                    message.extend_from_slice(b" missing from sublist \"");
                    let container = ops.index_operand_bytes(key_value::<O>(&current))?;
                    message.extend_from_slice(tcl_core_types::c_string_extent(&container));
                    message.push(b'"');
                }
                return Err(CmdError::new_bytes(message));
            }
            Ok(current)
        },
        |_, key| drop(key),
        |error| error,
    )
}

/// How `release` names an `-index` step that falls outside its sublist.
/// Measured (`lsort -index end-5 {{a b} {c d}}`, the step decoding to -4):
/// tclsh 9.0.4 and 9.1.0 name the word (`end-5`) when an `end` step falls
/// before the first element; 8.5.19 and 8.6.18 name the decoded index (`-4`);
/// 8.4.20 names its encoding, `end` being -2 (`-7`). An index past the end is
/// the decoded number everywhere.
fn failed_step_name(step: PathEncoding, decoded: i64, release: TclVersion) -> String {
    match step {
        PathEncoding::FromEnd(offset) if decoded < 0 => match release {
            TclVersion::V8_4 => (offset - 2).to_string(),
            TclVersion::V8_5 | TclVersion::V8_6 => decoded.to_string(),
            _ => format!("end-{}", -offset),
        },
        _ => decoded.to_string(),
    }
}

/// Whether an index `spec` is *encodable* the way `TclIndexEncode` requires for
/// `lsearch`/`lsort -index`: `Some(true)` for a normal index (an integer in
/// `0..=INT_MAX`, or `end`/`end-N`), `Some(false)` for one it refuses as out of
/// range, and `None` for a syntactically bad spec.
#[must_use]
pub fn encodable(spec: &str) -> Option<bool> {
    match PathEncoding::of(spec) {
        Ok(_) => Some(true),
        Err(Refusal::OutOfRange) => Some(false),
        Err(Refusal::Bad) => None,
    }
}

/// Whose integer range reads an index's integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Range {
    Tcl84,
    Tcl85,
    Tcl86,
    /// 8.5 or 8.6, the shared numeral grammar naming neither.
    Tcl85Or86,
    /// 9.0 on: a wide, a bignum truncated to one.
    Wide,
    Jim,
}

impl Range {
    const fn of_release(release: TclVersion) -> Self {
        match release {
            TclVersion::V8_4 => Self::Tcl84,
            TclVersion::V8_5 => Self::Tcl85,
            TclVersion::V8_6 => Self::Tcl86,
            TclVersion::V9_0 | TclVersion::V9_1 => Self::Wide,
        }
    }

    const fn of_numbers(numbers: NumberSyntax) -> Self {
        match numbers {
            NumberSyntax::Tcl84 => Self::Tcl84,
            NumberSyntax::Tcl85 => Self::Tcl85Or86,
            NumberSyntax::Tcl90 => Self::Wide,
            NumberSyntax::Jim | NumberSyntax::Jim080 => Self::Jim,
        }
    }
}

/// One index as a release reads it (`TclGetIntForIndex` up to 8.6,
/// `GetWideForIndex` from 9.0), measured with `string index` on tclsh
/// 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0:
///
/// - an integer, whitespace allowed either side (`Tcl_GetInt`);
/// - `end` from its first byte, which 8.4 to 8.6 also read abbreviated (`e`,
///   `en`), followed by an offset: `-N` in 8.4, whose `N` may start with
///   whitespace (`end- 1`), and `+N` or `-N` from 8.5, whose `N` starts at once;
/// - from 8.5, `M+N` or `M-N` after any leading whitespace, the operator right
///   after `M` and `N` right after the operator.
///
/// Nothing else is an index: not `end+1`, `1+1` or `1-1` in 8.4, not ` end`,
/// `end ` or `end -1` in any release. A Jim grammar reads the forms of Tcl 9.0
/// with whitespace around `end` and each operand.
///
/// Each integer is read in its release's range (measured on each
/// release's `tclsh`):
///
/// - 8.4 to 8.6 read one through `Tcl_GetInt`: a value within ±`UINT_MAX`
///   (4,294,967,295) wraps to the 32-bit `int` — on every platform, the 64-bit
///   `long` casting it and a 32-bit one taking it through
///   `Tcl_GetLongFromObj`'s ±`ULONG_MAX` — so `2147483648` is
///   −2147483648 and `-4294967295` is 1, and the sums and `end` offsets wrap
///   alike (8.6's `1+2147483647` is −2147483648, `end-4294967295` is
///   `end+1`); any other value is `bad index`, but for a magnitude from
///   2^64 − 2^32 + 1 to 2^64 − 1, which 8.4 (`strtoul`) and 8.5 (the bignum
///   branch) wrap into that range where `long` is 64 bits and raise on where it
///   is 32 bits, and 8.6 raises on everywhere;
/// - 9.0 and 9.1 read a wide, a bignum alone being the wide nearest it, and
///   encode `end+1` as the widest wide: a sum that reaches it and a bignum
///   offset after `end` name `end+1` (`lset` appends there), a larger `end+N`
///   any index past it. This is the 64-bit `Tcl_Size` build every oracle is;
///   a 32-bit one bounds an index at 2^31 − 1.
fn read(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
    match range {
        Range::Tcl84 => read_84(spec, len, numbers, range),
        Range::Tcl85 | Range::Tcl86 | Range::Tcl85Or86 => read_85(spec, len, numbers, range),
        Range::Wide => read_90(spec, len, numbers),
        Range::Jim => {
            let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
            jim(spec, end, numbers).map_or(IndexReading::Bad, IndexReading::At)
        }
    }
}

/// C's `int` index of `end` for a container of `len` elements.
fn end_32(len: usize) -> i64 {
    wrap_32(i64::try_from(len).unwrap_or(i64::MAX) - 1)
}

/// `value` cast to C's 32-bit `int`: its low 32 bits, sign-extended.
const fn wrap_32(value: i64) -> i64 {
    (value << 32) >> 32
}

/// 8.4: `Tcl_GetInt` on the word; then `e`, `en`, `end`, or `end-` and a
/// `Tcl_GetInt` integer, wherever `end` is the `int` before the first element.
fn read_84(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
    if let Some(number) = whole_number(spec, numbers) {
        return int_32(&number, range);
    }
    if !spec.starts_with('e') {
        return IndexReading::Bad;
    }
    let end = end_32(len);
    if spec.len() <= 3 {
        return if "end".starts_with(spec) {
            IndexReading::At(end)
        } else {
            IndexReading::Bad
        };
    }
    let Some(number) = spec
        .strip_prefix("end-")
        .and_then(|operand| whole_number(operand, numbers))
    else {
        return IndexReading::Bad;
    };
    int_32(&number, range).map(|offset| wrap_32(end + wrap_32(-offset)))
}

/// 8.5 and 8.6: `Tcl_GetInt` on the word; then `end` with an offset, or a sum.
fn read_85(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
    if let Some(number) = whole_number(spec, numbers) {
        return int_32(&number, range);
    }
    if spec.starts_with('e') {
        let end = end_32(len);
        if spec.len() <= 3 && "end".starts_with(spec) {
            return IndexReading::At(end);
        }
        return spec.strip_prefix("end").map_or(IndexReading::Bad, |rest| {
            signed_32(rest, numbers, range).map(|offset| wrap_32(end + offset))
        });
    }
    let Some((first, rest)) = sum_head(spec, numbers) else {
        return IndexReading::Bad;
    };
    int_32(&first, range).zip(signed_32(rest, numbers, range), |a, b| wrap_32(a + b))
}

/// 9.0 on: `GetWideForIndex` on the word, then `GetEndOffsetFromObj`.
fn read_90(spec: &str, len: usize, numbers: NumberSyntax) -> IndexReading {
    let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
    if let Some(number) = whole_number(spec, numbers) {
        return match number {
            Number::Int(value) => IndexReading::At(value),
            Number::Big { negative, .. } => {
                IndexReading::At(if negative { i64::MIN } else { i64::MAX })
            }
            _ => IndexReading::Bad,
        };
    }
    if spec.starts_with('e') {
        if spec == "end" {
            return IndexReading::At(end);
        }
        return spec
            .strip_prefix("end")
            .and_then(|rest| signed_operand(rest, numbers))
            .map_or(IndexReading::Bad, |(negative, number)| {
                IndexReading::At(end_offset_90(end, negative, &number))
            });
    }
    let Some((first, rest)) = sum_head(spec, numbers) else {
        return IndexReading::Bad;
    };
    let Some((negative, second)) = signed_operand(rest, numbers) else {
        return IndexReading::Bad;
    };
    match exact_sum(&first, negative, &second) {
        // The sum's encoding when it reaches the widest wide is `end+1`.
        Some(sum) if sum >= i128::from(i64::MAX) => IndexReading::At(end + 1),
        Some(sum) => IndexReading::At(i64::try_from(sum).unwrap_or(i64::MIN)),
        None => IndexReading::Unsure,
    }
}

/// 9.0's `end+N` or `end-N`: a bignum offset is `end+1` or before the first
/// element, `end+1` itself `end+1`, a larger offset any index past it.
fn end_offset_90(end: i64, negative: bool, number: &Number) -> i64 {
    let offset = match number {
        Number::Big {
            negative: below, ..
        } => return if *below == negative { end + 1 } else { -1 },
        Number::Int(value) if negative => value.checked_neg().unwrap_or(i64::MAX),
        Number::Int(value) => *value,
        _ => return -1,
    };
    end.saturating_add(offset)
}

/// The exact value of `first ± second` within `i128`, saturated where an
/// operand is past it, or `None` where two such operands of opposite signs
/// leave the sum unknown.
fn exact_sum(first: &Number, negative: bool, second: &Number) -> Option<i128> {
    let integer = |number: &Number| matches!(number, Number::Int(_) | Number::Big { .. });
    if !integer(first) || !integer(second) {
        return None;
    }
    let a = wide_128(first);
    let b = wide_128(second).map(|b| if negative { b.wrapping_neg() } else { b });
    let sign = |number: &Number, flip: bool| -> i128 {
        let below = matches!(number, Number::Int(v) if *v < 0)
            || matches!(number, Number::Big { negative: true, .. });
        if below == flip { i128::MAX } else { i128::MIN }
    };
    match (a, b) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        (Some(_), None) => Some(sign(second, negative)),
        (None, Some(_)) => Some(sign(first, false)),
        (None, None) => {
            let (x, y) = (sign(first, false), sign(second, negative));
            (x == y).then_some(x)
        }
    }
}

/// An integer as an `i128`, `None` past it.
fn wide_128(number: &Number) -> Option<i128> {
    match number {
        Number::Int(value) => Some(i128::from(*value)),
        Number::Big {
            negative,
            radix,
            digits,
        } => magnitude(*radix, digits)
            .and_then(|m| i128::try_from(m).ok())
            .map(|m| if *negative { -m } else { m }),
        _ => None,
    }
}

/// An integer as `Tcl_GetInt` reads it before 9.0: see
/// [`read`].
fn int_32(number: &Number, range: Range) -> IndexReading {
    const UINT_MAX: u64 = 0xffff_ffff;
    match number {
        Number::Int(value) if value.unsigned_abs() <= UINT_MAX => IndexReading::At(wrap_32(*value)),
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let Some(unsigned) = magnitude(*radix, digits).and_then(|m| u64::try_from(m).ok())
            else {
                return IndexReading::Bad;
            };
            let long = if *negative {
                unsigned.cast_signed().wrapping_neg()
            } else {
                unsigned.cast_signed()
            };
            if long.unsigned_abs() > UINT_MAX {
                return IndexReading::Bad;
            }
            match range {
                Range::Tcl84 | Range::Tcl85 => IndexReading::HostLong(wrap_32(long)),
                Range::Tcl85Or86 => IndexReading::Unsure,
                Range::Tcl86 | Range::Wide | Range::Jim => IndexReading::Bad,
            }
        }
        _ => IndexReading::Bad,
    }
}

/// A bignum's magnitude, `None` past `u128`.
fn magnitude(radix: Radix, digits: &str) -> Option<u128> {
    let base = match radix {
        Radix::Bin => 2,
        Radix::Oct => 8,
        Radix::Dec => 10,
        Radix::Hex => 16,
    };
    digits.chars().try_fold(0_u128, |value, digit| {
        value
            .checked_mul(u128::from(base))?
            .checked_add(u128::from(digit.to_digit(base)?))
    })
}

/// Tcl's whitespace (`TclIsSpaceProc`).
const fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r')
}

/// The first integer of a sum `M+N` or `M-N` from 8.5, after any leading
/// whitespace and with its operator right after it, and the rest from that
/// operator.
fn sum_head(spec: &str, numbers: NumberSyntax) -> Option<(Number, &str)> {
    let body = spec.trim_start_matches(is_space);
    let flags = ParseFlags {
        no_whitespace: true,
        ..index_int_flags(numbers)
    };
    let parsed = tcl_syntax::number::parse(body, flags)?;
    Some((parsed.number, &body[parsed.end..]))
}

/// `+N` or `-N` with `N` a whole integer starting right after the operator:
/// whether it subtracts, and `N`.
fn signed_operand(rest: &str, numbers: NumberSyntax) -> Option<(bool, Number)> {
    let negative = match rest.as_bytes().first()? {
        b'-' => true,
        b'+' => false,
        _ => return None,
    };
    let operand = &rest[1..];
    if operand.is_empty() || operand.starts_with(is_space) {
        return None;
    }
    Some((negative, whole_number(operand, numbers)?))
}

/// [`signed_operand`] read through `Tcl_GetInt` before 9.0, as the value it
/// adds.
fn signed_32(rest: &str, numbers: NumberSyntax, range: Range) -> IndexReading {
    signed_operand(rest, numbers).map_or(IndexReading::Bad, |(negative, number)| {
        int_32(&number, range).map(|value| if negative { wrap_32(-value) } else { value })
    })
}

/// A Jim index: a base (`end` or an integer) and an optional `+`/`-` operand,
/// whitespace allowed around each.
fn jim(spec: &str, end: i64, numbers: NumberSyntax) -> Option<i64> {
    let s = spec.trim_matches(is_space);
    let (base, rest) = if let Some(rest) = s.strip_prefix("end") {
        (end, rest)
    } else {
        parse_int_prefix(s, numbers)?
    };
    if rest.is_empty() {
        return Some(base);
    }
    let negative = match rest.as_bytes()[0] {
        b'-' => true,
        b'+' => false,
        _ => return None,
    };
    let operand = parse_int_whole(rest[1..].trim_matches(is_space), numbers)?;
    Some(base.saturating_add(if negative {
        operand.saturating_neg()
    } else {
        operand
    }))
}

/// [`ParseFlags`](tcl_syntax::number::ParseFlags) for an index integer: reject a
/// fractional part / exponent, but otherwise the full Tcl integer syntax of the
/// release in play — so a hex index like `0x2` resolves the way real Tcl's
/// `Tcl_GetIntForIndex` does.
///
/// An index word is read by `Tcl_GetIntForIndex`, so it inherits every version
/// difference in the numeral grammar. Pinned on tclsh 8.6.16 / 9.0.4 against a
/// 12-element list `a`…`l`:
///
/// | spec | 8.6 | 9.0 |
/// |---|---|---|
/// | `lindex $l 010` | `i` (index 8, octal) | `k` (index 10, decimal) |
/// | `lindex $l end-010` | `d` (end−8) | `b` (end−10) |
/// | `lindex $l 1_0` | `bad index` | `k` |
/// | `lindex $l 0d1` | `bad index` | `b` |
///
/// The ordinary runtime path takes the grammar the interpreter was built for —
/// the ambient installed once by `set_runtime_version`, exactly as C settles it
/// at build time; a compile-time caller that must name a different release
/// passes it explicitly through [`read_with`] or [`read_under`].
fn index_int_flags(numbers: NumberSyntax) -> ParseFlags {
    ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    }
}

/// A whole integer word, whitespace allowed either side (`Tcl_GetInt`): a wide
/// or a bignum.
fn whole_number(s: &str, numbers: NumberSyntax) -> Option<Number> {
    let parsed = tcl_syntax::number::parse(s, index_int_flags(numbers))?;
    (s[parsed.end..].trim_matches(is_space).is_empty()
        && matches!(parsed.number, Number::Int(_) | Number::Big { .. }))
    .then_some(parsed.number)
}

/// Parse a leading Tcl integer, returning its value and the unconsumed tail via
/// the canonical [`tcl_syntax::number`] parser (so every radix the runtime
/// accepts resolves identically). `None` if `s` does not start with an integer,
/// or the value is a non-integer / bignum (`Int` only).
fn parse_int_prefix(s: &str, numbers: NumberSyntax) -> Option<(i64, &str)> {
    let parsed = tcl_syntax::number::parse(s, index_int_flags(numbers))?;
    match parsed.number {
        Number::Int(v) => Some((v, &s[parsed.end..])),
        _ => None,
    }
}

/// Parse `s` as a whole Tcl integer — the [`parse_int_prefix`] value only when
/// nothing but trailing space follows it.
fn parse_int_whole(s: &str, numbers: NumberSyntax) -> Option<i64> {
    let (val, tail) = parse_int_prefix(s, numbers)?;
    tail.trim_matches(is_space).is_empty().then_some(val)
}

/// The canonical `bad index "<spec>": …` error, as Tcl 8.6 and later raise it
/// ([`bad_index_under`]).
#[must_use]
pub fn bad_index(spec: &str) -> CmdError {
    bad_index_under(spec, TclVersion::V8_6)
}

/// The `bad index` error for `spec` a caller reading it under `numbers`
/// raises: 8.4's wording under its grammar, 8.6's otherwise
/// ([`bad_index_under`]). The grammar does not tell 8.5 from 8.6, so 8.6's
/// `TCL VALUE INDEX` error code is left for an 8.5 runtime to drop.
#[must_use]
pub fn bad_index_with(spec: &str, numbers: NumberSyntax) -> CmdError {
    if numbers == NumberSyntax::Tcl84 {
        bad_index_under(spec, TclVersion::V8_4)
    } else {
        bad_index(spec)
    }
}

/// [`bad_index_with`] under the grammar this thread's interpreter installed,
/// as a message and the code it carries, for an option word read under it
/// (`lsearch -start`, an `-index` step).
#[must_use]
pub fn bad_index_parts(spec: &[u8]) -> (Vec<u8>, Option<&'static [u8]>) {
    let details = bad_index_with(
        &String::from_utf8_lossy(spec),
        tcl_syntax::number::runtime_syntax(),
    )
    .into_byte_details();
    let coded = matches!(details.error_code, crate::error::CmdErrorCodeUpdate::Set(_));
    (details.message, coded.then_some(&b"TCL VALUE INDEX"[..]))
}

/// The `bad index` error `release` raises for `spec`: 8.4 words the grammar
/// it reads (`integer or end?-integer?`), 8.5 on the sums too, and from 8.6
/// the error carries `TCL VALUE INDEX` (measured, 8.4.20 to 9.1.0).
#[must_use]
pub fn bad_index_under(spec: &str, release: TclVersion) -> CmdError {
    if release < TclVersion::V8_5 {
        return CmdError::new(format!(
            "bad index \"{spec}\": must be integer or end?-integer?"
        ));
    }
    let message = format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?");
    if release < TclVersion::V8_6 {
        CmdError::new(message)
    } else {
        CmdError::with_error_code(message, "TCL VALUE INDEX")
    }
}

#[cfg(test)]
mod tests {
    use super::IndexReading::{At, Bad, HostLong};
    use super::*;

    /// `lsort -index <step> {{a b} {c d}}` against each release (tclsh 8.4.20,
    /// 8.5.19, 8.6.18, 9.0.4, 9.1.0): a step before the first element names
    /// the word from 9.0, the decoded index in 8.5/8.6 and 8.4's `end` = -2
    /// encoding; a step past the end is the number everywhere.
    #[test]
    fn a_failed_step_is_named_as_each_release_names_it() {
        let rows: &[(&str, [&str; 5])] = &[
            ("end-5", ["-7", "-4", "-4", "end-5", "end-5"]),
            ("end-3", ["-5", "-2", "-2", "end-3", "end-3"]),
            ("7", ["7", "7", "7", "7", "7"]),
        ];
        let releases = [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ];
        for (spec, names) in rows {
            let step = PathEncoding::of(spec).expect("encodes");
            let decoded = step.decode(2);
            for (release, name) in releases.iter().zip(names) {
                assert_eq!(
                    failed_step_name(step, decoded, *release),
                    *name,
                    "{spec} {release:?}"
                );
            }
        }
    }

    /// `TclCompileStringReplaceCmd` (8.6.18) settles the command on its encoded
    /// indices without a replacement string, for a no-op it proves, for
    /// `first` before the first element, and for `last` after the end with
    /// `first` relative to `end`; otherwise `INST_STR_REPLACE` reads the words
    /// at run time. Measured in a proc on `abcdefghijkl`: `string replace $s
    /// end+2147483647 end X` is unchanged (the run-time reading replaces it
    /// all), and so is `string replace $s 0 end+2147483647 X`, as at run time.
    #[test]
    fn string_replace_settles_on_its_encoded_indices_where_8_6_does() {
        use super::Encoded::{After, Before, FromEnd, Index};
        for (first, last, replacing, encoded) in [
            (After, FromEnd(0), true, true),
            (Index(0), After, true, false),
            (Index(0), After, false, true),
            (Before, After, true, true),
            (FromEnd(-1), After, true, true),
            (Index(2), Before, true, true),
            (Index(2), FromEnd(-1), true, false),
        ] {
            assert_eq!(
                string_replace_compiles_encoded(first, last, replacing),
                encoded,
                "{first:?} {last:?} replacing: {replacing}"
            );
        }
    }

    /// Each command's index words as 8.6's compile procedure encodes them, or
    /// leaves them to the command. Measured on tclsh 8.6.18 in a proc with
    /// `$l` `{a b c}` and `$i` 1: `lreplace $l end-2147483649 $i X` is
    /// `X c` (read as it runs), `string range abcdef end-2147483649 $i` is
    /// empty (a `first` after the end settles it), and `lrange $l 0
    /// end+2147483647`, `lindex $l end-2147483649` and `linsert $l
    /// end-2147483649 X` read their literal after the end.
    #[test]
    fn each_compile_procedure_encodes_its_own_index_words() {
        let words = |compiler, words: &[Option<&str>]| {
            compiled_index_words(compiler, words, TclVersion::V8_6)
        };
        let past = Some("end-2147483649");
        let after = Some("2147483647");
        assert_eq!(words(IndexCompiler::Lindex, &[past]), [after]);
        assert_eq!(words(IndexCompiler::Linsert, &[past]), [after]);
        assert_eq!(
            words(IndexCompiler::Lrange, &[Some("0"), Some("end+2147483647")]),
            [None, after]
        );
        assert_eq!(words(IndexCompiler::Lrange, &[past, None]), [None, None]);
        assert_eq!(words(IndexCompiler::Lreplace, &[past, None]), [None, None]);
        assert_eq!(
            words(IndexCompiler::Lreplace, &[Some("2"), past]),
            [None, after]
        );
        assert_eq!(
            words(IndexCompiler::StringRange, &[past, None]),
            [after, None]
        );
        assert_eq!(
            words(
                IndexCompiler::StringReplace { replacing: true },
                &[Some("0"), Some("end+2147483647")]
            ),
            [None, None]
        );
        assert_eq!(
            words(
                IndexCompiler::StringReplace { replacing: false },
                &[Some("0"), Some("end+2147483647")]
            ),
            [None, after]
        );
        for release in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V9_0] {
            assert_eq!(
                compiled_index_words(IndexCompiler::Lindex, &[past], release),
                [None]
            );
        }
    }

    #[test]
    fn string_replace_settles_as_the_compiler_does() {
        use StringReplaceSettlement as S;
        let at = |index| Some(Encoded::Index(index));
        let end = Some(Encoded::FromEnd(0));
        assert_eq!(settle_string_replace(at(5), at(2), true), S::Unchanged);
        assert_eq!(settle_string_replace(at(0), at(2), true), S::Runtime);
        assert_eq!(
            settle_string_replace(Some(Encoded::Before), Some(Encoded::After), true),
            S::Replacement
        );
        assert_eq!(settle_string_replace(at(0), end, false), S::Empty);
        assert_eq!(settle_string_replace(at(1), end, false), S::Ranges);
        assert_eq!(settle_string_replace(None, end, false), S::Runtime);
    }

    #[test]
    fn jim_literal_index_expressions_preserve_end_encoding() {
        let syntax = tcl_dialect::IndexSyntax {
            numbers: tcl_dialect::NumberSyntax::Jim080,
            grammar: tcl_dialect::IndexGrammar::Jim,
            width: tcl_dialect::IndexIntegerWidth::Jim32,
            end_abbreviations: false,
        };
        for (source, expected) in [
            ("end--1", i64::from(i32::MAX)),
            ("end+-1", 3),
            ("(1+2)*1", 3),
            ("1 ? 2 : 1/0", 2),
            ("--1", 1),
        ] {
            assert_eq!(
                resolve_opt_in(source, 5, syntax),
                Some(expected),
                "{source}"
            );
        }
        for source in [
            "$x",
            "1 ? 2 : $x",
            "[set x]",
            "abs(-2)",
            "1/0",
            "1 #ignored",
        ] {
            assert_eq!(resolve_opt_in(source, 5, syntax), None, "{source}");
        }
    }

    #[test]
    fn resolve_integer_and_end_forms() {
        // Mirrors Tcl's `TclGetIntForIndex` grammar (end / signed integer /
        // base[+-]integer).
        assert_eq!(resolve("5", 10).unwrap(), 5);
        assert_eq!(resolve("0", 10).unwrap(), 0);
        assert_eq!(resolve("-1", 10).unwrap(), -1);
        assert_eq!(resolve("end", 10).unwrap(), 9);
        assert_eq!(resolve("end-2", 10).unwrap(), 7);
        assert_eq!(resolve("end+1", 10).unwrap(), 10);
        // `end--1` parses as `end - (-1)`.
        assert_eq!(resolve("end--1", 10).unwrap(), 10);
        assert_eq!(resolve("1+1", 10).unwrap(), 2);
        assert_eq!(resolve("0-1", 10).unwrap(), -1);
        // `end` of an empty container is -1.
        assert_eq!(resolve("end", 0).unwrap(), -1);
    }

    #[test]
    fn resolve_rejects_garbage_and_resolve_opt_returns_none() {
        assert!(resolve("bad", 10).is_err());
        assert!(resolve("", 10).is_err());
        assert!(resolve("1.5", 10).is_err());
        assert_eq!(resolve_opt("bad", 10), None);
        assert_eq!(resolve_opt("end-1", 10), Some(8));
    }

    #[test]
    fn resolve_matches_tclsh_oracle() {
        // Ground truth captured from real tclsh 8.6 and 9.0 (identical) via
        // `string index`/`lindex` over a length-5 container (end = 4). Each
        // `resolve_opt` returns the raw resolved index (callers clamp for
        // out-of-range); a `None` is a `bad index` error at runtime.
        let ok: &[(&str, i64)] = &[
            // integers, signs, whitespace
            ("0", 0),
            ("1", 1),
            ("4", 4),
            ("5", 5),
            ("-1", -1),
            ("-2", -2),
            ("-0", 0),
            (" 1", 1),
            ("1 ", 1),
            // end and end offsets (`end--1` = end - (-1) = end + 1)
            ("end", 4),
            ("end-1", 3),
            ("end+1", 5),
            ("end-4", 0),
            ("end-5", -1),
            ("end--1", 5),
            ("end+-1", 3),
            ("end+0", 4),
            // arithmetic operands
            ("1+1", 2),
            ("0-1", -1),
            ("2+2", 4),
            ("3-1", 2),
            ("end-2", 2),
            // radix prefixes (hex / octal / binary / 0d) — oracle accepts these
            ("0x2", 2),
            ("0xa", 10),
            ("0xf", 15),
            ("0o7", 7),
            ("0b101", 5),
            ("0d5", 5),
            ("+0x2", 2),
            ("-0x1", -1),
            // radices inside the arithmetic operands
            ("end-0x2", 2),
            ("end-0o3", 1),
            ("0x2+1", 3),
            ("1+0x1", 2),
        ];
        for &(spec, want) in ok {
            assert_eq!(resolve_opt(spec, 5), Some(want), "resolve_opt({spec:?})");
        }
        // Syntactically bad — a `bad index` error at runtime.
        for spec in ["1.0", "1+", "end-", "foo", "", "1 1", "0x", "0xg", "2e0"] {
            assert_eq!(
                resolve_opt(spec, 5),
                None,
                "resolve_opt({spec:?}) should be None"
            );
        }
    }

    #[test]
    fn encodable_classifies_specs() {
        // `TclIndexEncode` scale (used by `lsearch`/`lsort -index` parse-time
        // validation): Some(true) = encodable, Some(false) = out of range,
        // None = syntactically bad.
        assert_eq!(encodable("5"), Some(true)); // absolute, non-negative
        assert_eq!(encodable("0"), Some(true));
        assert_eq!(encodable("-1"), Some(false)); // absolute negative → out of range
        assert_eq!(encodable("end"), Some(true)); // offset 0 ≤ 0
        assert_eq!(encodable("end-2"), Some(true)); // offset -2 ≤ 0
        assert_eq!(encodable("end+1"), Some(false)); // offset +1 > 0 → out of range
        assert_eq!(encodable("bad"), None); // syntactically bad
    }

    /// An index word is read by `Tcl_GetIntForIndex`, so it inherits every
    /// version difference in the numeral grammar. Pinned on tclsh 8.6.16 /
    /// 9.0.4 against a 12-element list:
    ///
    /// ```text
    /// lindex $l 010      -> i (index 8, octal)  | k (index 10, decimal)
    /// lindex $l end-010  -> d (end-8)           | b (end-10)
    /// lindex $l 1_0      -> bad index           | k
    /// lindex $l 0d1      -> bad index           | b
    /// ```
    #[test]
    fn index_numerals_follow_the_named_release() {
        use tcl_syntax::number::NumberSyntax::{Tcl85, Tcl90};
        // (spec, len, 8.6 answer, 9.0 answer)
        let matrix = [
            ("010", 12, Some(8), Some(10)),
            ("end-010", 12, Some(3), Some(1)),
            ("1_0", 12, None, Some(10)),
            ("0d1", 12, None, Some(1)),
            // Unanimous spellings resolve the same either way.
            ("1", 12, Some(1), Some(1)),
            ("0x1", 12, Some(1), Some(1)),
            ("007", 12, Some(7), Some(7)),
            ("end", 12, Some(11), Some(11)),
            ("end-2", 12, Some(9), Some(9)),
        ];
        for (spec, len, on_86, on_90) in matrix {
            assert_eq!(
                resolve_opt_with(spec, len, Tcl85),
                on_86,
                "index {spec} on 8.6"
            );
            assert_eq!(
                resolve_opt_with(spec, len, Tcl90),
                on_90,
                "index {spec} on 9.0"
            );
        }
    }

    /// The rows past 32 bits of [`each_release_reads_an_index_as_its_tclsh_does`]:
    /// (spec, 8.4, 8.5, 8.6, 9.0 and 9.1).
    const PAST_32_BITS: &[(&str, [IndexReading; 4])] = &[
        (
            "2147483647",
            [
                At(2_147_483_647),
                At(2_147_483_647),
                At(2_147_483_647),
                At(2_147_483_647),
            ],
        ),
        (
            "2147483648",
            [
                At(-2_147_483_648),
                At(-2_147_483_648),
                At(-2_147_483_648),
                At(2_147_483_648),
            ],
        ),
        ("4294967295", [At(-1), At(-1), At(-1), At(4_294_967_295)]),
        ("0xffffffff", [At(-1), At(-1), At(-1), At(4_294_967_295)]),
        ("4294967296", [Bad, Bad, Bad, At(4_294_967_296)]),
        ("0x100000000", [Bad, Bad, Bad, At(4_294_967_296)]),
        (
            "-2147483649",
            [
                At(2_147_483_647),
                At(2_147_483_647),
                At(2_147_483_647),
                At(-2_147_483_649),
            ],
        ),
        ("-4294967295", [At(1), At(1), At(1), At(-4_294_967_295)]),
        ("-4294967296", [Bad, Bad, Bad, At(-4_294_967_296)]),
        (
            "end-2147483648",
            [
                At(-2_147_483_637),
                At(-2_147_483_637),
                At(-2_147_483_637),
                At(-2_147_483_637),
            ],
        ),
        (
            "end-4294967295",
            [At(12), At(12), At(12), At(-4_294_967_284)],
        ),
        ("end-4294967296", [Bad, Bad, Bad, At(-4_294_967_285)]),
        (
            "end+2147483647",
            [
                Bad,
                At(-2_147_483_638),
                At(-2_147_483_638),
                At(2_147_483_658),
            ],
        ),
        ("end+4294967295", [Bad, At(10), At(10), At(4_294_967_306)]),
        (
            "1+2147483647",
            [
                Bad,
                At(-2_147_483_648),
                At(-2_147_483_648),
                At(2_147_483_648),
            ],
        ),
        ("4294967295+1", [Bad, At(0), At(0), At(4_294_967_296)]),
        (
            "-2147483648-1",
            [
                Bad,
                At(2_147_483_647),
                At(2_147_483_647),
                At(-2_147_483_649),
            ],
        ),
        ("9223372036854775807", [Bad, Bad, Bad, At(i64::MAX)]),
        ("9223372036854775808", [Bad, Bad, Bad, At(i64::MAX)]),
        ("-9223372036854775809", [Bad, Bad, Bad, At(i64::MIN)]),
        ("0x8000000000000000", [Bad, Bad, Bad, At(i64::MAX)]),
        ("0x1ffffffffffffffff", [Bad, Bad, Bad, At(i64::MAX)]),
        ("1+9223372036854775807", [Bad, Bad, Bad, At(12)]),
        (
            "9223372036854775807+9223372036854775807",
            [Bad, Bad, Bad, At(12)],
        ),
        ("-9223372036854775808-1", [Bad, Bad, Bad, At(i64::MIN)]),
        ("end+9223372036854775807", [Bad, Bad, Bad, At(i64::MAX)]),
        ("end+9223372036854775808", [Bad, Bad, Bad, At(12)]),
        ("end+99999999999999999999", [Bad, Bad, Bad, At(12)]),
        ("end-9223372036854775808", [Bad, Bad, Bad, At(-1)]),
        (
            "18446744073709551615",
            [HostLong(-1), HostLong(-1), Bad, At(i64::MAX)],
        ),
        (
            "-0xffffffffffffffff",
            [HostLong(1), HostLong(1), Bad, At(i64::MIN)],
        ),
        (
            "18446744069414584321",
            [HostLong(1), HostLong(1), Bad, At(i64::MAX)],
        ),
        (
            "end-18446744073709551615",
            [HostLong(12), HostLong(12), Bad, At(-1)],
        ),
        ("0xffffffffffffffff+1", [Bad, HostLong(0), Bad, At(12)]),
    ];

    /// Each release reads one index as its own `string index` does, measured on
    /// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 over `abcdefghijkl` (8.5
    /// and 8.6 answer alike, as do 9.0 and 9.1, but for the integers the second
    /// table reads): `None` is `bad index`. Past 32 bits each release reads in
    /// its own range, measured with `string index`, `string
    /// range` from each side and `lset`: 8.4 to 8.6 wrap an integer within
    /// ±4294967295 to 32 bits and their sums and `end` offsets alike, 8.4 and
    /// 8.5 read a magnitude from 2^64 − 2^32 + 1 to 2^64 − 1 as a 64-bit `long`
    /// wraps it, which a 32-bit one cannot hold, and 9.0 reads a wide, a bignum
    /// alone as the nearest wide and a sum that reaches the widest, or a bignum
    /// offset after `end`, as `end+1`.
    #[test]
    fn each_release_reads_an_index_as_its_tclsh_does() {
        use tcl_syntax::number::NumberSyntax::{Tcl84, Tcl85, Tcl90};
        // (spec, 8.4, 8.5 and 8.6, 9.0 and 9.1)
        type Row = (&'static str, Option<i64>, Option<i64>, Option<i64>);
        let table: &[Row] = &[
            ("1", Some(1), Some(1), Some(1)),
            (" 1", Some(1), Some(1), Some(1)),
            ("1 ", Some(1), Some(1), Some(1)),
            ("+1", Some(1), Some(1), Some(1)),
            (" +1", Some(1), Some(1), Some(1)),
            ("- 1", None, None, None),
            ("-1", Some(-1), Some(-1), Some(-1)),
            ("0x2", Some(2), Some(2), Some(2)),
            ("010", Some(8), Some(8), Some(10)),
            ("0o10", None, Some(8), Some(8)),
            ("0b10", None, Some(2), Some(2)),
            ("1_0", None, None, Some(10)),
            ("1e0", None, None, None),
            ("end", Some(11), Some(11), Some(11)),
            ("e", Some(11), Some(11), None),
            ("en", Some(11), Some(11), None),
            ("endx", None, None, None),
            (" end", None, None, None),
            ("end ", None, None, None),
            ("end-1", Some(10), Some(10), Some(10)),
            ("end+1", None, Some(12), Some(12)),
            ("end--1", Some(12), Some(12), Some(12)),
            ("end-+1", Some(10), Some(10), Some(10)),
            ("end- 1", Some(10), None, None),
            ("end+ 1", None, None, None),
            ("end-1 ", Some(10), Some(10), Some(10)),
            ("end-0x1", Some(10), Some(10), Some(10)),
            ("end-010", Some(3), Some(3), Some(1)),
            ("end-", None, None, None),
            ("end+", None, None, None),
            ("end-1-1", None, None, None),
            ("end -1", None, None, None),
            ("1+1", None, Some(2), Some(2)),
            ("1-1", None, Some(0), Some(0)),
            ("1+-1", None, Some(0), Some(0)),
            ("1--1", None, Some(2), Some(2)),
            ("1++1", None, Some(2), Some(2)),
            ("+1+1", None, Some(2), Some(2)),
            ("1+ 1", None, None, None),
            ("1 +1", None, None, None),
            (" 1+1", None, Some(2), Some(2)),
            ("1+1 ", None, Some(2), Some(2)),
            ("0x1+1", None, Some(2), Some(2)),
            ("010+0", None, Some(8), Some(10)),
            ("-1+2", None, Some(1), Some(1)),
            ("1+end", None, None, None),
            ("end-1+1", None, None, None),
            ("1 0", None, None, None),
            ("{1}", None, None, None),
            ("", None, None, None),
            (" ", None, None, None),
        ];
        for &(spec, on_84, on_85, on_90) in table {
            assert_eq!(resolve_opt_with(spec, 12, Tcl84), on_84, "{spec:?} on 8.4");
            assert_eq!(resolve_opt_with(spec, 12, Tcl85), on_85, "{spec:?} on 8.5");
            assert_eq!(resolve_opt_with(spec, 12, Tcl90), on_90, "{spec:?} on 9.0");
        }

        for &(spec, readings) in PAST_32_BITS {
            for (release, want) in TclVersion::ALL.into_iter().zip([
                readings[0],
                readings[1],
                readings[2],
                readings[3],
                readings[3],
            ]) {
                assert_eq!(
                    read_under(spec, 12, release),
                    want,
                    "{spec:?} on {release:?}"
                );
            }
        }
        // The grammar 8.5 and 8.6 share names neither, so it cannot say which
        // reading a 64-bit `long` gives one of them.
        assert_eq!(
            read_with("18446744073709551615", 12, Tcl85),
            IndexReading::Unsure
        );
        assert_eq!(
            read_with("0xffffffffffffffff+1", 12, Tcl85),
            IndexReading::Unsure
        );
        assert_eq!(resolve_opt_with("18446744073709551615", 12, Tcl84), None);
    }

    /// 8.6 encodes a literal `end` offset that is positive after the 32-bit
    /// wrap as after the end, where the word read at run time sums with `end`
    /// in an `int`; the two part where that sum passes `INT_MAX`.
    /// Measured on a 12-element string and list, a literal against the same
    /// word in a variable (`programs/s1/litvar.tcl`): only 8.6 parts, and
    /// only on these rows.
    #[test]
    fn a_literal_end_offset_past_int_compiles_apart_in_8_6() {
        for (spec, apart) in [
            ("end+2147483647", true),
            ("end+2147483637", true),
            ("end+2147483636", false),
            ("end-2147483649", true),
            ("end--2147483647", true),
            ("end-2147483648", false),
            ("end-2147483647", false),
            ("end+4294967296", false),
            ("end+1", false),
            ("2147483647", false),
            ("1+2147483647", false),
        ] {
            assert_eq!(compiles_apart(spec, 12, TclVersion::V8_6), apart, "{spec}");
            for release in TclVersion::ALL {
                if release != TclVersion::V8_6 {
                    assert!(!compiles_apart(spec, 12, release), "{spec} on {release:?}");
                }
            }
        }
    }

    /// A literal index word fixed as each release reads it whatever the length,
    /// compiled in a body and read by its command at run time. Measured on
    /// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 with `string index`,
    /// `string range` from each side, `lindex` and `lrange` over a 12-element
    /// string and list, each word a literal inside a proc against the same
    /// word in a variable (`end+2147483647` in a proc ranges the whole string
    /// on 8.6 and nothing through a variable; 8.5 ranges nothing either way,
    /// 9.0 and 9.1 the whole string either way). `None` is a reading that turns
    /// on the length or the host, or `bad index`.
    #[test]
    fn a_literal_index_is_fixed_as_each_release_compiles_or_reads_it() {
        use Encoded::{After, Before, FromEnd, Index};
        use TclVersion::{V8_4, V8_5, V8_6, V9_0, V9_1};
        // (spec, release, compiled, read at run time)
        type Row = (&'static str, TclVersion, Option<Encoded>, Option<Encoded>);
        let table: &[Row] = &[
            ("end+2147483647", V8_6, Some(After), None),
            ("end+2147483646", V8_6, Some(After), None),
            ("end+2147483637", V8_6, Some(After), None),
            ("end+2147483636", V8_6, Some(After), None),
            ("end-2147483649", V8_6, Some(After), None),
            ("end--2147483647", V8_6, Some(After), None),
            ("end+1", V8_6, Some(After), None),
            (
                "end-2147483647",
                V8_6,
                Some(Before),
                Some(FromEnd(-2_147_483_647)),
            ),
            ("end-2147483648", V8_6, Some(Before), None),
            ("end+2147483648", V8_6, Some(Before), None),
            ("end-1", V8_6, Some(FromEnd(-1)), Some(FromEnd(-1))),
            ("end", V8_6, Some(FromEnd(0)), Some(FromEnd(0))),
            ("e", V8_6, Some(FromEnd(0)), Some(FromEnd(0))),
            ("end+4294967295", V8_6, Some(FromEnd(-1)), Some(FromEnd(-1))),
            ("end+4294967296", V8_6, None, None),
            (
                "2147483647",
                V8_6,
                Some(Index(2_147_483_647)),
                Some(Index(2_147_483_647)),
            ),
            ("2147483648", V8_6, Some(Before), Some(Before)),
            (
                "-2147483649",
                V8_6,
                Some(Index(2_147_483_647)),
                Some(Index(2_147_483_647)),
            ),
            ("1+2147483647", V8_6, Some(Before), Some(Before)),
            ("010", V8_6, Some(Index(8)), Some(Index(8))),
            ("-1", V8_6, Some(Before), Some(Before)),
            ("end+2147483647", V8_5, None, None),
            ("end-2147483649", V8_5, None, None),
            (
                "end-2147483647",
                V8_5,
                Some(FromEnd(-2_147_483_647)),
                Some(FromEnd(-2_147_483_647)),
            ),
            ("end-1", V8_5, Some(FromEnd(-1)), Some(FromEnd(-1))),
            ("2147483648", V8_5, Some(Before), Some(Before)),
            ("end-2147483649", V8_4, None, None),
            ("end+1", V8_4, None, None),
            ("end-1", V8_4, Some(FromEnd(-1)), Some(FromEnd(-1))),
            (
                "end+2147483647",
                V9_0,
                Some(FromEnd(2_147_483_647)),
                Some(FromEnd(2_147_483_647)),
            ),
            (
                "end-2147483649",
                V9_0,
                Some(FromEnd(-2_147_483_649)),
                Some(FromEnd(-2_147_483_649)),
            ),
            (
                "2147483648",
                V9_0,
                Some(Index(2_147_483_648)),
                Some(Index(2_147_483_648)),
            ),
            ("end+1", V9_1, Some(FromEnd(1)), Some(FromEnd(1))),
            ("010", V9_1, Some(Index(10)), Some(Index(10))),
        ];
        for &(spec, release, compiled, run_time) in table {
            assert_eq!(
                encode(spec, release, true),
                compiled,
                "{spec:?} compiled on {release:?}"
            );
            assert_eq!(
                encode(spec, release, false),
                run_time,
                "{spec:?} read on {release:?}"
            );
        }
    }

    /// The version-less entry takes the grammar this runtime was built for. The
    /// ambient starts at 9.0, so an unpinned process reads `010` as decimal —
    /// and pinning it to 8.5 flips the same spec to octal.
    #[test]
    fn the_versionless_entry_follows_the_installed_grammar() {
        use tcl_syntax::number::{NumberSyntax, runtime_syntax, set_runtime_syntax};
        let restore = runtime_syntax();
        set_runtime_syntax(NumberSyntax::Tcl90);
        assert_eq!(resolve_opt("010", 12), Some(10));
        set_runtime_syntax(NumberSyntax::Tcl85);
        assert_eq!(resolve_opt("010", 12), Some(8));
        set_runtime_syntax(restore);
    }

    /// The grammar-taking entry ignores the installed grammar. Measured on a
    /// 12-character subject with `string index` (LANG=C.UTF-8): `010` is 8
    /// and `end-010` is 3 on tclsh 8.4.20, 8.5.19 and 8.6.18, 10 and 1 on
    /// 9.0.4 and 9.1.0; `1_0` is `bad index` up to 8.6 and 10 from 9.0.
    #[test]
    fn the_grammar_taking_entry_ignores_the_installed_grammar() {
        use tcl_syntax::number::{NumberSyntax, runtime_syntax, set_runtime_syntax};
        let restore = runtime_syntax();
        for ambient in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
            set_runtime_syntax(ambient);
            for numbers in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
                assert_eq!(resolve_with("010", 12, numbers).ok(), Some(8));
                assert_eq!(resolve_with("end-010", 12, numbers).ok(), Some(3));
                assert!(resolve_with("1_0", 12, numbers).is_err());
            }
            assert_eq!(resolve_with("010", 12, NumberSyntax::Tcl90).ok(), Some(10));
            assert_eq!(
                resolve_with("end-010", 12, NumberSyntax::Tcl90).ok(),
                Some(1)
            );
            assert_eq!(resolve_with("1_0", 12, NumberSyntax::Tcl90).ok(), Some(10));
        }
        set_runtime_syntax(restore);
    }
}
#[test]
fn compiler_encoding_preserves_wide_target_uncertainty() {
    use tcl_dialect::{IndexSyntax, TclVersion};
    for version in TclVersion::ALL {
        let syntax = IndexSyntax::for_version(version);
        for index in ["0", "end", "end-2", "0x10", "-1"] {
            assert_eq!(compiler_encodable_in(index, syntax), Some(true));
        }
        if version >= TclVersion::V9_0 {
            assert_eq!(compiler_encodable_in("2147483648", syntax), None);
            assert_eq!(compiler_encodable_in("end-2147483648", syntax), None);
        }
    }
}
