# exists read trace quiet result purpose

ID: `naming.variable.exists-read-trace-quiet-result-purpose`

## Problem statement

A variable read trace can create a value or fail, but info exists deliberately has a quiet existence-result route. Borrowing that Boolean completion to justify an ordinary read or write would lose the trace failure and actual value/container obligations.

## Question

What original completion, result object/cache fields and opcode presence does info exists produce after a creating trace, failing scalar trace and failing local-array-element trace?

## Answers

### tcl8.4 — observed

Compile/run exit 0; all three completions are code 0, result hex 31/30/31 (1/0/1).

```text
R|0|0|int|0|1|31|
R|1|0|int|0|1|30|
R|2|0|int|0|1|31|
```

Version: 8.4.20. Build: Archive SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; probe SHA 9cc49e018c03d1e55f18f97b37fc81b9c51b8bec39723a39aff6614a5afefa01. Input channel: Exact original source via private compiled procedure artifact.

### tcl8.5 — observed

Compile/run exit 0; all three completions are code 0, result hex 31/30/31 (1/0/1).

```text
R|0|0|int|0|2|31|upvar:4,existScalar:4,
R|1|0|int|0|2|30|upvar:4,existScalar:4,
R|2|0|int|1|2|31|upvar:4,existScalar:4,
```

Version: 8.5.19. Build: Archive SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe SHA 609d5699ba087372ea1305aa6243602127f2552bd745b7261713db917392f2d7. Input channel: Exact original source via private compiled procedure artifact.

### tcl8.6 — observed

Compile/run exit 0; all three completions are code 0, result hex 31/30/31 (1/0/1).

```text
R|0|0|int|0|2|31|upvar:4,existScalar:4,
R|1|0|int|0|2|30|upvar:4,existScalar:4,
R|2|0|int|1|2|31|upvar:4,existScalar:4,
```

Version: 8.6.18. Build: Archive SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe SHA 1d030c30ea3e98df6e17ad845707194014af08469a51f5dc19b248e545813146. Input channel: Exact original source via private compiled procedure artifact.

### tcl9.0 — observed

Compile/run exit 0; all three completions are code 0, result hex 31/30/31 (1/0/1).

```text
R|0|0|int|0|2|31|upvar:4,existScalar:4,
R|1|0|int|0|2|30|upvar:4,existScalar:4,
R|2|0|int|1|2|31|upvar:4,existScalar:4,
```

Version: 9.0.4. Build: Archive SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe SHA 02205a1077a30d6cfad12368cc37325a67639e7126cb007052be1f9741d2af59. Input channel: Exact original source via private compiled procedure artifact.

### tcl9.1 — observed

Compile/run exit 0; all three completions are code 0, result hex 31/30/31 (1/0/1).

```text
R|0|0|int|0|2|31|upvar:4,existScalar:4,
R|1|0|int|0|2|30|upvar:4,existScalar:4,
R|2|0|int|1|2|31|upvar:4,existScalar:4,
```

Version: 9.1.0. Build: Archive SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe SHA e66c69aa8a0f37ea93eea0ba2fe6b528c1397e43240e84849d43cbb24d7716aa. Input channel: Exact original source via private compiled procedure artifact.

### jim — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

## Conclusion

Every measured C provider completes these three original exists probes normally with results 1, 0 and 1 respectively. Modern compiler opcode presence and result cache/refcount fields remain separately recorded. This quiet Boolean result purpose does not certify an ordinary read, suppress all callbacks, prove returned byte values or close a write operation.

## Scope

Fifteen original windows in the retained modern trace-add probe/cases, C8.4.20–9.1.0, private compiled procedures plus original native objects. Exact three trace bodies and aliases only. Jim/BIG-IP not queried; no unsupported classification is inferred from their absence.

## Rust comparisons

- `interp::native_body_artifact::native_upvar_info_exists_tests::original_exists_quiet_trace_semantics_preserve_15_native_result_windows` in `runtime/rust/src/interp/native_body_artifact/native_upvar_info_exists_tests.rs`: Checks quiet Boolean existence semantics as an independent operation purpose. No execution result is recorded here.
- `interp::native_upvar_info_exists_tests::original_exists_quiet_trace_semantics_preserve_15_native_result_windows` in `rust/tcl-vm/src/interp/native_upvar_info_exists_tests.rs`: Checks every retained VM result window. No execution result is recorded here.
