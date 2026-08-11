# Optimisation passes

The Rust optimiser consumes the retained `CompilationUnit` facts after CFG,
SSA, SCCP, type, effect, and interprocedural analysis. Its implementation is
under `rust/tcl-compiler/src/optimiser/`, with GVN in `src/gvn.rs`.

## Pass ownership

- `manager.rs` orchestrates the pass sequence and groups findings.
- `propagation.rs` performs constant, copy, load, and command-substitution
  propagation, including O100–O103 and related literal folds.
- `expr_simplify.rs`, `branch_folding.rs`, and `pattern_recognition.rs`
  implement expression and structural rewrites.
- `elimination.rs` owns dead-code, dead-store, and scope-aware elimination;
  optimiser-authoritative O109 findings also feed Explorer dead-store views.
- `code_sinking.rs`, `tail_call.rs`, `unused_procs.rs`, `chain_fold.rs`, and
  `end_offset.rs` implement their named specialised rewrites.
- `gvn.rs` handles value-numbering and CSE candidates. A Tcl command call is
  only eligible when registry purity, result stability, completion, mutable
  world state, dispatch dependencies, and trace policy are all proven.

The stable O-code catalogue and priorities are registry/compiler data consumed
by these modules. Add a pass beside its Rust owner, add focused tests, and
surface any durable result through the generic Explorer contract.

## Soundness rule

Missing alias, frame, dispatch, completion, trace, or world-state evidence
causes a pass to abstain. Purity alone does not authorise command-call CSE or
code motion. Optimisation findings are facts and edit plans; consumers do not
reconstruct pass logic.

## Related

- [Compiler pipeline overview](compiler-pipeline-overview.md)
- [Common semantic compiler contract](common-semantic-compiler.md)
- [Explorer coverage contract](../contracts/explorer-compiler-coverage.md)
