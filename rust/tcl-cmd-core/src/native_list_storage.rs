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

//! Native range disposition from the original header and allocated List store.

use std::ops::Range;
use tcl_dialect::TclVersion;
use tcl_syntax::{native_compiled_index::NativeCompiledListRange, value::ValueError};

/// Resident string state sampled before the original List getter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeListStringState {
    /// No resident bytes, or a resident empty string.
    AbsentOrEmpty,
    /// A resident string containing at least one byte.
    Nonempty,
}

impl NativeListStringState {
    /// Preserve the original resident-string observation without requesting bytes.
    #[must_use]
    pub const fn from_empty(empty: bool) -> Self {
        if empty {
            Self::AbsentOrEmpty
        } else {
            Self::Nonempty
        }
    }
}

/// Physical state sampled after the original List getter, before a result hold.
#[derive(Clone, Copy, Debug)]
pub struct NativeListRangeStorage {
    /// Visible original members.
    pub length: usize,
    /// Actual header references before a result hold.
    pub header_shared: bool,
    /// Multiple genuine header owners of this member store.
    pub store_shared: bool,
    /// An actual selected span record is present.
    pub has_span: bool,
    /// Original owning slots, including members outside shared windows.
    pub used: usize,
    /// Extent issued at native construction or conversion.
    pub allocated: Option<usize>,
    /// Resident string is absent or empty, before any getter.
    pub string: NativeListStringState,
}

/// Result header and backing mutation selected by the actual range protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeListRangeAction {
    /// Retain the original empty header without changing its primary.
    OriginalEmpty,
    /// Allocate native `Tcl_NewObj` without a List primary.
    FreshEmpty,
    /// C9 command range installs an empty List store of capacity one.
    /// This is distinct from an opcode's native `Tcl_NewObj` result.
    EmptyList,
    /// Allocate a genuine new store owning these original members.
    FreshMembers(Range<usize>),
    /// Retain the complete original store and its existing span.
    Whole,
    /// Select a physical window, sharing the store or retiring unreachable slots.
    Window {
        /// Visible member indices relative to the original header's window.
        range: Range<usize>,
        /// Create or retain a native span instead of moving slots to the front.
        span: bool,
    },
}

/// Decide the original `LIST_RANGE_IMM` transaction without acquiring child owners.
///
/// # Errors
/// A C9 span decision needs an originally issued allocation extent.
pub fn range_action(
    version: TclVersion,
    coordinate: NativeCompiledListRange,
    storage: NativeListRangeStorage,
) -> Result<NativeListRangeAction, ValueError> {
    use NativeListRangeAction as Action;
    if version >= TclVersion::V8_6 && storage.length == 0 {
        return Ok(if storage.string == NativeListStringState::AbsentOrEmpty {
            Action::OriginalEmpty
        } else {
            Action::FreshEmpty
        });
    }
    let end = storage.length as i128 - 1;
    // C9 NONE in the last coordinate is an opcode-level empty result.
    if version >= TclVersion::V9_0 && coordinate.last.encoded() == -1 {
        return Ok(Action::FreshEmpty);
    }
    let first = coordinate.first.decode(end).max(0);
    let last = coordinate.last.decode(end).min(end);
    if first > last {
        return Ok(Action::FreshEmpty);
    }
    let range = usize::try_from(first).expect("clamped native first")
        ..usize::try_from(last + 1).expect("clamped native last");
    range_storage_action(version, range, storage)
}

/// Select the reached ordinary C command range after both original index
/// getters. C9's `TclListObjRange` always installs a List primary, including
/// an empty range; the immediate opcode has its separate early empty result.
///
/// # Errors
/// A C9 span decision needs the originally issued allocation extent.
pub fn command_range_action(
    version: TclVersion,
    first: i64,
    last: i64,
    storage: NativeListRangeStorage,
) -> Result<NativeListRangeAction, ValueError> {
    let end = storage.length as i128 - 1;
    let first = i128::from(first).max(0);
    let last = i128::from(last).min(end);
    if first > last {
        return Ok(if version >= TclVersion::V9_0 {
            NativeListRangeAction::EmptyList
        } else {
            NativeListRangeAction::FreshEmpty
        });
    }
    let range = usize::try_from(first).expect("clamped native first")
        ..usize::try_from(last + 1).expect("clamped native last");
    // C8.6's command worker copies a shared member store as well as a
    // shared header. The immediate opcode's existing transaction differs.
    if version == TclVersion::V8_6 && storage.store_shared {
        return Ok(NativeListRangeAction::FreshMembers(range));
    }
    range_storage_action(version, range, storage)
}

/// Original selected range door, independently of the physical adapter.
#[derive(Clone, Copy, Debug)]
pub enum NativeListRangeSelection {
    /// Compiler-issued immediate coordinates.
    Immediate(NativeCompiledListRange),
    /// Both original command index objects have already been converted.
    Command {
        /// Reached first index getter result.
        first: i64,
        /// Reached last index getter result.
        last: i64,
    },
}
impl NativeListRangeSelection {
    /// Decide a transaction from physical state before any result hold.
    ///
    /// # Errors
    /// Required original allocation geometry is absent.
    pub fn action(
        self,
        version: TclVersion,
        storage: NativeListRangeStorage,
    ) -> Result<NativeListRangeAction, ValueError> {
        match self {
            Self::Immediate(coordinates) => range_action(version, coordinates, storage),
            Self::Command { first, last } => command_range_action(version, first, last, storage),
        }
    }
    /// Whether the immediate opcode's canonical-empty short circuit applies.
    #[must_use]
    pub const fn is_immediate(self) -> bool {
        matches!(self, Self::Immediate(_))
    }
}

/// Select a validated range of the current `ListStore`. Private pure deletion
/// uses this without the opcode's early empty-coordinate return.
///
/// # Errors
/// A C9 span decision needs an originally issued allocation extent.
pub fn range_storage_action(
    version: TclVersion,
    range: Range<usize>,
    storage: NativeListRangeStorage,
) -> Result<NativeListRangeAction, ValueError> {
    use NativeListRangeAction as Action;
    if version < TclVersion::V8_6 || (version < TclVersion::V9_0 && storage.header_shared) {
        return Ok(Action::FreshMembers(range));
    }
    if version < TclVersion::V9_0 {
        return Ok(Action::Window { range, span: false });
    }
    if range.start == 0 && range.end == storage.length {
        return Ok(Action::Whole);
    }
    // Tcl's prefix trim precedes the otherwise eligible span branch.
    if range.start == 0 && !storage.header_shared && !storage.store_shared && !storage.has_span {
        return Ok(Action::Window { range, span: false });
    }
    let length = range.len();
    let span = if length < 101 || length < storage.used / 2 {
        false
    } else {
        let allocated = storage
            .allocated
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native List allocation extent",
            ))?;
        length >= allocated / 2 - allocated / 8
    };
    if span || (!storage.header_shared && !storage.store_shared) {
        Ok(Action::Window { range, span })
    } else {
        Ok(Action::FreshMembers(range))
    }
}

/// Original store geometry for an unshared selected header's replacement.
#[derive(Clone, Copy, Debug)]
pub struct NativeListReplaceStorage {
    /// Visible original members.
    pub length: usize,
    /// Original owning slots, including members outside shared windows.
    pub used: usize,
    /// Selected allocation extent in slots.
    pub allocated: usize,
    /// First occupied slot in the allocated store.
    pub first_used: usize,
    /// Original header window starts at this physical slot.
    pub window_start: usize,
    /// An actual selected span record is present.
    pub has_span: bool,
    /// Multiple genuine header owners of this member store.
    pub store_shared: bool,
}

/// Physical slots selected by Tcl 9 `ListStore` replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeListReplaceLayout {
    /// Allocate a new genuine member store.
    pub new_store: bool,
    /// Insert before existing used slots without touching other windows.
    pub shared_prepend: bool,
    /// Selected allocation extent in slots.
    pub allocated: usize,
    /// First occupied slot in the allocated store.
    pub first_used: usize,
    /// Selected header retains a span.
    pub span: bool,
}

/// Allocation attempt of the selected Tcl 9 growing `ListStore` constructor.
/// The adapter allocates this extent before publishing its storage metadata.
#[must_use]
fn growing_capacity(needed: usize) -> usize {
    needed
        .checked_add(needed / 2)
        .expect("native List capacity overflow")
}

/// Select storage for nonempty insertions and interior deletions. Pure end
/// deletions belong to the range transaction and no-op retains its backing.
#[must_use]
pub fn replace_layout(
    storage: NativeListReplaceStorage,
    first: usize,
    delete: usize,
    insert: usize,
) -> NativeListReplaceLayout {
    let final_length = storage.length - delete + insert;
    if delete == 0 && first == storage.length {
        return append_layout(storage, insert);
    }
    if delete == 0
        && first == 0
        && storage.window_start == storage.first_used
        && insert <= storage.first_used
    {
        return NativeListReplaceLayout {
            new_store: false,
            shared_prepend: storage.store_shared,
            allocated: storage.allocated,
            first_used: storage.first_used - insert,
            span: true,
        };
    }
    let allocated = if !storage.store_shared && final_length > storage.allocated {
        growing_capacity(final_length)
    } else {
        storage.allocated
    };
    if storage.store_shared || final_length < allocated / 4 {
        let allocated = growing_capacity(final_length.max(1));
        let extra = allocated - final_length;
        let first_used = if first + delete >= storage.length {
            extra / 4
        } else if first == 0 {
            extra - extra / 4
        } else {
            extra / 2
        };
        return NativeListReplaceLayout {
            new_store: true,
            shared_prepend: false,
            allocated,
            first_used,
            span: first_used != 0,
        };
    }
    let shift = replacement_lead_shift(storage, allocated, first, delete, insert);
    let first_used =
        usize::try_from(storage.window_start as i128 + shift).expect("native List leading space");
    NativeListReplaceLayout {
        new_store: false,
        shared_prepend: false,
        allocated,
        first_used,
        span: storage.has_span || first_used != 0,
    }
}

fn append_layout(storage: NativeListReplaceStorage, insert: usize) -> NativeListReplaceLayout {
    let final_length = storage.length + insert;
    if storage.store_shared {
        let allocated = growing_capacity(final_length);
        let first_used = if storage.has_span {
            (allocated - final_length) / 4
        } else {
            0
        };
        return NativeListReplaceLayout {
            new_store: true,
            shared_prepend: false,
            allocated,
            first_used,
            span: first_used != 0,
        };
    }
    let allocated = if final_length > storage.allocated {
        growing_capacity(final_length)
    } else {
        storage.allocated
    };
    let tail_free = allocated - storage.window_start - storage.length;
    let first_used = if tail_free < insert {
        storage.window_start - (insert - tail_free + (allocated - final_length) / 2)
    } else {
        storage.window_start
    };
    NativeListReplaceLayout {
        new_store: false,
        shared_prepend: false,
        allocated,
        first_used,
        span: storage.has_span || first_used != 0,
    }
}

fn replacement_lead_shift(
    storage: NativeListReplaceStorage,
    allocated: usize,
    first: usize,
    delete: usize,
    insert: usize,
) -> i128 {
    let change = insert as i128 - delete as i128;
    let tail = storage.length - first - delete;
    if change <= 0 {
        return if first > tail { 0 } else { -change };
    }
    let change = usize::try_from(change).expect("positive native List change");
    let head_space = storage.window_start;
    let tail_space = allocated - storage.window_start - storage.length;
    let final_free = head_space + tail_space - change;
    if head_space >= change && (first < tail || tail_space < change) {
        let mut shift = -(change as i128);
        if final_free > 1 && (tail_space == 0 || tail == 0) {
            let remaining = head_space - change;
            if remaining > final_free / 2 {
                shift -= (remaining - final_free / 2) as i128;
            }
        }
        shift
    } else if tail_space >= change {
        if final_free > 1 && (head_space == 0 || first == 0) {
            let remaining = tail_space - change;
            if remaining > final_free / 2 {
                return (remaining - final_free / 2) as i128;
            }
        }
        0
    } else {
        let mut down = head_space - final_free / 2;
        if change - down > tail_space {
            down += 1;
        }
        -(down as i128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex;
    fn coordinates(first: i32, last: i32) -> NativeCompiledListRange {
        NativeCompiledListRange {
            first: NativeCompiledListIndex::from_encoded(first),
            last: NativeCompiledListIndex::from_encoded(last),
        }
    }
    fn shared(used: usize, allocated: usize) -> NativeListRangeStorage {
        NativeListRangeStorage {
            length: used,
            header_shared: true,
            store_shared: false,
            has_span: false,
            used,
            allocated: Some(allocated),
            string: NativeListStringState::Nonempty,
        }
    }
    #[test]
    fn command_empty_range_keeps_c9_list_birth_separate_from_opcode_empty() {
        // naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md
        // Actual R3 has no listRangeImm in either C9 captured instruction stream.
        for (version, fixture) in [
            (
                TclVersion::V9_0,
                include_str!("../../tcl-registry/tests/data/native_list_operations/9.0.4.txt"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../../tcl-registry/tests/data/native_list_operations/9.1.0.txt"),
            ),
        ] {
            let row = fixture
                .lines()
                .find(|line| line.starts_with("R|3|"))
                .unwrap();
            let fields = row.split('|').collect::<Vec<_>>();
            assert_eq!(&fields[3..6], &["list", "0", "1"]);
            assert!(!fields[7].contains("listRangeImm"));
            let state = shared(3, 3);
            assert_eq!(
                command_range_action(version, 12, 2, state).unwrap(),
                NativeListRangeAction::EmptyList
            );
            assert_eq!(
                range_action(version, coordinates(12, -2), state).unwrap(),
                NativeListRangeAction::FreshEmpty
            );
        }
        assert_eq!(
            command_range_action(TclVersion::V8_6, 12, 2, shared(3, 3)).unwrap(),
            NativeListRangeAction::FreshEmpty
        );
    }

    #[test]
    fn range_storage_geometry_matches_288_actual_c9_owner_windows() {
        // Native proof naming.list-storage.original-whole-header-publication:
        // docs/design/analysis/name-resolution-proofs/list-storage-original-whole-header-publication.md
        // Native proof naming.list-storage.original-prefix-and-span-geometry:
        // docs/design/analysis/name-resolution-proofs/list-storage-original-prefix-and-span-geometry.md
        // Native proof naming.list-storage.original-shared-header-versus-store:
        // docs/design/analysis/name-resolution-proofs/list-storage-original-shared-header-versus-store.md
        for (version, fixture) in [
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_list_storage/9.0.4.txt"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_list_storage/9.1.0.txt"),
            ),
        ] {
            check_captured_ranges(version, fixture);
        }
    }

    fn check_captured_ranges(version: TclVersion, fixture: &str) {
        let mut captured = None;
        let mut checked = 0;
        for line in fixture.lines() {
            let fields = line.split('|').collect::<Vec<_>>();
            if fields[0] == "C" {
                let number = |index: usize| fields[index].parse::<usize>().unwrap();
                captured = Some((
                    number(1),
                    number(2),
                    number(3),
                    number(4),
                    number(5),
                    number(6),
                ));
            } else if fields[0] == "S" && fields[2] == "result" {
                let (id, used, allocated, sharing, first, last) = captured.unwrap();
                let action = range_action(
                    version,
                    coordinates(i32::try_from(first).unwrap(), i32::try_from(last).unwrap()),
                    NativeListRangeStorage {
                        length: used,
                        header_shared: sharing == 1,
                        store_shared: sharing == 2,
                        has_span: false,
                        used,
                        allocated: Some(allocated),
                        string: NativeListStringState::AbsentOrEmpty,
                    },
                )
                .unwrap();
                let (same, span) = match action {
                    NativeListRangeAction::Whole => (true, false),
                    NativeListRangeAction::Window { span, .. } => (true, span),
                    NativeListRangeAction::FreshMembers(_) => (false, false),
                    _ => panic!("nonempty captured range {id}"),
                };
                let length = last - first + 1;
                let result_used =
                    if same && (sharing != 0 || matches!(action, NativeListRangeAction::Whole)) {
                        used
                    } else {
                        length
                    };
                let capacity = if same { allocated } else { length };
                let offset = if span && sharing == 0 { first } else { 0 };
                assert_eq!(
                    fields[5].parse::<usize>().unwrap(),
                    offset,
                    "{version:?}/{id} firstUsed"
                );
                assert_eq!(
                    fields[6].parse::<usize>().unwrap(),
                    result_used,
                    "{version:?}/{id} numUsed"
                );
                assert_eq!(
                    fields[7].parse::<usize>().unwrap(),
                    capacity,
                    "{version:?}/{id} numAllocated"
                );
                assert_eq!(fields[9] == "1", span, "{version:?}/{id} span");
                assert_eq!(fields[13] == "1", same, "{version:?}/{id} same backing");
                if span {
                    assert_eq!(
                        fields[10].parse::<usize>().unwrap(),
                        first,
                        "{version:?}/{id} spanStart"
                    );
                    assert_eq!(
                        fields[11].parse::<usize>().unwrap(),
                        length,
                        "{version:?}/{id} spanLength"
                    );
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 144);
    }

    #[test]
    fn native_c9_span_uses_allocated_and_used_extents_in_native_order() {
        assert_eq!(
            range_action(TclVersion::V9_0, coordinates(0, 100), shared(202, 202)).unwrap(),
            NativeListRangeAction::Window {
                range: 0..101,
                span: true
            }
        );
        assert_eq!(
            range_action(TclVersion::V9_0, coordinates(0, 100), shared(202, 606)).unwrap(),
            NativeListRangeAction::FreshMembers(0..101)
        );
        let mut unshared = shared(301, 301);
        unshared.header_shared = false;
        assert_eq!(
            range_action(TclVersion::V9_0, coordinates(0, 100), unshared).unwrap(),
            NativeListRangeAction::Window {
                range: 0..101,
                span: false
            }
        );
        assert_eq!(
            range_action(TclVersion::V9_0, coordinates(75, 224), unshared).unwrap(),
            NativeListRangeAction::Window {
                range: 75..225,
                span: true
            }
        );
    }
}
