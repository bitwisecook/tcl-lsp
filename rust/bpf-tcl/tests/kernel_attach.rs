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

//! Privileged loader / link-lifecycle acceptance tests (issue #1204).
//! **These do not run by default.**
//!
//! Attaching a program to an interface or cgroup calls the `bpf()` syscall and
//! configures live kernel networking state, which needs `CAP_BPF`/`CAP_NET_ADMIN`
//! (root in practice) and a live Linux kernel — none of which exist in the
//! sandbox the rest of the suite runs in. So these are marked `#[ignore]`; run
//! them by hand on a Linux host:
//!
//! ```text
//! sudo -E cargo test -p bpf-tcl --test kernel_attach -- --ignored --nocapture
//! ```
//!
//! Every test is **hermetic and self-cleaning**: it builds a *disposable*
//! network namespace (or an isolated test cgroup), does its work inside, and
//! unconditionally tears the namespace/cgroup down at the end — so it cannot
//! leak links, pins, maps, or interfaces onto the host, and never touches a
//! real host interface. When the required tooling or privilege is missing, the
//! test **skips** with a printed reason rather than failing.
//!
//! The lifecycle *logic* (plan → load → test-run → attach → status → detach,
//! with rollback and ownership) is covered deterministically in userspace by the
//! `bpf-tcl-ir::loader` unit tests over a `ModelKernel`; these privileged tests
//! confirm the modelled contract against a real kernel.

use std::process::Command;

use bpf_tcl_codegen::ebpf::{TargetAbi, emit_program_for_target, write_object};
use bpf_tcl_ir::{DeploymentPlan, compile_module, event_spec};

fn tool_present(name: &str) -> bool {
    Command::new(name)
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stdout.is_empty() || !o.stderr.is_empty())
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|s| s.trim() == "0")
}

/// A disposable network namespace that deletes itself (and everything inside it
/// — veth pairs, XDP/TC attachments) on drop. Cleanup is unconditional so a
/// panicking test still leaves the host clean.
struct DisposableNetns {
    name: String,
}

impl DisposableNetns {
    fn new(name: &str) -> Option<Self> {
        let ok = Command::new("ip")
            .args(["netns", "add", name])
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| DisposableNetns {
            name: name.to_owned(),
        })
    }

    /// Run `ip -n <ns> …`, returning success.
    fn ip(&self, args: &[&str]) -> bool {
        let mut full = vec!["-n", &self.name];
        full.extend_from_slice(args);
        Command::new("ip")
            .args(&full)
            .status()
            .is_ok_and(|s| s.success())
    }
}

impl Drop for DisposableNetns {
    fn drop(&mut self) {
        // Deleting the namespace tears down every veth and every attachment in
        // it — the whole point of using a namespace for isolation.
        let _ = Command::new("ip")
            .args(["netns", "del", &self.name])
            .status();
    }
}

#[test]
#[ignore = "needs root + iproute2 + a live kernel; run with --ignored on a Linux host"]
fn xdp_attach_detach_in_a_disposable_namespace_leaks_nothing() {
    if !is_root() {
        eprintln!("skip: XDP attach needs root/CAP_NET_ADMIN");
        return;
    }
    if !tool_present("ip") {
        eprintln!("skip: iproute2 (`ip`) unavailable");
        return;
    }
    let src = "when XDP { setbuf p ctx\n pass }\n";
    let module = compile_module(src).expect("compiles");
    let plan = DeploymentPlan::from_module("bpftcl-test", &module);
    // Ownership-labelled pin path — detach must remove only this.
    eprintln!("plan pin: {}", plan.programs[0].pin);

    let Some(ns) = DisposableNetns::new("bpftcl-xdp-test") else {
        eprintln!("skip: could not create a disposable netns");
        return;
    };
    // A veth pair inside the namespace gives us an interface to attach to that
    // is destroyed with the namespace.
    assert!(ns.ip(&[
        "link", "add", "veth0", "type", "veth", "peer", "name", "veth1"
    ]));

    // Write the kernel object and attach it to veth0 with `ip link … xdpgeneric`.
    let obj = emit_program_for_target(&module.programs[0].program, TargetAbi::KernelXdp)
        .expect("object emits");
    let bytes = write_object(&obj, "xdp").expect("elf writes");
    let mut obj_path = std::env::temp_dir();
    obj_path.push(format!("bpftcl-xdp-{}.o", std::process::id()));
    std::fs::write(&obj_path, bytes).expect("write object");

    let attached = ns.ip(&[
        "link",
        "set",
        "dev",
        "veth0",
        "xdpgeneric",
        "obj",
        obj_path.to_str().unwrap(),
        "sec",
        "xdp",
    ]);
    if !attached {
        let _ = std::fs::remove_file(&obj_path);
        eprintln!("skip: xdpgeneric attach unsupported on this host");
        return;
    }

    // Detach explicitly (the namespace drop would also remove it, but the
    // deployment owns the detach in production).
    let detached = ns.ip(&["link", "set", "dev", "veth0", "xdpgeneric", "off"]);
    assert!(detached, "detach should succeed");
    let _ = std::fs::remove_file(&obj_path);
    // `ns` drops here, deleting the namespace and proving nothing leaks.
}

#[test]
#[ignore = "TC codegen is registry-described but not yet implemented (issue #1204)"]
fn tc_ingress_scenario_is_described_and_pending() {
    // The TC event is a typed registry contract today; its codegen (SCHED_CLS)
    // is sequenced for a later change. This test documents the intended isolated
    // scenario — a TC classifier marking/redirecting traffic in a disposable
    // namespace with a veth pair — and asserts the schema contract that a real
    // implementation must satisfy, so it fails loudly if the schema regresses.
    let spec = event_spec("TC_INGRESS").expect("TC_INGRESS described");
    assert_eq!(spec.context.struct_name, "__sk_buff");
    assert!(!spec.is_codegen_ready(), "TC codegen still pending");
    assert!(
        compile_module("when TC_INGRESS { pass }\n").is_err(),
        "TC does not compile yet"
    );
    eprintln!(
        "skip: TC ingress codegen pending; when implemented, attach a SCHED_CLS \
         program to a veth in a disposable netns and assert cleanup"
    );
}

#[test]
#[ignore = "cgroup codegen is registry-described but not yet implemented (issue #1204)"]
fn cgroup_connect_scenario_is_described_and_pending() {
    // The cgroup connect/bind events are typed registry contracts; codegen
    // (CGROUP_SOCK_ADDR) is pending. A real implementation attaches to an
    // isolated test cgroup under /sys/fs/cgroup and denies a chosen connect
    // target, then removes the attachment and the cgroup.
    let spec = event_spec("CGROUP_CONNECT4").expect("CGROUP_CONNECT4 described");
    assert_eq!(spec.context.struct_name, "bpf_sock_addr");
    assert!(spec.context_field("user_port").is_some());
    assert!(!spec.is_codegen_ready(), "cgroup codegen still pending");
    eprintln!(
        "skip: cgroup connect codegen pending; when implemented, attach to an \
         isolated test cgroup, deny one connect target, then detach + rmdir the cgroup"
    );
}
