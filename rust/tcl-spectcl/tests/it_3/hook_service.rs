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
    let _guard = SERVICE_TEST.lock().unwrap();
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

static SERVICE_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn edited_pack(source: &str) -> tcl_spectcl::PackSet {
    tcl_spectcl::pack::load_in_memory(vec![(
        PackFile {
            tier: Tier::Workspace,
            path: PathBuf::from("/svc/reload-control.tclspec"),
            origin: Origin::DotDir,
            dependency_tier: None,
        },
        source.to_owned(),
    )])
}

fn bound_resolver(
    packs: &tcl_spectcl::PackSet,
) -> (tcl_registry::pack_hooks::HookSlot, ArgRoleResolver) {
    let plan = tcl_spectcl::hooks::plan_for(packs);
    let slot = plan.packs()[0].programs[0].slot.expect("bound source slot");
    let command = packs.packs[0]
        .command("svc::changing")
        .expect("declared command");
    (
        slot,
        tcl_spectcl::hooks::specialise(command, &plan)
            .arg_role_resolver
            .expect("bound resolver"),
    )
}

#[test]
fn a_stable_source_slot_retires_its_count_table_on_a_word_sensitive_edit() {
    let _guard = SERVICE_TEST.lock().unwrap();
    let counted = edited_pack(
        r"
speclib svc_reload 2.0 {
    command svc::changing {
        arity 0..
        arg_role_resolver -inputs {words} {words ctx} {
            if {[llength $words] >= 1} {role [expr {[llength $words] - 1}] Body}
        }
        arg_role_resolver_roles Body
    }
}
",
    );
    tcl_spectcl::hooks::publish(&counted);
    let (old_slot, old) = bound_resolver(&counted);
    // Invocation synchronises behind the native actor's install/tabulation.
    // At 33 words it falls outside the table, so the actor has necessarily
    // completed its probes before the following small call.
    let many = ["x"; 33];
    assert_eq!(old(&many), vec![(32, ArgRole::Body)]);
    assert_eq!(old(&["x"]), vec![(0, ArgRole::Body)]);
    let before = tcl_registry::pack_hooks::cache_stats();
    assert_eq!(old(&["y"]), vec![(0, ArgRole::Body)]);
    assert_eq!(
        tcl_registry::pack_hooks::cache_stats().misses,
        before.misses,
        "the small call used the count table"
    );

    let changed = edited_pack(
        r"
speclib svc_reload 2.0 {
    command svc::changing {
        arity 0..
        arg_role_resolver -inputs {words} {words ctx} {
            if {[lindex $words 0] eq {x}} {role 0 Body}
        }
        arg_role_resolver_roles Body
    }
}
",
    );
    let (new_slot, new) = bound_resolver(&changed);
    assert_eq!(
        new_slot, old_slot,
        "editing retains the exact authored source slot"
    );
    tcl_spectcl::hooks::publish(&changed);
    assert_eq!(
        new(&["y"]),
        Vec::new(),
        "the retired count table cannot answer this word-sensitive body"
    );
    assert_eq!(new(&["x"]), vec![(0, ArgRole::Body)]);
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

#[test]
fn a_worker_warm_answer_obeys_actor_quarantine_and_a_healthy_reload() {
    let _guard = SERVICE_TEST.lock().unwrap();
    let source = r"
speclib svc_reload 2.0 {
    command svc::changing {
        arity 1
        arg_role_resolver -inputs {words} {words ctx} {
            if {[lindex $words 0] eq {y}} {while 1 {}}
            role 0 Body
        }
        arg_role_resolver_roles Body
    }
}
";
    let packs = edited_pack(source);
    tcl_spectcl::hooks::publish(&packs);
    let (slot, resolver) = bound_resolver(&packs);
    let (ready, waiting) = std::sync::mpsc::sync_channel(0);
    let (resume, held) = std::sync::mpsc::sync_channel(0);
    let worker = std::thread::spawn(move || {
        assert_eq!(resolver(&["x"]), vec![(0, ArgRole::Body)]);
        let before = tcl_registry::pack_hooks::cache_stats();
        assert_eq!(resolver(&["x"]), vec![(0, ArgRole::Body)]);
        assert_eq!(
            tcl_registry::pack_hooks::cache_stats().hits,
            before.hits + 1,
            "x was warm on the actual requesting worker"
        );
        assert_eq!(
            resolver(&["y"]),
            Vec::new(),
            "actual VM command budget quarantines the hook"
        );
        assert!(!tcl_registry::pack_hooks::slot_available(slot));
        assert_eq!(
            resolver(&["x"]),
            Vec::new(),
            "the previous worker cache cannot bypass actor quarantine"
        );
        ready.send(()).unwrap();
        held.recv().unwrap();
        assert_eq!(
            resolver(&["y"]),
            vec![(0, ArgRole::Body)],
            "same worker and source slot recover under the newly published healthy body"
        );
        assert_eq!(resolver(&["x"]), vec![(0, ArgRole::Body)]);
    });
    waiting.recv().unwrap();
    let healthy = edited_pack(&source.replace(
        "if {[lindex $words 0] eq {y}} {while 1 {}}",
        "set healthy 1",
    ));
    assert_eq!(bound_resolver(&healthy).0, slot);
    tcl_spectcl::hooks::publish(&healthy);
    resume.send(()).unwrap();
    worker.join().unwrap();
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

#[test]
fn a_tabulated_hook_stops_answering_after_its_large_call_is_quarantined() {
    let _guard = SERVICE_TEST.lock().unwrap();
    let packs = edited_pack(
        r"
speclib svc_reload 2.0 {
    command svc::changing {
        arity 0..
        arg_role_resolver -inputs {words} {words ctx} {
            if {[llength $words] > 32} {while 1 {}}
            if {[llength $words] >= 1} {role 0 Body}
        }
        arg_role_resolver_roles Body
    }
}
",
    );
    tcl_spectcl::hooks::publish(&packs);
    let (slot, resolver) = bound_resolver(&packs);
    assert_eq!(resolver(&["x"]), vec![(0, ArgRole::Body)]);
    let before = tcl_registry::pack_hooks::cache_stats();
    assert_eq!(resolver(&["y"]), vec![(0, ArgRole::Body)]);
    assert_eq!(
        tcl_registry::pack_hooks::cache_stats().misses,
        before.misses,
        "the small calls are served by the actual table"
    );
    assert_eq!(
        resolver(&["x"; 33]),
        Vec::new(),
        "the actual VM budget stops a large call outside the table"
    );
    assert!(!tcl_registry::pack_hooks::slot_available(slot));
    assert_eq!(
        resolver(&["x"]),
        Vec::new(),
        "the small-call table must obey the same quarantine"
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}
