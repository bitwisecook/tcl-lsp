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

//! `pkg::create` — construct a `package ifneeded` command for a package
//! specification.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "::pkg::create -name packageName -version packageVersion ?-load filespec? ... ?-source filespec? ...",
    dialects: None,
}];

/// Command spec for `pkg::create`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg::create",
        // A Tcl-level library procedure (`library/package.tcl`, documented
        // on its own dedicated `packagens.n` manual page — identical NAME
        // and SYNOPSIS text on Tcl 8.4, 8.5, 8.6, 9.0, and 9.1; OPTIONS
        // text identical too except one wording nuance: -load's target is
        // documented as "a binary library" on 8.4-8.6 and "a library" (the
        // word "binary" dropped, no other change) on 9.0-9.1 — plain
        // prose in the hover snippet below, not a `dialects:` gate, since
        // the option itself is legal and required-if-no-`-source` on every
        // one of the five fetched versions),
        // not a `Tcl_CreateObjCommand` builtin — same footing as
        // `auto_mkindex`/`tclPkgSetup`/`tclPkgUnknown`, none of which carry
        // a C `CmdInfo` row either. Stays `dialects: None` (fully
        // universal — `supports_dialect` then holds for every dialect,
        // iRules included) rather than `Some(DialectSet::ALL_TCL)` (as
        // `auto_mkindex`/`tclPkgSetup`/`tclPkgUnknown` have it): `package`
        // and `pkg_mkindex` are dropped from iRules by their own
        // `Some(DialectSet::ALL_TCL)` groups — no `IRULES` bit, so they
        // never intersect the bare `IRULES` mask (K36322151 — the TMM
        // sandbox has no real package-loading surface) — whereas nothing
        // restricts `pkg::create`, so a `dialects:` group here would
        // instead drop it too. There is no disable list.
        dialects: None,
        // A redefinable Tcl library proc (see `Traits::OVERRIDABLE_LIBRARY_PROC`,
        // whose own doc comment names the `pkg_*` family directly) that only
        // builds and returns a string — it never touches a variable, channel,
        // or the interpreter's own state, so it is `PURE` in the same sense
        // as `list`/`format`/`concat`.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::PURE,
        // `pkg::create`'s documented grammar is entirely `-flag value`
        // pairs with no leading positional word, and `-load`/`-source` are
        // explicitly repeatable ("Any number of -load parameters may be
        // specified") — a shape only a variadic `args`-style Tcl proc can
        // implement. That does *not* mean the call boundary is
        // unconstrained, though: the real `library/package.tcl` source
        // (fetched from the exact git tags matching each manpage's own
        // reported point version — core-8-4-20, core-8-5-19, core-8-6-18,
        // core-9-0-4, core-9-1-b0) shows the implementing proc — named
        // directly `::pkg::create` in 8.4, `::tcl::Pkg::Create` with
        // `interp alias {} ::pkg::create {} ::tcl::Pkg::Create` from 8.5
        // onward — unconditionally rejects fewer than 6 words with
        // `error "wrong # args: should be ..."` *before* parsing a single
        // flag, identically in all five fetched versions. Past that
        // floor, the hand-rolled parser only accepts strict `-flag value`
        // pairs (an unpaired trailing flag hits a separate "value ...
        // missing" error instead), so every call that clears the
        // structural floor also has an even word count — exactly the
        // "N, N+2, N+4, …" shape `Arity::stepped` exists for (cf. `dict
        // create`'s `Arity::stepped(0, UNLIMITED, 2)` in dict.rs). Hence
        // `stepped(6, UNLIMITED, 2)`, not `any()`. `-name`/`-version`
        // being required, and needing at least one `-load` or `-source`,
        // remain semantic checks the proc body raises *after* that
        // structural gate passes (documented identically in `packagens.n`
        // for every one of 8.4 through 9.1) — see the hover snippet and
        // each option's `detail` for those.
        arity: Arity::stepped(6, Arity::UNLIMITED, 2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Construct a package ifneeded command for a package specification.",
            synopsis: &[
                "::pkg::create -name packageName -version packageVersion ?-load filespec? ... ?-source filespec? ...",
            ],
            snippet: "Builds the `package ifneeded name version script` command for a package and returns it as a string — pkg::create only assembles the command text, it never itself calls `package`, `load`, or `source`, so the caller (typically a pkgIndex.tcl, or a directly `eval`'d expression) is the one that runs it. -name and -version are always required; at least one -load or -source must also be given, and either may repeat to fold several files into the one generated script. Each -load filespec is a {file ?procs?} list: file is loaded with `load`, and the optional procs element lists the commands it supplies — when procs is empty or omitted, the generated script sets the package up for pkg_mkIndex-style direct loading instead of naming specific commands. Each -source filespec has the same {file ?procs?} shape but for a Tcl script file loaded with `source`. Tcl 8.4 through 8.6 document -load's target specifically as a binary library; Tcl 9.0 and 9.1 describe it simply as a library, with no other documented change.",
            source: "Tcl packagens(n)",
            examples: "eval [::pkg::create -name mypkg -version 1.0 -source mypkg.tcl]\neval [::pkg::create -name mypkg -version 1.0 -load [list mypkg[info sharedlibextension] Mypkg_Init]]",
            return_value: "The generated package ifneeded command, as a string.",
        }),
        forms: FORMS,
        options: const {
            &[
                OptionSpec {
                    name: "-name",
                    value: OptionValue::value("packageName"),
                    detail: "Name of the package. Required.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-version",
                    value: OptionValue::value("packageVersion"),
                    detail: "Version of the package. Required.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-load",
                    value: OptionValue::value("filespec"),
                    detail: "A {file ?procs?} list naming a library to load with `load`; procs, if given, lists the commands it supplies (empty or omitted sets up pkg_mkIndex-style direct loading instead). May be repeated. At least one -load or -source is required overall.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-source",
                    value: OptionValue::value("filespec"),
                    detail: "A {file ?procs?} list naming a Tcl script to load with `source`. May be repeated. At least one -load or -source is required overall.",
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        ..CommandSpec::DEFAULT
    }
}
