# naming.list.original-value-append-storage-currency

Kind: `implementation-contract`

## Problem statement

Appending through the generic value interface must preserve the selected original storage producer for the next native mutation. Rebuilding equal list elements after an unavailable storage receipt would replace that authority with a new value.

## Question

When may the value interface append to retained selected list storage, and why must unavailable conversion/storage remain a typed refusal rather than a fresh-list fallback?

## Conclusion

ValueOps::try_list_append_in_place returns Result<bool,ValueError>. Its default Ok(false) permits the ordinary model rebuild; the Runtime override first checks both original objects for liveness and returns Ok(false) only for shared receiver storage. The actual invocation dialect selects the shared string/list materialisation protocol, including independently required Jim object context. append_prepared_native_elements consumes that selected original allocation extent and preserves its capacity/producer for subsequent native mutation. A missing selected protocol, unavailable allocation receipt, conversion or liveness failure stays Err. lappend_value propagates it before the ordinary false-result rebuild. Equal element values cannot recover missing storage authority. The fixed C9 recipe controls test interoperability of the value-interface append with the next selected mutation and deliberate receipt-loss refusal; they are not observations of a native allocator, refcount, header, frame or external interpreter.

## Scope

Two fixed Runtime implementation controls use authored selected C9.0/C9.1 list producers and an explicitly removed allocation receipt. They compare the retained Runtime receiver/members and refusal without invoking an external native provider. All seven providers are not tested; no independent native allocation/header/current-store success is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-list-original-value-append-storage-currency-list.rs` (implementation): [runtime/rust/src/list.rs](../../../../runtime/rust/src/list.rs). SHA-256 `58fe5a4d3b3a11b7ffecf2dde82a3737bbb64a6d35410d0db14dd5af2267943c`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-list-original-value-append-storage-currency-value_ops.rs` (implementation): [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs). SHA-256 `e46c5b1b283ffc6d8fb754f6f210bbb2063f89875f21419ecc2ce297476639e7`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-list-original-value-append-storage-currency-var.rs` (implementation): [rust/tcl-cmd-core/src/var.rs](../../../../rust/tcl-cmd-core/src/var.rs). SHA-256 `59eddbc76d5849eebdfe263c8e349798dc3114fbdea163823b3b2485decd8e31`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-list-original-value-append-storage-currency-value.rs` (implementation): [rust/tcl-syntax/src/value.rs](../../../../rust/tcl-syntax/src/value.rs). SHA-256 `d412e3a6736721b78a27c805245568689617faf248811acf8a046c6e556ed037`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/value.rs](../../../../rust/tcl-syntax/src/value.rs), `ValueOps::try_list_append_in_place`: Separate ordinary no-in-place capability from typed selected storage/conversion refusal.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `try_list_append_in_place`: Preserve actual selected original list storage and protocol; unavailable storage cannot create a fresh donor.
- [runtime/rust/src/list.rs](../../../../runtime/rust/src/list.rs), `append_prepared_native_elements`: Consume existing selected list allocation/capacity producer for prepared original members.
- [rust/tcl-cmd-core/src/var.rs](../../../../rust/tcl-cmd-core/src/var.rs), `lappend_value`: Propagate typed refusal before ordinary false-capability rebuild; retain already appended member prefix.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `value_ops::tests::original_value_append_preserves_storage_for_the_next_native_mutation` (linked): The authored selected C9 list producer remains compatible with a subsequent shared original mutation and retains the same Runtime receiver/member handles; no native allocator observation.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `value_ops::tests::original_value_append_does_not_rebuild_after_unavailable_storage` (linked): Deliberate allocation-receipt loss yields the typed native allocation-extent refusal with the original Runtime receiver/member vector unchanged; equal values cannot recover storage authority.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
