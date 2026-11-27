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

use super::{PossibleBodyTopology, PossibleHandlerBodyFlow};
use crate::catch_invocation::{CatchInvocation, CatchInvocationSelection};
use crate::{VariableAliasFrame, script_body_flow::ScriptBodyFlow};

#[test]
fn possible_case_bodies_keep_unknown_subjects_without_accepting_bad_lists() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let dialect = tcl_native_model::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("f5-irules").unit_profile(),
    );
    for (cases, possible) in [
        ("loud {set debug 1} default {set debug 0}", true),
        ("loud", false),
        ("{", false),
    ] {
        let arguments = [Dynamic, Literal(cases)];
        let words =
            crate::InvocationWords::structured(Literal("switch"), &arguments).with_dialect(dialect);
        let resolution = registry.resolve_structured_invocation(words, None);
        let facts = resolution.resolved().unwrap().facts();
        assert_eq!(
            facts
                .possible_handler_body_flow(
                    registry,
                    words.arguments(),
                    VariableAliasFrame::Procedure
                )
                .is_some(),
            possible,
            "{cases}"
        );
    }
}

#[test]
fn possible_capture_retains_selected_filter_without_licensing_filtered_propagation() {
    let selected = CatchInvocation::CAPTURE_TCL_PHASE;
    let mut flow = PossibleHandlerBodyFlow {
        flow: ScriptBodyFlow::Capture(CatchInvocationSelection::Valid(selected)),
        frame: VariableAliasFrame::Procedure,
    };
    assert_eq!(
        flow.topology(),
        Some(PossibleBodyTopology::Captured(selected))
    );
    flow.flow = ScriptBodyFlow::Capture(CatchInvocationSelection::Valid(CatchInvocation {
        ignored_codes: 1 << 2,
        ..selected
    }));
    assert_eq!(flow.topology(), None);
}
