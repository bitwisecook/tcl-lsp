# Variable trace outcomes

The shared trace coordinator returns `Result<VariableTraceGuestOutcome, NativeExecutionError>`. `Complete` means no callback Guest error aborts the selected operation. `GuestError` means the actual callback populated its pending Guest error. The outer error is the full original Host refusal. A callback error, stale entered operation and unavailable protocol are different outcomes; consumers cannot substitute a message or a protocol label for the first typed cause.

`fire_var_trace_resolved_with_errors` retains the actual `NativeOperationScope` through callbacks and mandatory trace cleanup. It inherits an already entered caller and otherwise enters the genuine current interpreter. That scope checks currency and grants no operand, name, grammar, frame or command protocol. Both hidden trace stages and each user callback are checked before any later stage. A changed-and-restored context cannot revive the original operation.

Read, write, array, existence, dictionary, append and increment consumers handle the outer error before reading a later value, formatting a Guest variable error or publishing a result. Read-for-update operations may swallow a genuine Guest read error where their selected protocol requires it. They never swallow an outer Host refusal. The `Code` and `VarError` adapters retain the complete original cause on the actual interpreter before projecting their unwind carrier; that carrier is not a Guest error message.

The public `Traces` facade returns `Result<Result<(), Value>, NativeExecutionError>`. Its Unicode `fire` method delegates to the same checked `fire_bytes` implementation. Consumers handle the outer result first, then the callback's inner Guest result. Runtime Guest error values are fresh objects and require the caller's usual adoption or release. VM values retain their existing ownership. No lossy `ValueError` conversion or Guest-only compatibility route replaces the full Host result.

```rust
let guest = runtime.fire_bytes(original_name, "read")?;
match guest {
    Ok(()) => continue_the_original_access(),
    Err(value) => handle_the_actual_guest_trace_error(value),
}
```

The original trace cell, active trace scopes and saved result owner remain selected across callback cleanup. A Host refusal stops later callbacks and Guest formatting while preserving reached callback effects. Restoration and retirement can themselves reach extension code, so the coordinator checks the original cause after mandatory cleanup. Tcl 8.4's explicit error-code setter still settles its actual code-set flag after its write callback; it does not convert that callback's Host refusal into a Guest result.

The [embedding proof](../analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md) separates Host publication and transport from external provider execution. Software controls check the exact typed payload, changed-and-restored context, later-callback suppression, original result identity, reached storage and completed cleanup. Those controls do not establish native Tcl or BIG-IP callback chronology.
