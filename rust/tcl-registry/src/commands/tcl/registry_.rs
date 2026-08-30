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

//! `registry` — Windows registry access (Windows-only Tcl command).
//!
//! Module name has a trailing underscore to avoid a collision with the
//! crate's [`crate::registry`] module.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "registry option keyName ?arg arg ...?",
        ..FormSpec::DEFAULT
    },
    // The optional leading `-32bit`/`-64bit` mode flag was added to the
    // SYNOPSIS in Tcl 8.6 (registry.n, 8.6/9.0/9.1: "registry ?-mode?
    // option keyName ?arg arg ...?", with -mode spelled out in the
    // DESCRIPTION as "-32bit" or "-64bit"). Confirmed absent from the 8.4
    // and 8.5 SYNOPSIS, which document only the bare form above with no
    // leading flag mentioned anywhere in their DESCRIPTION either.
    FormSpec {
        synopsis: "registry ?-32bit|-64bit? option keyName ?arg arg ...?",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    // Tcl 9.1 adds `tcl::registry` as a second, always-present spelling of
    // the same command — usable without `package require registry` first
    // (registry.n, Tcl 9.1: "Starting with Tcl 9.1, the command is also
    // available as tcl::registry without requiring the registry package
    // to be loaded"). Confirmed absent from the 8.4, 8.5, 8.6, and 9.0
    // SYNOPSIS sections, which document only the bare `registry` spelling.
    FormSpec {
        synopsis: "tcl::registry ?-32bit|-64bit? option keyName ?arg arg ...?",
        surface: Some(SpecSurface::TCL91),
        ..FormSpec::DEFAULT
    },
];

/// `option` (position 0, or position 1 when a leading `-32bit`/`-64bit`
/// mode flag is given) selects one of these seven subcommands — an
/// identical set across 8.4 through 9.1. Real Tcl accepts any unique
/// abbreviation of `option` (registry.n, every version: "Any unique
/// abbreviation for option is acceptable"), and the command-level
/// (non-subcommand) `closed_value_args` check only does exact whole-word
/// matching with no prefix support, so listing this index there would
/// misreport a legitimate abbreviation (e.g. `registry se keyName ...`)
/// as an invalid value. These are completion/hover data only.
const REGISTRY_OPTION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "broadcast",
        detail: "Notify the system and running programs of a registry change.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "delete",
        detail: "Delete a value, or an entire key with its subkeys and values.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "get",
        detail: "Return the data of a value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "keys",
        detail: "List the subkey names of a key, optionally filtered by pattern.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "set",
        detail: "Create a key, or set a value's data and type (default sz).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "type",
        detail: "Return the type of a value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "values",
        detail: "List the value names of a key, optionally filtered by pattern.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `registry`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "registry",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::HAS_DESTRUCTIVE_OPS,
        arity: Arity::new(2, 5),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Manipulate the Windows registry (Windows-only).",
            synopsis: &[
                "registry ?-32bit|-64bit? option keyName ?arg arg ...?",
                "registry broadcast keyName ?-timeout milliseconds?",
                "registry delete keyName ?valueName?",
                "registry get keyName valueName",
                "registry keys keyName ?pattern?",
                "registry set keyName ?valueName data ?type??",
                "registry type keyName valueName",
                "registry values keyName ?pattern?",
            ],
            snippet: "Requires `package require registry` (bundled with Windows Tcl only, not auto-loaded); since Tcl 9.1 also callable as `tcl::registry` without loading the package first. `option` accepts any unique abbreviation. `keyName` is rootname, or rootname\\keypath with subkeys joined by \\; a leading pair of backslashes plus a host name reaches another machine's registry. rootname is one of HKEY_LOCAL_MACHINE, HKEY_USERS, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_CURRENT_CONFIG, HKEY_PERFORMANCE_DATA, or HKEY_DYN_DATA. Since Tcl 8.6, an optional leading `-32bit` or `-64bit` selects which registry view to use instead of the system default. `delete` does nothing (not an error) when the key is already absent, but errors if an existing key or value cannot be removed. `get`, `keys`, `type`, and `values` error if keyName does not exist. `set` creates keyName (and valueName, if given) as needed and assumes type `sz` when type is omitted. Data is typed as binary, none, sz, expand_sz, dword, dword_big_endian, link, multi_sz, or resource_list, or an unrecognised 32-bit type code; multi_sz renders as a Tcl list, the rest as strings or exact binary data. Not available in this WASM sandbox (Windows-specific) — calling it fails with `invalid command name \"registry\"`.",
            source: "Tcl registry(n)",
            examples: "registry get {HKEY_CLASSES_ROOT\\.tcl} {}\nregistry set {HKEY_CURRENT_USER\\Software\\MyApp} Path $installDir\nregistry set {HKEY_CURRENT_USER\\Software\\MyApp} Port 8080 dword\nregistry keys {HKEY_LOCAL_MACHINE\\SOFTWARE}\nregistry broadcast Environment -timeout 5000",
            return_value: "Depends on option: get returns the value's data; keys/values return a list of names; type returns the value's type; broadcast/delete/set return an empty string.",
        }),
        forms: FORMS,
        arg_values: &[(0, REGISTRY_OPTION_VALUES)],
        options: const {
            &[
                OptionSpec {
                    name: "-32bit",
                    value: OptionValue::flag(),
                    detail: "Operate on the 32-bit registry view instead of the system default. Precedes option; mutually exclusive with -64bit.",
                    surface: Some(SpecSurface::TCL86_PLUS),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-64bit",
                    value: OptionValue::flag(),
                    detail: "Operate on the 64-bit registry view instead of the system default. Precedes option; mutually exclusive with -32bit.",
                    surface: Some(SpecSurface::TCL86_PLUS),
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-timeout",
                    value: OptionValue::Takes(OptionArg {
                        integer: Some(IntegerDomain::Any),
                        hint: "milliseconds",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "registry broadcast only: milliseconds to wait for applications to respond to the notification. Defaults to 3000.",
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        ..CommandSpec::DEFAULT
    }
}
