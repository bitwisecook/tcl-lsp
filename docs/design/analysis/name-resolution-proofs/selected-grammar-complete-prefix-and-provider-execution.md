# naming.source.selected-grammar-complete-prefix-and-provider-execution

Kind: `native-observation`

## Problem statement

A selected source grammar can retain a complete command prefix while the same whole source reports incomplete or raises a terminal parse error. Treating info complete as successful parsing or as permission to enter every prefix conflates source analysis with the actual provider operation. These two original inputs expose release-specific braced-variable grammar and different reached prefix effects.

## Question

Does the selected release consider the malformed braced-variable source incomplete, and which completed prefix/suffix effects occur when that same original source is evaluated? Does a terminal word-weld parse error remain complete while suppressing its suffix?

## Conclusion

For the exact braced-variable source, Tcl8.4.20/8.5.19/8.6.18 and Jim0.84-9-g5bac7c9 report complete and evaluate successfully, leaving before=1 and after=2. Tcl9.0.4/9.1.0 report incomplete, return missing close-brace for variable name, leave before=1 and leave after absent. Every provider reports the terminal word-weld source complete and returns extra characters after close-brace with after absent. The five C providers leave before=1 in that case; the recorded Jim process leaves before absent. Completeness, exact selected grammar and these reached script effects therefore require separate facts. Neither the complete flag nor the retained source prefix supplies a compiler, binding or Native execution grant. BIG-IP is not tested.

## Scope

Two exact counted ASCII script values under the retained public info complete query, then public uplevel #0 evaluation and separate post-evaluation before/after variable reads. Six actual CLI processes and twelve ROWs are retained: five measured C releases and the measured Jim build. Every process exits0 with empty stderr; catch reports each script outcome independently. VERSION, extra VALUE output, counted source bytes, exact receipts, invoked ELF and captured Tcl library .tcl input pins remain distinct. Loaded shared-library identity, full SDK/build replay, internal parser strategy, compiled bytecode, original headers, lifetimes, callbacks, frames, Host refusal metadata, compiler-purpose admission, current Rust assertions and TMM are unmeasured. These two cases establish no general Jim command-prefix effect rule.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual invoked ELF whole bytes and printed VERSION are retained independently. Canonical existing payload matches the exact whole executable hash. Captured Tcl library .tcl inputs retain before/after pins and exact byte payloads; configure/compiler/link and loaded shared-library closure are unrecorded.. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.. Dialect: tcl8.4.

The original process exits0 with empty stderr. The braced-variable case reports complete=1, script catch=0, before=1 and after=2. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual invoked ELF whole bytes and printed VERSION are retained independently. Canonical existing payload matches the exact whole executable hash. Captured Tcl library .tcl inputs retain before/after pins and exact byte payloads; configure/compiler/link and loaded shared-library closure are unrecorded.. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.. Dialect: tcl8.5.

The original process exits0 with empty stderr. The braced-variable case reports complete=1, script catch=0, before=1 and after=2. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual invoked ELF whole bytes and printed VERSION are retained independently. Canonical existing payload matches the exact whole executable hash. Captured Tcl library .tcl inputs retain before/after pins and exact byte payloads; configure/compiler/link and loaded shared-library closure are unrecorded.. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.. Dialect: tcl8.6.

The original process exits0 with empty stderr. The braced-variable case reports complete=1, script catch=0, before=1 and after=2. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual invoked ELF whole bytes and printed VERSION are retained independently. Canonical existing payload matches the exact whole executable hash. Captured Tcl library .tcl inputs retain before/after pins and exact byte payloads; configure/compiler/link and loaded shared-library closure are unrecorded.. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.. Dialect: tcl9.0.

The original process exits0 with empty stderr. The braced-variable case reports complete=0, script catch=1, before=1 and after=MISSING. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual invoked ELF whole bytes and printed VERSION are retained independently. Canonical existing payload matches the exact whole executable hash. Captured Tcl library .tcl inputs retain before/after pins and exact byte payloads; configure/compiler/link and loaded shared-library closure are unrecorded.. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.. Dialect: tcl9.1.

The original process exits0 with empty stderr. The braced-variable case reports complete=0, script catch=1, before=1 and after=MISSING. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual invoked ELF whole bytes and printed VERSION are retained independently. Canonical existing payload matches the exact whole executable hash. Captured Tcl library .tcl inputs retain before/after pins and exact byte payloads; configure/compiler/link and loaded shared-library closure are unrecorded.. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads.. Dialect: jim.

The original process exits0 with empty stderr. The braced-variable case reports complete=1, script catch=0, before=1 and after=2. The terminal word-weld case reports complete=1, script catch=1, before=MISSING and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### bigip

Status: `not-tested`. Version: not tested. Build: No actual BIG-IP process for these two original script values. Channel: No BIG-IP query/evaluation capture. Dialect: bigip.

No actual BIG-IP response to these exact inputs is retained. Neither the six local processes nor source-only profiles establish BIG-IP/TMM behavior.

## Exact evidence

- `original-probe` (input): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/probe.tcl](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/probe.tcl). SHA-256 `6ed458994510989a4567820460f508e8a5d966f70bfda2db9fa6f37c4dc04e99`. Exact original public query/evaluation/observation script. Each case is reconstructed from its original counted hexadecimal ASCII bytes.
- `original-cases` (input): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/cases.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/cases.json). SHA-256 `c721a0036d1db54e1be3e382a9c3e47b388f0d320b7fd7d3c381bf8ea2fac935`. Exact original two counted byte sources and their whole source hashes; query and reached effects are separate purposes.
- `original-runner` (provider): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/run.py](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/run.py). SHA-256 `f41367789eccb55a09eaeee2aa71cff59a5bf1fec06ff617fcdb93dfa8afb349`. Exact actual CLI command/cwd/environment and original process/stream capture protocol; it is not a C/API compiler or dependency producer.
- `original-results` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/results.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/results.json). SHA-256 `f02a6adbb17643eefe24194a2eef66167e91772c1d35e7de985df1241e97fc5b`. Exact independently closed six process associations and twelve original public ROWs.
- `independent-projection` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/interpretation.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/interpretation.json). SHA-256 `0202983c086e5fbee493cd4775c28fd0acb11ecbcac00386fc4a02d279610170`. Read-only exact receipt/stream/input rejoin and counted twelve-row projection; no additional native process or worker chronology.
- `exact-provider-input-retention` (provider): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/provider-retention.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/provider-retention.json). SHA-256 `34bd7d3a2a3b6decd4cb88616b42f50a530254c25fe55ab0c03cf0314a4a95e6`. Whole invoked ELF joins reuse independently verified canonical payloads by full hash; every captured Tcl-library .tcl input retains exact bytes by whole hash. Shared-library/build/header closure is unrecorded.
- `tcl8.4-receipt-json` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20/receipt.json). SHA-256 `af4357be0f26ac06890af7a6f8b87b67b7b906963a8aaa76070bb0e33beb6555`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.4-stdout` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20/stdout](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20/stdout). SHA-256 `3dab6b4a0b4b1b202669e2cb7b55fa9e607fa5322fe592cf783546add7d59b2e`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.4-stderr` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20/stderr](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.5-receipt-json` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19/receipt.json). SHA-256 `a7ae57200bd3776f8d245fe113b0a7720a97b90f70bac32d1283ac3adf67343d`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.5-stdout` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19/stdout](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19/stdout). SHA-256 `bc3f8144e23fc8d9af647118f5d12e69232b91bf0f940429fbf1f7309ebeaf79`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.5-stderr` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19/stderr](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.6-receipt-json` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18/receipt.json). SHA-256 `c1e7081c1cfe316866bd704454bf6e9bfd58e158f12cd2564ae238ee5e4e2702`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.6-stdout` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18/stdout](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18/stdout). SHA-256 `8e44c1ad43a8f494e090fd4512a8930e4d6ce97578e4ec34cd6428f9b79f53c2`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl8.6-stderr` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18/stderr](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl9.0-receipt-json` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4/receipt.json). SHA-256 `121cb65136619f6ed456d32770396d45b7d0f0dfdf4cc7efc9474e2fd9a36bac`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl9.0-stdout` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4/stdout](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4/stdout). SHA-256 `31cddde13cba6e34e96816c1eacbfe9861823dd2063d0d42e18144229a6282ac`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl9.0-stderr` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4/stderr](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl9.1-receipt-json` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0/receipt.json). SHA-256 `f7a9d621143f5a4cae9986d576946496540c1ea18663adb48a05a0302e1d9d20`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl9.1-stdout` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0/stdout](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0/stdout). SHA-256 `cd19b3f24168a0b1465869fdf0362d0d3e6c42bd04fb6fe7bff65f20457e91c4`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `tcl9.1-stderr` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0/stderr](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `jim-receipt-json` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim/receipt.json](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim/receipt.json). SHA-256 `d3971ed71a1857ea4bfc06d9f6952efcb531b0dd07dea0c00d880143c1c4d44a`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `jim-stdout` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim/stdout](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim/stdout). SHA-256 `3623d567afed1ab206088869754336f67b0b9a07478b77080220a7baaaf9b448`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.
- `jim-stderr` (observation): [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim/stderr](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original closed process receipt/stream for these exact two inputs; no worker/header/source-authority inference.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/run.py"
]
```

The runner requires a fresh output directory and the exact original absolute executable and Tcl-library paths. The retained original runner refuses to replace results.json. Full replay requires restoring every declared input from its exact payload, verifying whole hashes, and supplying a separate output root without altering the measured inputs. Compiler/configure/build, loaded shared-library state and complete environment reproducibility are unrecorded. Replaying creates new observations and never replaces these original receipts.
