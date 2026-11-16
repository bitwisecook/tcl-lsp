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
use tcl_compiler::analyser::Analyser;

#[test]
fn original_roles_keep_opaque_declarations_distinct_after_ui_is_cleared() {
    let source = "proc p\\uD800 {v} {upvar 1 $v local; set local 1}\nproc p\\uD801 {v} {upvar 1 $v local; set local}\np\\uD800 written\np\\uD801 read\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let roles = VarNameArgRoles::from_analysis(&analysis);
    assert_eq!(roles.original.len(), 2);
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    let write_at = u32::try_from(source.rfind("p\\uD800 written").unwrap()).unwrap();
    let read_at = u32::try_from(source.rfind("p\\uD801 read").unwrap()).unwrap();
    let write = roles
        .original_roles_at(&analysis, source, write_at)
        .unwrap();
    let read = roles.original_roles_at(&analysis, source, read_at).unwrap();
    assert_eq!(write.write, [0]);
    assert_eq!(read.read, [0]);
    assert_eq!(read.write, [] as [u32; 0]);
    assert_ne!(write.name.slot(), read.name.slot());
    assert!(
        roles
            .original_roles_at(&analysis, &format!("{source} "), write_at)
            .is_none()
    );
}

#[test]
fn original_role_lookup_requires_current_full_source_and_own_head_input() {
    let source = "proc p {value} {return $value}; p ordinary";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let roles = VarNameArgRoles::from_analysis(&analysis);
    let offset = u32::try_from(source.rfind("p ordinary").unwrap()).unwrap();
    assert!(roles.original_roles_at(&analysis, source, offset).is_some());
    assert!(
        roles
            .original_roles_at(&analysis, &format!("{source} "), offset)
            .is_none()
    );
    let invocation = analysis
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.range.start() == offset)
        .unwrap();
    let input = invocation.original_name_input.take();
    assert!(roles.original_roles_at(&analysis, source, offset).is_none());
    analysis
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.range.start() == offset)
        .unwrap()
        .original_name_input = input;
    analysis.body_lexer_config = None;
    assert!(roles.original_roles_at(&analysis, source, offset).is_none());
}

#[test]
fn original_role_lookup_respects_namespace_shadow_and_ordinary_arguments() {
    let source = "proc touch {v} {upvar 1 $v local; set local 1}\nnamespace eval N {proc touch {v} {return $v}; touch ordinary}\ntouch declared\nputs touch\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let roles = VarNameArgRoles::from_analysis(&analysis);
    let local_at = u32::try_from(source.find("touch ordinary").unwrap()).unwrap();
    let global_at = u32::try_from(source.find("touch declared").unwrap()).unwrap();
    assert_eq!(
        roles
            .original_roles_at(&analysis, source, local_at)
            .unwrap()
            .write,
        [] as [u32; 0]
    );
    assert_eq!(
        roles
            .original_roles_at(&analysis, source, global_at)
            .unwrap()
            .write,
        [0]
    );
    let data_at = u32::try_from(source.rfind("touch").unwrap()).unwrap();
    assert!(
        roles
            .original_roles_at(&analysis, source, data_at)
            .is_none()
    );
}
