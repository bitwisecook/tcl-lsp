# naming.variable.copied-prefix-storage-removal-report-evaluation

Kind: `native-observation`

## Problem statement

A trace prefix containing a counted zero may have different storage, removal, reporting and execution extents. Reusing one byte equality or retaining the original prefix object can make removal or later callback arguments differ from the actual interpreter. The public object-vector inputs distinguish a raw zero from encoded C080, equal versus unequal counted lengths, and mutation of the caller-owned prefix after registration.

## Question

Does variable trace registration copy a counted raw-zero prefix, and do removal, reporting and callback evaluation use the same extent?

## Conclusion

All five tested C releases copy the full prefix without retaining its input object, report only watch A, and evaluate a callback whose first argument is the full counted A00X. A same-length A00Y prefix removes it; a longer A00YY or encoded C080 prefix leaves it registered. C8.4 uses legacy trace syntax and reports callback operation w; C8.5 through C9.1 use the modern form and report write. Current pinned Jim rejects trace. These observations establish four distinct extents for the exact selected variable trace purpose.

## Scope

Public object-vector trace registration/removal, fresh interpreters, an original nine-byte prefix watch<space>A00X, public prefix-object mutation after registration, ASCII top-level source, and direct callback argv byte observation. Callback argument rendering is observed through its counted public string getter. This does not measure command/execution trace removal, callback bytecode cache, arbitrary object class, variable cell grants or BIG-IP trace behavior.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded header 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf, static library 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47 and executed probe 74317b59af81d67da0b604d415dde72a17742fd39b47e17923b93a33f2a5b6b0; original compiler arguments retained.. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability.. Dialect: Tcl.

Registration leaves the input at refCount1; changing it does not change copied reporting or callback A00X. Same-length raw-zero removal leaves0 registrations; longer and C080 negatives each leave1.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded header c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5, static library 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc and executed probe 08acee8e6390b5fef87d251212dce23e61dea32aac32c211cb41132574ac57ff; original compiler arguments retained.. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability.. Dialect: Tcl.

Registration leaves the input at refCount1; changing it does not change copied reporting or callback A00X. Same-length raw-zero removal leaves0 registrations; longer and C080 negatives each leave1.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded header aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245, static library 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb and executed probe 7a656de6f30186ca48979345770f1c1ddbe5c566e69e2101645a671ae0d04085; original compiler arguments retained.. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability.. Dialect: Tcl.

Registration leaves the input at refCount1; changing it does not change copied reporting or callback A00X. Same-length raw-zero removal leaves0 registrations; longer and C080 negatives each leave1.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded header eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a, static library dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4 and executed probe 338d45ea1d8f12d29f8c1877ff6eabec846e7b0df365e7e85de6fbc79b19700e; original compiler arguments retained.. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability.. Dialect: Tcl.

Registration leaves the input at refCount1; changing it does not change copied reporting or callback A00X. Same-length raw-zero removal leaves0 registrations; longer and C080 negatives each leave1.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded header 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950, static library 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db and executed probe 6cc9406689540b77d53f11aa8ad1712243eb28da47273fbdd362c38693357718; original compiler arguments retained.. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability.. Dialect: Tcl.

Registration leaves the input at refCount1; changing it does not change copied reporting or callback A00X. Same-length raw-zero removal leaves0 registrations; longer and C080 negatives each leave1.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Recorded header d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d, static library a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da and executed probe 7e42c9d6e403da9fc893387c28099c1968f4078902550efa65ad236cc18c300a; original compiler arguments retained.. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability.. Dialect: Jim Tcl.

Guest Error: invalid command name "trace"; no prefix law is measured.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (input): [rust/tcl-registry/tests/data/native_variable_trace_prefix/probe.c](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/probe.c). SHA-256 `4bf1ba2f34197a0a113a47ae3898bfd700c32cdd4983d1f770591d42c68cb817`. Exact public C/Jim API program; all raw-zero and encoded-zero arrays and three discriminating removal controls retained.
- `e1` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/receipt.json). SHA-256 `338bcd94c04983c5ec26d7a07f4f920af04a56dfd15092a6293473e0d332bf43`. Complete six-provider compile/process attribution, header/library/executable/source hashes and exact native rows.
- `e2` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.4.20/stdout.tsv). SHA-256 `bba528bfb2340df2b52fa2a18e050710190e32befdb5555eaa209e6876863b09`. Exact original stdout: expected native guest failures remain rows.
- `e3` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.4.20/receipt.json). SHA-256 `576d836878ca8c7e7e0f058716ba7a3c9cba27bd0b49158e8b4a858f141a6cef`. This provider compile/process result and original input/build/stream hashes.
- `e4` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.5.19/stdout.tsv). SHA-256 `3b8012378ffba88fc693acc63104556fe5fd8f1a1dcc7773f8224e761234abed`. Exact original stdout: expected native guest failures remain rows.
- `e5` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.5.19/receipt.json). SHA-256 `da557a2f9582e9b29ae97200b8ce877de326d8f697c3f32cdaed0d8043af5787`. This provider compile/process result and original input/build/stream hashes.
- `e6` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.6.18/stdout.tsv). SHA-256 `3b8012378ffba88fc693acc63104556fe5fd8f1a1dcc7773f8224e761234abed`. Exact original stdout: expected native guest failures remain rows.
- `e7` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.6.18/receipt.json). SHA-256 `9d9a96ee4abe6b237b36dcf072db04ce0221a283263a6357ad953ed8c6c2864b`. This provider compile/process result and original input/build/stream hashes.
- `e8` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/9.0.4/stdout.tsv). SHA-256 `3b8012378ffba88fc693acc63104556fe5fd8f1a1dcc7773f8224e761234abed`. Exact original stdout: expected native guest failures remain rows.
- `e9` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/9.0.4/receipt.json). SHA-256 `591f794aa040166e0a10847579b1a992ae5e8b440dbab69c177df8fccb749f0b`. This provider compile/process result and original input/build/stream hashes.
- `e10` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/9.1.0/stdout.tsv). SHA-256 `3b8012378ffba88fc693acc63104556fe5fd8f1a1dcc7773f8224e761234abed`. Exact original stdout: expected native guest failures remain rows.
- `e11` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/9.1.0/receipt.json). SHA-256 `5a14a0abeddc373ece727615ed4dc364d3a2e49a355dc3c4ae0e9488928d5e21`. This provider compile/process result and original input/build/stream hashes.
- `e12` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/jim/stdout.tsv). SHA-256 `7f018cd7c2761ed94f734a96126881c684788be92346dc8750a8a87479c6dc93`. Exact original stdout: expected native guest failures remain rows.
- `e13` (observation): [rust/tcl-registry/tests/data/native_variable_trace_prefix/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/jim/receipt.json). SHA-256 `982f12b94dbecd9335b64f38ecc2ef172d4a3023e35eb572c2d94c586e29e3d7`. This provider compile/process result and original input/build/stream hashes.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_prefix_storage`: Selected immutable counted extent, independently of registration and cell.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_prefix_matches`: Independent length-plus-strncmp removal selector.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_prefix_report`: CString result projection.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_callback_source`: Counted assembled script extent.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `native_variable_trace::tests::variable_prefix_copy_remove_report_and_counted_eval_keep_independent_extents` (linked): Check same-length strncmp match, unequal length and C080 negative, copied byte extent and independent report/callback extents.
- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls` (linked): Register original object, mutate input, fire counted copied argument and compare all three removal outcomes.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls` (linked): Copied prefix survives original representation invalidation, reports CString and fires counted argument with independent removal negatives.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `variable_bindings::tests::original_variable_trace_copies_producers_and_removes_by_selected_native_purpose` (linked): Checks compiler copied original prefix lineage and selected same-length native removal purpose; Rust lifetime ownership remains an independent obligation rather than a native trace callback guarantee.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_variable_trace_prefix/replay.py",
  "--tcl-source-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-source-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

Exact original probe/header/static library hashes required; independently configured Jim static extensions and C library trees must match. A fresh output directory is required. Rebuilds retain their own executable SHA and compare exact process exit/stdout/stderr, including unsupported Jim. Run serially or with global native concurrency at most two; each process has a60-second budget. This command reconfirms the oracle, not Rust parity.
