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

use tcl_bigip::links::*;

#[test]
fn irule_body_pool_ref_emits_resolved_link() {
    crate::install();
    let source = "ltm pool /Common/web_pool { }\nltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/web_pool }\n}\n";
    let links = document_links(source);
    assert!(!links.is_empty(), "expected an iRule pool-ref link");
    let link = links
        .iter()
        .find(|l| l.tooltip.contains("/Common/web_pool"))
        .expect("pool ref link");
    // Resolves to the pool stanza on line 0.
    assert_eq!(link.target_line, Some(0), "{link:?}");
    assert_eq!(link.tooltip, "Go to /Common/web_pool");
}

#[test]
fn irule_body_unresolved_ref_emits_link_without_target() {
    crate::install();
    let source = "ltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/missing }\n}\n";
    let links = document_links(source);
    assert_ne!(links, [] as [tcl_bigip::links::BigipLink; 0]);
    let link = links
        .iter()
        .find(|l| l.tooltip.contains("/Common/missing"))
        .expect("missing ref link");
    assert_eq!(link.target_line, None, "{link:?}");
    assert!(
        link.tooltip.contains("no definition found"),
        "{}",
        link.tooltip,
    );
}

#[test]
fn irule_links_follow_reached_helpers_but_not_dormant_procedures() {
    crate::install();
    let source = concat!(
        "ltm pool /Common/helper_pool { }\n",
        "ltm pool /Common/dormant_pool { }\n",
        "ltm rule /Common/r {\n",
        "proc helper {} { pool /Common/helper_pool }\n",
        "proc dormant {} { pool /Common/dormant_pool }\n",
        "when HTTP_REQUEST { call helper; dormant }\n",
        "}\n",
    );
    let links = document_links(source);
    assert!(
        links
            .iter()
            .any(|link| link.tooltip == "Go to /Common/helper_pool"),
        "reached helper reference must participate in BIG-IP links: {links:?}"
    );
    assert!(
        !links
            .iter()
            .any(|link| link.tooltip == "Go to /Common/dormant_pool"),
        "dormant procedure data must not create a graph edge: {links:?}"
    );
}

#[test]
fn property_value_emits_bounded_link_to_target() {
    crate::install();
    let source = "ltm monitor http /Common/http_probe { }\nltm pool /Common/web_pool {\n    monitor /Common/http_probe\n}\n";
    let links = document_links(source);
    let link = links
        .iter()
        .find(|l| l.tooltip == "Go to /Common/http_probe")
        .expect("monitor property link");
    // Resolves to the monitor stanza on line 0.
    assert_eq!(link.target_line, Some(0), "{link:?}");
    // The link range is the exact path token — bounded, not the whole
    // line, and (inclusive end) spans exactly the path length minus one.
    assert!(link.range.start.character > 0, "{link:?}");
    let span = link.range.end.character - link.range.start.character + 1;
    assert_eq!(
        span as usize,
        "/Common/http_probe".len(),
        "link range should cover exactly the path token: {link:?}",
    );
    // The reference sits on line 2 (the `monitor …` property line).
    assert_eq!(link.range.start.line, 2, "{link:?}");
}

#[test]
fn keyed_block_profiles_emit_per_item_links() {
    crate::install();
    let source = "ltm virtual /Common/v {\n    profiles {\n        /Common/clientssl { context clientside }\n        /Common/http { }\n    }\n}\nltm profile client-ssl /Common/clientssl { }\nltm profile http /Common/http { }\n";
    let links = document_links(source);
    let tooltips: Vec<&str> = links.iter().map(|l| l.tooltip.as_str()).collect();
    assert!(
        tooltips.contains(&"Go to /Common/clientssl"),
        "{tooltips:?}",
    );
    assert!(tooltips.contains(&"Go to /Common/http"), "{tooltips:?}");
}

#[test]
fn no_irule_no_property_refs_yields_no_links() {
    crate::install();
    // A lone pool with no references contributes nothing.
    assert_eq!(
        document_links("ltm pool /Common/p { }\n"),
        [] as [tcl_bigip::links::BigipLink; 0]
    );
}

#[test]
fn empty_source_is_safe() {
    crate::install();
    assert_eq!(document_links(""), [] as [tcl_bigip::links::BigipLink; 0]);
    assert_eq!(
        document_links("   \n  "),
        [] as [tcl_bigip::links::BigipLink; 0]
    );
}
