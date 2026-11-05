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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This record tests the Rust model; no engine observation for this question.

## Consumer bindings

- [rust/tcl-compiler/src/connection_scope.rs](../../../../rust/tcl-compiler/src/connection_scope.rs), `cell_from_place`: Retain exact native byte cell and independently supplied cross-event execution overlay.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `join`: Withdraw contents, observer, binding and cache certainty across unequal execution or interpreter owners.
- [rust/tcl-compiler/src/connection_scope.rs](../../../../rust/tcl-compiler/src/connection_scope.rs), `connection_scope::tests::cross_event_cells_keep_native_bytes_and_supplied_execution_context` (linked): Opaque names with equal reporting strings remain distinct; worker, epoch, connection and unknown execution context distinguish cross-event cells.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `var_resolve::worker_execution_ingress_tests::execution_context_join_cannot_retain_equal_values_from_different_cells` (linked): Same execution preserves known values; differing execution/interpreter owners withdraw equal bytes, cache and contents receipts.

## Evidence

- [rust/tcl-compiler/src/connection_scope.rs](../../../../rust/tcl-compiler/src/connection_scope.rs)
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs)
