# naming.class.jim-source-rooted-factory-initialiser-replacement

Kind: `native-observation`

## Problem statement

Literal leading roots in class names and two-word procedures require actual public factory/body and object-call controls. Independent alias versus procedure publication recipes cannot establish distinct live comparison slots from stored spellings alone.

## Question

Does the original rooted Jim class factory leave an earlier constructor procedure body inspectable, change an earlier get body, and admit the measured early and later constructor calls?

## Conclusion

Recorded Jim 0.84-9-g5bac7c9 supplies oo package 1.0 and class. Its rooted early constructor row is 0 1 0: class creation catch code zero, info-body catch code one and equality with the earlier body false. The original constructor body is inaccessible through that public query; no alias or command-cell identity is inferred. The early new FIRST call is caught with code one and leaves the recorded constructor observation list empty; this probe does not print the guest error or establish a successful early construction. Rooted early get is 0 0 0, with successful class creation and body query but a changed body. A later raw rooted constructor declaration yields a subsequent call code zero with {{REPLACEMENT SECOND}}. All exact rows are retained. The five stock C installations reject package require oo, expose no class command and mark these class controls not applicable; TclOO absence is not asserted. Six process exits are zero with empty stderr. BIG-IP is not tested. No private alias hash/key, installed cell, Native object/header/frame, successful Compiler source-carrier admission or generic allocation/dispatch claim follows.

## Scope

Original fixed ASCII LF source file with two rooted authored class/procedure names, public caught class and object calls, exact info-body comparison and one later constructor replacement. The earlier constructor observation list is separate from the call result code; the guest error/body contents and generated object values are not printed. No NUL, non-ASCII or multiline/control escape is exercised. Exact public outcomes are independent of conditional Compiler source barriers and pure name publication/comparison recipes. Required executable/library/header/build/source/package pins retain associations, not a compiled-source causal proof.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Exact CLI executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.. Dialect: C Tcl stock CLI.

Actual stock 8.4.20 catches package require oo rejection, exposes no class command and prints class_initialiser_scope not_applicable. All four class controls are unexecuted; no rooted factory/adoption answer or TclOO absence follows. Process exits zero with empty stderr.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Exact CLI executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.. Dialect: C Tcl stock CLI.

Actual stock 8.5.19 catches package require oo rejection, exposes no class command and prints class_initialiser_scope not_applicable. All four class controls are unexecuted; no rooted factory/adoption answer or TclOO absence follows. Process exits zero with empty stderr.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Exact CLI executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.. Dialect: C Tcl stock CLI.

Actual stock 8.6.18 catches package require oo rejection, exposes no class command and prints class_initialiser_scope not_applicable. All four class controls are unexecuted; no rooted factory/adoption answer or TclOO absence follows. Process exits zero with empty stderr.

### tcl9.0

Status: `unsupported`. Version: 9.0.4. Build: Exact CLI executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.. Dialect: C Tcl stock CLI.

Actual stock 9.0.4 catches package require oo rejection, exposes no class command and prints class_initialiser_scope not_applicable. All four class controls are unexecuted; no rooted factory/adoption answer or TclOO absence follows. Process exits zero with empty stderr.

### tcl9.1

Status: `unsupported`. Version: 9.1.0. Build: Exact CLI executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.. Dialect: C Tcl stock CLI.

Actual stock 9.1.0 catches package require oo rejection, exposes no class command and prints class_initialiser_scope not_applicable. All four class controls are unexecuted; no rooted factory/adoption answer or TclOO absence follows. Process exits zero with empty stderr.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact CLI executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; every required artifact independently rechecked and retained. No fresh SDK build or compiler version recorded.. Channel: Exact ASCII LF CLI script file; public caught class/object calls and info-body equality. No object names/addresses or body contents printed. Generated two-word names retain literal leading ::; no NUL, non-ASCII or multiline escapes.. Dialect: Jim Tcl.

Actual Jim 0.84-9-g5bac7c9 has oo1.0/class. Exact public rows: rooted_early_constructor 0 1 0; rooted_early_constructor_call 1 {}; rooted_early_get 0 0 0; rooted_later_constructor_call 0 {{REPLACEMENT SECOND}}. Initial constructor body query fails with code1/equalityfalse; the earlier new call fails with no constructor observations. Rooted get body changes; only the later replacement call succeeds with REPLACEMENT SECOND. No alias comparison slot or native object/cell identity is inferred.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: BIG-IP.

No appliance execution answers this exact original input question; stock C and Jim results supply no hosted result.

## Exact evidence

- `rooted275-probe.tcl` (input): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/probe.tcl). SHA-256 `d007bd69fd83374e310ed8d770a01fad6fc76153f40d864c4a064dc2aeaf97db`. Exact original ASCII LF source file executed by all six providers.
- `rooted275-request.json` (input): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/request.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/request.json). SHA-256 `527c7fab37f33a05b6371fd258c56286c7e0702f65c861ad35376a2d5eee25d3`. Exact original authored question/provider/source request and explicit authority boundary.
- `rooted275-capture.py` (input): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/capture.py). SHA-256 `9f4af9a942e17be9461cfce0f78d016188392dfe0bf28eafd7894eb617298051`. Exact Root-executed native launcher; original commands/environment/executable/artifact checks, no new run.
- `rooted275-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.4.20/receipt.json). SHA-256 `e85c36664031794406c769a99cd24c8db5bed68fbdefc8bbba4fb667b1f890d8`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `rooted275-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.4.20/stdout). SHA-256 `f1e97df0196c409de743db388b4f12cb5fc00489afb24b309bc206b9949e825d`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `rooted275-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `rooted275-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.5.19/receipt.json). SHA-256 `8a3647591a43faedc755512f485eea21f7a7769a1d7bbc46d90463676bcf51b2`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `rooted275-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.5.19/stdout). SHA-256 `f0f072562c13647b89b29934080eb8eae9110c62937ffcbd04109f045e7ed696`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `rooted275-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `rooted275-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.6.18/receipt.json). SHA-256 `6dc532bb3d8158f91913050c66dbeb4e77e86d42d0d035e1cae7e5342faa96de`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `rooted275-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.6.18/stdout). SHA-256 `3795d9ccc78e324cee2348e20691899bfefcb93be79de60e48f753aa8d1f6022`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `rooted275-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `rooted275-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.0.4/receipt.json). SHA-256 `385a2cb12c08fe3bc2ab7dc371623bfb9ebfcf32e4a885c431dc3c92be0be6ee`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `rooted275-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.0.4/stdout). SHA-256 `908bb58d35176d7112afd3374b892295620ce38f2513f05d1cb6cf04879469e6`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `rooted275-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `rooted275-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.1.0/receipt.json). SHA-256 `9da78cba53b4d246e4b25f8c73e1e6445668911ea3e49b2e88ad476a5a989180`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `rooted275-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.1.0/stdout). SHA-256 `6ee7790645d6e9bd8038d1191ee6f51978706c1efc81edea61b8ac07f5ae83ce`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `rooted275-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `rooted275-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/jim/receipt.json). SHA-256 `53a02a432b2ffc28e25e007639f3eff4822cf6deb3b4b6b33a6693b1aad5bb44`. Exact actual process/version/environment and complete executable/required-artifact SHA associations.
- `rooted275-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/jim/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/jim/stdout). SHA-256 `9e9eb57441f1b31b86072b6232a8ecd96c9c80ba883c08bc0ab4bdf7952559f7`. Whole original stdout; exact caught codes, body/query outcomes and value spellings retained.
- `rooted275-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/jim/stderr](../../../../rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class_reference.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class_reference.rs), `command_binding::source_transition_advice::source_class_reference::tests::original_class_factory_initialisers_keep_alias_and_procedure_publication_distinct` (linked): Independent source barriers withhold earlier rooted constructor/get headers under selected factory initialisers; no successful object call, private alias slot or extra native input follows.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::alias_publication_retains_its_distinct_native_entry_point` (linked): Pure native-name purpose coverage retains alias original spelling and shared counted root comparison; C/Jim/NUL unit cases are implementation coverage, not extra observations of the ASCII rooted factory probe.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original commands, environment overrides, exact sources/whole streams and required executable/SDK/source pins remain unchanged. Other inherited environment variables are unrecorded. The actual launcher depends on retained earlier native receipts and external SDK paths and refuses existing output directories. Its retained bytes and scratch paths do not promise portable replay; re-running creates new observations. No native or Rust process is launched by publication.
