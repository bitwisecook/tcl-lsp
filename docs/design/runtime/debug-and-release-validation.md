# Debug and release validation

Both engines — the bytecode VM (`rust/tcl-vm`) and the runtime
(`runtime/rust`) — wrap each native operation in an entered scope that
captures its interpreter, host, pinned context, profile, frame and namespace.
Every stage of the operation can then re-prove that the world it entered is
still current, and refuse rather than continue against a changed one.

Those re-proofs exist to catch invariant violations: a callback or host call
that changed the world under a running operation. Ordinary scripts never fail
them, yet they ran on every variable access, name projection and frame exit.
Each engine therefore has one switch:

```rust
pub(crate) const VERIFY_CURRENCY: bool = cfg!(debug_assertions);
```

| Build | `debug_assertions` | Behaviour |
| --- | --- | --- |
| Debug (`cargo build`, `cargo test`, CI test shards) | on | Every currency, frame, namespace and object-header re-proof runs as before. |
| Release (`--release`, shipped binaries) | off | Re-proofs are skipped. A retained execution refusal still stops the operation at its next check. |

## What release keeps

- **Refusal propagation.** A host or execution refusal recorded on the
  interpreter is still returned by every check, so a refused host command
  still unwinds.
- **Structural selection.** Scope entry, inherited currency, saved execution
  targets and their cleanup attachments are unchanged. Only the comparisons
  that re-prove an unchanged world are gated.
- **Semantic freshness.** Bytecode staleness after a profile, compiler, policy
  or namespace change, command-binding epochs and compiled-selection
  prerequisites are not gated: they decide what runs, not whether an
  invariant held. A retained procedure body's command and procedure bindings
  are memoised instead: release trusts them while the binding stamp (a binding
  epoch advanced by every command, trace, namespace-resolver, compiler-cache
  and ensemble-export change, with the command and trace epochs, current
  interpreter and profile and compiler generations) is unchanged since they
  last held. Debug builds re-prove them on every call and assert that the
  memo agreed, so a mutation path that skips the epoch fails the debug
  suite.

## What release skips

| Engine | Gate | Re-proof skipped |
| --- | --- | --- |
| VM | `NativeOperationCurrency::check_saved_target_at_generation` | owner, generation, pinned context, profile, engine and host identity |
| VM | `SavedExecutionTarget::check_current` | target ownership and current interpreter |
| VM | `OriginalNameConsumerState::check_using` | captured world, namespace owners, frame activation, namespace context |
| VM | `OriginalVariableCleanup::check` | physical frame, level, namespace depth and activation |
| VM | `OriginalNumericOps::current_for_value_native` | object header currentness |
| Runtime | `NativeOperationCurrency::ensure_current` | epoch, host, pinned context, profile and release |
| Runtime | `Interp::check_entered_native_operation`, inherited `NativeOperationCurrency::issue` | re-check of the interpreter's own entered currency after its refusal check |
| Runtime | `NativeOperationScope::enter` | re-check of a just-issued currency |
| Runtime | `NativeOriginalNameCell::check` | selected variable cell and table identity |
| VM | `activation_namespace_stale_message_in_context` | frame namespace ownership and path while the name-world generation, namespace token and actual profile are unchanged |
| VM | `lookup_resolved_command_in_context` | second resolution of a dispatch head already resolved under unchanged command, binding, trace and name-world epochs |
| VM | `SavedExecutionTarget::settle_in_original` | re-entry and re-proof around cleanup whose currency is already the current interpreter's innermost live entry |

In release, an operation whose world changes under it continues against the
changed world instead of refusing, as C Tcl does. Tests that assert those
refusals are therefore debug-only; they run in every debug test build.

## Keeping debug re-proofs pure

A re-proof is an assertion about the world, not a second semantics. Where
release reaches a defined result, debug must reach the same one: the same
completion, refusal, first cause and stage counts. Three rules keep it so.

- **Release failures come first.** A stage that reports a failure release
  reaches (a failed 8.4 host integer formatter) runs its stage checks inside
  `crate::with_release_checks`, so a debug re-proof cannot retain its own
  refusal ahead of that failure. Every gate above reads
  `crate::verify_currency()`, which is `VERIFY_CURRENCY` outside that scope.
- **Re-prove the interpreter the work runs in.** Original cleanup runs in its
  saved interpreter even when another one selected it (OO teardown from a
  child), so `OriginalOoCleanup::in_original` and
  `SavedExecutionTarget::settle_in_original` re-prove against that target,
  not the current selection.
- **Leave semantic changes to freshness.** A compile-service change is not an
  invariant violation: bytecode freshness reports `cannot continue bytecode
  after compile service changed` where bytecode continues, and other work
  carries on. The saved-target re-proof therefore accepts a currency whose
  interpreter's compiler generation has moved since capture.

## Shared entries

Independently of the switch, nested entry reuses work that is identical by
construction. A nested runtime scope shares its inherited currency instead of
copying it. A nested VM scope, or a saved cleanup whose currency has the same
issue as the innermost live entry, reuses that entry when both it and the
innermost allocator are unpooled and the engine is not Jim, since a fresh
unpooled entry would shadow nothing.

## Adding a gate

Gate only a comparison that re-proves state an ordinary script cannot change
mid-operation, and keep its retained-refusal check ahead of the gate. Read
`crate::verify_currency()` rather than the constant, and add the gate to the
table above.
