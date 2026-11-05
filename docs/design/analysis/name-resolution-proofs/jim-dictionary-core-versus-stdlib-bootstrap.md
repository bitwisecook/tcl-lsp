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

Status: `not-tested`.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl8.4 API/CLI/appliance observation answers this question.

### tcl8.5

Status: `not-tested`.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl8.5 API/CLI/appliance observation answers this question.

### tcl8.6

Status: `not-tested`.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl8.6 API/CLI/appliance observation answers this question.

### tcl9.0

Status: `not-tested`.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl9.0 API/CLI/appliance observation answers this question.

### tcl9.1

Status: `not-tested`.

This exact probe selects Jim API core and Jim_stdlibInit; no tcl9.1 API/CLI/appliance observation answers this question.

### jim

Status: `observed`. Channel: Compiled C API probe: original Jim_CreateInterp then Jim_RegisterCoreCommands, original Jim_Eval ASCII source, explicit Jim_stdlibInit, same original Jim_Eval ASCII source. Each result reports guest code and counted hex bytes; initialization return is distinct. No full jimsh/optional extension roster or private frame/header claim. Version: jim 0.84-9-g5bac7c9 (independent pinned SDK/CLI association).

Compiler and probe exit0 with empty stderr. Original hex result rows decode to core: {} 1 {invalid command name "dict update"} BEFORE 0 {first NEW}; explicit stdlib initialization returns0; stdlib: {} 0 1 NEW 1 {first NEW}. Both Jim_Eval codes are0 because guest worker calls are caught. Worker invocation/caller target/body fields distinguish the two states; info commands returns{} in both and cannot establish absence. Exact version is independently associated from CLI/library pins, not queried by this probe. No complete shell/extension roster or private frame/header mechanism is claimed.

### bigip

Status: `not-tested`.

This exact probe selects Jim API core and Jim_stdlibInit; no bigip API/CLI/appliance observation answers this question.

## Consumer bindings

- [runtime/rust/src/cmd_array.rs](../../../../runtime/rust/src/cmd_array.rs), `cmd_array::tests::jim_dictionary_worker_requires_explicit_distribution_initialisation` (linked): Explicit core ingress catches the missing worker and retains BEFORE/body-zero; shared distribution installation admits the same caller-value projection. The separate enumeration field supplies no binding inference.
- [rust/tcl-vm/src/cmd_dict/scripted_tests.rs](../../../../rust/tcl-vm/src/cmd_dict/scripted_tests.rs), `cmd_dict::scripted_tests::native_jim_dictionary_worker_requires_explicit_distribution_initialisation` (linked): VM same-interpreter core-before/shared-installer-after control compares exact selected invocation/caller-value fields of original Native191 source. Equal empty info commands enumeration is excluded from availability inference; no executed Rust result is claimed.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/probe.c](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/probe.c)
- [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stdout](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stdout)
- [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stderr](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/compile.stderr)
- [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stdout](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stdout)
- [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stderr](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/execute.stderr)
- [rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191/capture.py)
