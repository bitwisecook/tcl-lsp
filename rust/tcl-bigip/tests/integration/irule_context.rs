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

//! iRule context regressions that require the executable inventory.

use tcl_bigip::irule_context::build_irule_context;
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn context_resolves_data_group_in_braced_expression_command() {
    let source = concat!(
        "ltm data-group internal /Common/braced_dg {\n",
        "    type string\n",
        "    records { host { } }\n",
        "}\n",
        "ltm rule /Common/braced_expr {\n",
        "    when HTTP_REQUEST {\n",
        "        if {[class match [HTTP::host] equals /Common/braced_dg]} {\n",
        "            pool /Common/selected_pool\n",
        "        }\n",
        "    }\n",
        "}\n",
    );
    let config = parse_bigip_conf(source, "Common");
    let rule = config
        .objects
        .iter()
        .find_map(|placed| match &placed.object {
            ModelObject::Rule(rule) => Some(rule),
            _ => None,
        })
        .expect("the fixture has its iRule");
    let registry = tcl_registry::model::ingress::static_context_for_profile(
        tcl_dialect::DialectProfile::irules(),
    )
    .commands();
    let bundle = build_irule_context(rule, &config, false, Some(source), registry);

    assert_eq!(
        bundle
            .data_groups
            .iter()
            .map(|data_group| data_group.full_path.as_str())
            .collect::<Vec<_>>(),
        ["/Common/braced_dg"],
    );
}
