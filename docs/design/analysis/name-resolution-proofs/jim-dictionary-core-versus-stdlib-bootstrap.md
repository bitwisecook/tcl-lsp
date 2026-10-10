# naming.jim.dictionary-core-versus-stdlib-bootstrap

Kind: `native-observation`

## Problem statement

Jim core API registration and its distribution scripted workers are separate ingress states. A shell transcript or an empty info-commands result cannot answer whether an actual caught dictionary worker invocation reaches the body before or after explicit audited initialization.

## Question

Does current Jim core registration itself install the scripted dictionary worker, and how does explicit audited stdlib initialization affect the same public caller-mapping program?

## Conclusion

The retained compiled Jim API probe creates one interpreter, registers core commands, runs the original public mapping source, explicitly calls Jim_stdlibInit, then runs the same source again. Both Jim_Eval result codes are zero because the guest operation is caught. Before initialization the decoded result is {} 1 {invalid command name "dict update"} BEFORE 0 {first NEW}; the caught worker call fails and the body is unentered. Jim_stdlibInit returns zero. After initialization the decoded result is {} 0 1 NEW 1 {first NEW}; the worker call completes, updates the caller target and enters the body. info commands {dict update} returns {} in both result lists, so enumeration supplies no command absence or installation conclusion. Exact counted-result hex bytes, compile/run commands and whole streams are retained. The compiler and probe processes both exit zero with empty stderr; compilation stdout is empty. Jim0.84-9-g5bac7c9 is independently associated through the exact prior CLI/library/header pins, not a version query printed by this C process. All C providers and BIG-IP are not tested for this Jim API question. No full jimsh/extension roster, internal frame/header/argv/cache identity, arbitrary stdlib loading equivalence, successful Runtime parity or Compiler authority follows.

## Scope

Original fixed ASCII C99 program links the pinned static Jim library and invokes Jim_CreateInterp/Jim_RegisterCoreCommands/Jim_Eval/Jim_stdlibInit on the same interpreter. The public result getter is emitted as hex bytes with its returned count; it exercises no input NUL/non-ASCII/control escape or counted-versus-CString comparison. Initialization return and each result code/value tuple remain distinct. Exact C source, built executable and compile/run receipts/streams are retained; the same original source resets its authored variables before each call. Compiler executable/version and dynamically linked system library versions/digests are unrecorded, so the retained command is not a reproducible-build or ABI equivalence claim. Required Jim source/build/header/library associations do not alone explain compiled implementation behaviour. No other API provider or extension bootstrap is measured.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl8.4 API/CLI/appliance observation answers this question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl8.5 API/CLI/appliance observation answers this question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl8.6 API/CLI/appliance observation answers this question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl9.0 API/CLI/appliance observation answers this question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl9.1 API/CLI/appliance observation answers this question.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9 (independent pinned SDK/CLI association). Build: Compiled probe executable SHA-256 010d4349935240584eeac37a237bdd2907250fe78a72d47412c10f4485d3de1b links the exact retained Jim static library/header/source/build pins. Compiler/system dynamic library versions are unrecorded.. Channel: Compiled C API probe: original Jim_CreateInterp then Jim_RegisterCoreCommands, original Jim_Eval ASCII source, explicit Jim_stdlibInit, same original Jim_Eval ASCII source. Each result reports guest code and counted hex bytes; initialization return is distinct. No full jimsh/optional extension roster or private frame/header claim.. Dialect: Jim C API core registration plus explicit audited stdlib initialization.

Compiler and probe exit0 with empty stderr. Original hex result rows decode to core: {} 1 {invalid command name "dict update"} BEFORE 0 {first NEW}; explicit stdlib initialization returns0; stdlib: {} 0 1 NEW 1 {first NEW}. Both Jim_Eval codes are0 because guest worker calls are caught. Worker invocation/caller target/body fields distinguish the two states; info commands returns{} in both and cannot establish absence. Exact version is independently associated from CLI/library pins, not queried by this probe. No complete shell/extension roster or private frame/header mechanism is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This exact probe selects Jim API core and Jim_stdlibInit; no bigip API/CLI/appliance observation answers this question.

## Exact evidence

- `bootstrap191-request.json` (input): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/request.json](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/request.json). SHA-256 `544d3db678ff5daedf7faee1e8434b0e7bce9ff1a8f3db942ed386a6d1c73c2e`. Exact original compiled-API input/artifact/receipt/whole stream for request.json; no rewriting or new process.
- `bootstrap191-probe.c` (input): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/probe.c). SHA-256 `e74ee5e45b0b9f4f30bfaac4521339b2728fdb466c52fb62682be7b467760b8f`. Exact original compiled-API input/artifact/receipt/whole stream for probe.c; no rewriting or new process.
- `bootstrap191-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.receipt.json). SHA-256 `a63171a57d37064083f194f19b504fcf37dad2e2bd0f5e025e4e7b687eaaacdf`. Exact original compiled-API input/artifact/receipt/whole stream for compile.receipt.json; no rewriting or new process.
- `bootstrap191-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiled-API input/artifact/receipt/whole stream for compile.stdout; no rewriting or new process.
- `bootstrap191-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiled-API input/artifact/receipt/whole stream for compile.stderr; no rewriting or new process.
- `bootstrap191-execute.receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.receipt.json). SHA-256 `4771e05fcbbe79c89a79a72091bfac5739250b50bc48e2ff6372024ce8a413a5`. Exact original compiled-API input/artifact/receipt/whole stream for execute.receipt.json; no rewriting or new process.
- `bootstrap191-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stdout](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stdout). SHA-256 `e14c0a4e1e24b4883b3f4e40b39319c9fb04bf6fcf90652372d710110d41b617`. Exact original compiled-API input/artifact/receipt/whole stream for execute.stdout; no rewriting or new process.
- `bootstrap191-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stderr](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiled-API input/artifact/receipt/whole stream for execute.stderr; no rewriting or new process.
- `bootstrap191-probe.elf` (provider): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/probe.elf](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/probe.elf). SHA-256 `010d4349935240584eeac37a237bdd2907250fe78a72d47412c10f4485d3de1b`. Exact original compiled-API input/artifact/receipt/whole stream for probe.elf; no rewriting or new process.
- `bootstrap191-capture.py` (input): [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/capture.py). SHA-256 `82f8489be4c698c43709ba489ec30d40796731f67128166098185ba1d228257c`. Exact Root-executed C compiler/probe launcher with pinned Jim associations and full original streams.
- `bootstrap191-version-pin` (provider): [rust/tcl-registry/tests/data/native_jim_class_initialisers271/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_initialisers271/jim/receipt.json). SHA-256 `ed3e08891bafdf1de830c539a445598c02c083f8e9bbc9119fd47ea6f28c644b`. Exact independent current Jim CLI reported-version association and shared static library/header pins; the API probe does not query its own version.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_array.rs](../../../../runtime/rust/src/cmd_array.rs), `cmd_array::tests::jim_dictionary_worker_requires_explicit_distribution_initialisation` (linked): Explicit core ingress catches the missing worker and retains BEFORE/body-zero; shared distribution installation admits the same caller-value projection. The separate enumeration field supplies no binding inference.
- [rust/tcl-vm/src/cmd_dict/scripted_tests.rs](../../../../rust/tcl-vm/src/cmd_dict/scripted_tests.rs), `cmd_dict::scripted_tests::native_jim_dictionary_worker_requires_explicit_distribution_initialisation` (linked): VM same-interpreter core-before/shared-installer-after control compares exact selected invocation/caller-value fields of original Native191 source. Equal empty info commands enumeration is excluded from availability inference; no executed Rust result is claimed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original compile and execution commands, environment overrides, source/executable digests and whole streams remain unchanged. Other inherited environment variables are unrecorded. The actual launcher depends on original current Jim SDK/CLI receipts and output paths; rerunning produces new observations. Retaining the compiled executable is an exact artifact, not a promise of portability. Publication launches no compiler/native/Rust process.
