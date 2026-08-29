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

//! `ACCESS::uuid` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::uuid",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enumerates the session IDs that belongs to a specified uuid key by the order of its creation and provides them in a Tcl list.",
            synopsis: &[
                "ACCESS::uuid getsid SESSION_ID",
                "ACCESS::uuid ACCESS_UUID_COMMAND (ACCESS_UUID_INFO)?",
            ],
            snippet: "Enumerates the session IDs that belongs to a specified uuid key by the\norder of its creation and provides them in a Tcl list. By default, the\nuuid created by AAC is using the following format.\n  * {profile_name}.{user_name}\n\nHowever, the admin can manually override this by specifying their own\nuuid key via assigning that value to session.user.uuid session\nvariable. This can be done via iRule using ACCESS::session data set\nsession.user.uuid or via VPE using Variable Assignment Agent. The\nreturn value of ACCESS::uuid getsid is a Tcl list.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__uuid.html",
            examples: "when HTTP_REQUEST {\n    set apm_cookie_list [ ACCESS::uuid getsid \"[PROFILE::access name].[HTTP::username]\" ]\n    log local0. \"[PROFILE::access name].[HTTP::username] => session number [llength $apm_cookie_list]\"\n    for {set i 0} {$i < [llength $apm_cookie_list]} {incr i} {\n        log local0. \"MRHSession => [ lindex $apm_cookie_list $i]\"\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::uuid getsid SESSION_ID",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
