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

//! `BWC::policy` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "The bwc irule allows a bwc policy to be attached or detached to a specific flow.",
            synopsis: &["BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?"],
            snippet: "A bwc policy must exist for the given policy name, the irule will return an error if the policy cannot be found. The policy name should be give without a path name: e.g. \"gold_user\" not \"/Common/gold_user\". The irule will internally try to determine the correct pathname through lookup_folder_path_obj().\n\nOnce the irule has found the correct bwc policy name, it will know if the policy is static or dynamic. If the policy is dynamic a third arg session is required. The session is used as the bwc_cookie_t argument to the bwc public api bwc_dynamic_policy_instantiate().",
            source: "https://clouddocs.f5.com/api/irules/BWC__policy.html",
            examples: "when CLIENT_ACCEPTED {\n            BWC::policy attach gold_class\n        }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
