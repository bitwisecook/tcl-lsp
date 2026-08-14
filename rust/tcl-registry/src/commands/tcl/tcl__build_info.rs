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

//! `tcl::build-info` — report Tcl's build-configuration and version
//! metadata (Tcl 9.0+, TIP 599).
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/buildinfo.html, `.htm` for the 8.6 tree, each
// fetched directly rather than assumed): the 8.4, 8.5, and 8.6 URLs each
// resolve to the site's standard "URL Not Found" page, and `buildinfo` is
// absent from the 8.6 branch's own alphabetical TclCmd/index.html (checked
// against the live 8.6.18 index) — `tcl::build-info` genuinely does not
// exist before 9.0. TIP 599 ("Extended build information") originally
// targeted the abandoned Tcl 8.7 release, the same fate as `lremove`
// (TIP 367) / `lpop` (TIP 523) / `ledit` (TIP 631); the command landed for
// real only when Tcl 9.0 shipped. The 9.0 and 9.1 pages serve
// byte-identical NAME/SYNOPSIS/DESCRIPTION (25 documented fields, same
// order)/EXAMPLES/SEE ALSO/KEYWORDS text — the worked EXAMPLES on the 9.1
// page still show a literal "9.0.2" patchlevel, i.e. the 9.1 doc source
// was never regenerated for 9.1 — so nothing about this command's
// documented behaviour changed between the two releases.
//
// The manpage's own NAME is "buildinfo" (hence `buildinfo.html`, not
// `build-info.html`), even though the command itself is spelled
// `tcl::build-info` (SYNOPSIS: "::tcl::build-info ?field?").
//
// No dialect directory (irules/, expect/, eda_*/, iapps/, tk/, itcl/)
// references `build-info`/`build_info`, and none of those dialects model a
// base Tcl core version of 9.0 or later
// (`DialectSet::expr_grammar_base_version` tops out at 8.6 for every one
// of them; f5-tmsh in particular runs an 8.5 base per
// `tcl-dialect/src/profile.rs`), so a `TCL90_PLUS` gate below already
// excludes every one of them on the version axis alone — the same
// reasoning `lremove.rs` documents for its own plain `TCL90_PLUS` gate.
// That version gate is the whole mechanism for both spellings
// ("::tcl::build-info" and "tcl::build-info"): `TCL90_PLUS` carries no
// `IRULES` bit, so neither ever intersects iRules' bare `IRULES` mask, and
// iRules' genuine embedded Tcl 8.4.6 core (`return_.rs`'s/`pwd.rs`'s notes
// on `f5-irules`'s signature_base/runtime_base/version_ceiling) could never
// reach a Tcl-9.0-only command regardless. No separate per-dialect
// exclusion is needed, and there is no disable list. Separately, a stray,
// generator-artifact `CommandSpec` literally named "::tcl::build-info" that
// used to sit inside
// `commands/tcl/mathop_generated.rs`'s hand-expanded `tcl::mathop` operator
// table (between the `-` and `/` entries) is also gone now that file is
// derived directly from `tcl_syntax::expr::operators`.

use crate::prelude::*;

// `?field?` is Tcl 9.0+ only (see the module doc comment above for the
// full version evidence) — the form's own `dialects` says so directly, and
// it matches the command-level `TCL90_PLUS` gate below: `TCL90_PLUS`
// carries no `IRULES` bit, so `tcl::build-info` is excluded from iRules by
// its version gate alone, not by any disable list.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "tcl::build-info ?field?",
    dialects: Some(DialectSet::TCL90_PLUS),
    ..FormSpec::DEFAULT
}];

/// `field` (position 0) accepts any word — an unrecognised field is not an
/// error, it simply returns "0" the same as a recognised-but-unset
/// boolean flag (buildinfo(n) DESCRIPTION: "Unknown fields return 0").
/// Like `open`'s POSIX access-flag list (`open_.rs`'s `ACCESS_VALUES`),
/// that makes this completion/hover data only: `tcl::build-info` has no
/// `closed_value_args` entry for this position, since a
/// legitimate-but-registry-unrecognised field name would otherwise be
/// misreported as an invalid value. All 25 fields below are documented
/// identically, in the same order, on both the Tcl 9.0 and Tcl 9.1
/// manpages (see the module doc comment), so none carries its own
/// `min_tcl`.
const FIELD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "clang",
        detail: "Clang version (as 4 digits) if Tcl was compiled with clang, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "commit",
        detail: "The Fossil (or Git) commit ID Tcl was built from.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "compiledebug",
        detail: "1 if compiled with -DTCL_COMPILE_DEBUG, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "compiler",
        detail: "The compiler name and a 4-digit version number, e.g. gcc-1204 or clang-1200.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "compilestats",
        detail: "1 if compiled with -DTCL_COMPILE_STATS, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "cplusplus",
        detail: "1 if compiled with a C++ compiler, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "debug",
        detail: "1 if Tcl is not compiled with -DNDEBUG, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "gcc",
        detail: "GCC version (as 4 digits) if Tcl was compiled with gcc, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "icc",
        detail: "ICC version (as 4 digits) if Tcl was compiled with icc, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ilp32",
        detail: "1 if int, long, and pointer are all 32-bit, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "memdebug",
        detail: "1 if compiled with -DTCL_MEM_DEBUG, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "msvc",
        detail: "MSVC version (as 4 digits) if Tcl was compiled with msvc, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "nmake",
        detail: "1 if built using nmake, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "no-deprecate",
        detail: "1 if compiled with -DTCL_NO_DEPRECATED, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "no-thread",
        detail: "1 if compiled with -DTCL_THREADS=0, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "no-optimize",
        detail: "1 if not compiled with -DTCL_CFG_OPTIMIZED, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "objective-c",
        detail: "1 if compiled with an Objective-C compiler, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "objective-cplusplus",
        detail: "1 if compiled with an Objective-C++ compiler, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "patchlevel",
        detail: "The Tcl patchlevel, the same value as [info patchlevel].",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "profile",
        detail: "1 if compiled with -DTCL_CFG_PROFILED, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "purify",
        detail: "1 if compiled with -DPURIFY, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "static",
        detail: "1 if Tcl is compiled as a static library, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "tommath",
        detail: "libtommath version (as 4 digits) if built into Tcl, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "version",
        detail: "The Tcl version, the same value as [info tclversion].",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "zlib",
        detail: "zlib version (as 4 digits) if built into Tcl, 0 otherwise.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `tcl::build-info`.
///
/// Tcl 9.0+ only — see the module doc comment above for the full
/// per-version evidence. The `TCL90_PLUS` gate below carries no `IRULES`
/// bit, so `tcl::build-info` is excluded from iRules by that version gate
/// alone, not by any disable list. A genuine
/// `Tcl_CreateObjCommand`-registered core builtin — not a Tcl-coded
/// library proc like `bgerror`/`tcl_findLibrary` — so it carries
/// `BYTE_COMPILED` the same way its 9.0-native siblings `lremove`/`lpop`/
/// `tcl::process`/`zipfs` do. Not part of C Tcl's documented
/// safe-interpreter hidden-command set (`Traits::SAFE_INTERP_HIDDEN`'s own
/// doc comment lists that exact set — `cd`, `encoding`, `exec`, `exit`,
/// `fconfigure`, `file`, `glob`, `load`, `open`, `pwd`, `socket`, `source`,
/// `unload` — and this command is not on it); it exposes only static,
/// read-only compile-time metadata (no filesystem, network, or process
/// access), the same low-risk shape as `info tclversion`/`info
/// patchlevel`, so it is not stamped hidden here either. Left off
/// `Traits::PURE`: unlike a value-transforming command such as `lremove`
/// (deterministic given only its own arguments), this command's result
/// depends on which Tcl binary happens to be running it — state that
/// lives outside the argument list, the same reason `Traits::PURE` stays
/// off `pid`/`pwd` in this registry (see `pid.rs`'s note) even though
/// none of the three ever mutates anything.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::build-info",
        dialects: Some(DialectSet::TCL90_PLUS),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Return the Tcl interpreter's build-configuration and version metadata.",
            synopsis: &["tcl::build-info ?field?"],
            snippet: "With no field, returns the interpreter's full build-info string: the patchlevel, a literal +, the Fossil (or Git) commit ID Tcl was built from, and one or more dot-separated build tags (typically the compiler identifier). With a field argument, returns just that one piece of information instead: version and patchlevel return the same strings as info tclversion and info patchlevel; commit and compiler return the commit ID and compiler-name-and-version strings embedded in the bare form; gcc/clang/icc/msvc and tommath/zlib each return a 4-digit version number when Tcl was built with that compiler or that bundled library, or \"0\" when it was not; every other documented field (debug, compiledebug, compilestats, memdebug, purify, profile, no-optimize, no-deprecate, no-thread, static, cplusplus, objective-c, objective-cplusplus, ilp32, nmake) is a plain boolean flag returning \"1\" or \"0\". A field name build-info does not recognise is not an error — it simply returns \"0\", indistinguishable from a recognised boolean flag that was not set. Added in Tcl 9.0 (TIP 599); tcl::build-info does not exist in Tcl 8.4, 8.5, or 8.6.",
            source: "Tcl buildinfo(n)",
            examples: "tcl::build-info                 ;# 9.0.2+af16c07b81655fabde8028374161ad54b84ef9956843c63f49976b4ef601b611.gcc-1204\ntcl::build-info version          ;# 9.0\ntcl::build-info patchlevel       ;# 9.0.2\ntcl::build-info commit           ;# af16c07b81655fabde8028374161ad54b84ef9956843c63f49976b4ef601b611\ntcl::build-info compiler         ;# gcc-1204\ntcl::build-info debug            ;# 0 on a release build\ntcl::build-info made-up-field    ;# 0 -- unrecognised fields are not an error",
            return_value: "With no field: the build-info string (patchlevel+commit.tags). With field: that field's value — a version/commit/compiler string for version/patchlevel/commit/compiler, a 4-digit version number or \"0\" for a compiler- or bundled-library-version field, or \"1\"/\"0\" for every other (boolean) field, including any field name build-info does not recognise.",
        }),
        forms: FORMS,
        arg_values: &[(0, FIELD_VALUES)],
        ..CommandSpec::DEFAULT
    }
}
