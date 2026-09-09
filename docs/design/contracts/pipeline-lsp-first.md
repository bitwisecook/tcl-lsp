# LSP-first compiler pipeline layering

Where semantic facts belong in the pipeline. Editor features need
CFG/SSA/bytecode-like semantics, so those facts are modelled early and shared,
rather than surfacing only late in codegen or being re-derived by every pass
that wants them.

## Decision rules / contracts

A fact an editor feature can use is available early, range-preserving,
reusable across diagnostics / completions / code actions, and independent of
bytecode text formatting. So:

1. Model semantic facts in IR/CFG/SSA-adjacent types first.
2. Treat codegen as a consumer of those facts, not the first producer.
3. Keep one orchestration path (`CompilationUnit`) so all passes agree on
   the source → facts mapping.
4. Expose pass outputs with stable IDs and related ranges.

## Anti-patterns

- Rebuilding IR/CFG/SSA ad hoc inside individual pass entry points.
- Encoding analysis-only semantics as codegen-only branches.
- Large "god modules" that combine IR interpretation, opcode policy, and formatting.
- Fixing bugs only at the VM runtime layer when the same error is
  statically detectable — always prefer adding a diagnostic so the user
  sees the problem in the editor before running the code.

## Cross-links

- Architecture: [`../compiler/architecture.md`](../compiler/architecture.md).
- Fuzz-finding workflow (early-pipeline fix priority): the `fuzz-findings` skill.
