# naming.expression.braced-static-literal-completion

Kind: `native-observation`

## Problem statement

Changing the original word delimiters can change the value supplied to expression evaluation; successful literal controls must be measured in each actual source channel.

## Question

For the seven retained static literal pairs, what completion and result does each actual provider return before and after bracing?

## Conclusion

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked library SHA256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47; original executable SHA256 598d9b72ed8e4c235656b9f9e5223f2b7859085fc4e5e0f11d10f7dac108a029; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked library SHA256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; original executable SHA256 f9949823402f6f9661c4da4c9eb8c713a789eae596a37434d2ceaafb5c2396e4; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked library SHA256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; original executable SHA256 fe6291baefd99c21fe25d5ee33342738be2f88facc51e649e7981ac073237eb0; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked library SHA256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; original executable SHA256 2a54ce88d18b84c1b97637c5d2ce83f6bdf8e37ad153325fc72214fe2e780f1c; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked library SHA256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; original executable SHA256 e4f14fe01bbedb3f521c7a1c06f1f80ffdf5fe1b2a50d2633983e5d2b3fbf4ee; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked library SHA256 a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da; original executable SHA256 3a4fbb7ce3c3a54e0ff42e3c633e794a12af62a696653f6465f2b3c12eb3f6ca; exact header/Makefile/source/compile command in receipt.. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context.. Dialect: Jim Tcl.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `inputs` (input): [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/inputs.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/inputs.json). JSON pointer `/cases`. Exact original/candidate native source and separate authored collector bytes; case claim_scope is input intent, not a result.
- `probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/probe.c). Exact API scheduling, original-counted input, public channel read, getter chronology and unavailable branches.
- `named-row-audit` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json). Offline named-field projection, original line coordinates and pair differences; no fresh execution.
- `v1-probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol/probe.c). Exact earlier probe source; original attempt APIs and abort behavior are preserved.
- `v1-aggregate` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/receipt.json). Independent earlier partial capture; its recorded provider/process coverage is never loaned to later rows.
- `v3-probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol/probe.c). Exact earlier probe source; original attempt APIs and abort behavior are preserved.
- `v3-aggregate` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/receipt.json). Independent earlier partial capture; its recorded provider/process coverage is never loaned to later rows.
- `tcl8.4-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/stdout.tsv). Exact captured original rows for this provider; finite cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim.
- `tcl8.4-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.4-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.4.20/receipt.json). Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.4-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.4.20/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.5-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/stdout.tsv). Exact captured original rows for this provider; finite cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim.
- `tcl8.5-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.5-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.5.19/receipt.json). Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.5-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.5.19/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.6-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/stdout.tsv). Exact captured original rows for this provider; finite cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim.
- `tcl8.6-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.6-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.6.18/receipt.json). Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.6-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.6.18/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl9.0-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/stdout.tsv). Exact captured original rows for this provider; finite cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim.
- `tcl9.0-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl9.0-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/9.0.4/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl9.1-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/stdout.tsv). Exact captured original rows for this provider; finite cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim.
- `tcl9.1-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `jim-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/stdout.tsv). Exact captured original rows for this provider; finite cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim.
- `jim-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.

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
  "rust/tcl-compiler/tests/data/native_brace_expression/replay.py",
  "--c-root",
  "/workspace/tcl-lsp/tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "<new-output-directory>"
]
```

Replay reproduces v4 only with original pinned headers/Makefiles/libraries/sources and exact streams. Earlier failed/partial attempts remain immutable limitations. Public result getters may materialise strings after the recorded primary/resident sample. No optimiser or Rust test is executed by this protocol.
