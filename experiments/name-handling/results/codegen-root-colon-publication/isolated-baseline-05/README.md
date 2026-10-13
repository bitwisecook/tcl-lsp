# Independent original-head and materialised-name WASM controls

The Compiler library and real-link test binaries build successfully with jobs1
and testdebug0 in the independent codegen checkout at7aa200ed1. Both genuine
Runtime-linked WASM controls pass: renamed stock definer and constant-materialised
name. TCL_REQUIRE_WASM_LINK=1 prevents skipping; the checkout's own reserved
Runtime was compiled and run through Wasmtime. Neither control claims a native
procedure entry or table positive.

The 10,221 pre-captured inputs are unchanged. All 3,655 dependency inputs bound
to recorded local Cargo outputs are covered identically before/after, with no
unresolved output/dependency or changed source. Runtime dep-info separately has
3,073 inputs: 3,017 covered and56 post-build-only dependency/generated inputs,
with no unresolved or changed source. Broad cached dep-info is an audit seed,
not a selected dependency proof. No full third-party-before/after claim is made.

Rust/Cargo/Wasmtime/Binaryen executable identities, exact argv/environment/status,
actual binary/Runtime identities, metadata/source files and raw outputs are kept.
Configured pinned SDK tool hashes are a labelled post-capture supplement.
Initial missing sparse inputs and the own memory-coordination interruption are
retained as prerequisite failures; exact restored tracked files match the same
Git commit. No sibling artefacts were used. No Tcltest or F5 execution was used.
