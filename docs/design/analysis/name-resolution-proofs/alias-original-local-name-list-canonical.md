# Does the original one-element List local-name object become canonical when upvar obtains its name string?

Proof ID: `naming.alias.original-local-name-list-canonical`

## Problem statement

Retaining a List primary after using it as a local name does not show whether its string representation became canonical. Sharing rules and a later list operation can depend on that distinct flag. This private-header check examines the original List canonical bit at the same upvar boundary, separately from reference counts and alias-cell authority.

## Question

Does the original one-element List local-name object become canonical when upvar obtains its name string?

## Scope

The retained public object-vector probe and its exact C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0 output rows. Object primary class, reference counts or canonical flags are measured only at the specified snapshots. Jim and BIG-IP have no observation for this question. No source/argv/header/compiler/normal-completion authority follows from matching a label, reference count or name spelling.

## Provider answers

| Provider | Recorded release | Status | Answer |
|---|---|---|---|
| tcl8.4 | 8.4.20 | observed | The probe has no C8.4 native canonical flag and explicitly reports 0 before/after for List; untyped String is -1. |
| tcl8.5 | 8.5.19 | observed | Both List operands report before 0 and after 1; the untyped counted-zero String is -1. |
| tcl8.6 | 8.6.18 | observed | Both List operands report before 0 and after 1; the untyped counted-zero String is -1. |
| tcl9.0 | 9.0.4 | observed | Both List operands report before 0 and after 1; the untyped counted-zero String is -1. |
| tcl9.1 | 9.1.0 | observed | Both List operands report before 0 and after 1; the untyped counted-zero String is -1. |
| jim | not recorded | not-tested | No observation for this question. |
| bigip | not recorded | not-tested | No appliance observation for this question. |

## Conclusion

The C8.5–C9.1 captures change both original one-element List flags from noncanonical 0 to canonical 1 while preserving the List primary. C8.4 has no equivalent flag and the probe reports its explicit 0 compatibility value. The counted-zero untyped operand reports -1 and is not a List-canonical claim.

## Rust comparisons

- `rust/tcl-vm/src/interp/native_variable_names.rs`: `interp::native_variable_names::tests::alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners`. Compare canonical snapshots independently of type/reference-count rows. The C8.4 probe flag is a compatibility value, not a measured field.

The retained native process outcomes do not prove that the Rust comparison tests pass.

## Reconfirmation

```text
cc -I${TCL_SOURCE}/generic -I${TCL_SOURCE}/unix -I${TCL_BUILD} rust/tcl-vm/tests/data/native_alias_simple/canonical/probe.c ${TCL_BUILD}/libtcl${TCL_ABI}.a -ldl -lpthread -lm -o ${PROBE_EXE}
${PROBE_EXE}
```

Use the matching release source/build and ABI library, then run ${PROBE_EXE}. Compare stdout byte-for-byte with the release output file, allowing no primary/refcount/canonical normalization. The process must exit zero; observed Tcl guest Error rows in the global-cache question are expected output. This corpus retains exact programs/results but lacks build hashes and stderr/compile receipts, so it cannot attest a particular binary or a fresh reconfirmation. The canonical-list probe additionally needs the matching private tclInt.h headers; compare its before/after label and final flag column to the retained two-column projection. Its full stdout hash is recorded but full raw stream is not retained in the canonical directory. Rust fixture-comparison tests are separate contracts and their pass is not asserted here.
