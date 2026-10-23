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

//! Original index conversion and real List/header ownership of C lindex.

use crate::CmdError;
use tcl_dialect::TclVersion;
use tcl_syntax::{
    native_end_offset::NativeEndOffset,
    number::Number,
    value::{ValueError, ValueOps},
};

/// Independent original consumer guard threaded through the existing selected
/// getter and mandatory owner-retirement stages. It grants no native role.
pub type NativeListIndexCheck<'a, O> = Option<&'a dyn Fn(&O) -> Result<(), CmdError>>;

fn check_stage<O: NativeListIndexOps>(
    ops: &mut O,
    check: NativeListIndexCheck<'_, O>,
) -> Result<(), CmdError> {
    let reached = check.map_or(Ok(()), |check| check(ops));
    if let Err(first) = &reached {
        ops.retain_index_failure(first);
    }
    reached
}

fn finish_using<O: NativeListIndexOps>(
    ops: &mut O,
    reached: Result<O::Owner, CmdError>,
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    if check.is_none() {
        return reached;
    }
    if let Err(error) = &reached {
        ops.retain_index_failure(error);
    }
    if let Err(first) = check_stage(ops, check) {
        drop(reached);
        let _ = check_stage(ops, check);
        return Err(first);
    }
    reached
}

/// Checked borrowed members of an actual native List header.
pub trait NativeIndexMembers<V> {
    /// Reject a stale original backing instead of reviving its retired children.
    ///
    /// # Errors
    /// The original native List backing is no longer live.
    fn index_elements(&self) -> Result<&[V], ValueError>;
}
impl<V> NativeIndexMembers<V> for Vec<V> {
    fn index_elements(&self) -> Result<&[V], ValueError> {
        Ok(self)
    }
}

/// Concrete object owners and borrowed List storage, independent of command ABI.
pub trait NativeListIndexOps: crate::index::NativeIndexOps {
    /// One real native header reference, released by its concrete owner.
    type Owner;
    /// Borrowed original members; inspection never adds child references.
    type Members: NativeIndexMembers<Self::Value>;
    /// Original option-path member storage; this lifetime view adds no native
    /// member references and refuses a withdrawn or mutated selected backing.
    type OptionMembers: NativeIndexMembers<Self::Value>;
    /// Selected physical C release, independently of the analysis profile.
    ///
    /// # Errors
    /// The actual C object issuer is missing.
    fn index_version(&self) -> Result<TclVersion, ValueError>;
    /// Actual local backend `Tcl_Size` and `ListStore` bounds, independently of original Int32.
    /// This descriptive layout does not grant a foreign C library issuer.
    ///
    /// # Errors
    /// The authentic original C operation/layout is unavailable.
    fn index_option_layout(
        &self,
    ) -> Result<crate::native_option_index::NativeOptionIndexLayout, CmdError>;
    /// Borrow the original header from its real owner.
    fn index_original(owner: &Self::Owner) -> &Self::Value;
    /// Retain one native header reference.
    fn index_retain(value: &Self::Value) -> Self::Owner;
    /// Allocate and own native `Tcl_NewObj`.
    fn index_empty(&mut self) -> Self::Owner;
    /// Independently selected `CString` Int32 temporary, separate from an
    /// original index/member header and its primary cache.
    ///
    /// # Errors
    /// Actual CString/Int32 worker or complete first Host refusal.
    fn index_int_c_string(&mut self, source: &[u8]) -> Result<Option<i32>, CmdError>;
    /// Original option-path List getter with a checked allocation-only view.
    ///
    /// # Errors
    /// Original conversion, selected backing withdrawal or full Host refusal.
    fn index_option_members_checked(
        &mut self,
        original: &Self::Value,
    ) -> Result<Self::OptionMembers, CmdError>;
    /// Actual List representation, without a getter.
    fn index_is_list(value: &Self::Value) -> bool;
    /// Original List getter with a memory-only member view.
    ///
    /// # Errors
    /// The original native List getter rejects its input or backing.
    fn index_members(&mut self, value: &Self::Value) -> Result<Self::Members, ValueError>;
    /// Reach ordinary `SetListFromAny`, independently of an abstract getter.
    /// Valid flat-index coordinates require this conversion before selection;
    /// a rejected coordinate does not enter it.
    ///
    /// # Errors
    /// The actual conversion or the host materialisation limit rejects input.
    fn index_ordinary_members(&mut self, value: &Self::Value) -> Result<Self::Members, ValueError>;
    /// Genuine `TclListObjCopy`; its whole backing shares the original children.
    ///
    /// # Errors
    /// The selected original List getter or copy capability is unavailable.
    fn index_list_copy(&mut self, value: &Self::Value) -> Result<Self::Owner, ValueError>;
    /// Checked original member conversion preserving the complete first Host cause.
    fn index_members_checked(&mut self, value: &Self::Value) -> Result<Self::Members, CmdError> {
        self.index_members(value).map_err(Into::into)
    }
    /// Checked ordinary conversion, separate from abstract `GetElements`.
    fn index_ordinary_members_checked(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::Members, CmdError> {
        self.index_ordinary_members(value).map_err(Into::into)
    }
    /// Checked source-selected List copy, including original conversion/duplication.
    fn index_list_copy_checked(&mut self, value: &Self::Value) -> Result<Self::Owner, CmdError> {
        self.index_list_copy(value).map_err(Into::into)
    }
    /// Error-neutral actual numeric getter on the SAME original header.
    ///
    /// # Errors
    /// The actual numeric getter or original header is unavailable.
    fn index_number(&mut self, value: &Self::Value) -> Result<Option<Number>, ValueError>;
    /// Checked original numeric stage preserving the complete Host cause.
    ///
    /// # Errors
    /// Original numeric/cache access or full first execution refusal.
    fn index_number_checked(&mut self, value: &Self::Value) -> Result<Option<Number>, CmdError> {
        self.index_number(value).map_err(Into::into)
    }
    /// Checked offset adoption after reached old-primary retirement.
    ///
    /// # Errors
    /// Original header/cache or full first execution refusal.
    fn index_install_offset_checked(
        &mut self,
        value: &Self::Value,
        offset: NativeEndOffset,
    ) -> Result<(), CmdError> {
        Self::index_install_offset(value, offset).map_err(Into::into)
    }
    /// Checked retirement of a reached arithmetic-prefix primary.
    ///
    /// # Errors
    /// Original header or full first execution refusal.
    fn index_clear_arithmetic_primary_checked(
        &mut self,
        value: &Self::Value,
    ) -> Result<(), CmdError> {
        Self::index_clear_arithmetic_primary(value);
        Ok(())
    }
    /// Same member converter under an independent original consumer guard.
    /// # Errors
    /// Original conversion or the complete first consumer refusal.
    fn index_members_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<Self::Members, CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_members_checked(value);
        check_stage(self, check)?;
        reached
    }
    /// Same ordinary converter under an independent original consumer guard.
    /// # Errors
    /// Original conversion or the complete first consumer refusal.
    fn index_ordinary_members_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<Self::Members, CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_ordinary_members_checked(value);
        check_stage(self, check)?;
        reached
    }
    /// Same selected Copy worker under an original consumer guard.
    /// # Errors
    /// Original copy/conversion or the complete first consumer refusal.
    fn index_list_copy_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<Self::Owner, CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_list_copy_checked(value);
        finish_using(self, reached, check)
    }
    /// Same scalar probe under an original consumer guard.
    /// # Errors
    /// Original scalar/cache access or the complete first consumer refusal.
    fn index_number_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<Option<Number>, CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_number_checked(value);
        check_stage(self, check)?;
        reached
    }
    /// Same String worker under an original consumer guard.
    /// # Errors
    /// Original String access or the complete first consumer refusal.
    fn index_bytes_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<std::rc::Rc<[u8]>, CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_operand_bytes(value);
        check_stage(self, check)?;
        reached
    }
    /// Same lazy Length worker under an original consumer guard.
    /// # Errors
    /// Original length/conversion or the complete first consumer refusal.
    fn index_length_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<usize, CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_list_length(value);
        check_stage(self, check)?;
        reached
    }
    /// Same offset publication under an original consumer guard.
    /// # Errors
    /// Original primary/cache access or the complete first consumer refusal.
    fn index_install_offset_using(
        &mut self,
        value: &Self::Value,
        offset: NativeEndOffset,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<(), CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_install_offset_checked(value, offset);
        check_stage(self, check)?;
        reached
    }
    /// Same arithmetic primary retirement under an original consumer guard.
    /// # Errors
    /// Original primary access or the complete first consumer refusal.
    fn index_clear_arithmetic_primary_using(
        &mut self,
        value: &Self::Value,
        check: NativeListIndexCheck<'_, Self>,
    ) -> Result<(), CmdError>
    where
        Self: Sized,
    {
        check_stage(self, check)?;
        let reached = self.index_clear_arithmetic_primary_checked(value);
        check_stage(self, check)?;
        reached
    }
    /// Existing original end-offset cache.
    fn index_offset(value: &Self::Value) -> Option<NativeEndOffset>;
    /// Install the conversion on the original resident object.
    ///
    /// # Errors
    /// The original header is retired or lacks its required resident string.
    fn index_install_offset(value: &Self::Value, offset: NativeEndOffset)
    -> Result<(), ValueError>;
    /// Release temporary C9 numeric primaries after arithmetic prefix parsing.
    fn index_clear_arithmetic_primary(value: &Self::Value);
    /// Actual abstract-list index hook, absent on ordinary List headers.
    fn index_is_abstract(_value: &Self::Value) -> bool {
        false
    }
}

/// Actual ordinary List range transaction after original index conversion.
pub trait NativeListRangeOps: NativeListIndexOps {
    /// Slice the original physical header, sampling sharing before a result hold.
    ///
    /// # Errors
    /// The original List storage, abstract slice provider or recipe is unavailable.
    fn range_original(
        &mut self,
        original: &Self::Value,
        first: i64,
        last: i64,
    ) -> Result<Self::Owner, ValueError>;
    /// Same actual selected range owner under the retained original command.
    fn range_original_checked(
        &mut self,
        original: &Self::Value,
        first: i64,
        last: i64,
    ) -> Result<Self::Owner, CmdError> {
        self.range_original(original, first, last)
            .map_err(Into::into)
    }
}

/// Reached C command range getter order and actual result ownership.
///
/// # Errors
/// An original List/index getter or physical slicing capability rejects input.
pub fn command_range<O: NativeListRangeOps>(
    ops: &mut O,
    original: &O::Value,
    first: &O::Value,
    last: &O::Value,
) -> Result<O::Owner, CmdError> {
    crate::index::run_index_output(
        ops,
        |ops| command_range_inner(ops, original, first, last),
        |_, owner| drop(owner),
        std::convert::identity,
    )
}

fn command_range_inner<O: NativeListRangeOps>(
    ops: &mut O,
    original: &O::Value,
    first: &O::Value,
    last: &O::Value,
) -> Result<O::Owner, CmdError> {
    // naming.list.original-range-objects-and-instructions

    // Actual command worker reaches ListLength, then each SAME index object.
    ops.index_version()?;
    let length = ops.index_list_length(original)?;
    let end = i64::try_from(length).expect("native List length") - 1;
    let first = original_index(ops, first, end, true)?.expect("reported original first");
    let last = original_index(ops, last, end, true)?.expect("reported original last");
    ops.range_original_checked(original, first, last)
}

/// Native immediate extraction converts the original header before selection.
/// A missing member creates a fresh empty object; it does not borrow a shared result.
///
/// # Errors
/// The selected original getter or abstract-list provider is unavailable.
pub fn immediate<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
    coordinate: tcl_syntax::native_compiled_index::NativeCompiledListIndex,
) -> Result<O::Owner, CmdError> {
    immediate_using(ops, original, coordinate, None)
}

/// Same selected worker with the captured original consumer guard.
/// # Errors
/// Original getter/cache access or the complete first consumer refusal.
pub fn immediate_using<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
    coordinate: tcl_syntax::native_compiled_index::NativeCompiledListIndex,
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    check_stage(ops, check)?;
    let reached = crate::index::run_index_output(
        ops,
        |ops| immediate_inner(ops, original, coordinate, check),
        |_, owner| drop(owner),
        std::convert::identity,
    );
    finish_using(ops, reached, check)
}

fn immediate_inner<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
    coordinate: tcl_syntax::native_compiled_index::NativeCompiledListIndex,
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    if ops.index_version()? >= TclVersion::V9_0 && O::index_is_abstract(original) {
        let length = ops.index_length_using(original, check)?;
        return match coordinate.resolve(length) {
            Some(index) => Ok(ops
                .list_index(original, index)?
                .as_ref()
                .map_or_else(|| ops.index_empty(), O::index_retain)),
            None => Ok(ops.index_empty()),
        };
    }
    let members = ops.index_members_using(original, check)?;
    let members = members.index_elements()?;
    Ok(coordinate.resolve(members.len()).map_or_else(
        || ops.index_empty(),
        |index| O::index_retain(&members[index]),
    ))
}

/// Original `Tcl_GetIntForIndex` conversion; a silent failed probe returns None.
///
/// # Errors
/// Missing physical getter/storage authority; reported probes also return the native bad-index error.
pub fn original_index<O: NativeListIndexOps>(
    ops: &mut O,
    value: &O::Value,
    end: i64,
    report: bool,
) -> Result<Option<i64>, CmdError> {
    original_index_using(ops, value, end, report, None)
}

/// Same selected worker with the captured original consumer guard.
/// # Errors
/// Original getter/cache access or the complete first consumer refusal.
pub fn original_index_using<O: NativeListIndexOps>(
    ops: &mut O,
    value: &O::Value,
    end: i64,
    report: bool,
    check: NativeListIndexCheck<'_, O>,
) -> Result<Option<i64>, CmdError> {
    check_stage(ops, check)?;
    let reached = crate::index::run_index_metadata(ops, |ops| {
        original_index_inner(ops, value, end, report, check)
    });
    if check.is_some()
        && let Err(first) = &reached
    {
        ops.retain_index_failure(first);
    }
    check_stage(ops, check)?;
    reached
}
fn original_index_inner<O: NativeListIndexOps>(
    ops: &mut O,
    value: &O::Value,
    end: i64,
    report: bool,
    check: NativeListIndexCheck<'_, O>,
) -> Result<Option<i64>, CmdError> {
    let version = ops.index_version()?;
    match ops.index_number_using(value, check)? {
        Some(Number::Int(integer)) => {
            return Ok(Some(if version >= TclVersion::V9_0 && integer < 0 {
                if end == -1 { i64::MIN } else { -1 }
            } else {
                integer
            }));
        }
        Some(Number::Big { negative, .. }) if version >= TclVersion::V9_0 => {
            return Ok(Some(if negative { i64::MIN } else { i64::MAX }));
        }
        _ => {}
    }
    if let Some(offset) = O::index_offset(value) {
        if offset.version() != version {
            return Err(
                ValueError::CommandProtocolUnavailable("original end-offset version").into(),
            );
        }
        return Ok(Some(offset.resolve(end)));
    }
    let bytes = ops.index_bytes_using(value, check)?;
    let Ok(spelling) = std::str::from_utf8(&bytes) else {
        return if report {
            Err(crate::index::bad_index_for_ops(
                ops,
                &bytes,
                tcl_dialect::IndexSyntax::for_version(version),
            ))
        } else {
            Ok(None)
        };
    };
    convert_spelling(ops, value, version, spelling, end, report, check)
}

fn convert_spelling<O: NativeListIndexOps>(
    ops: &mut O,
    original: &O::Value,
    version: TclVersion,
    spelling: &str,
    end: i64,
    report: bool,
    check: NativeListIndexCheck<'_, O>,
) -> Result<Option<i64>, CmdError> {
    let syntax = tcl_dialect::IndexSyntax::for_version(version);
    let spelling = if version < TclVersion::V9_0 && spelling.starts_with('e') {
        std::str::from_utf8(tcl_core_types::c_string_extent(spelling.as_bytes()))
            .expect("prefix of reached UTF spelling")
    } else {
        spelling
    };
    if version >= TclVersion::V9_0 && !spelling.starts_with('e') {
        if tcl_syntax::list::split_list(spelling).is_ok_and(|parts| parts.len() > 1) {
            drop(ops.index_members_using(original, check)?);
            return failed_spelling(ops, spelling, syntax, report);
        }
        let mut flags = tcl_syntax::number::ParseFlags::for_syntax(syntax.numbers);
        flags.integer_only = true;
        if tcl_syntax::number::parse(spelling.trim_start(), flags).is_some() {
            ops.index_clear_arithmetic_primary_using(original, check)?;
        }
    }
    let length = usize::try_from(end.saturating_add(1)).unwrap_or(0);
    if let Some(result) = crate::index::resolve_opt_in(spelling, length, syntax) {
        if version >= TclVersion::V9_0 || spelling.starts_with('e') {
            let value = crate::index::resolve_opt_in(spelling, 1, syntax)
                .expect("same valid index grammar");
            let encoded = if version >= TclVersion::V9_0 {
                wide_offset_encoding(spelling, value, syntax)
            } else {
                value
            };
            let offset = NativeEndOffset::new(version, encoded);
            ops.index_install_offset_using(original, offset, check)?;
            return Ok(Some(offset.resolve(end)));
        }
        return Ok(Some(result));
    }
    if version >= TclVersion::V9_0 && crate::index::wide_expression(spelling, syntax).is_some() {
        match crate::index::resolve_for_ops(ops, spelling, 1) {
            Ok(result) => {
                let offset =
                    NativeEndOffset::new(version, wide_offset_encoding(spelling, result, syntax));
                ops.index_install_offset_using(original, offset, check)?;
                return Ok(Some(offset.resolve(end)));
            }
            Err(error) if report || error.native_execution_refusal().is_some() => {
                return Err(error);
            }
            Err(_) => return Ok(None),
        }
    }
    failed_spelling(ops, spelling, syntax, report)
}
fn failed_spelling<O: ValueOps>(
    ops: &O,
    spelling: &str,
    syntax: tcl_dialect::IndexSyntax,
    report: bool,
) -> Result<Option<i64>, CmdError> {
    if report {
        Err(crate::index::bad_index_for_ops(ops, spelling, syntax))
    } else {
        Ok(None)
    }
}

fn wide_offset_encoding(spelling: &str, value: i64, syntax: tcl_dialect::IndexSyntax) -> i64 {
    if let Some(rest) = spelling.strip_prefix("end") {
        if !rest.is_empty() {
            let mut flags = tcl_syntax::number::ParseFlags::for_syntax(syntax.numbers);
            flags.integer_only = true;
            match tcl_syntax::number::parse_whole_with(&rest[1..], flags) {
                Some(Number::Big { negative, .. }) => {
                    return if negative == rest.starts_with('-') {
                        i64::MAX
                    } else {
                        i64::MIN
                    };
                }
                Some(Number::Int(i64::MIN)) if rest.starts_with('-') => return i64::MAX,
                _ => {}
            }
        }
        match value {
            1 => i64::MAX,
            positive if positive > 1 => i64::MAX - 1,
            i64::MIN => i64::MIN,
            relative => relative - 1,
        }
    } else if value < 0 {
        if value == -1 { i64::MIN } else { i64::MIN + 1 }
    } else {
        value
    }
}

/// Native single-index/path entry, preserving the original index object.
///
/// # Errors
/// Original getter, List conversion, or native index errors.
pub fn single<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    index: &O::Value,
) -> Result<O::Owner, CmdError> {
    single_using(ops, list, index, None)
}

/// Same selected worker with the captured original consumer guard.
/// # Errors
/// Original getter/cache access or the complete first consumer refusal.
pub fn single_using<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    index: &O::Value,
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    check_stage(ops, check)?;
    let reached = crate::index::run_index_output(
        ops,
        |ops| single_inner(ops, list, index, check),
        |_, owner| drop(owner),
        std::convert::identity,
    );
    finish_using(ops, reached, check)
}

fn single_inner<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    index: &O::Value,
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    let version = ops.index_version()?;
    let initial_end = if version >= TclVersion::V9_0 {
        i64::MAX - 1
    } else {
        0
    };
    if !O::index_is_list(index)
        && original_index_using(ops, index, initial_end, false, check)?.is_some()
    {
        return flat_using(ops, list, std::slice::from_ref(index), check);
    }
    if version == TclVersion::V8_4 {
        return match ops.index_members_using(index, check) {
            Ok(_) => path84(ops, list, index, check),
            Err(error) if error.native_execution_refusal().is_some() => Err(error),
            Err(_) => flat_using(ops, list, std::slice::from_ref(index), check),
        };
    }
    let private = match ops.index_list_copy_using(index, check) {
        Ok(copy) => copy,
        Err(error) if error.native_execution_refusal().is_some() => return Err(error),
        Err(_) => return flat_using(ops, list, std::slice::from_ref(index), check),
    };
    let members = ops.index_members_using(O::index_original(&private), check)?;
    flat_using(ops, list, members.index_elements()?, check)
}

fn path84<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    path: &O::Value,
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    let mut current = O::index_retain(list);
    let mut step = 0;
    loop {
        let indices = ops.index_members_using(path, check)?;
        let Some(index) = indices.index_elements()?.get(step) else {
            return Ok(current);
        };
        let length = ops
            .index_members_using(O::index_original(&current), check)?
            .index_elements()?
            .len();
        let selected = original_index_using(
            ops,
            index,
            i64::try_from(length).unwrap_or(i64::MAX) - 1,
            true,
            check,
        )?
        .expect("reported original index");
        let Some(selected) = usize::try_from(selected)
            .ok()
            .filter(|&index| index < length)
        else {
            drop(current);
            check_stage(ops, check)?;
            return Ok(ops.index_empty());
        };
        let members = ops.index_members_using(O::index_original(&current), check)?;
        let next = O::index_retain(&members.index_elements()?[selected]);
        drop(current);
        check_stage(ops, check)?;
        current = next;
        step += 1;
    }
}

/// `TclLindexFlat` with source-selected sublist copies and real returned ownership.
///
/// # Errors
/// Original getter, List conversion, or native index errors.
pub fn flat<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    indices: &[O::Value],
) -> Result<O::Owner, CmdError> {
    flat_using(ops, list, indices, None)
}

/// Same selected worker with the captured original consumer guard.
/// # Errors
/// Original getter/cache access or the complete first consumer refusal.
pub fn flat_using<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    indices: &[O::Value],
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    check_stage(ops, check)?;
    let reached = crate::index::run_index_output(
        ops,
        |ops| flat_inner(ops, list, indices, check),
        |_, owner| drop(owner),
        std::convert::identity,
    );
    finish_using(ops, reached, check)
}

fn flat_inner<O: NativeListIndexOps>(
    ops: &mut O,
    list: &O::Value,
    indices: &[O::Value],
    check: NativeListIndexCheck<'_, O>,
) -> Result<O::Owner, CmdError> {
    let version = ops.index_version()?;
    if version >= TclVersion::V9_0 && indices.len() == 1 && O::index_is_abstract(list) {
        let length = ops.index_length_using(list, check)?;
        let index = original_index_using(
            ops,
            &indices[0],
            i64::try_from(length).unwrap_or(i64::MAX) - 1,
            true,
            check,
        )?
        .expect("reported original index");
        return match usize::try_from(index).ok().filter(|&index| index < length) {
            Some(index) => Ok(ops
                .list_index(list, index)?
                .as_ref()
                .map_or_else(|| ops.index_empty(), O::index_retain)),
            None => Ok(ops.index_empty()),
        };
    }
    let mut current = O::index_retain(list);
    for (position, index) in indices.iter().enumerate() {
        if matches!(version, TclVersion::V8_5 | TclVersion::V8_6) {
            let copy = ops.index_list_copy_using(O::index_original(&current), check);
            drop(current);
            check_stage(ops, check)?;
            current = copy?;
        }
        let length = if version >= TclVersion::V9_0 {
            ops.index_length_using(O::index_original(&current), check)?
        } else {
            ops.index_members_using(O::index_original(&current), check)?
                .index_elements()?
                .len()
        };
        let selected = original_index_using(
            ops,
            index,
            i64::try_from(length).unwrap_or(i64::MAX) - 1,
            true,
            check,
        )?
        .expect("reported original index");
        let Some(selected) = usize::try_from(selected)
            .ok()
            .filter(|&index| index < length)
        else {
            if version != TclVersion::V8_4 {
                let end = if version >= TclVersion::V9_0 {
                    i64::MAX - 1
                } else {
                    -1
                };
                for rest in &indices[position + 1..] {
                    original_index_using(ops, rest, end, true, check)?;
                }
            }
            drop(current);
            check_stage(ops, check)?;
            return Ok(ops.index_empty());
        };
        let members = ops.index_ordinary_members_using(O::index_original(&current), check)?;
        let next = O::index_retain(&members.index_elements()?[selected]);
        drop(current);
        check_stage(ops, check)?;
        current = next;
    }
    Ok(current)
}
