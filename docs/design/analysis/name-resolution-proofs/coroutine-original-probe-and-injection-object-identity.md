# naming.coroutine.original-probe-and-injection-object-identity

Kind: `native-observation`

## Problem statement

Coroutine probe and injection can preserve visible bytes while replacing the original argument or returned object; yieldto resumes have a distinct list-result identity.

## Question

Do C9.1 probe and queued injection preserve the original raw-zero, C080, FF and D800 argument/result objects through opaque coroutine and callback names?

## Conclusion

C9.1 returns the exact original argument object from all eight probes. Injected callbacks receive the actual delivered object. Yield resumes return that argument pointer; yieldto resumes construct a list and therefore differ from the original argument pointer while still returning the exact callback-result object. Both kinds retain the measured counted result bytes and unchanged kind field. Pointer comparisons occur before result getters. This says nothing about foreign object hooks, private cache classes or refcount parity.

## Scope

Eight C9.1 yield/yieldto controls, four original counted string values (raw00, C080, FF, D800), opaque FF coroutine and collector names, pre-getter pointer comparisons for probe/resume/callback result and counted result bytes. No other engine exercised the C9.1 probe/inject API; earlier provider not-tested markers are not guest rejection or API availability observations. Collector observes no original header/refcount/custom method state.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

C9.1 returns the exact original argument object from all eight probes. Injected callbacks receive the actual delivered object. Yield resumes return that argument pointer; yieldto resumes construct a list and therefore differ from the original argument pointer while still returning the exact callback-result object. Both kinds retain the measured counted result bytes and unchanged kind field. Pointer comparisons occur before result getters. This says nothing about foreign object hooks, private cache classes or refcount parity.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Jim.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### bigip

Status: `not-tested`. Version: not measured. Build: not recorded. Channel: No input supplied. Dialect: BIG-IP.

No observation for this question.

## Exact evidence

- `probe-v1` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/probe.c). SHA-256 `e47227595500d3399848c7a42fe0867de33df4f25c87a8f7b765692f7ee397bb`. Exact immutable original-object v1 probe source; count constructors, pointer timing, per-stage getters and untested API branches are inspectable.
- `probe-v2` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c). SHA-256 `ab41e399af2ba7e074d779a1ee7e76fd9fe16e127c9d07f060010e2c1aae6a44`. Exact v2 probe, adding separate info-coroutine fullname controls without changing the v1 outcomes.
- `tcl9.1-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json). SHA-256 `13209de752ec751ea45bb1c8136d13959a3d54f4001b79fefce9f2f06405734f`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv). SHA-256 `94805bbf1a96f65176a2aff451195912444e292b6ceec360898ebb1204830d04`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl9.1-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json). SHA-256 `9bd5decf3fa0e8b465b817c8eef53551aa674896bfe582592011660c006300a6`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv). SHA-256 `acaa106607bb5e8cbc663efaf2d39bc08cbcf5fe754fb8bb9524e743ae3429d3`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_coro/native_injection_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_injection_tests.rs), `original_coroutine_probe_and_injection_match_counted_object_controls`: Finite original native object-vector comparisons; no Rust execution claimed.
- [runtime/rust/src/cmd_coro/native_injection_tests.rs](../../../../runtime/rust/src/cmd_coro/native_injection_tests.rs), `original_coroutine_probe_and_injection_match_counted_object_controls`: Finite original native object-vector comparisons; no Rust execution claimed.
- [rust/tcl-vm/src/cmd_coro/native_injection_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_injection_tests.rs), `cmd_coro::native_injection_tests::original_coroutine_probe_and_injection_match_counted_object_controls` (linked): 56 original-object stage comparisons and kind bytes.
- [runtime/rust/src/cmd_coro/native_injection_tests.rs](../../../../runtime/rust/src/cmd_coro/native_injection_tests.rs), `cmd_coro::native_injection_tests::original_coroutine_probe_and_injection_match_counted_object_controls` (linked): 56 original-object stage comparisons and kind bytes.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_coroutine_publication/verify.py"
]
```

Offline exact receipt/input/stream association verification only; zero native/compiler/Rust launches. Original capture.py and queue retain absolute provisioned input pins and immutable output paths; fresh recapture must use new output directories and recheck every required hash. Source windows reproduce separately from exact pinned full source LF ranges. No missing ELF is fabricated or claimed portable.
