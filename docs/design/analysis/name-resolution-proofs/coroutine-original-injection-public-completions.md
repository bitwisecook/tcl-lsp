# naming.coroutine.original-injection-public-completions

Kind: `native-observation`

## Problem statement

Queued coroutine injections need public completion codes and values together with the exact reached callback log. A result byte equal to a success value cannot establish that an Error branch occurred or was replaced.

## Question

For the original yield/yieldto controls, what public callback order, suspension kind, passed value and terminal or replaced Error completion are observed?

## Conclusion

C9.0.4/C9.1.0 report the original success and error-labelled two-injection controls as caught code0/result A with newest B observing ORIGINAL followed by older A observing B; their logs retain yield or yieldto respectively. These identical B bytes alone cannot distinguish B success from B Error. The separately marked terminal controls catch code1/result ERROR_B and record the reached branch-B-error marker. Their older-A replacement controls catch code0/result A, record that same reached error marker, and show A receiving ERROR_B. C8.4/C8.5/C8.6/Jim return the original UNAVAILABLE branch for all eight controls. All 48 fresh original source processes and twelve compilations exit 0 with empty stderr. These complete public logs/caught values distinguish terminal from replaced Error without object/header identity, callback frame, compiler/cache/admission, arbitrary reentrancy or Native Normal authority. BIG-IP and Rust assertions are not tested. The shared NativeCoroutineInjectionProtocol selects the finite C9.0/C9.1 public chronology separately from CPP, object transport and frame protocols. Its pure scheduler consumes pending callbacks newest-first and lets an older callback replace the completion after an error; suspend kind follows the selected resume arity. The linked Runtime control uses all eight untouched original/error sources and the complete caught completion/result/log values for both C9 providers (16 comparisons). These software controls grant no additional native process, private header, callback purity, frame or Normal authority.

## Scope

Two independently retained four-control source groups run exact ASCII LF originals in fresh full provider initialisation through the unchanged counted C API driver, with no added outer source envelope. Their catches and explicit reached-error markers are original source inputs. The original four inputs and independent marked error inputs stay collision-free under original/ and errors/. Whole original requests/sources/driver/launchers/ELFs/commands/receipts/stdout/stderr and required provider/source/header/archive/build pins remain exact. Public suspension-kind/callback/value observations supply no private frame/token or abstract interpreter identity.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

All eight original availability guards return UNAVAILABLE with driver ORIGINAL code0. No injection callback, suspension or terminal/replacement Error branch is reached; a different coroutine API is not substituted.

### tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

All eight ORIGINAL driver codes are0. Original yield controls: `0 A {{B yield ORIGINAL} {A yield B}}`; yieldto uses the corresponding kind, for both success/error-labelled inputs. Distinct marked terminal results are `1 ERROR_B` with reached `{branch B error}`; older-A replacements are `0 A` with that marker and A receiving ERROR_B. Complete original rows are retained, with terminal and replaced Error distinguished by caught code/value and marker together.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs), `NativeCoroutineInjectionProtocol`: Select finite C9.0/C9.1 public chronology independently of private object/frame protocols and fold pending completions newest-first.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `coroinject_cmd`: Use selected public injection chronology and current callback result route without expanding object/frame admission.
- [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs), `native_coroutine_compilation::injection_tests::original_injection_chronology_replaces_errors_and_retains_suspend_kind` (linked): Pure selected C9.0/C9.1 newest-first scheduler, replacement after an error and exact yield/yieldto kind; C8.x, Logical and Jim recipe selection refuses. Native public and private object observations remain independent.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `cmd_coro::tests::original_injection_public_completions_match_both_c9_releases` (linked): All eight untouched official original/error sources and both C9.0/C9.1 whole public columns retain caught code/result/log/kind values (16 Runtime comparisons); terminal/replacement controls remain separate from original identical-B outcomes.

## Evidence

- [rust/tcl-registry/tests/data/native_coroutine_public_injection255/errors](../../../../rust/tcl-registry/tests/data/native_coroutine_public_injection255/errors)
- [rust/tcl-registry/tests/data/native_coroutine_public_injection255](../../../../rust/tcl-registry/tests/data/native_coroutine_public_injection255)
- [rust/tcl-registry/tests/data/native_coroutine_public_injection255/original](../../../../rust/tcl-registry/tests/data/native_coroutine_public_injection255/original)
