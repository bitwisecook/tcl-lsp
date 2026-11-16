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

//! The host boundary: `NativeIndexOps`, the index result constructors and the entered-operation runners.

use crate::error::CmdError;
use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

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
pub(super) type IndexBinarySelection<R> = Result<Option<(R, std::rc::Rc<[u8]>)>, CmdError>;

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
