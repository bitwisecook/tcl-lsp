# Tcl VM real-library initialisation lane

## Goal

Implement issue #1737 and only the missing backend-constraint-overlay asset
from issue #1412. The bytecode VM must source the selected Tcl 9.0.4
`$TCL_LIBRARY/init.tcl` through its ordinary compiler and evaluator, discover
`tcltest` with `package require`, and share the exact source/library selection
with the C side of `cargo xtask tcltest-sweep`.

The other #1412 `rename` and `interp` defects are explicitly outside this
lane.

## Design decisions

- `Vm::init_library` is the public startup seam. It reads the VM's existing
  global `tcl_library`, verifies that it is non-empty, and evaluates a normal
  `source` command through the injected `CompileService`. It does not duplicate
  `init.tcl` logic in Rust.
- The VM package registry must retain `package unknown` and `package ifneeded`
  scripts and execute them from `package require`; otherwise sourcing real
  `init.tcl` would not make package discovery real.
- The sweep chooses one `TclSourceTree` and derives the test directory,
  `$TCL_LIBRARY`, reference `tclsh`, and exact patch-level assertions from it.
- A checked-in Tcl constraint overlay is shared by both backends. Its entries
  may cover only host/platform capabilities and internal representation probes,
  never ordinary Tcl semantics.
- The embedded runtime standard-library subset remains an unmodified,
  licence-preserving read-closure copied from the official Tcl 9.0.4 tree. A
  deterministic manifest/check owns its exact version and bytes.

## Site inventory

- [ ] `rust/tcl-vm/src/interp.rs`: real library initialisation entry point.
- [ ] `rust/tcl-vm/src/cmd_package.rs`: package unknown/ifneeded discovery.
- [ ] `rust/tcl-vm/examples/run_test.rs`: initialise, require, then source test.
- [ ] `rust/tcl-vm/tests/`: mutation-resistant startup and package tests.
- [ ] `rust/xtask/src/tcltest_sweep.rs`: exact shared Tcl 9.0.4 selection and
      default overlay for both backends.
- [ ] `tests/external/backend_constraints.tcl`: backend-only constraints and
      scope tests.
- [ ] `runtime/rust/vendor/tcl_library/`: refresh from
      `/home/jimd/src/tcl9.0.4/library` with provenance/version drift check.
- [ ] Active harness, KCS, tier, README, and design documentation.
- [ ] Focused upstream `.test` run using
      `/home/jimd/src/tcl9.0.4/unix/tclsh` and matching `LD_LIBRARY_PATH`.

## Accepted behavioural deltas

None yet.

## Open uncertainties

- Which exact upstream constraints are triggered by the current VM needs to be
  measured after real startup works; every proposed skip will be traced to a
  platform, host capability, or representation-only assertion before inclusion.
- The Tcl 9.0.4 read-closure may differ from the existing 9.0.3 subset; trace
  and deterministic comparison will decide the final file list.
