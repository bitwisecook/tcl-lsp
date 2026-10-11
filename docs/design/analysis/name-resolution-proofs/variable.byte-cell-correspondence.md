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

Software original variable projections retain Document units, producer kind, exact alias/namespace byte-cell targets and missing-owner refusal. Independent before-store kind, observer, old-class and currency requirements remain explicit.

## Scope

Compiler typed cell/index/alias transport and runtime input facade only. No native frame/header/compiled-local slot, value bytes, normal completion, BIG-IP broadcast or object release proof follows.

Current direct test attributes and real module parents attach these source/API contracts only. They do not certify an executable inventory, passed assertion, complete Native comparison or changed-current-image result. Each original provider answer and independently closed software command retains its exact input/channel/source purpose.

## Exact retained evidence

- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `analyser::scope::tests::qualified_variable_inputs_keep_document_units_and_producer_kind` (linked): Actual source analysis keeps Document unit conversion and independent variable-root/value producer kinds, including counted zero spelling and retained namespace geometry; it does not equate display with a runtime cell.
- [rust/tcl-compiler/src/var_resolve/original_bytes.rs](../../../../rust/tcl-compiler/src/var_resolve/original_bytes.rs), `var_resolve::original_bytes::tests::compiler_primary_retains_authored_unknown_links_and_exact_namespace_targets` (linked): The authored software compiled-variable model keeps unknown links Unknown and explicit alias/global namespace targets at their exact byte-cell keys; independent Native variable execution is not observed here.
- [rust/tcl-compiler/src/var_resolve/original_name_write.rs](../../../../rust/tcl-compiler/src/var_resolve/original_name_write.rs), `var_resolve::original_name_write::tests::before_store_completion_requires_kind_observers_and_old_class_independently` (linked): The software before-store query independently rechecks original receiver kind, observer footprint, old class and currency: known text, prior write, dynamic traces and scalar/array conflict withhold its modelled completion.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `variable_bindings::tests::original_alias_operands_ignore_display_and_preserve_runtime_units` (linked): Software alias binding consumes retained original word units and independently selected name policy, preserves NUL/surrogate cell keys and refuses missing original input; a conflicting display transition does not select the cell.

Current source linkage supplies no execution or assertion credit.

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


