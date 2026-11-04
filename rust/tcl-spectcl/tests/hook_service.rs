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

//! Hook bodies run on the one hook service: a resolver that reads only its
//! word count is answered from a table, any other body through the service's
//! warm host, from whichever thread asks, and a runaway body is contained.
//!
//! Its own test binary: the published plan and the service are process-wide.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tcl_registry::arg_role::ArgRole;
use tcl_registry::spec::ArgRoleResolver;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};

const SOURCE: &str = r#"
speclib svc 2.0 {
    command svc::last {
        arity 0..
        arg_role_resolver {words ctx} {
            set n [llength $words]
            if {$n >= 1} { role [expr {$n - 1}] Body }
        }
        arg_role_resolver_roles Body
    }
    command svc::first {
        arity 0..
        arg_role_resolver {words ctx} {
            if {[lindex $words 0] eq "x"} { role 0 Body }
        }
        arg_role_resolver_roles Body
    }
    command svc::spin {
        arity 0..
        arg_role_resolver {words ctx} { while 1 {} }
        arg_role_resolver_roles Body
    }
}
"#;

fn resolvers() -> (ArgRoleResolver, ArgRoleResolver, ArgRoleResolver) {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        PackFile {
            tier: Tier::Workspace,
            path: PathBuf::from("/svc/pack.tclspec"),
            origin: Origin::DotDir,
            dependency_tier: None,
        },
        SOURCE.to_owned(),
    )]);
    tcl_spectcl::hooks::publish(&packs);
    let plan = tcl_spectcl::hooks::plan_for(&packs);
    let resolver = |name: &str| {
        let command = packs.packs[0].command(name).expect("declared");
        tcl_spectcl::hooks::specialise(command, &plan)
            .arg_role_resolver
            .expect("a bound resolver")
    };
    (
        resolver("svc::last"),
        resolver("svc::first"),
        resolver("svc::spin"),
    )
}

#[test]
fn hook_bodies_answer_from_the_service_on_any_thread() {
    let (last, first, spin) = resolvers();

    for _ in 0..2 {
        assert_eq!(last(&["a", "b"]), vec![(1, ArgRole::Body)]);
        assert_eq!(last(&["a", "b", "c", "d"]), vec![(3, ArgRole::Body)]);
        assert_eq!(last(&[]), Vec::new());
        assert_eq!(first(&["x"]), vec![(0, ArgRole::Body)]);
        assert_eq!(first(&["y"]), Vec::new());
    }

    // A fresh thread builds no VM of its own and answers alike.
    std::thread::spawn(move || {
        let started = Instant::now();
        assert_eq!(last(&["a", "b", "c"]), vec![(2, ArgRole::Body)]);
        assert_eq!(first(&["x", "y"]), vec![(0, ArgRole::Body)]);
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "a fresh thread answered in {:?}: it should not bootstrap a VM",
            started.elapsed()
        );
    })
    .join()
    .expect("worker thread");

    // A body that never finishes is stopped by its budget and abstains, and
    // the service keeps answering everything else.
    assert_eq!(spin(&["a"]), Vec::new());
    assert_eq!(spin(&["a"]), Vec::new());
    assert_eq!(last(&["a", "b"]), vec![(1, ArgRole::Body)]);
    assert_eq!(first(&["x"]), vec![(0, ArgRole::Body)]);
}
