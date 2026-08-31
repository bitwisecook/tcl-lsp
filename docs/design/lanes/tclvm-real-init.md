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

- [x] `rust/tcl-vm/src/interp.rs`: real library initialisation entry point.
- [x] `rust/tcl-vm/src/cmd_package.rs`: package unknown/ifneeded discovery.
- [x] `rust/tcl-vm/examples/run_test.rs`: initialise, require, then source test.
- [ ] `rust/tcl-vm/tests/`: mutation-resistant startup and package tests.
- [x] `rust/xtask/src/tcltest_sweep.rs`: exact shared Tcl 9.0.4 selection and
      default overlay for both backends.
- [x] `tests/external/backend_constraints.tcl`: backend-only constraints and
      scope tests.
- [ ] `runtime/rust/vendor/tcl_library/`: refresh from
      `/home/jimd/src/tcl9.0.4/library` with provenance/version drift check.
- [ ] Active harness, KCS, tier, README, and design documentation.
- [ ] Focused upstream `.test` run using
      `/home/jimd/src/tcl9.0.4/unix/tclsh` and matching `LD_LIBRARY_PATH`.

## Accepted behavioural deltas

- Tcl 9 `source -nopkg` follows the ordinary VM source path because the VM has
  no package-source bookkeeping to suppress.
- `glob -directory DIR -join PATTERN ...` now matches successive path
  components. Real `tclPkgUnknown` requires this to find only child
  `pkgIndex.tcl` files rather than treating every directory entry as an index.
- `::tcl::unsupported::clock::configure -init-complete` is accepted as the
  internal startup notification. Other clock-configuration options remain
  unsupported and fail explicitly.
- Focused evidence: real Tcl 9.0.4 `set.test` test `set-1.1` completed through
  the new path with `Total 64 Passed 1 Skipped 63 Failed 0`.
- The sweep now selects `unix/tclsh`, `library/`, and `tests/` from one
  `TclSourceTree`, validates the interpreter with that tree's `unix/` on
  `LD_LIBRARY_PATH`, and rejects every patch level except 9.0.4. This caught
  the selected 9.0.4 binary resolving a host 9.0.3 library before the loader
  path was applied.
- The default overlay registers backend identity constraints and excludes only
  the exact extended-platform-key assertion plus whole socket, exec, thread,
  asynchronous-thread, filesystem-host, and file-command groups when the
  corresponding host capability is absent. An xtask unit test pins that allow
  list and rejects representative ordinary-semantic stem globs.
- Focused paired evidence: `set-1.1` under
  `/home/jimd/src/tcl9.0.4/{unix/tclsh,library,tests}` is
  `C 1/63/0 | VM 1/63/0 | MATCH`.

## Open uncertainties

- Which exact upstream constraints are triggered by the current VM needs to be
  measured after real startup works; every proposed skip will be traced to a
  platform, host capability, or representation-only assertion before inclusion.
- The Tcl 9.0.4 read-closure may differ from the existing 9.0.3 subset; trace
  and deterministic comparison will decide the final file list.
