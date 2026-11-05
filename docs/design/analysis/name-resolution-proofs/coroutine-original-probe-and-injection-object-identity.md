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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; jim 0.84-9-g5bac7c9.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl9.1

Status: `observed`. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped. Version: tcl9.1 9.1.0.

C9.1 returns the exact original argument object from all eight probes. Injected callbacks receive the actual delivered object. Yield resumes return that argument pointer; yieldto resumes construct a list and therefore differ from the original argument pointer while still returning the exact callback-result object. Both kinds retain the measured counted result bytes and unchanged kind field. Pointer comparisons occur before result getters. This says nothing about foreign object hooks, private cache classes or refcount parity.

### bigip

Status: `not-tested`. Channel: No input supplied.

No observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_coro/native_injection_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_injection_tests.rs), `original_coroutine_probe_and_injection_match_counted_object_controls`: Finite original native object-vector comparisons; no Rust execution claimed.
- [runtime/rust/src/cmd_coro/native_injection_tests.rs](../../../../runtime/rust/src/cmd_coro/native_injection_tests.rs), `original_coroutine_probe_and_injection_match_counted_object_controls`: Finite original native object-vector comparisons; no Rust execution claimed.
- [rust/tcl-vm/src/cmd_coro/native_injection_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_injection_tests.rs), `cmd_coro::native_injection_tests::original_coroutine_probe_and_injection_match_counted_object_controls` (linked): 56 original-object stage comparisons and kind bytes.
- [runtime/rust/src/cmd_coro/native_injection_tests.rs](../../../../runtime/rust/src/cmd_coro/native_injection_tests.rs), `cmd_coro::native_injection_tests::original_coroutine_probe_and_injection_match_counted_object_controls` (linked): 56 original-object stage comparisons and kind bytes.

## Evidence

- [rust/tcl-registry/tests/data/native_coroutine_publication/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/probe.c)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c)
- [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv)
- [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr)
