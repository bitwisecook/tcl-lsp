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

Status: `observed`. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Version: tcl8.4 8.4.20.

Actual stock 8.4.20 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl8.5

Status: `observed`. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Version: tcl8.5 8.5.19.

Actual stock 8.5.19 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl8.6

Status: `observed`. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Version: tcl8.6 8.6.18.

Actual stock 8.6.18 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Version: tcl9.0 9.0.4.

Actual stock 9.0.4 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Version: tcl9.1 9.1.0.

Actual stock 9.1.0 records these exact public rows: relative_qualified 0 q::a ::N::q::a {}; relative_calls 0 VALUE 1 {invalid command name "::q::a"}; unqualified 0 plain {} ::plain; explicitly_rooted 0 ::N::q::rooted ::N::q::rooted 0 ROOTED. Relative qualified publication selects the measured N/q name; plain selects the root name; explicit rooted input retains its own public spelling. No private namespace token/table key or lifetime is inspected. Whole process exits zero with empty stderr.

### jim

Status: `unsupported`. Channel: ASCII source file, LF endings, no counted NUL or Unicode; public same-interpreter alias creation and info commands/calls only. Version: jim 0.84-9-g5bac7c9.

Actual Jim 0.84-9-g5bac7c9 exposes interp but catches each C-style alias operation with its exact wrong-argument error. The three queried alias call names are invalid in this fixed script. Exact rows: relative_qualified 1 {wrong # args: should be "interp"} {} {}; relative_calls 1 {invalid command name "::N::q::a"} 1 {invalid command name "::q::a"}; unqualified 1 {wrong # args: should be "interp"} {} {}; explicitly_rooted 1 {wrong # args: should be "interp"} {} 1 {invalid command name "::N::q::rooted"}. No standalone Jim alias or child-handle operation is substituted; no C-publication or private namespace-holder answer follows. Whole process exits zero with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact API/input question; stock C/Jim public outcomes supply no hosted result.

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api` (linked): Independent Runtime alias conformance compares the complete original measured public rows after provider-version metadata through the selected supported API; no extra native process, private key/cell or frame identity follows.
- [rust/tcl-vm/tests/native_compilation_conformance.rs](../../../../rust/tcl-vm/tests/native_compilation_conformance.rs), `native_compilation_conformance::alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api` (linked): Full original source and retained provider stdout bind VM public conformance after version metadata; actual assertions require separate validation receipts and do not establish private interpreter storage or API equivalence.

## Evidence

- [rust/tcl-registry/tests/data/native_c_alias_holder197](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0)
- [rust/tcl-registry/tests/data/native_c_alias_holder197/jim](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim)
