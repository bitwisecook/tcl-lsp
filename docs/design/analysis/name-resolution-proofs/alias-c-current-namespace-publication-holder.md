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

Status: `observed`. Version: 8.4.20. Build: Exact CLI executable; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 8.4.20 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact CLI executable; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 8.5.19 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact CLI executable; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 8.6.18 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact CLI executable; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 9.0.4 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact CLI executable; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: C Tcl stock interp alias CLI.

Actual stock 9.1.0 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Exact CLI executable; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Dialect: Jim actual interp interface.

Actual Jim 0.84-9-g5bac7c9 exposes interp but catches each C-style alias operation with its exact wrong-argument error. The three queried alias call names are invalid in this fixed script. Exact rows: relative_qualified 1 {wrong # args: should be "interp"} {} {}; relative_calls 1 {invalid command name "::N::q::a"} 1 {invalid command name "::q::a"}; unqualified 1 {wrong # args: should be "interp"} {} {}; explicitly_rooted 1 {wrong # args: should be "interp"} {} 1 {invalid command name "::N::q::rooted"}. No standalone Jim alias or child-handle operation is substituted; no C-publication or private namespace-holder answer follows. Whole process exits zero with empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: BIG-IP.

No appliance execution answers this exact API/input question; stock C/Jim public outcomes supply no hosted result.

## Historical references

- `alias197-probe.tcl` (input): [rust/tcl-registry/tests/data/native_c_alias_holder197/probe.tcl](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/probe.tcl). Exact original ASCII LF source executed by all six providers.
- `alias197-request.json` (input): [rust/tcl-registry/tests/data/native_c_alias_holder197/request.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/request.json). Unchanged authored human question, required providers and original source/input channel; canonical catalogue ID is assigned independently.
- `alias197-capture.py` (input): [rust/tcl-registry/tests/data/native_c_alias_holder197/capture.py](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/capture.py). Exact Root-executed launcher bytes; original external provider paths, explicit source/executable/artifact checks and recorded process commands. No new launch is claimed.
- `alias197-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/receipt.json). Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stdout). Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stderr). Whole exact empty process stderr.
- `alias197-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/receipt.json). Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stdout). Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stderr). Whole exact empty process stderr.
- `alias197-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/receipt.json). Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stdout). Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stderr). Whole exact empty process stderr.
- `alias197-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/receipt.json). Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stdout). Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stderr). Whole exact empty process stderr.
- `alias197-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/receipt.json). Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stdout). Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stderr). Whole exact empty process stderr.
- `alias197-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_c_alias_holder197/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/receipt.json). Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias197-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stdout](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stdout). Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias197-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stderr). Whole exact empty process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api` (linked): Independent Runtime alias conformance compares the complete original measured public rows after provider-version metadata through the selected supported API; no extra native process, private key/cell or frame identity follows.
- [rust/tcl-vm/tests/native_compilation_conformance.rs](../../../../rust/tcl-vm/tests/native_compilation_conformance.rs), `native_compilation_conformance::alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api` (linked): Full original source and retained provider stdout bind VM public conformance after version metadata; actual assertions require separate validation receipts and do not establish private interpreter storage or API equivalence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole exact commands, streams, environment overrides and required executable/library/header/Makefile/source associations remain in unchanged receipts. Other inherited environment variables are unrecorded. The exact executed launcher depends on original external provider paths and prior receipts, and is not a portable replay promise. Re-running produces new observations. A current source hash association does not prove the executable was built from that source. Publication and proof checking execute no native or Rust process.
