# What happens to original one-element List local names and a counted n-zero-(k) String when upvar #0 x receives them in a procedure?

Proof ID: `naming.alias.original-local-name-object`

## Problem statement

An upvar local operand may already be a List object or contain a counted zero. A consumer that first renders the name can overlook whether the alias operation retains the original object or changes its primary representation. This check compares the original local-name object before and after the upvar handler; it does not authenticate an activation, alias table entry or callable command.

## Question

What happens to original one-element List local names and a counted n-zero-(k) String when upvar #0 x receives them in a procedure?

## Scope

The retained public object-vector probe and its exact C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0 output rows. Object primary class, reference counts or canonical flags are measured only at the specified snapshots. Jim and BIG-IP have no observation for this question. No source/argv/header/compiler/normal-completion authority follows from matching a label, reference count or name spelling.

## Provider answers

| Provider | Recorded release | Status | Answer |
|---|---|---|---|
| tcl8.4 | 8.4.20 | observed | Both List primaries remain list and the raw-zero String remains NULL; each after reference count is 1; final completion is 0/X. |
| tcl8.5 | 8.5.19 | observed | Both List primaries remain list and the raw-zero String remains NULL; after reference counts are 1,2,2; final completion is 0/X. |
| tcl8.6 | 8.6.18 | observed | Both List primaries remain list and the raw-zero String remains NULL; after reference counts are 1,2,2; final completion is 0/X. |
| tcl9.0 | 9.0.4 | observed | Both List primaries remain list and the raw-zero String remains NULL; after reference counts are 1,2,2; final completion is 0/X. |
| tcl9.1 | 9.1.0 | observed | Both List primaries remain list and the raw-zero String remains NULL; after reference counts are 1,2,2; final completion is 0/X. |
| jim | not recorded | not-tested | No observation for this question. |
| bigip | not recorded | not-tested | No appliance observation for this question. |

## Conclusion

All five captures retain the List primary for both one-element List local names, retain no typed primary for the counted zero String, and return X through the alias. C8.4 retains reference count 1 in all three after snapshots. C8.5 through C9.1 keep the first name at 1 but retain the second List and counted zero operand at 2. These exact snapshots do not establish a general object lifetime or name-table identity rule.

## Exact evidence

- `runtime/rust/tests/data/native_alias_simple/manifest.json`, SHA-256 `abee378ef2008d25f429f9938d5e579b4f444de0f1035ebbeb42c240a0e879ef`: Recorded release, process completion and exact output hashes; canonical-list files retain flag-column projections with separate hashes. Compile command, executable and linked library hashes are not retained here.
- `runtime/rust/tests/data/native_alias_simple/probe.c`, SHA-256 `2c9da4659bc2e41dfba6d921e38f5257d9d79b883d64133cebba48308e2aa51d`: Exact native object producers, object-vector calls and observation order. Public source scripts are ASCII and evaluated through Tcl_Eval; name operands are directly constructed counted Tcl_Obj values.
- `runtime/rust/tests/data/native_alias_simple/8.4.20.txt`, SHA-256 `5401dc32778ae8571e0214543ff14f749a1b52765311003e310f0adc98d64201`: Exact recorded output for C Tcl 8.4.20
- `runtime/rust/tests/data/native_alias_simple/8.5.19.txt`, SHA-256 `bc54852fafc4f0e9aa8d02476310d7588240cac69b5fe57cae59d4671b956431`: Exact recorded output for C Tcl 8.5.19
- `runtime/rust/tests/data/native_alias_simple/8.6.18.txt`, SHA-256 `bc54852fafc4f0e9aa8d02476310d7588240cac69b5fe57cae59d4671b956431`: Exact recorded output for C Tcl 8.6.18
- `runtime/rust/tests/data/native_alias_simple/9.0.4.txt`, SHA-256 `bc54852fafc4f0e9aa8d02476310d7588240cac69b5fe57cae59d4671b956431`: Exact recorded output for C Tcl 9.0.4
- `runtime/rust/tests/data/native_alias_simple/9.1.0.txt`, SHA-256 `bc54852fafc4f0e9aa8d02476310d7588240cac69b5fe57cae59d4671b956431`: Exact recorded output for C Tcl 9.1.0
- `rust/tcl-vm/tests/data/native_alias_simple/manifest.json`, SHA-256 `abee378ef2008d25f429f9938d5e579b4f444de0f1035ebbeb42c240a0e879ef`: Whole-byte identical second consumer copy of the canonical native capture manifest; no additional experiment is inferred.

## Rust comparisons

- `rust/tcl-vm/src/interp/native_variable_names.rs`: `interp::native_variable_names::tests::alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners`. Compare original primary/reference-count snapshots and exact final result to the retained public native probe.
- `runtime/rust/src/interp/native_variable_names.rs`: `interp::native_variable_names::tests::alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners`. Compare original primary/reference-count snapshots to the same retained public native probe.

The retained native process outcomes do not prove that the Rust comparison tests pass.

## Reconfirmation

```text
cc -I${TCL_SOURCE}/generic -I${TCL_SOURCE}/unix -I${TCL_BUILD} runtime/rust/tests/data/native_alias_simple/probe.c ${TCL_BUILD}/libtcl${TCL_ABI}.a -ldl -lpthread -lm -o ${PROBE_EXE}
${PROBE_EXE}
```

Use the matching release source/build and ABI library, then run ${PROBE_EXE}. Compare stdout byte-for-byte with the release output file, allowing no primary/refcount/canonical normalization. The process must exit zero; observed Tcl guest Error rows in the global-cache question are expected output. This corpus retains exact programs/results but lacks build hashes and stderr/compile receipts, so it cannot attest a particular binary or a fresh reconfirmation. The canonical-list probe additionally needs the matching private tclInt.h headers; compare its before/after label and final flag column to the retained two-column projection. Its full stdout hash is recorded but full raw stream is not retained in the canonical directory. Rust fixture-comparison tests are separate contracts and their pass is not asserted here.
