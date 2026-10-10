# Which exact byte and owner facts does the Compiler variable cell bridge preserve independently of its reporting label?


Proof ID: `naming.variable.byte-cell-correspondence`


## Problem statement

Opaque or counted-zero names may share a Unicode reporting label while selecting different storage. Converting a diagnostic name back to bytes would merge cells, array indices or aliases. The implementation therefore needs typed byte storage and independently selected interpreter/frame/namespace/lifetime identities before any read or mutation claim.


## Question

Which exact byte and owner facts does the Compiler variable cell bridge preserve independently of its reporting label?


## Answers

| Provider | Status | Version/build | Answer |

|---|---|---|---|

| tcl8.4 | not-tested | not recorded; not recorded | No observation for this question. |

| tcl8.5 | not-tested | not recorded; not recorded | No observation for this question. |

| tcl8.6 | not-tested | not recorded; not recorded | No observation for this question. |

| tcl9.0 | not-tested | not recorded; not recorded | No observation for this question. |

| tcl9.1 | not-tested | not recorded; not recorded | No observation for this question. |

| jim | not-tested | not recorded; not recorded | No observation for this question. |

| bigip | not-tested | not recorded; not recorded | No appliance observation for this question. |


## Conclusion

The implementation contracts preserve NameBytes in CellIdentity, VariableCellKey and literal Index values. Cell and alias keys use these fields independently of Place display text. Runtime input resolution requires an explicitly selected compatible name recipe and namespace/frame ownership; unavailable execution policies stay unknown. These Rust contracts are separate from native runtime observations and normal-value evidence.


## Scope

Compiler typed cell/index/alias transport and runtime input facade only. No native frame/header/compiled-local slot, value bytes, normal completion, BIG-IP broadcast or object release proof follows.


## Exact retained evidence


## Replay

```sh

cargo test -p tcl-compiler var_resolve::original_bytes::tests -- --test-threads=1

```

This is a Rust implementation-contract replay, not a native reconfirmation. The independent byte-cell selector above must also run. No result is claimed by this record.


## Rust contracts

Rust assertions and native observations are separate. No Rust execution result is inferred from this record.

- `var_resolve::byte_cell_identity_tests::retained_cell_key_uses_exact_bytes_independently_of_display` in `rust/tcl-compiler/src/var_resolve.rs`: Distinct exact counted cell bytes and lifetime/owner axes remain distinct despite identical display. (linked).

- `var_resolve::original_bytes::tests::counted_runtime_roots_indices_aliases_and_observers_share_one_cell_owner` in `rust/tcl-compiler/src/var_resolve/original_bytes.rs`: Counted root/index and typed alias/observer identity share the selected byte owner. (linked).

- `var_resolve::original_bytes::tests::counted_runtime_input_rejects_missing_or_conflicting_recipe_and_preserves_extent` in `rust/tcl-compiler/src/var_resolve/original_bytes.rs`: Missing/conflicting execution recipe remains unknown and counted input extent is preserved. (linked).


