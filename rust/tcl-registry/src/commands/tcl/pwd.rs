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

//! `pwd` — return the absolute path of the current working directory.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "pwd",
    ..FormSpec::DEFAULT
}];

/// Command spec for `pwd`.
///
/// A plain core builtin, textually identical across Tcl 8.4 through 9.1: the
/// bare `pwd` synopsis, the DESCRIPTION ("Returns the absolute path name of
/// the current working directory."), the EXAMPLE, and the SEE ALSO/KEYWORDS
/// read word-for-word the same on all five official manpages (8.4, 8.5, 8.6,
/// 9.0, 9.1) — the only difference across them is a single British/American
/// spelling swap ("minimises" in 8.4 vs. "minimizes" from 8.5 on) inside the
/// EXAMPLE prose, which carries no behavioural meaning. `pwd` takes no
/// arguments and has no options/switches in any of the five versions.
///
/// `surface: Some(SpecSurface::ALL_TCL)` here is deliberate, not an
/// oversight: F5 iRules is the one modelled dialect that drops `pwd` — it
/// is one of the K36322151 commands the TMM event sandbox excludes,
/// having no real per-request filesystem/cwd concept, the same reasoning
/// that drops `cd`/`open`/`glob`/`exec`/`fcopy`/`pid`. That exclusion now
/// falls straight out of this `dialects` gate: `ALL_TCL` carries no
/// `IRULES` bit, so it never intersects the bare `IRULES` mask — the same
/// treatment those commands get in their own spec files, with no disable
/// list. Every other modelled dialect (Expect, Tk, the EDA vendor
/// consoles, F5 iApps, F5 tmsh, incr Tcl) hosts a real filesystem-backed
/// Tcl whose mask carries a version bit `ALL_TCL` intersects, so `pwd`
/// resolves there normally.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pwd",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED | Traits::RETURNS_PATH | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::exact(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Return the absolute path of the current working directory.",
            synopsis: &["pwd"],
            snippet: "Returns the absolute path name of the current working directory. The working directory is a per-process resource shared by every interpreter and, in a threaded build, every thread — the same one `cd` changes — so the result always reflects the most recent `cd` in the process, not a per-interpreter starting point. Errors if the working directory can no longer be determined, for example if it has been deleted, unmounted, or otherwise made inaccessible while the process was running.",
            source: "Tcl pwd(n)",
            examples: "set tarFile [file normalize somefile.tar]\nset savedDir [pwd]\ncd /tmp\nexec tar -xf $tarFile\ncd $savedDir",
            return_value: "The absolute path of the current working directory.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
