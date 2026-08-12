# VM / bytecode test boundary and identity expectations

## Symptom

Bytecode identity tests fail across Tcl versions, or disassembly comparisons
look nondeterministic after a compiler or codegen change.

## Operational context

Two different questions live at this boundary, and conflating them is the main
failure mode:

* **Identity** — does our emitted bytecode match C Tcl's for a given snippet
  and version? Checked against captured reference disassembly.
* **Behaviour** — does the VM produce C Tcl's *result*? Checked by the
  execution suites and the differential fuzzer
  ([differential-fuzzing.md](differential-fuzzing.md)).

Identity is version-specific and inherently brittle; behaviour is the contract
that actually matters to a user. Analysis and diagnostics contracts must
depend on neither backend formatting nor C's codegen choices.

## Decision rules / contracts

1. **Bytecode identity checks are version-aware.** 8.4, 8.5, 8.6, and 9.x
   differ, and 8.4 in particular is captured differently (`tcl_traceCompile 2`
   rather than `tcl::unsupported::disassemble`), which is why it has its own
   capture script.
2. **A codegen correctness change updates the expected fixtures explicitly**,
   in the same change, so the diff shows exactly which snippets moved.
3. **Refreshing reference fixtures never mixes intended and unintended
   changes.** Re-capture, then read the diff — a fixture refresh that also
   silently absorbs a regression is how a behavioural break ships green.
4. Fixtures are split by verdict: `matching/` snippets are the ones our
   codegen is expected to reproduce identically, `divergent/` are the ones
   where we deliberately differ. Moving a snippet between them is a stated
   decision.
5. **Diagnostics and analysis contracts do not depend on backend formatting
   quirks.** Where a check needs a semantic fact, it reads the IR/CFG/SSA
   fact, not disassembly text
   ([pipeline-lsp-first.md](pipeline-lsp-first.md)).
6. Which constructs C *chooses* to bytecode-compile versus invoke is internal
   to C; we mirror the observable result, not the codegen decision
   ([compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md)).

## File-path anchors

- `rust/tcl-bytecode/src/` — the bytecode model (`layout.rs`, `format.rs`).
- `rust/tcl-compiler/src/codegen/` — the emitter.
- `rust/tcl-compiler/tests/fixtures/codegen/{matching,divergent}/` — the
  snippet corpus.
- `tests/bytecode_reference/<version>/` — captured reference disassembly.
- `scripts/capture/bytecode.sh` — capture for 8.5 / 8.6 / 9.0.
- `scripts/capture/bytecode_84.sh` — the 8.4 capture path.
- `rust/tcl-vm/src/exec.rs` — the VM's instruction execution.

## Failure modes

- Opcode layout drift causing a fixture mismatch with no semantic breakage
  context to explain it.
- A reference-fixture refresh mixing intended and unintended behaviour changes.
- Cross-version assumptions leaking into a single-version assertion.
- An analysis contract accidentally pinned to disassembly text.

## Test anchors

- `rust/tcl-compiler/tests/codegen_golden.rs`, `codegen.rs`,
  `differential_codegen.rs`, `body_command_dis_oracle.rs`.
- `rust/tcl-vm/tests/` — the execution suites (`language_e2e.rs`, the
  `cmd_*_e2e.rs` family, `command_resolution_conformance.rs`).
- The `bytecode-compare` skill drives a snippet against real tclsh
  disassembly for 8.4–9.0.

## Discoverability

- [Design doc index](../README.md)
- [compiler bytecode boundary](../compiler/bytecode-boundary.md)
- [compiler pipeline overview](../compiler/compiler-pipeline-overview.md)
- [VM opcode coverage](../runtime/tclvm-opcode-status.md)
