# Original argv compiler, API, CLI and strict WASM capture

All three selected Cargo builds succeeded. Eighteen API descriptor/layout tests
passed. The selected controls ran 49 effective Rust tests: 34 passed and 15
failed. No selector ran zero tests. The CLI control covers eleven exact metadata
query spellings; all passed. These are scoped checks, not full-suite acceptance.

Compiler native group: five pass, four fail. The original root-colon declaration
stays GenericInvoke; the replaced-definer control refuses executable IR with
InconsistentCommandTokens. Two existing Boolean/control positives also fail.
One tail-rewrite positive fails; seven other lookup controls pass. Both compiled
table positives fail because no table is attached; the Host-refusal control
passes. No native receipt or callable stub was substituted.

All eight strict real-link controls initially failed: two native positives failed
the required shared-table assertion before linking, and six reached Wasmtime's
identity bootstrap but were refused by its default read-only cache directory.
The genuine reserved wasm32-wasip1 Runtime built with the configured wasi-sdk,
libtommath and reserved-region linker flag. The separate
[cache correction capture](../wasm-cache-validation-02/README.md) retains actual
execution after changing only XDG_CACHE_HOME. Original failures remain here.

The 36,534 pre-capture inputs are unchanged after testing. The broad cached
dep-info audit covers target history and is not selected-output provenance.
selected-local-dep-info.json plus additional-output-dep-info-bindings.json bind
all 137 recorded local Cargo output rows to 64 distinct dependency files. The
correct normalized selected source audit has 3,759 inputs: 3,757 are identical
before/after, and two newly generated CLI OUT_DIR files are identified only after
build. No selected source is unresolved or changed. The invalid package-relative
audit attempt is retained separately and is not evidence.

Reserved Runtime relative paths require its runtime/rust package root. The
separate normalized Runtime audit has 3,073 inputs: 3,017 captured unchanged and
56 post-build-only inputs, with no unresolved or changed source. These include
new third-party and generated inputs; no full third-party pre/post freeze is
claimed. Directory libtommath dependencies have their C/header children captured.

Exact argv/environment/status, tools and binaries, metadata, source hashes,
raw streams and capture programs are retained. API/native/compiler software
controls do not certify emitted-WASM publication. No Tcltest or F5 execution was
used in this capture.
