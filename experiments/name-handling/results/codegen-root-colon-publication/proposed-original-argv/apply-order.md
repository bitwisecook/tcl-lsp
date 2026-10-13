# Which files do the two unapplied patches own?

Status: **UNAPPLIED / UNCOMPILED / UNEXECUTED**. This is an application recipe;
the preparation scripts only read source and write temporary proposed diffs.

Validate every artifact hash and every complete source preimage in
[provenance.json](provenance.json) before preparing any production mutation.
Re-run both retained preparation recipes and require byte equality with the two
reviewed patches. A changed or missing preimage requires regeneration and review;
do not silently offset a patch onto an unreviewed source image.

| Repository file | Implementation patch | Test patch |
| --- | --- | --- |
| `rust/tcl-compiler/src/native_lowering/ir.rs` | Evaluated original argv and entry key | — |
| `rust/tcl-compiler/src/native_lowering/lower.rs` | Original evaluated vector; existing admission gates | — |
| `rust/tcl-compiler/src/codegen/wasm/native_emit.rs` | Additive import, frame sizing and completion | — |
| `rust/tcl-compiler/src/codegen/wasm/backend.rs` | General argv path, private cleanup and internal table key | Actual generic argument/body plan checks |
| `rust/tcl-runtime-api/src/codegen_abi.rs` | Additive descriptor and `ALL` | Signature, default refusal and fingerprint |
| `runtime/rust/src/cmd_proc.rs` | Shared original parser with optional entry | — |
| `runtime/rust/src/codegen_abi.rs` | Borrowed-argv entry | Actual namespace/name objects, compiled entry and replacement lifecycle |
| `rust/tcl-compiler/src/native_lowering/tests.rs` | — | Evaluated name, C86/C9 admitted definition and dynamic-name fallback |
| `rust/tcl-compiler/src/codegen/wasm/pipeline.rs` | — | Both admission refusals, lazy body and import counters |
| `rust/tcl-compiler/tests/wasm_codegen.rs` | — | Generic count and additive native-import counters |
| `rust/tcl-compiler/tests/wasm_real_link.rs` | — | Actual table invocation, strict loader, signature, once-only/inert bodies |

The patches share the original preimages of the backend, runtime API and Runtime
ABI files. Review the combined result for those files. Apply the implementation
changes and then the test changes to the checked source image, preserving the
non-overlapping edits from both. A second preimage check after the first patch
cannot compare a shared file against its original hash; the combined proposed
image, rather than a waived mismatch, governs that step. Neither patch changes
registry vocabulary or replaces the independent native admission/literal owner.

The test recipe inserts Runtime controls inside the existing `codegen_abi::tests`
module, which owns its actual interpreter, borrowed-object and software callable
helpers. It appends real-link controls in `wasm_real_link.rs`, where the existing
actual Runtime/Wasmtime/table/manifest helpers are available. The descriptor and
legacy-compatibility tests remain in the runtime API owner. Control source files
remain separately inspectable; their status does not supply a test pass.
The test preparation recipe excludes proposal headers and non-executable tracking
notes from the production diff while retaining the actual controls and necessary
ownership comments.

Validation must run the actual updated Runtime and compiler together. Require the
new-import replacement control, C86/C9 definition presence, both admission
refusals, two compiled table targets, once-only body effects and strict manifest
checks. Optional harness unavailability is not an execution result. The existing
old-import/lazy-body/frame/cleanup coverage remains independent. Production
application and builds have not been performed by these artifacts.
