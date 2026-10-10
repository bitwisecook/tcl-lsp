# naming.alias.c-current-namespace-publication-holder

Kind: `native-observation`

## Problem statement

Same-interpreter alias publication can expose different public names for plain, relative-qualified and explicitly rooted input. A global-only installer can lose that distinction, and an incompatible Jim interp interface cannot answer the C API question.

## Question

Does interp alias publication in the same interpreter use the current namespace holder for an unqualified or relative-qualified alias name, or the root holder, and how do explicit root names compare?

## Conclusion

All five recorded stock C releases catch same-interpreter interp alias creation successfully. Under namespace N, original q::a returns q::a and info commands selects ::N::q::a while ::q::a prints empty; the explicit ::N::q::a call returns VALUE with code zero and ::q::a is caught with code one and its exact invalid-command result. Plain alias creation returns plain, exposes ::plain and prints empty for ::N::plain. Explicit ::N::q::rooted returns that spelling, is exposed at that name, and returns ROOTED with code zero. These exact public naming/query/call results distinguish the tested publication contexts without inspecting a private namespace holder. Jim 0.84-9-g5bac7c9 exposes interp but catches each requested C-style alias operation with wrong # args: should be "interp"; subsequent exact calls are caught as invalid commands. The C-style alias scope is unsupported in that process, not replaced by standalone Jim alias or child-handle syntax. All six processes exit zero with empty stderr; BIG-IP is not tested. No private key, namespace token/allocation, alias cell/header, entered frame, compiler prerequisite, source carrier, complete command inventory or edit claim follows.

## Scope

One fixed ASCII LF file per fresh provider process creates namespace N and its child q, then measures same-interpreter alias calls for q::a, plain and ::N::q::rooted. Exact info commands patterns and caught call results are retained. No NUL, Unicode, control escape, arbitrary namespace path, failed namespace creation, missing destination namespace, multiple interpreter or alias lifetime scenario is exercised. This is the actual C interp alias syntax on every provider; Jim incompatible syntax is caught, with no alternative API substituted.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact CLI executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 8.4.20 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact CLI executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 8.5.19 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact CLI executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 8.6.18 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact CLI executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 9.0.4 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact CLI executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 9.1.0 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Exact CLI executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: Jim actual interp interface.

Actual Jim 0.84-9-g5bac7c9 exposes interp but catches each C-style alias operation with its exact wrong-argument error. The three queried alias call names are invalid in this fixed script. Exact rows: relative_qualified 1 {wrong # args: should be "interp"} {} {}; relative_calls 1 {invalid command name "::N::q::a"} 1 {invalid command name "::q::a"}; unqualified 1 {wrong # args: should be "interp"} {} {}; explicitly_rooted 1 {wrong # args: should be "interp"} {} 1 {invalid command name "::N::q::rooted"}. No standalone Jim alias or child-handle operation is substituted; no C-publication or private namespace-holder answer follows. Whole process exits zero with empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: BIG-IP.

No appliance execution answers this exact API/input question; stock C/Jim public outcomes supply no hosted result.

## Exact evidence

- `alias197-probe.tcl` (input): [rust/tcl-registry/tests/data/native_c_alias_holder197/probe.tcl](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/probe.tcl). SHA-256 `94e8c02f0fb2abd9b0f56800072498c2cca8fdee026500d75b2cc58747c13498`. Exact original ASCII LF source executed by all six providers.
- `alias197-request.json` (input): [rust/tcl-registry/tests/data/native_c_alias_holder197/request.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/request.json). SHA-256 `8c02040fe8a3de83c05697e205c6f06b7856b16577d5c8fd44661e4d23da7777`. Unchanged authored human question, required providers and original source/input channel; canonical catalogue ID is assigned independently.
- `alias197-capture.py` (input): [rust/tcl-registry/tests/data/native_c_alias_holder197/capture.py](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/capture.py). SHA-256 `ca7876f13e5f1d7ef72c8ea4cee52943bd2e05afec0a660497597f98b06e424d`. Exact Root-executed launcher bytes; original external provider paths, explicit source/executable/artifact checks and recorded process commands. No new launch is claimed.
- `alias197-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/receipt.json). SHA-256 `0de08bc19821ba7aefbfcb23055bbddd8e279dc512da62dd094a19c875d601e7`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stdout). SHA-256 `e3d4f64cee02c9ac12fbec2cd50a55d4587c078e0b3c572afc4fde69fc0194c5`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias197-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/receipt.json). SHA-256 `0b18657eca342116d3515559e4ec4c431099998abe5d17e3a95a20bc1c57bf14`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stdout). SHA-256 `6f6f5d12c54d8c6662a0fa07159873a019d2fa2f0398f72563e46f58693047bc`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias197-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/receipt.json). SHA-256 `11454fa335866908672e4754f2cf0c412a14cc731780f5655062e005d422aa11`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stdout). SHA-256 `cc8da76f377d9a75a475d09fe50c0c822aa07c3063759de3913ea52838e764f7`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias197-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/receipt.json). SHA-256 `a0d1a984eee39613dc6489f5b086309fab9ee3f923de3c7aa67686e1cbd01223`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stdout). SHA-256 `347b8fd4a96cf479e9944da91d774a7a59c6920777319e9b31d1ab678a55f431`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias197-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/receipt.json). SHA-256 `5053de60f273b6f9d3b470a8d8b22593ea007043f0e7d66bd30b2825a429bc44`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stdout). SHA-256 `cdab969647a4086fd4a91bfb1c478892b28e0d150e2b9cda00a60c6a102d4e84`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias197-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/receipt.json). SHA-256 `a743a7f3e639dc618c52ff3474474085f69f56d80497160b57dadf29c0bd3682`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stdout). SHA-256 `804f79a15afb4bda58fe682b5c0e710ea1a59aaf9919d783206005d57d642da6`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api` (linked): Independent Runtime alias conformance compares the complete original measured public rows after provider-version metadata through the selected supported API; no extra native process, private key/cell or frame identity follows.
- [rust/tcl-vm/tests/native_compilation_conformance.rs](../../../../rust/tcl-vm/tests/native_compilation_conformance.rs), `native_compilation_conformance::alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api` (linked): Full original source and retained provider stdout bind VM public conformance after version metadata; actual assertions require separate validation receipts and do not establish private interpreter storage or API equivalence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole exact commands, streams, environment overrides and required executable/library/header/Makefile/source associations remain in unchanged receipts. Other inherited environment variables are unrecorded. The exact executed launcher depends on original external provider paths and prior receipts, and is not a portable replay promise. Re-running produces new observations. A current source hash association does not prove the executable was built from that source. Publication and proof checking execute no native or Rust process.
