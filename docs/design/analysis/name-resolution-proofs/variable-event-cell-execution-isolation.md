# naming.variable.event-cell-execution-isolation

Kind: `implementation-contract`

## Problem statement

Two iRules reads may have the same reported variable name and equal known bytes while belonging to different workers, event epochs or connections. A source navigation symbol cannot identify their executing storage. Joining those states or matching cross-event cells by the display name could propagate a value between different execution domains.

## Question

Does the Rust variable model keep exact native name bytes and supplied execution context separate, and withdraw contents when independently selected execution contexts disagree?

## Conclusion

Cross-event cell identity retains native name bytes and supplied interpreter, worker, event epoch, connection and storage domain. ResolveContext joins of unequal execution or interpreter owners withdraw contents, representations, binding certainty and observer closure. Equal source symbols supply none of these execution facts.

## Scope

Rust implementation contract for supplied WorkerExecution and interpreter owners, including unknown versus known contexts. No BIG-IP appliance run, RULE_INIT broadcast, per-TMM value agreement, native variable incarnation or successful handler claim is made.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record tests the Rust model; no engine observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record tests the Rust model; no engine observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record tests the Rust model; no engine observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record tests the Rust model; no engine observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record tests the Rust model; no engine observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This record tests the Rust model; no engine observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This record tests the Rust model; no engine observation for this question.

## Exact evidence

- `event-cell` (implementation): [rust/tcl-compiler/src/connection_scope.rs](../../../../rust/tcl-compiler/src/connection_scope.rs). SHA-256 `a92311da8977b3b419da7f40215efb344ee77500e162b15ea99768bab6aa0a19`. Current shared Rust implementation, independently of native provider observations.
- `execution-join` (implementation): [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs). SHA-256 `df0d9718c24ebba475720e5fcc66bdc9b95435e9a0b71d3d9584ef406bd4b0eb`. Current shared Rust implementation, independently of native provider observations.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/connection_scope.rs](../../../../rust/tcl-compiler/src/connection_scope.rs), `cell_from_place`: Retain exact native byte cell and independently supplied cross-event execution overlay.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `join`: Withdraw contents, observer, binding and cache certainty across unequal execution or interpreter owners.
- [rust/tcl-compiler/src/connection_scope.rs](../../../../rust/tcl-compiler/src/connection_scope.rs), `connection_scope::tests::cross_event_cells_keep_native_bytes_and_supplied_execution_context` (linked): Opaque names with equal reporting strings remain distinct; worker, epoch, connection and unknown execution context distinguish cross-event cells.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `var_resolve::worker_execution_ingress_tests::execution_context_join_cannot_retain_equal_values_from_different_cells` (linked): Same execution preserves known values; differing execution/interpreter owners withdraw equal bytes, cache and contents receipts.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-compiler",
  "--lib",
  "execution_context_join_cannot_retain_equal_values_from_different_cells"
]
```

Run the second linked selector separately. Root executed both focused tests in frozen snapshot 039 and reported both passing; this is Rust execution, not an appliance observation. No native broadcast protocol is attached.
