# Tcl conformance harness

Tcl 9.0 is the reference standard for the bytecode VM. Conformance tests and
developer tools therefore need a trustworthy way to find the C interpreter,
find the matching upstream source suite, and retain values that are not valid
UTF-8. `tcl-test-support` owns that boundary for the Rust workspace.

## Ownership and dependency direction

`rust/tcl-test-support` owns the release matrix, environment-variable names,
binary validation, source-tree validation, and raw child-process execution.
It depends only on `tcl-dialect` for the release axis. Semantic test cases,
Tcltest result parsing, VM adapters, and runtime adapters stay with their
respective consumers.

Consumers must use:

- `locate_tclsh` or `available_tclshs` to select a C oracle;
- `locate_source_tree` to select upstream `library/` and `tests/` content; and
- `run_script` when invoking the oracle directly.

`ScriptOutcome` preserves stdout and stderr as bytes. A consumer may call
`strict_text` only when its contract is text-valued. This prevents byte-array
tests from silently accepting lossy UTF-8 replacement characters.

## Resolution and validation

An explicit CLI source path wins, followed by the release-specific
`TCL_LSP_TCL_ROOT*` variable, the repository's pinned `tmp/tcl*` tree, sibling
source trees, and `$HOME/src/tcl*`. Interpreter discovery uses the matching
`TCL_LSP_TCLSH*` variable before conventional versioned names on `PATH`.

Explicit overrides are promises: missing paths, invalid layouts, and wrong
release lines are hard errors. Source trees are validated from
`generic/tcl.h` plus the required `library/init.tcl` and `tests/all.tcl` files;
interpreters are validated using both `info tclversion` and `info patchlevel`.

The interpreter and source tree must both match the exact reference patchlevel
pinned by `TclVersion::V9_0`, including for focused `--stem` runs. Explicit
`--tcl-root` and `TCL_LSP_TCL_ROOT90` overrides select a location, not a
different oracle version; a same-release stale tree is rejected rather than
silently changing the comparison baseline.

## Focused and full execution

`cargo xtask tcltest-sweep` has two distinct modes:

- no `--stem`: run the complete capability ladder against the pinned Tcl 9.0
  patchlevel and write or check the baseline and scoreboard;
- one or more `--stem` values: run only those files and print results without
  touching generated artifacts. `--match` narrows Tcltest IDs further and is
  valid only with a focused run.

The same `--match` filter is passed to C Tcl and the VM example runner. This is
the normal regression loop for a TclVM issue: first pin a small upstream test
selection, then implement the semantic fix at its shared compiler/runtime
owner, and finally run the owning crate's deterministic regression tests.

Full Tcltest sweeps remain manual/exhaustive work. They are not added to smoke
or ordinary CI merely because the shared harness makes them easier to invoke.
