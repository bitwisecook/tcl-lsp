# naming.source.original-content-currency

Kind: `implementation-contract`

## Problem statement

A retained source value can outlive a cache conversion or a known overwrite of its source variable, while a live variable-read receipt must expire when that cell changes. Reusing the representation epoch for immutable data either loses valid copied bytes or revives an old producer after an unknown effect.

## Question

Which independent stamp and cell obligations keep immutable original source bytes current through cache changes, frame transfer, joins and unknown effects?

## Conclusion

OriginalProducedNameValue retains complete source producers, naming policy and an independent original-content stamp. Cache-only invalidation preserves this stamp; unknown content or observer effects advance it, and unequal or exhausted histories do not reuse an old stamp. Live reads additionally require the exact current cell, generation, origin, kind and observer closure.

An additional marked software definition binds a produced array-index value to its exact word and executable-part arena. Equal-looking concatenation or list output cannot replace that producer, invalidated contents withdraw it, and a computed index does not acquire a written-word key.

## Scope

Implementation data correspondence for independently selected C/Jim policies; no physical object, successful handler, compiler-local table, editable word or native interpreter is established. Concatenation currently admits ASCII non-NUL source data only.

The version loop selects software source grammars; it is not a new original C process comparison. These source-value carrier checks grant no reached substitution, Native header, actual array read/store, frame, completion or passing assertion.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is an implementation contract; no interpreter or appliance observation is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is an implementation contract; no interpreter or appliance observation is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is an implementation contract; no interpreter or appliance observation is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is an implementation contract; no interpreter or appliance observation is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is an implementation contract; no interpreter or appliance observation is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is an implementation contract; no interpreter or appliance observation is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is an implementation contract; no interpreter or appliance observation is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs), `OriginalProducedNameValue`: Retain complete immutable source-data producers, selected naming policy and independent content currency.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `ResolveContext::invalidate_original_contents`: Retire uncertain content histories on the independent content axis without stamp reuse.
- [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs), `command_binding::original_name_value::tests::original_contents_currency_preserves_cache_changes_and_retires_unknown_effects` (linked): Verify the stated implementation boundary and its discriminating rejection cases; an actual test run is independently required.
- [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs), `command_binding::original_name_value::tests::original_content_joins_keep_every_origin_and_do_not_reuse_unknown_currency` (linked): Verify the stated implementation boundary and its discriminating rejection cases; an actual test run is independently required.
- [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs), `command_binding::original_name_value::tests::original_child_lineage_is_bounded_across_repeated_capture` (linked): Repeated readonly list-child capture retains exact bytes and current lineage only within a finite complete nested graph budget; it declines before unbounded child/store chains form.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs), `OriginalProducedNameValue::evaluated_variable_index`: Require exact original word/arena, current independent contents and the held value naming policy before projecting an evaluated source index.
- [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs), `command_binding::original_name_value::tests::original_evaluated_index_keeps_its_exact_word_and_arena` (linked): An evaluated source array index retains its exact original NativeWord, executable part arena and held producer value. Wrong word, concatenation/list-result replacement or invalidated contents refuse; joining the same held producer preserves the index without inventing an original written-word key.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Run the exact named Rust selectors in the relevant crate. Policy enumeration is not native interpreter execution; no performance pass is claimed.
