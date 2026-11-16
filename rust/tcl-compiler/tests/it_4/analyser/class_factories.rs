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

use super::*;
use tcl_compiler::analyser::{AnalysisResult, ClassDef, MetaclassProvenance};

mod computed_names;
mod cross_file;
mod deferred_creations;
mod load_time_paths;
mod reflective_members;
mod unknown_dispatch;
mod user_metaclasses;
mod walk_blockers;

fn analysis(src: &str, dialect: &str) -> AnalysisResult {
    Analyser::new().analyse(src, dialect)
}

fn class(src: &str, qn: &str) -> ClassDef {
    analysis(src, "tcl9.0")
        .all_classes
        .get(qn)
        .cloned()
        .unwrap_or_else(|| panic!("class {qn} recorded"))
}

/// tcllib 2.0's `oo::dialect` shape, retaining its namespace-normalising
/// helper and intermediate `NSPACE` assignment: the metaclass is created
/// under a name computed from that local, and the one call site passes a
/// literal.
///
/// tclsh 9.0.4: the whole three-file chain really runs — `::T::mkdialect
/// ::T::D` then `::T::D::class create ::T::W {method go {} {return went}}`
/// then `[[::T::W new] go]` answers `went`.
const COMPUTED_METACLASS: &str = concat!(
    "namespace eval ::T {}\n",
    "oo::class create ::T::Mother { superclass oo::class }\n",
    "proc ::T::NSNormalize {namespace qualname} {\n",
    "    if {![string match ::* $qualname]} {\n",
    "        set qualname ${namespace}::$qualname\n",
    "    }\n",
    "    regsub -all {::+} $qualname \"::\"\n",
    "}\n",
    "proc ::T::mkdialect {name} {\n",
    "    set NSPACE [NSNormalize [uplevel 1 {namespace current}] $name]\n",
    "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
    "}\n",
    "::T::mkdialect ::T::D\n",
);
