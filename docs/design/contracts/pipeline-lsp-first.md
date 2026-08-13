# LSP-first compiler pipeline layering

Where semantic facts belong in the pipeline. Editor features need
CFG/SSA/bytecode-like semantics, so those facts are modelled early and shared,
rather than surfacing only late in codegen or being re-derived by every pass
that wants them.

## Context

In this repository, editor features benefit most from facts that are:

- available early,
- range-preserving,
- reusable across diagnostics/completions/code actions,
- independent of bytecode text formatting.

## Recommended approach

1. Model semantic facts in IR/CFG/SSA-adjacent types first.
2. Treat codegen as a consumer of those facts, not the first producer.
3. Keep one orchestration path (`CompilationUnit`) so all passes agree on source->facts mapping.
4. Expose pass outputs with stable IDs and related ranges for LSP UX quality.

## Anti-patterns

- Rebuilding IR/CFG/SSA ad hoc inside individual pass entry points.
- Encoding analysis-only semantics as codegen-only branches.
- Large "god modules" that combine IR interpretation, opcode policy, and formatting.
- Fixing bugs only at the VM runtime layer when the same error is
  statically detectable — always prefer adding a diagnostic so the user
  sees the problem in the editor before running the code.

## Cross-links

- Architecture: [`../compiler-architecture.md`](../compiler-architecture.md).
- Fuzz-finding workflow (early-pipeline fix priority): the `fuzz-findings` skill.
