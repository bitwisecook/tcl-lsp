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

use tcl_bigip::irule_context::*;
use tcl_bigip::model::ModelObject;

#[test]
fn transitive_context_includes_pool_and_member_monitors() {
    crate::install();
    let source = r#"
ltm rule /Tenant/select_pool { when HTTP_REQUEST { pool /Tenant/web } }
ltm monitor https /Common/custom { }
ltm monitor https /Tenant/custom { }
ltm monitor http /Common/http { }
ltm monitor tcp /Common/tcp { }
ltm pool /Tenant/web {
    monitor "custom and /Common/http"
    members {
        /Tenant/node:80 { monitor min 1 of { /Common/tcp /Tenant/missing } }
    }
}
"#;
    let cfg = tcl_bigip::parser::parse_bigip_conf(source, "Common");
    let rule = cfg
        .objects
        .iter()
        .find_map(|placed| match &placed.object {
            ModelObject::Rule(rule) => Some(rule),
            _ => None,
        })
        .unwrap();
    let registry = tcl_registry::model::static_context_for("f5-irules").commands();
    let bundle = build_irule_context(rule, &cfg, true, Some(source), registry);
    let monitors: Vec<_> = bundle
        .monitors
        .iter()
        .map(|monitor| monitor.full_path.as_str())
        .collect();
    assert_eq!(monitors, ["/Tenant/custom", "/Common/http", "/Common/tcp"]);
    assert_eq!(
        bundle.unresolved,
        [("monitor".to_owned(), vec!["/Tenant/missing".to_owned()])]
    );
}
