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

fn point(uri: &str, at: u32) -> RunPoint<'_> {
    RunPoint {
        uri,
        at,
        enclosing_body: None,
    }
}

#[test]
fn package_alternatives_cannot_replace_unowned_dynamic_deferred_or_missing_input_points() {
    let parent_source = "package require P\nset after 1\n";
    let parent = Analyser::new().analyse(parent_source, "tcl8.6");
    let query_at = u32::try_from(parent_source.find("set after").unwrap()).unwrap();
    let edge = RunEdge {
        parent: "parent".into(),
        child: "child".into(),
        at: 0,
        enclosing_body: None,
        kind: RunEdgeKind::PackageRequire,
    };
    let parent_context = SourceOrderContext::capture(&parent).unwrap();
    assert!(parent_context.edge_site(&edge).is_some());
    assert!(parent_context.site(query_at).is_some());

    let child_source = "set owned 1\npackage provide P 1\n$head arbitrary\n";
    let child = Analyser::new().analyse(child_source, "tcl8.6");
    let child_context = SourceOrderContext::capture(&child).unwrap();
    assert!(child_context.site(0).is_some());
    let dynamic_at = u32::try_from(child_source.find("$head").unwrap()).unwrap();
    assert!(child_context.site(dynamic_at).is_none());
    let order = RunOrder::for_source_navigation(
        std::slice::from_ref(&edge),
        [
            ("parent".into(), parent_context.clone()),
            ("child".into(), child_context),
        ]
        .into_iter(),
    );
    let parent_point = point("parent", query_at);
    let owned = point("child", 0);
    assert_ne!(order.alternatives(owned), []);
    let positive = order.navigation_precedence(owned, parent_point).unwrap();
    assert!(*positive.conditional_relation());
    assert_eq!(
        positive.purpose(),
        SourceNavigationOrderPurpose::Conditional
    );
    assert_eq!(positive.modeled_source_exclusion(), None);
    let dynamic = point("child", dynamic_at);
    assert!(order.navigation_precedence(dynamic, parent_point).is_none());
    assert!(order.navigation_precedence(parent_point, dynamic).is_none());
    assert!(order.navigation_comparison(dynamic, parent_point).is_none());
    assert!(order.navigation_comparison(parent_point, dynamic).is_none());

    let deferred_source = "package provide P 1\nproc pending {} {$head arbitrary}\n";
    let deferred = Analyser::new().analyse(deferred_source, "tcl8.6");
    let deferred_at = u32::try_from(deferred_source.find("$head").unwrap()).unwrap();
    assert!(
        deferred
            .original_definition_body_span(deferred_at)
            .is_some()
    );
    let context = SourceOrderContext::capture(&deferred).unwrap();
    assert!(context.site(deferred_at).is_none());
    let order = RunOrder::for_source_navigation(
        std::slice::from_ref(&edge),
        [
            ("parent".into(), parent_context.clone()),
            ("child".into(), context),
        ]
        .into_iter(),
    );
    let unowned_body = point("child", deferred_at);
    assert_ne!(order.alternatives(unowned_body), []);
    assert!(
        order
            .navigation_precedence(unowned_body, parent_point)
            .is_none()
    );
    assert!(
        order
            .navigation_precedence(parent_point, unowned_body)
            .is_none()
    );
    assert!(
        order
            .navigation_comparison(unowned_body, parent_point)
            .is_none()
    );
    assert!(
        order
            .navigation_comparison(parent_point, unowned_body)
            .is_none()
    );

    let mut missing = child.clone();
    missing.resolved_input = None;
    assert!(SourceOrderContext::capture(&missing).is_none());
    let order =
        RunOrder::for_source_navigation(&[edge], [("parent".into(), parent_context)].into_iter());
    // The actual require route remains. Missing original source transport
    // cannot become a point merely because its document has that route.
    assert_ne!(order.alternatives(owned), []);
    assert!(order.navigation_precedence(owned, parent_point).is_none());
    assert!(order.navigation_precedence(parent_point, owned).is_none());
    assert!(order.navigation_comparison(owned, parent_point).is_none());
    assert!(order.navigation_comparison(parent_point, owned).is_none());
    assert_eq!(order.has_run(owned, parent_point), None);
    assert_eq!(order.cmp_run(owned, parent_point), None);
}
