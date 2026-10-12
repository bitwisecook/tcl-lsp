# How does reuse of a cached original dynamic global name differ between C8.4/8.5 and C8.6–9.1 source compilation?

Proof ID: `naming.variable.original-global-cache-token`

## Problem statement

A name object used to read a procedure local can acquire a localVarName cache, then be reused as a dynamic global operand. Inferring the compiler-selected local alias from its current displayed value can choose a different variable than the original source token. This probe preserves the same object across those operations and records cache class and the actual guest completion.

## Question

How does reuse of a cached original dynamic global name differ between C8.4/8.5 and C8.6–9.1 source compilation?

## Scope

The retained public object-vector probe and its exact C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0 output rows. Object primary class, reference counts or canonical flags are measured only at the specified snapshots. Jim and BIG-IP have no observation for this question. No source/argv/header/compiler/normal-completion authority follows from matching a label, reference count or name spelling.

## Provider answers

| Provider | Recorded release | Status | Answer |
|---|---|---|---|
| tcl8.4 | 8.4.20 | observed | The simple sequence records localVarName/LOCAL then localVarName/GLOBAL and success/GLOBAL; qualified records parsedVarName/QUALIFIED twice and success/QUALIFIED. |
| tcl8.5 | 8.5.19 | observed | The simple sequence records localVarName/LOCAL then localVarName/GLOBAL and success/GLOBAL; qualified records parsedVarName/QUALIFIED twice and success/QUALIFIED. |
| tcl8.6 | 8.6.18 | observed | The simple sequence records localVarName/LOCAL then guest Error cannot-read-v; qualified records parsedVarName/QUALIFIED twice then the same guest Error. Probe process exits zero after recording both. |
| tcl9.0 | 9.0.4 | observed | The simple sequence records localVarName/LOCAL then guest Error cannot-read-v; qualified records parsedVarName/QUALIFIED twice then the same guest Error. Probe process exits zero after recording both. |
| tcl9.1 | 9.1.0 | observed | The simple sequence records localVarName/LOCAL then guest Error cannot-read-v; qualified records parsedVarName/QUALIFIED twice then the same guest Error. Probe process exits zero after recording both. |
| jim | not recorded | not-tested | No observation for this question. |
| bigip | not recorded | not-tested | No appliance observation for this question. |

## Conclusion

C8.4/8.5 read LOCAL, then GLOBAL through the reused localVarName object and return GLOBAL. C8.6–C9.1 read LOCAL once and finish the simple sequence with the observed cannot-read-v Error; their compiler-selected source tail does not restore local v. The qualified sequence records parsedVarName/QUALIFIED twice in all five releases, but its final completion is success only in C8.4/8.5. These are source-token/handler/cache observations for the exact two sequences, not a general cache validity or alias identity grant.

## Exact evidence

- `runtime/rust/tests/data/native_global_cache_token/manifest.json`, SHA-256 `32fd592ec8270e02072e43455702b3010de9bfc55c24cc814c5a73c1849981f0`: Recorded release, process completion and exact output hashes; canonical-list files retain flag-column projections with separate hashes. Compile command, executable and linked library hashes are not retained here.
- `runtime/rust/tests/data/native_global_cache_token/probe.c`, SHA-256 `0d690fca1392b40a5a5aa43b0c011ca276e9927014a872599ce6c8070e6f11d9`: Exact native object producers, object-vector calls and observation order. Public source scripts are ASCII and evaluated through Tcl_Eval; name operands are directly constructed counted Tcl_Obj values.
- `runtime/rust/tests/data/native_global_cache_token/8.4.20.txt`, SHA-256 `58ceb067a7340cd4e20f8f354e80181227d63207bdb21edf8b4b7c52d689d4f2`: Exact recorded output for C Tcl 8.4.20
- `runtime/rust/tests/data/native_global_cache_token/8.5.19.txt`, SHA-256 `58ceb067a7340cd4e20f8f354e80181227d63207bdb21edf8b4b7c52d689d4f2`: Exact recorded output for C Tcl 8.5.19
- `runtime/rust/tests/data/native_global_cache_token/8.6.18.txt`, SHA-256 `a1cf181152fb73ce3b5159ab34bcbb2b6627339372e649633746aafdde934d14`: Exact recorded output for C Tcl 8.6.18
- `runtime/rust/tests/data/native_global_cache_token/9.0.4.txt`, SHA-256 `a1cf181152fb73ce3b5159ab34bcbb2b6627339372e649633746aafdde934d14`: Exact recorded output for C Tcl 9.0.4
- `runtime/rust/tests/data/native_global_cache_token/9.1.0.txt`, SHA-256 `a1cf181152fb73ce3b5159ab34bcbb2b6627339372e649633746aafdde934d14`: Exact recorded output for C Tcl 9.1.0
- `rust/tcl-vm/tests/data/native_global_cache_token/manifest.json`, SHA-256 `32fd592ec8270e02072e43455702b3010de9bfc55c24cc814c5a73c1849981f0`: Whole-byte identical second consumer copy of the canonical native capture manifest; no additional experiment is inferred.

## Rust comparisons

- `rust/tcl-vm/src/interp/native_variable_names.rs`: `interp::native_variable_names::tests::dynamic_global_uses_original_compiler_token_tail_and_name_cache`. Compare exact cache primary labels, read values and guest success/Error completions.
- `runtime/rust/src/interp/native_variable_names.rs`: `interp::native_variable_names::tests::dynamic_global_uses_original_compiler_token_tail_and_name_cache`. Compare the same original-token cache and completion rows.

The retained native process outcomes do not prove that the Rust comparison tests pass.

## Reconfirmation

```text
cc -I${TCL_SOURCE}/generic -I${TCL_SOURCE}/unix -I${TCL_BUILD} runtime/rust/tests/data/native_global_cache_token/probe.c ${TCL_BUILD}/libtcl${TCL_ABI}.a -ldl -lpthread -lm -o ${PROBE_EXE}
${PROBE_EXE}
```

Use the matching release source/build and ABI library, then run ${PROBE_EXE}. Compare stdout byte-for-byte with the release output file, allowing no primary/refcount/canonical normalization. The process must exit zero; observed Tcl guest Error rows in the global-cache question are expected output. This corpus retains exact programs/results but lacks build hashes and stderr/compile receipts, so it cannot attest a particular binary or a fresh reconfirmation. The canonical-list probe additionally needs the matching private tclInt.h headers; compare its before/after label and final flag column to the retained two-column projection. Its full stdout hash is recorded but full raw stream is not retained in the canonical directory. Rust fixture-comparison tests are separate contracts and their pass is not asserted here.
