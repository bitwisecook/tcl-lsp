// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pinned `Tcl_UniCharToLower` unit mapping, independent of a matching purpose.
use tcl_dialect::TclVersion;
#[path = "native_glob_case_mapping.rs"]
mod mapping;

#[must_use]
pub fn lower(version: TclVersion, unit: u32) -> u32 {
    let ranges = match version {
        TclVersion::V8_4 => mapping::C84,
        TclVersion::V8_5 => mapping::C85,
        TclVersion::V8_6 => mapping::C86,
        TclVersion::V9_0 => mapping::C90,
        TclVersion::V9_1 => mapping::C91,
    };
    let index = ranges.partition_point(|&(start, _, _, _)| start <= unit);
    if let Some(&(start, end, stride, delta)) =
        index.checked_sub(1).and_then(|index| ranges.get(index))
        && unit <= end
        && (unit - start).is_multiple_of(stride)
    {
        return unit
            .checked_add_signed(delta)
            .expect("pinned Tcl lowercase unit range");
    }
    unit
}
