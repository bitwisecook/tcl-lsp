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

//! The loader lifecycle driven from a real compiled module (issue #1204), over
//! the deterministic `ModelKernel`. Real `bpf()`-syscall attachment is exercised
//! only by the `#[ignore]`d `kernel_attach` tests; this proves the
//! plan → load → test-run → attach → status → detach state machine and its
//! ownership/cleanup guarantees end to end without a kernel.

use bpf_tcl_ir::{Deployment, DeploymentPlan, ModelKernel, State, compile_module};

const CHAIN: &str = "when XDP priority 10 { setbuf p ctx\n pass }\nwhen XDP priority 20 { pass }\n";

#[test]
fn full_lifecycle_from_a_compiled_module_leaves_no_residue() {
    let module = compile_module(CHAIN).expect("compiles");
    let plan = DeploymentPlan::from_module("lifecycle", &module);
    let mut d = Deployment::new(plan, ModelKernel::new());

    // plan -> load
    assert_eq!(d.state(), State::Planned);
    d.load().expect("verifier-load");
    assert_eq!(d.state(), State::Loaded);

    // test-run each program before any live attach (non-destructive default).
    for i in 0..2 {
        d.test_run(i, &[0u8; 64]).expect("test-run");
    }

    // attach -> status
    d.attach().expect("attach");
    assert_eq!(d.state(), State::Attached);
    let status = d.status();
    assert_eq!(status.attached, 2);
    assert!(status.pins.iter().all(|p| p.contains("/bpftcl/lifecycle/")));
    assert_eq!(d.kernel().pin_count(), 2);

    // detach removes only our resources.
    d.detach().expect("detach");
    assert_eq!(d.state(), State::Detached);
    assert_eq!(d.kernel().pin_count(), 0);
    assert_eq!(d.kernel().live_program_count(), 0);
}

#[test]
fn two_deployments_use_disjoint_ownership_labelled_pins() {
    let module = compile_module("when XDP { pass }\n").expect("compiles");
    let a = DeploymentPlan::from_module("alpha", &module);
    let b = DeploymentPlan::from_module("beta", &module);
    assert_ne!(a.programs[0].pin, b.programs[0].pin);
    assert!(a.owns_pin(&a.programs[0].pin));
    assert!(!a.owns_pin(&b.programs[0].pin));
    assert!(!b.owns_pin(&a.programs[0].pin));
}
