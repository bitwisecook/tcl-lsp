# naming.namespace.procedure-call-level-context

Kind: `native-observation`

## Problem statement

A namespace-qualified procedure can be called while namespace eval remains active or after it returns. Using the namespace spelling alone to choose a procedure call-level expectation confuses these two actual script contexts; printed levels must remain separate from physical frame and Host metadata claims.

## Question

For the exact original script, what public info level does the same procedure report when called inside namespace eval and when called directly by its namespace-qualified name after namespace eval returns?

## Conclusion

All five measured Tcl releases and the measured Jim build close the original script exit0 with empty stderr and print NESTED|2 then DIRECT|1. The namespace-eval call context therefore changes this script-visible procedure level by one in these six finite observations. These printed values establish neither native frame memory/layout, Host callback/refusal metadata nor a Rust test outcome. BIG-IP is not tested. Two source-bound software controls remain unrun.

## Scope

Exact three-line ordinary stdin script and two integer observations per recorded CLI process: Tcl8.4.20,8.5.19,8.6.18,9.0.4,9.1.0 and Jim0.84-9-g5bac7c9. VERSION, stdout, empty stderr, original exit/duration and invocation-file pins are retained independently. Five C pins identify shell wrappers; downstream C ELF/library closure is unrecorded. Jim invocation ELF bytes round-trip exactly, but shared dependencies and full environment/build replay are unrecorded. No physical frame/header/pointer layout, interpreter/Scope ownership, Host metadata ABI, exceptional completion/catch behavior, worker admission, TMM or implementation equivalence is measured. The two current Rust controls supply source coverage only.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr. Version: tcl8.4 8.4.20.

This original 8.4.20 process closes exit0 with empty stderr and reports NESTED|2 for the procedure called inside namespace eval, then DIRECT|1 for the same namespace-qualified procedure called after namespace eval returns. These two values do not measure physical frame layout, Host failure metadata or a Rust implementation outcome.

### tcl8.5

Status: `observed`. Channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr. Version: tcl8.5 8.5.19.

This original 8.5.19 process closes exit0 with empty stderr and reports NESTED|2 for the procedure called inside namespace eval, then DIRECT|1 for the same namespace-qualified procedure called after namespace eval returns. These two values do not measure physical frame layout, Host failure metadata or a Rust implementation outcome.

### tcl8.6

Status: `observed`. Channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr. Version: tcl8.6 8.6.18.

This original 8.6.18 process closes exit0 with empty stderr and reports NESTED|2 for the procedure called inside namespace eval, then DIRECT|1 for the same namespace-qualified procedure called after namespace eval returns. These two values do not measure physical frame layout, Host failure metadata or a Rust implementation outcome.

### tcl9.0

Status: `observed`. Channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr. Version: tcl9.0 9.0.4.

This original 9.0.4 process closes exit0 with empty stderr and reports NESTED|2 for the procedure called inside namespace eval, then DIRECT|1 for the same namespace-qualified procedure called after namespace eval returns. These two values do not measure physical frame layout, Host failure metadata or a Rust implementation outcome.

### tcl9.1

Status: `observed`. Channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr. Version: tcl9.1 9.1.0.

This original 9.1.0 process closes exit0 with empty stderr and reports NESTED|2 for the procedure called inside namespace eval, then DIRECT|1 for the same namespace-qualified procedure called after namespace eval returns. These two values do not measure physical frame layout, Host failure metadata or a Rust implementation outcome.

### jim

Status: `observed`. Channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr. Version: jim 0.84-9-g5bac7c9.

This original 0.84-9-g5bac7c9 process closes exit0 with empty stderr and reports NESTED|2 for the procedure called inside namespace eval, then DIRECT|1 for the same namespace-qualified procedure called after namespace eval returns. These two values do not measure physical frame layout, Host failure metadata or a Rust implementation outcome.

### bigip

Status: `not-tested`. Channel: No BIG-IP script-visible info-level capture.

No original BIG-IP response to this exact namespace/procedure call-context script is attached. Authored Runtime profiles and the six local CLI results do not establish BIG-IP/TMM frame or worker behavior.

## Consumer bindings

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::eval_completion`: Current software completion entry used by the authored namespace-qualified call. Its owning Host/Guest transport and metadata have an independent software scope; two native printed integers supply no physical frame/Host metadata correspondence.
- [runtime/rust/src/completion.rs](../../../../runtime/rust/src/completion.rs), `capture_bytes`: Current checked owning completion capture preserves the actual first typed software refusal. The native script contains no Host callback and grants no capture/layout/metadata equivalence.
- [runtime/rust/src/completion.rs](../../../../runtime/rust/src/completion.rs), `completion::tests::namespace_eval_and_direct_procedure_calls_preserve_native_level_distinction` (linked): The authored software loop selects six actual engine profiles and expects the script-visible nested/direct values 2 1. The two integers are bounded native observations; this test definition has no attached Rust assertion outcome and does not assert native frame structures or Host metadata ABI.
- [runtime/rust/src/completion.rs](../../../../runtime/rust/src/completion.rs), `completion::tests::public_completion_returns_original_reached_host_metadata_outside_catch` (linked): A namespace-qualified software procedure is invoked after namespace eval returns. Its actual first Host cause, authored frame1/namespace/profile metadata and reached-before/unreached-after effects remain an independent software contract. The original CLI probe observes no Host callback, Rust metadata, catch bypass or physical frame layout; this control is unrun.

## Evidence

- [runtime/rust/tests/data/native_namespace_call_levels](../../../../runtime/rust/tests/data/native_namespace_call_levels)
- [runtime/rust/tests/data/native_namespace_call_levels/recorded-launch-files](../../../../runtime/rust/tests/data/native_namespace_call_levels/recorded-launch-files)
- [runtime/rust](../../../../runtime/rust)
