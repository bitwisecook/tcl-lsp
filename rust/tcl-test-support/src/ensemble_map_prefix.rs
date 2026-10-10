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

//! Fixed original counted String ensemble map vectors and public observations.

/// A measured original constructor/caller control, without native capabilities.
pub struct EnsembleMapPrefixControl {
    /// Original fixed probe case index.
    pub index: usize,
    /// Original case identifier.
    pub case: &'static str,
    /// Whole immutable stdout for C8.5, C8.6, C9.0 and C9.1 respectively.
    pub columns: [&'static str; 4],
}

/// Exactly twenty successful C map construction/query/invocation windows.
pub const CONTROLS: &[EnsembleMapPrefixControl] = &[
    EnsembleMapPrefixControl {
        index: 0,
        case: "create-relative-raw-nul",
        columns: [
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.5.19/create-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.6.18/create-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.0.4/create-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.1.0/create-relative-raw-nul/stdout"
            ),
        ],
    },
    EnsembleMapPrefixControl {
        index: 1,
        case: "create-rooted-raw-nul",
        columns: [
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.5.19/create-rooted-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.6.18/create-rooted-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.0.4/create-rooted-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.1.0/create-rooted-raw-nul/stdout"
            ),
        ],
    },
    EnsembleMapPrefixControl {
        index: 2,
        case: "configure-relative-raw-nul",
        columns: [
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.5.19/configure-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.6.18/configure-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.0.4/configure-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.1.0/configure-relative-raw-nul/stdout"
            ),
        ],
    },
    EnsembleMapPrefixControl {
        index: 3,
        case: "configure-rooted-raw-nul",
        columns: [
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.5.19/configure-rooted-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.6.18/configure-rooted-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.0.4/configure-rooted-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.1.0/configure-rooted-raw-nul/stdout"
            ),
        ],
    },
    EnsembleMapPrefixControl {
        index: 4,
        case: "create-global-relative-raw-nul",
        columns: [
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.5.19/create-global-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/8.6.18/create-global-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.0.4/create-global-relative-raw-nul/stdout"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_ensemble_map_prefix312/9.1.0/create-global-relative-raw-nul/stdout"
            ),
        ],
    },
];

/// Original ASCII setup, used unchanged before the object-vector callback.
pub const SETUP: &[u8] = b"proc ::p args {list GLOBAL $args}; namespace eval ::r2286_map_N {proc p args {list N $args}}; namespace eval ::r2286_map_Q {proc p args {list Q $args}}";
/// Original separate creation before the two configure vectors.
pub const INITIAL: &[u8] = b"namespace eval ::r2286_map_N {namespace ensemble create -command ::r2286_map_E -map {go {::r2286_map_N::p INITIAL}}}";
/// Original entered-namespace callback source for each fixed case.
pub const OPERATIONS: [&[u8]; 5] = [
    b"namespace eval ::r2286_map_N {::r2286_original_map_callback}",
    b"namespace eval ::r2286_map_N {::r2286_original_map_callback}",
    b"namespace eval ::r2286_map_Q {::r2286_original_map_callback}",
    b"namespace eval ::r2286_map_Q {::r2286_original_map_callback}",
    b"::r2286_original_map_callback",
];

/// The original counted String constructor bytes; no binary/source replacement.
#[must_use]
pub fn original_head(index: usize) -> &'static [u8] {
    if index == 1 || index == 3 {
        b"::r2286_map_N::p\0tail"
    } else {
        b"p\0tail"
    }
}

/// Original vector words before its final, independently constructed map object.
#[must_use]
pub fn operation_words(index: usize) -> &'static [&'static [u8]] {
    if index == 2 || index == 3 {
        &[
            b"namespace",
            b"ensemble",
            b"configure",
            b"::r2286_map_E",
            b"-map",
        ]
    } else {
        &[
            b"namespace",
            b"ensemble",
            b"create",
            b"-command",
            b"::r2286_map_E",
            b"-map",
        ]
    }
}

/// Original non-version rows. The backend's provider labels are separate from
/// the captured native executable version; every operation/getter/call row stays.
#[must_use]
pub fn observation_rows(column: &str) -> Vec<String> {
    column
        .lines()
        .filter(|row| !row.starts_with("VERSION|"))
        .map(str::to_owned)
        .collect()
}
