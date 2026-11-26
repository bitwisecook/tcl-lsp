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

//! The capability in a route identity.

use super::*;

/// The capability is part of the route (`value-evaluation.md` § *The
/// capability declaration*): two declared implementations that differ in any
/// one field — the pack, the id, the body's content hash, the target axes,
/// an input, a dependency, the budget — are two routes, so nothing keyed by
/// the route can serve one's answer for the other. The same declaration
/// twice is one route.
#[test]
fn the_capability_is_part_of_the_route_identity() {
    static INPUTS: [DeclaredInput; 1] = [DeclaredInput::Operand {
        index: 0,
        exactness: Exactness::Exact,
    }];
    static OTHER_INPUTS: [DeclaredInput; 1] = [DeclaredInput::IncomingTarget { index: 0 }];
    static DEPENDS: [ContextDependency; 2] = [
        ContextDependency::TclProfile,
        ContextDependency::ImplementationIdentity,
    ];
    static FEWER_DEPENDS: [ContextDependency; 1] = [ContextDependency::TclProfile];
    let base = EvaluatorCapability {
        identity: ImplementationIdentity {
            pack: "tenant",
            id: "tenant.label.v1",
            content_hash: 1,
        },
        host: HostKind::BoundedTcl,
        target: Needs::NONE,
        inputs: &INPUTS,
        depends: &DEPENDS,
        budget: ImplementationBudget {
            commands: Some(2000),
            wall_clock_ms: Some(20),
            value_bytes: Some(65536),
        },
        completion: CompletionSupport::NormalOnly,
    };
    let variants = [
        EvaluatorCapability {
            identity: ImplementationIdentity {
                pack: "other",
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            identity: ImplementationIdentity {
                id: "tenant.label.v2",
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            identity: ImplementationIdentity {
                content_hash: 2,
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            target: Needs::NUMERAL_GRAMMAR,
            ..base
        },
        EvaluatorCapability {
            inputs: &OTHER_INPUTS,
            ..base
        },
        EvaluatorCapability {
            depends: &FEWER_DEPENDS,
            ..base
        },
        EvaluatorCapability {
            budget: ImplementationBudget {
                commands: Some(1000),
                ..base.budget
            },
            ..base
        },
    ];
    let route = EvalRoute::Implementation(base);
    assert_eq!(route, EvalRoute::Implementation(base));
    assert_eq!(route.family(), "implementation");
    assert!(route.is_enabled());
    let mut routes = BTreeSet::new();
    routes.insert(format!("{route:?}"));
    let mut hashed = std::collections::HashSet::from([route]);
    for variant in variants {
        let other = EvalRoute::Implementation(variant);
        assert_ne!(other, route, "{variant:?}");
        assert!(hashed.insert(other), "{variant:?}");
        assert!(routes.insert(format!("{other:?}")), "{variant:?}");
    }
    assert_eq!(hashed.len(), variants.len() + 1);
}
